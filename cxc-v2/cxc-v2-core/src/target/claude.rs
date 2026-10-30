use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

use crate::agent::{AgentConfig, AgentKind, ClaudeModels, claude_model_for_context};
use crate::config::Env;
use crate::supplier::Supplier;

use super::{
    AgentAdapter, TargetError, backup, read_to_string, resolve_config_dir, write_secure_file,
};

pub struct ClaudeAdapter;

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for ClaudeAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Claude
    }

    fn config_path(&self, env: Env, custom_dir: Option<&Path>) -> Result<PathBuf, TargetError> {
        Ok(resolve_config_dir(AgentKind::Claude, env, custom_dir)?.join("settings.json"))
    }

    fn apply(
        &self,
        supplier: &Supplier,
        config: &AgentConfig,
        custom_dir: Option<&Path>,
        env: Env,
    ) -> Result<(), TargetError> {
        if config.protocol != crate::protocol::Protocol::Messages {
            return Err(TargetError::ParseError(
                "Claude 只支持 Anthropic Messages 协议".to_string(),
            ));
        }

        let dir = resolve_config_dir(AgentKind::Claude, env, custom_dir)?;
        fs::create_dir_all(&dir).map_err(|error| TargetError::WriteError {
            path: dir.clone(),
            source: error,
        })?;
        let settings_path = dir.join("settings.json");
        backup(&settings_path)?;

        let mut settings: Value = if settings_path.exists() {
            let data = read_to_string(&settings_path)?;
            serde_json::from_str(&data).map_err(|error| {
                TargetError::ParseError(format!("settings.json 解析失败: {error}"))
            })?
        } else {
            json!({})
        };

        if !settings.get("env").map(Value::is_object).unwrap_or(false) {
            settings["env"] = json!({});
        }
        let env_obj = settings["env"].as_object_mut().unwrap();

        env_obj.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            json!(supplier.base_url.trim()),
        );
        env_obj.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            json!(supplier.api_key.trim()),
        );
        env_obj.remove("ANTHROPIC_API_KEY");

        let context = config.context_tokens();
        let main = config.model.trim();
        let opus = tier_model(
            config.claude_models.as_ref(),
            |models| models.opus.as_ref(),
            main,
            context,
        );
        let sonnet = tier_model(
            config.claude_models.as_ref(),
            |models| models.sonnet.as_ref(),
            main,
            context,
        );
        let haiku = tier_model(
            config.claude_models.as_ref(),
            |models| models.haiku.as_ref(),
            main,
            context,
        );
        let subagent = tier_model(
            config.claude_models.as_ref(),
            |models| models.subagent.as_ref(),
            main,
            context,
        );

        env_obj.insert(
            "ANTHROPIC_MODEL".to_string(),
            json!(claude_model_for_context(main, context)),
        );
        env_obj.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), json!(opus));
        env_obj.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), json!(sonnet));
        env_obj.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), json!(haiku));
        env_obj.insert(
            "ANTHROPIC_DEFAULT_FABLE_MODEL".to_string(),
            json!(claude_model_for_context(main, context)),
        );
        env_obj.insert("CLAUDE_CODE_SUBAGENT_MODEL".to_string(), json!(subagent));

        apply_reasoning_effort(&mut settings, config);
        apply_compact_window(&mut settings, config);

        let output = serde_json::to_string_pretty(&settings)
            .map_err(|error| TargetError::SerializeError(error.to_string()))?;
        write_secure_file(&settings_path, output.as_bytes()).map_err(|error| {
            TargetError::WriteError {
                path: settings_path.clone(),
                source: error,
            }
        })?;
        Ok(())
    }
}

fn tier_model(
    models: Option<&ClaudeModels>,
    pick: impl Fn(&ClaudeModels) -> Option<&String>,
    main: &str,
    context: Option<u64>,
) -> String {
    let explicit = models
        .and_then(pick)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    claude_model_for_context(explicit.unwrap_or(main), context)
}

fn apply_compact_window(settings: &mut Value, config: &AgentConfig) {
    let Some(env_obj) = settings.get_mut("env").and_then(Value::as_object_mut) else {
        return;
    };
    if let Some(tokens) = config.compact_token_limit() {
        env_obj.insert(
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW".to_string(),
            json!(tokens.to_string()),
        );
    } else {
        env_obj.remove("CLAUDE_CODE_AUTO_COMPACT_WINDOW");
    }
}

fn apply_reasoning_effort(settings: &mut Value, config: &AgentConfig) {
    let Some(level) = config.reasoning_effort_level() else {
        if let Some(object) = settings.as_object_mut() {
            object.remove("effortLevel");
        }
        if let Some(env_obj) = settings.get_mut("env").and_then(Value::as_object_mut) {
            env_obj.remove("CLAUDE_CODE_EFFORT_LEVEL");
        }
        return;
    };

    if let Some(object) = settings.as_object_mut() {
        object.remove("effortLevel");
    }
    if let Some(env_obj) = settings.get_mut("env").and_then(Value::as_object_mut) {
        env_obj.insert("CLAUDE_CODE_EFFORT_LEVEL".to_string(), json!(level));
    }
    if let Some(models) = settings
        .get_mut("modelSettings")
        .and_then(Value::as_object_mut)
    {
        for entry in models.values_mut() {
            if let Some(object) = entry.as_object_mut() {
                object.remove("effortLevel");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::ClaudeModels;
    use crate::protocol::Protocol;
    use crate::supplier::Supplier;
    use tempfile::TempDir;

    fn supplier() -> Supplier {
        Supplier {
            id: "abcdef1234567890".to_string(),
            name: "relay".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "sk-ant-test".to_string(),
            protocols: vec![Protocol::Messages],
            remark: None,
            last_probe: None,
            models: vec![],
        }
    }

    fn config() -> AgentConfig {
        AgentConfig {
            supplier_id: "abcdef1234567890".to_string(),
            protocol: Protocol::Messages,
            model: "claude-sonnet".to_string(),
            reasoning_effort: Some("high".to_string()),
            plan_reasoning_effort: None,
            context_length: Some("1m".to_string()),
            compact_ratio: Some(80),
            claude_models: Some(ClaudeModels {
                opus: Some("claude-opus".to_string()),
                sonnet: None,
                haiku: None,
                subagent: Some("claude-sub".to_string()),
            }),
            grok_web_search: None,
        }
    }

    #[test]
    fn writes_env_and_preserves_other_keys() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("settings.json"),
            r#"{"theme":"dark","env":{"KEEP":"1"},"modelSettings":{"x":{"effortLevel":"low","maxEffortLevel":"high"}}}"#,
        )
        .unwrap();
        ClaudeAdapter::new()
            .apply(&supplier(), &config(), Some(dir.path()), Env::Wsl)
            .unwrap();

        let settings: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(settings["theme"], "dark");
        assert_eq!(settings["env"]["KEEP"], "1");
        assert_eq!(
            settings["env"]["ANTHROPIC_BASE_URL"],
            "https://api.example.com/v1"
        );
        assert_eq!(settings["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ant-test");
        assert_eq!(settings["env"]["ANTHROPIC_MODEL"], "claude-sonnet[1m]");
        assert_eq!(
            settings["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"],
            "claude-opus[1m]"
        );
        assert_eq!(
            settings["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"],
            "claude-sonnet[1m]"
        );
        assert_eq!(
            settings["env"]["CLAUDE_CODE_SUBAGENT_MODEL"],
            "claude-sub[1m]"
        );
        assert_eq!(settings["env"]["CLAUDE_CODE_EFFORT_LEVEL"], "high");
        assert_eq!(settings["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"], "800000");
        assert!(settings.get("effortLevel").is_none());
        assert!(settings["modelSettings"]["x"].get("effortLevel").is_none());
        assert_eq!(settings["modelSettings"]["x"]["maxEffortLevel"], "high");
    }

    #[test]
    fn rejects_non_messages_protocol() {
        let dir = TempDir::new().unwrap();
        let mut config = config();
        config.protocol = Protocol::Responses;
        assert!(
            ClaudeAdapter::new()
                .apply(&supplier(), &config, Some(dir.path()), Env::Wsl)
                .is_err()
        );
    }
}
