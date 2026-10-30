use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::agent::{AgentConfig, AgentKind, GrokWebSearch};
use crate::config::Env;
use crate::protocol::Protocol;
use crate::supplier::Supplier;

use super::{
    AgentAdapter, TargetError, backup, provider_key, read_to_string, resolve_config_dir,
    write_secure_file,
};

const MODELS_SECTION: &str = "models";
const MODEL_SECTION: &str = "model";
const MODEL_PROVIDERS_SECTION: &str = "model_providers";
const ENDPOINT_FIELDS: [&str; 3] = ["base_url", "api_key", "api_backend"];

pub struct GrokAdapter;

impl GrokAdapter {
    pub fn new() -> Self {
        Self
    }

    fn api_backend(protocol: Protocol) -> &'static str {
        match protocol {
            Protocol::ChatCompletions => "chat_completions",
            Protocol::Responses => "responses",
            Protocol::Messages => "messages",
        }
    }
}

impl Default for GrokAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for GrokAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Grok
    }

    fn config_path(&self, env: Env, custom_dir: Option<&Path>) -> Result<PathBuf, TargetError> {
        Ok(resolve_config_dir(AgentKind::Grok, env, custom_dir)?.join("config.toml"))
    }

    fn apply(
        &self,
        supplier: &Supplier,
        config: &AgentConfig,
        custom_dir: Option<&Path>,
        env: Env,
    ) -> Result<(), TargetError> {
        let dir = resolve_config_dir(AgentKind::Grok, env, custom_dir)?;
        fs::create_dir_all(&dir).map_err(|error| TargetError::WriteError {
            path: dir.clone(),
            source: error,
        })?;
        let config_path = dir.join("config.toml");
        backup(&config_path)?;

        let toml_data = if config_path.exists() {
            read_to_string(&config_path)?
        } else {
            String::from("[models]\ndefault = \"\"\n")
        };
        let mut doc: DocumentMut = toml_data
            .parse()
            .map_err(|error: toml_edit::TomlError| TargetError::ParseError(error.to_string()))?;

        let key = provider_key(supplier);
        ensure_table(&mut doc, MODELS_SECTION);
        doc[MODELS_SECTION]["default"] = toml_edit::value(config.model.trim());

        write_provider_block(
            &mut doc,
            &key,
            supplier.base_url.trim(),
            supplier.api_key.trim(),
            Self::api_backend(config.protocol),
        );

        let mut models = vec![config.model.trim().to_string()];
        if let Some(GrokWebSearch::Model { model }) = &config.grok_web_search {
            let model = model.trim();
            if !model.is_empty() && !models.iter().any(|existing| existing == model) {
                models.push(model.to_string());
            }
        }

        for model in &models {
            ensure_table(&mut doc, MODEL_SECTION);
            ensure_standard_table(&mut doc, MODEL_SECTION, model);
            let block = &mut doc[MODEL_SECTION][model.as_str()];
            block["model"] = toml_edit::value(model.as_str());
            block["model_provider"] = toml_edit::value(key.as_str());
            clear_model_endpoint_fields(block);
            apply_model_context_window(block, config.context_tokens());
        }

        apply_web_search(&mut doc, config);
        apply_reasoning_effort(&mut doc, config);
        apply_session_compact(&mut doc, config.compact_ratio_percent());

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
        Ok(())
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
        Some(Item::Value(Value::InlineTable(inline))) => {
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

fn write_provider_block(
    doc: &mut DocumentMut,
    key: &str,
    base_url: &str,
    api_key: &str,
    api_backend: &str,
) {
    ensure_table(doc, MODEL_PROVIDERS_SECTION);
    ensure_standard_table(doc, MODEL_PROVIDERS_SECTION, key);
    let block = &mut doc[MODEL_PROVIDERS_SECTION][key];
    block["base_url"] = toml_edit::value(base_url);
    block["api_key"] = toml_edit::value(api_key);
    block["api_backend"] = toml_edit::value(api_backend);
}

fn clear_model_endpoint_fields(block: &mut Item) {
    let Some(table) = block.as_table_mut() else {
        return;
    };
    for field in ENDPOINT_FIELDS {
        table.remove(field);
    }
}

fn apply_model_context_window(block: &mut Item, tokens: Option<u64>) {
    if let Some(tokens) = tokens {
        block["context_window"] = toml_edit::value(tokens as i64);
    } else if let Some(table) = block.as_table_mut() {
        table.remove("context_window");
    }
}

fn apply_web_search(doc: &mut DocumentMut, config: &AgentConfig) {
    let policy = config
        .grok_web_search
        .clone()
        .unwrap_or(GrokWebSearch::Follow);
    if matches!(policy, GrokWebSearch::Keep) {
        return;
    }
    ensure_table(doc, MODELS_SECTION);
    match policy {
        GrokWebSearch::Keep => {}
        GrokWebSearch::Unset => {
            if let Some(table) = doc[MODELS_SECTION].as_table_mut() {
                table.remove("web_search");
            }
        }
        GrokWebSearch::Follow => {
            doc[MODELS_SECTION]["web_search"] = toml_edit::value(config.model.trim());
        }
        GrokWebSearch::Model { model } => {
            doc[MODELS_SECTION]["web_search"] = toml_edit::value(model.trim());
        }
    }
}

fn apply_reasoning_effort(doc: &mut DocumentMut, config: &AgentConfig) {
    let model = config.model.trim();
    if model.is_empty() {
        return;
    }
    if let Some(level) = config.reasoning_effort_level() {
        ensure_table(doc, MODEL_SECTION);
        ensure_standard_table(doc, MODEL_SECTION, model);
        doc[MODEL_SECTION][model]["reasoning_effort"] = toml_edit::value(level);
        return;
    }
    if let Some(block) = doc
        .get_mut(MODEL_SECTION)
        .and_then(|section| section.get_mut(model))
        .and_then(|item| item.as_table_mut())
    {
        block.remove("reasoning_effort");
    }
}

fn apply_session_compact(doc: &mut DocumentMut, ratio: Option<u8>) {
    if let Some(ratio) = ratio {
        ensure_table(doc, "session");
        doc["session"]["auto_compact_threshold_percent"] = toml_edit::value(i64::from(ratio));
    } else if let Some(table) = doc
        .get_mut("session")
        .and_then(|value| value.as_table_mut())
    {
        table.remove("auto_compact_threshold_percent");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Protocol;
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
            supplier_id: "abcdef1234567890".to_string(),
            protocol: Protocol::Responses,
            model: "grok-4.5".to_string(),
            reasoning_effort: Some("high".to_string()),
            plan_reasoning_effort: None,
            context_length: Some("200k".to_string()),
            compact_ratio: Some(80),
            claude_models: None,
            grok_web_search: None,
        }
    }

    #[test]
    fn writes_provider_centric_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            "[cli]\ninstaller = \"internal\"\n\n[models]\ndefault = \"old\"\n\n[ui]\ntheme = \"dark\"\n",
        )
        .unwrap();
        GrokAdapter::new()
            .apply(&supplier(), &config(), Some(dir.path()), Env::Wsl)
            .unwrap();

        let doc: DocumentMut = fs::read_to_string(dir.path().join("config.toml"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(doc["models"]["default"].as_str(), Some("grok-4.5"));
        assert_eq!(
            doc["model_providers"]["cxc-abcdef12"]["base_url"].as_str(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(
            doc["model_providers"]["cxc-abcdef12"]["api_backend"].as_str(),
            Some("responses")
        );
        assert_eq!(
            doc["model"]["grok-4.5"]["model_provider"].as_str(),
            Some("cxc-abcdef12")
        );
        assert_eq!(
            doc["model"]["grok-4.5"]["context_window"].as_integer(),
            Some(200_000)
        );
        assert_eq!(
            doc["model"]["grok-4.5"]["reasoning_effort"].as_str(),
            Some("high")
        );
        assert_eq!(
            doc["session"]["auto_compact_threshold_percent"].as_integer(),
            Some(80)
        );
        assert_eq!(doc["ui"]["theme"].as_str(), Some("dark"));
    }

    #[test]
    fn web_search_policy_is_applied() {
        let dir = TempDir::new().unwrap();
        let mut config = config();
        config.grok_web_search = Some(GrokWebSearch::Model {
            model: "grok-search".to_string(),
        });
        GrokAdapter::new()
            .apply(&supplier(), &config, Some(dir.path()), Env::Wsl)
            .unwrap();
        let doc: DocumentMut = fs::read_to_string(dir.path().join("config.toml"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(doc["models"]["web_search"].as_str(), Some("grok-search"));
        assert_eq!(
            doc["model"]["grok-search"]["model_provider"].as_str(),
            Some("cxc-abcdef12")
        );
    }
}
