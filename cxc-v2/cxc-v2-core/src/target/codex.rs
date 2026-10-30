use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table, Value as TomlValue};

use crate::agent::{AgentConfig, AgentKind};
use crate::config::Env;
use crate::protocol::Protocol;
use crate::supplier::Supplier;

use super::{
    AgentAdapter, TargetError, backup, provider_key, read_to_string, resolve_config_dir,
    write_secure_file,
};

pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }

    fn wire_api(protocol: Protocol) -> Result<&'static str, TargetError> {
        match protocol {
            Protocol::Responses => Ok("responses"),
            Protocol::ChatCompletions => Ok("chat"),
            Protocol::Messages => Err(TargetError::ParseError(
                "Codex 不支持 Anthropic Messages 协议".to_string(),
            )),
        }
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for CodexAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn config_path(&self, env: Env, custom_dir: Option<&Path>) -> Result<PathBuf, TargetError> {
        Ok(resolve_config_dir(AgentKind::Codex, env, custom_dir)?.join("config.toml"))
    }

    fn apply(
        &self,
        supplier: &Supplier,
        config: &AgentConfig,
        custom_dir: Option<&Path>,
        env: Env,
    ) -> Result<(), TargetError> {
        let dir = resolve_config_dir(AgentKind::Codex, env, custom_dir)?;
        fs::create_dir_all(&dir).map_err(|error| TargetError::WriteError {
            path: dir.clone(),
            source: error,
        })?;

        let config_path = dir.join("config.toml");
        let auth_path = dir.join("auth.json");
        backup(&config_path)?;
        backup(&auth_path)?;

        let toml_data = if config_path.exists() {
            read_to_string(&config_path)?
        } else {
            String::from("model_provider = \"codex\"\nmodel = \"\"\n")
        };
        let mut doc: DocumentMut = toml_data
            .parse()
            .map_err(|error: toml_edit::TomlError| TargetError::ParseError(error.to_string()))?;

        let key = provider_key(supplier);
        doc["model"] = toml_edit::value(config.model.trim());
        doc["model_provider"] = toml_edit::value(&key);
        apply_optional_key(
            &mut doc,
            "model_reasoning_effort",
            config.reasoning_effort_level(),
        );
        apply_optional_key(
            &mut doc,
            "plan_mode_reasoning_effort",
            config.plan_reasoning_effort_level(),
        );
        apply_optional_int(&mut doc, "model_context_window", config.context_tokens());
        apply_optional_int(
            &mut doc,
            "model_auto_compact_token_limit",
            config.compact_token_limit(),
        );

        ensure_table(&mut doc, "model_providers");
        ensure_standard_table(&mut doc, "model_providers", &key);
        let block = &mut doc["model_providers"][&key];
        block["base_url"] = toml_edit::value(supplier.base_url.trim());
        block["wire_api"] = toml_edit::value(Self::wire_api(config.protocol)?);
        block["name"] = toml_edit::value(&key);
        block["requires_openai_auth"] = toml_edit::value(true);

        let output = doc.to_string();
        output
            .parse::<DocumentMut>()
            .map_err(|error| TargetError::SerializeError(error.to_string()))?;
        write_secure_file(&config_path, output.as_bytes()).map_err(|error| {
            TargetError::WriteError {
                path: config_path.clone(),
                source: error,
            }
        })?;

        let auth_data = if auth_path.exists() {
            read_to_string(&auth_path)?
        } else {
            String::from("{\"auth_mode\": \"apikey\", \"OPENAI_API_KEY\": \"\"}")
        };
        let mut auth: Value = serde_json::from_str(&auth_data)
            .map_err(|error| TargetError::ParseError(error.to_string()))?;
        auth["OPENAI_API_KEY"] = Value::String(supplier.api_key.clone());
        let auth_output = serde_json::to_string_pretty(&auth)
            .map_err(|error| TargetError::SerializeError(error.to_string()))?;
        write_secure_file(&auth_path, auth_output.as_bytes()).map_err(|error| {
            TargetError::WriteError {
                path: auth_path.clone(),
                source: error,
            }
        })?;

        Ok(())
    }
}

fn apply_optional_key(doc: &mut DocumentMut, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        doc[key] = toml_edit::value(value);
    } else {
        doc.as_table_mut().remove(key);
    }
}

fn apply_optional_int(doc: &mut DocumentMut, key: &str, value: Option<u64>) {
    if let Some(value) = value {
        doc[key] = toml_edit::value(value as i64);
    } else {
        doc.as_table_mut().remove(key);
    }
}

fn ensure_table(doc: &mut DocumentMut, section: &str) {
    if doc.get(section).and_then(|item| item.as_table()).is_none() {
        let mut table = Table::new();
        table.set_implicit(true);
        doc.insert(section, Item::Table(table));
    }
}

fn ensure_standard_table(doc: &mut DocumentMut, section: &str, key: &str) {
    let existing = doc
        .get(section)
        .and_then(|item| item.as_table())
        .and_then(|table| table.get(key))
        .cloned();
    match existing {
        Some(Item::Table(_)) => {}
        Some(Item::Value(TomlValue::InlineTable(inline))) => {
            let mut table = inline.into_table();
            table.set_implicit(false);
            doc[section][key] = Item::Table(table);
        }
        _ => {
            let mut table = Table::new();
            table.set_implicit(false);
            doc[section][key] = Item::Table(table);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentConfig, AgentKind};
    use crate::supplier::Supplier;
    use tempfile::TempDir;

    fn supplier() -> Supplier {
        Supplier {
            id: "abcdef1234567890".to_string(),
            name: "relay".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            protocols: vec![Protocol::Responses],
            remark: None,
            last_probe: None,
            models: vec![],
        }
    }

    fn config() -> AgentConfig {
        AgentConfig {
            id: "b1".to_string(),
            supplier_id: "abcdef1234567890".to_string(),
            protocol: Protocol::Responses,
            model: "gpt-5".to_string(),
            reasoning_effort: Some("high".to_string()),
            plan_reasoning_effort: Some("xhigh".to_string()),
            context_length: Some("200k".to_string()),
            compact_ratio: Some(80),
            claude_models: None,
            grok_web_search: None,
        }
    }

    #[test]
    fn writes_config_and_auth() {
        let dir = TempDir::new().unwrap();
        let adapter = CodexAdapter::new();
        adapter
            .apply(&supplier(), &config(), Some(dir.path()), Env::Wsl)
            .unwrap();

        let config = fs::read_to_string(dir.path().join("config.toml")).unwrap();
        let doc: DocumentMut = config.parse().unwrap();
        assert_eq!(doc["model"].as_str(), Some("gpt-5"));
        assert_eq!(doc["model_provider"].as_str(), Some("cxc-abcdef12"));
        assert_eq!(doc["model_reasoning_effort"].as_str(), Some("high"));
        assert_eq!(doc["plan_mode_reasoning_effort"].as_str(), Some("xhigh"));
        assert_eq!(doc["model_context_window"].as_integer(), Some(200_000));
        assert_eq!(
            doc["model_auto_compact_token_limit"].as_integer(),
            Some(160_000)
        );
        assert_eq!(
            doc["model_providers"]["cxc-abcdef12"]["wire_api"].as_str(),
            Some("responses")
        );

        let auth: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("auth.json")).unwrap())
                .unwrap();
        assert_eq!(auth["OPENAI_API_KEY"].as_str(), Some("sk-test"));
    }

    #[test]
    fn preserves_unrelated_sections_and_clears_unset_keys() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            "[ui]\ntheme = \"dark\"\nmodel_reasoning_effort = \"low\"\n",
        )
        .unwrap();
        let mut config = config();
        config.reasoning_effort = None;
        config.plan_reasoning_effort = None;
        config.context_length = None;
        config.compact_ratio = None;
        CodexAdapter::new()
            .apply(&supplier(), &config, Some(dir.path()), Env::Wsl)
            .unwrap();
        let doc: DocumentMut = fs::read_to_string(dir.path().join("config.toml"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(doc["ui"]["theme"].as_str(), Some("dark"));
        assert!(doc.get("model_reasoning_effort").is_none());
        assert!(doc.get("plan_mode_reasoning_effort").is_none());
        assert!(doc.get("model_context_window").is_none());
    }

    #[test]
    fn rejects_messages_protocol() {
        let dir = TempDir::new().unwrap();
        let mut config = config();
        config.protocol = Protocol::Messages;
        assert!(
            CodexAdapter::new()
                .apply(&supplier(), &config, Some(dir.path()), Env::Wsl)
                .is_err()
        );
    }

    #[test]
    fn kind_is_codex() {
        assert_eq!(CodexAdapter::new().kind(), AgentKind::Codex);
    }
}
