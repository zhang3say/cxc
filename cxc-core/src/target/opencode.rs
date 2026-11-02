use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};

use crate::agent::{AgentConfig, AgentKind};
use crate::config::Env;
use crate::protocol::Protocol;
use crate::supplier::{Supplier, SupplierKey};

use super::{
    AgentAdapter, TargetError, backup, provider_key, read_to_string, resolve_config_dir,
    write_secure_file,
};

pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn npm(protocol: Protocol) -> &'static str {
        match protocol {
            Protocol::ChatCompletions => "@ai-sdk/openai-compatible",
            Protocol::Messages => "@ai-sdk/anthropic",
            Protocol::Responses => "@ai-sdk/openai",
        }
    }
}

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for OpenCodeAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::OpenCode
    }

    fn config_path(&self, env: Env, custom_dir: Option<&Path>) -> Result<PathBuf, TargetError> {
        Ok(resolve_config_dir(AgentKind::OpenCode, env, custom_dir)?.join("opencode.json"))
    }

    fn apply(
        &self,
        supplier: &Supplier,
        account: &SupplierKey,
        config: &AgentConfig,
        custom_dir: Option<&Path>,
        env: Env,
    ) -> Result<(), TargetError> {
        let dir = resolve_config_dir(AgentKind::OpenCode, env, custom_dir)?;
        fs::create_dir_all(&dir).map_err(|error| TargetError::WriteError {
            path: dir.clone(),
            source: error,
        })?;
        let config_path = dir.join("opencode.json");
        backup(&config_path)?;

        let mut doc: Value = if config_path.exists() {
            let data = read_to_string(&config_path)?;
            serde_json::from_str(&data).map_err(|error| {
                TargetError::ParseError(format!("opencode.json 解析失败: {error}"))
            })?
        } else {
            json!({ "$schema": "https://opencode.ai/config.json" })
        };

        if !doc.is_object() {
            doc = json!({ "$schema": "https://opencode.ai/config.json" });
        }
        if !doc.get("provider").map(Value::is_object).unwrap_or(false) {
            doc["provider"] = json!({});
        }

        let key = provider_key(supplier);
        let model = config.model.trim();
        let mut model_config = Map::new();
        model_config.insert("name".to_string(), json!(model));

        if let Some(tokens) = config.context_tokens() {
            model_config.insert("limit".to_string(), json!({ "context": tokens }));
        }
        let mut options = Map::new();
        if let Some(level) = config.reasoning_effort_level() {
            options.insert("reasoningEffort".to_string(), json!(level));
        }
        if config.protocol == Protocol::Responses {
            options.insert("useResponses".to_string(), json!(true));
        }
        if !options.is_empty() {
            model_config.insert("options".to_string(), Value::Object(options));
        }

        let mut models = Map::new();
        models.insert(model.to_string(), Value::Object(model_config));

        let provider = json!({
            "npm": Self::npm(config.protocol),
            "name": supplier.name,
            "options": {
                "baseURL": supplier.base_url,
                "apiKey": account.api_key
            },
            "models": Value::Object(models)
        });
        doc["provider"][&key] = provider;

        doc["model"] = json!(format!("{key}/{model}"));

        if let (Some(tokens), Some(ratio)) =
            (config.context_tokens(), config.compact_ratio_percent())
        {
            let reserved = (tokens.saturating_mul(u64::from(100 - ratio)) / 100).max(1_000);
            doc["compaction"] = json!({
                "auto": true,
                "reserved": reserved
            });
        } else if let Some(compaction) = doc.get_mut("compaction").and_then(Value::as_object_mut) {
            compaction.remove("reserved");
        }

        let output = serde_json::to_string_pretty(&doc)
            .map_err(|error| TargetError::SerializeError(error.to_string()))?;
        write_secure_file(&config_path, output.as_bytes()).map_err(|error| {
            TargetError::WriteError {
                path: config_path.clone(),
                source: error,
            }
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Protocol;
    use crate::supplier::{Supplier, SupplierKey};
    use tempfile::TempDir;

    fn key() -> SupplierKey {
        SupplierKey {
            id: "k1".to_string(),
            name: "默认".to_string(),
            api_key: "sk-test".to_string(),
            remark: None,
        }
    }

    fn supplier() -> Supplier {
        Supplier {
            id: "abcdef1234567890".to_string(),
            name: "relay".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            keys: vec![key()],
            default_key_id: Some("k1".to_string()),
            protocols: vec![Protocol::ChatCompletions],
            remark: None,
            last_probe: None,
            models: vec![],
        }
    }

    fn config() -> AgentConfig {
        AgentConfig {
            id: "b1".to_string(),
            supplier_id: "abcdef1234567890".to_string(),
            key_id: None,
            protocol: Protocol::ChatCompletions,
            model: "gpt-5".to_string(),
            reasoning_effort: Some("high".to_string()),
            plan_reasoning_effort: None,
            context_length: Some("200k".to_string()),
            compact_ratio: Some(80),
            claude_models: None,
            grok_web_search: None,
        }
    }

    #[test]
    fn writes_provider_and_preserves_other_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("opencode.json"),
            r#"{"$schema":"https://opencode.ai/config.json","mcp":{},"provider":{"other":{"npm":"x"}}}"#,
        )
        .unwrap();
        OpenCodeAdapter::new()
            .apply(&supplier(), &key(), &config(), Some(dir.path()), Env::Wsl)
            .unwrap();

        let config: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("opencode.json")).unwrap())
                .unwrap();
        assert_eq!(config["mcp"], json!({}));
        assert_eq!(config["provider"]["other"]["npm"], "x");
        assert_eq!(
            config["provider"]["cxc-abcdef12"]["npm"],
            "@ai-sdk/openai-compatible"
        );
        assert_eq!(
            config["provider"]["cxc-abcdef12"]["options"]["baseURL"],
            "https://api.example.com/v1"
        );
        assert_eq!(
            config["provider"]["cxc-abcdef12"]["models"]["gpt-5"]["limit"]["context"],
            200_000
        );
        assert_eq!(
            config["provider"]["cxc-abcdef12"]["models"]["gpt-5"]["options"]["reasoningEffort"],
            "high"
        );
        assert_eq!(config["model"], "cxc-abcdef12/gpt-5");
        assert_eq!(config["compaction"]["auto"], true);
        assert_eq!(config["compaction"]["reserved"], 40_000);
    }

    #[test]
    fn maps_messages_to_anthropic_provider() {
        let dir = TempDir::new().unwrap();
        let mut config = config();
        config.protocol = Protocol::Messages;
        OpenCodeAdapter::new()
            .apply(&supplier(), &key(), &config, Some(dir.path()), Env::Wsl)
            .unwrap();
        let config: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("opencode.json")).unwrap())
                .unwrap();
        assert_eq!(
            config["provider"]["cxc-abcdef12"]["npm"],
            "@ai-sdk/anthropic"
        );
    }
}
