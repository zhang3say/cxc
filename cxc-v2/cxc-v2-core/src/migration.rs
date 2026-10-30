use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

use crate::agent::{self, AgentKind, BindingInput, ClaudeModels, GrokWebSearch};
use crate::config::{self, Config, Source};
use crate::protocol::Protocol;
use crate::supplier::{self, SupplierInput};
use crate::sync::SyncConfig;

#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("读取旧配置失败：{0}")]
    Read(String),
    #[error("解析旧配置失败：{0}")]
    Parse(String),
    #[error("保存新配置失败：{0}")]
    Save(String),
}

#[derive(serde::Serialize, Debug, Clone, Default)]
pub struct ImportReport {
    pub imported: bool,
    pub reason: String,
    pub suppliers: usize,
    pub bindings: usize,
    pub skipped: Vec<String>,
}

#[derive(Deserialize, Default)]
struct LegacyConfig {
    #[serde(default)]
    codex_active_app: String,
    #[serde(default)]
    codex_active_wsl: String,
    #[serde(default)]
    codex_providers: Vec<LegacyProvider>,
    #[serde(default)]
    codex_source: Option<String>,
    #[serde(default)]
    codex_custom_dir: String,

    #[serde(default)]
    claude_active_app: String,
    #[serde(default)]
    claude_active_wsl: String,
    #[serde(default)]
    claude_providers: Vec<LegacyProvider>,
    #[serde(default)]
    claude_custom_dir: String,

    #[serde(default)]
    grok_active_app: String,
    #[serde(default)]
    grok_active_wsl: String,
    #[serde(default)]
    grok_providers: Vec<LegacyProvider>,
    #[serde(default)]
    grok_custom_dir: String,

    #[serde(default)]
    sync: Option<SyncConfig>,

    // 极早期版本字段
    #[serde(default)]
    providers: Vec<LegacyProvider>,
    #[serde(default)]
    active: String,
    #[serde(default)]
    codex_active: String,
    #[serde(default)]
    claude_active: String,
}

#[derive(Deserialize, Clone, Default)]
pub(crate) struct LegacyProvider {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) wire_api: String,
    #[serde(default)]
    pub(crate) remark: Option<String>,
    #[serde(default)]
    pub(crate) claude_models: Option<ClaudeModels>,
    #[serde(default)]
    pub(crate) grok_web_search: Option<GrokWebSearch>,
    #[serde(default)]
    pub(crate) reasoning_effort: Option<String>,
    #[serde(default)]
    pub(crate) plan_reasoning_effort: Option<String>,
    #[serde(default)]
    pub(crate) context_length: Option<String>,
    #[serde(default)]
    pub(crate) compact_ratio: Option<u8>,
}

pub fn import_if_needed(
    config_path: &Path,
    legacy_path: &Path,
) -> Result<ImportReport, MigrationError> {
    if config_path.exists() {
        return Ok(ImportReport {
            imported: false,
            reason: "新版配置已存在，跳过导入".to_string(),
            ..ImportReport::default()
        });
    }
    if !legacy_path.exists() {
        return Ok(ImportReport {
            imported: false,
            reason: "没有找到旧版配置".to_string(),
            ..ImportReport::default()
        });
    }

    let data = fs::read(legacy_path).map_err(|error| MigrationError::Read(error.to_string()))?;
    let mut legacy: LegacyConfig =
        serde_yaml::from_slice(&data).map_err(|error| MigrationError::Parse(error.to_string()))?;
    normalize_legacy(&mut legacy);

    let mut cfg = Config {
        source: legacy
            .codex_source
            .as_deref()
            .map(Source::parse)
            .unwrap_or(Source::Wsl),
        ..Config::default()
    };
    cfg.custom_dirs.codex = legacy.codex_custom_dir.trim().to_string();
    cfg.custom_dirs.claude = legacy.claude_custom_dir.trim().to_string();
    cfg.custom_dirs.grok = legacy.grok_custom_dir.trim().to_string();
    cfg.sync = legacy.sync.clone();
    cfg.migrated_from_v1 = true;

    let mut report = ImportReport {
        imported: true,
        reason: "已从旧版配置导入".to_string(),
        ..ImportReport::default()
    };
    let mut supplier_ids: HashMap<(String, String), String> = HashMap::new();

    import_agent(
        &mut cfg,
        AgentKind::Codex,
        &legacy.codex_providers,
        &legacy.codex_active_app,
        &legacy.codex_active_wsl,
        &mut supplier_ids,
        &mut report,
    );
    import_agent(
        &mut cfg,
        AgentKind::Claude,
        &legacy.claude_providers,
        &legacy.claude_active_app,
        &legacy.claude_active_wsl,
        &mut supplier_ids,
        &mut report,
    );
    import_agent(
        &mut cfg,
        AgentKind::Grok,
        &legacy.grok_providers,
        &legacy.grok_active_app,
        &legacy.grok_active_wsl,
        &mut supplier_ids,
        &mut report,
    );

    config::save_to(config_path, &cfg).map_err(|error| MigrationError::Save(error.to_string()))?;
    Ok(report)
}

/// 把旧配置合并进一份已存在的配置。用于设置里的「重新导入」。
pub fn import_into_config(
    cfg: &mut Config,
    legacy_path: &Path,
) -> Result<ImportReport, MigrationError> {
    if !legacy_path.exists() {
        return Ok(ImportReport {
            imported: false,
            reason: "没有找到旧版配置".to_string(),
            ..ImportReport::default()
        });
    }
    let data = fs::read(legacy_path).map_err(|error| MigrationError::Read(error.to_string()))?;
    let mut legacy: LegacyConfig =
        serde_yaml::from_slice(&data).map_err(|error| MigrationError::Parse(error.to_string()))?;
    normalize_legacy(&mut legacy);

    let mut report = ImportReport {
        imported: true,
        reason: "已从旧版配置导入".to_string(),
        ..ImportReport::default()
    };
    let mut supplier_ids: HashMap<(String, String), String> = HashMap::new();
    for supplier in &cfg.suppliers {
        supplier_ids.insert(
            (supplier.base_url.clone(), supplier.api_key.clone()),
            supplier.id.clone(),
        );
    }

    import_agent(
        cfg,
        AgentKind::Codex,
        &legacy.codex_providers,
        &legacy.codex_active_app,
        &legacy.codex_active_wsl,
        &mut supplier_ids,
        &mut report,
    );
    import_agent(
        cfg,
        AgentKind::Claude,
        &legacy.claude_providers,
        &legacy.claude_active_app,
        &legacy.claude_active_wsl,
        &mut supplier_ids,
        &mut report,
    );
    import_agent(
        cfg,
        AgentKind::Grok,
        &legacy.grok_providers,
        &legacy.grok_active_app,
        &legacy.grok_active_wsl,
        &mut supplier_ids,
        &mut report,
    );
    cfg.migrated_from_v1 = true;
    Ok(report)
}

fn normalize_legacy(legacy: &mut LegacyConfig) {
    // 极早期版本：providers / active 视为 Codex。
    if legacy.codex_providers.is_empty() && !legacy.providers.is_empty() {
        legacy.codex_providers = legacy.providers.clone();
        legacy.codex_active_app = legacy.active.clone();
    }
    if !legacy.codex_active.is_empty() {
        legacy.codex_active_wsl = legacy.codex_active.clone();
    }
    if !legacy.claude_active.is_empty() {
        legacy.claude_active_wsl = legacy.claude_active.clone();
    }
}

pub(crate) fn import_agent(
    cfg: &mut Config,
    kind: AgentKind,
    providers: &[LegacyProvider],
    active_app: &str,
    active_wsl: &str,
    supplier_ids: &mut HashMap<(String, String), String>,
    report: &mut ImportReport,
) {
    let mut binding_ids: HashMap<String, String> = HashMap::new();

    for provider in providers {
        let base_url = match supplier::normalize_base_url(&provider.base_url) {
            Ok(url) => url,
            Err(_) => {
                report.skipped.push(format!(
                    "{}: 地址无效 ({})",
                    kind.display_name(),
                    provider.name
                ));
                continue;
            }
        };
        let api_key = provider.api_key.trim();
        if api_key.is_empty() {
            report.skipped.push(format!(
                "{}: 缺少 API Key ({})",
                kind.display_name(),
                provider.name
            ));
            continue;
        }
        let model = provider.model.trim();
        if model.is_empty() {
            report.skipped.push(format!(
                "{}: 模型为空 ({})",
                kind.display_name(),
                provider.name
            ));
            continue;
        }

        let protocol = protocol_for(kind, &provider.wire_api);
        let key = (base_url.clone(), api_key.to_string());
        let supplier_id = if let Some(id) = supplier_ids.get(&key) {
            id.clone()
        } else {
            let name = unique_supplier_name(cfg, &provider.name, &base_url);
            let supplier = match supplier::add_supplier(
                cfg,
                SupplierInput {
                    name,
                    base_url: base_url.clone(),
                    api_key: api_key.to_string(),
                    remark: provider.remark.clone(),
                    protocols: Some(vec![protocol]),
                },
            ) {
                Ok(supplier) => supplier,
                Err(error) => {
                    report
                        .skipped
                        .push(format!("{}: {error}", kind.display_name()));
                    continue;
                }
            };
            supplier_ids.insert(key.clone(), supplier.id.clone());
            report.suppliers += 1;
            supplier.id
        };

        if let Some(supplier) = cfg.supplier_mut(&supplier_id) {
            if !supplier.protocols.contains(&protocol) {
                supplier.protocols.push(protocol);
                supplier.protocols = Protocol::normalize_all(supplier.protocols.iter().copied());
            }
            supplier::merge_discovered_models(supplier, vec![model.to_string()]);
        }

        let duplicate = cfg.agent(kind).bindings.iter().any(|binding| {
            binding.supplier_id == supplier_id
                && binding.protocol == protocol
                && binding.model == model
        });
        if duplicate {
            continue;
        }

        let input = BindingInput {
            supplier_id: supplier_id.clone(),
            protocol,
            model: model.to_string(),
            reasoning_effort: provider.reasoning_effort.clone(),
            plan_reasoning_effort: provider.plan_reasoning_effort.clone(),
            context_length: provider.context_length.clone(),
            compact_ratio: provider.compact_ratio,
            claude_models: provider.claude_models.clone(),
            grok_web_search: provider.grok_web_search.clone(),
        };

        match agent::add_binding(cfg, kind, input) {
            Ok(binding) => {
                report.bindings += 1;
                if let Some(supplier) = cfg.supplier_mut(&supplier_id)
                    && let Some(entry) = supplier
                        .models
                        .iter_mut()
                        .find(|entry| entry.model == model)
                    && !entry.protocols.contains(&protocol)
                {
                    entry.protocols.push(protocol);
                    entry.protocols = Protocol::normalize_all(entry.protocols.iter().copied());
                }
                if !provider.name.trim().is_empty() {
                    binding_ids.insert(provider.name.trim().to_string(), binding.id);
                }
            }
            Err(error) => report
                .skipped
                .push(format!("{}: {error}", kind.display_name())),
        }
    }

    let agent_cfg = cfg.agent_mut(kind);
    if let Some(id) = binding_ids.get(active_app.trim()) {
        agent_cfg.active_app = Some(id.clone());
    }
    if let Some(id) = binding_ids.get(active_wsl.trim()) {
        agent_cfg.active_wsl = Some(id.clone());
    }
}

fn protocol_for(kind: AgentKind, wire_api: &str) -> Protocol {
    match kind {
        AgentKind::Claude => Protocol::Messages,
        AgentKind::Codex => {
            if wire_api.trim() == "chat" {
                Protocol::ChatCompletions
            } else {
                Protocol::Responses
            }
        }
        AgentKind::Grok | AgentKind::OpenCode => {
            Protocol::parse(wire_api).unwrap_or(Protocol::ChatCompletions)
        }
    }
}

fn unique_supplier_name(cfg: &Config, raw: &str, base_url: &str) -> String {
    let fallback = url::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| "relay".to_string());
    let base = if raw.trim().is_empty() {
        fallback
    } else {
        raw.trim().to_string()
    };
    if !cfg.suppliers.iter().any(|supplier| supplier.name == base) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base} ({index})");
        if !cfg
            .suppliers
            .iter()
            .any(|supplier| supplier.name == candidate)
        {
            return candidate;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY: &str = r#"
codex_active_app: ''
codex_active_wsl: relay-a
codex_providers:
  - name: relay-a
    base_url: https://a.example.com/v1
    api_key: sk-a
    model: gpt-5
    wire_api: responses
    reasoning_effort: high
  - name: relay-b
    base_url: https://b.example.com/v1
    api_key: sk-b
    model: gpt-5-mini
    wire_api: chat
claude_active_app: ''
claude_active_wsl: relay-a
claude_providers:
  - name: relay-a
    base_url: https://a.example.com/v1
    api_key: sk-a
    model: claude-sonnet
    wire_api: messages
    context_length: 1m
    compact_ratio: 80
grok_providers: []
grok_active_app: ''
grok_active_wsl: ''
codex_source: wsl
codex_custom_dir: ''
claude_custom_dir: ''
grok_custom_dir: ''
"#;

    #[test]
    fn imports_and_merges_suppliers() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("legacy.yaml");
        let config_path = dir.path().join("config.yaml");
        fs::write(&legacy_path, LEGACY).unwrap();

        let report = import_if_needed(&config_path, &legacy_path).unwrap();
        assert!(report.imported);
        assert_eq!(report.suppliers, 2);
        assert_eq!(report.bindings, 3);

        let cfg = config::load_from(&config_path).unwrap();
        assert_eq!(cfg.suppliers.len(), 2);
        let relay_a = cfg.supplier_by_name("relay-a").unwrap();
        assert!(relay_a.protocols.contains(&Protocol::Responses));
        assert!(relay_a.protocols.contains(&Protocol::Messages));
        assert_eq!(cfg.agent(AgentKind::Codex).bindings.len(), 2);
        assert_eq!(cfg.agent(AgentKind::Claude).bindings.len(), 1);
        assert!(cfg.agent(AgentKind::Codex).active_wsl.is_some());
        assert!(cfg.agent(AgentKind::Claude).active_wsl.is_some());
    }

    #[test]
    fn import_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_path = dir.path().join("legacy.yaml");
        let config_path = dir.path().join("config.yaml");
        fs::write(&legacy_path, LEGACY).unwrap();
        import_if_needed(&config_path, &legacy_path).unwrap();
        let second = import_if_needed(&config_path, &legacy_path).unwrap();
        assert!(!second.imported);
        let cfg = config::load_from(&config_path).unwrap();
        assert_eq!(cfg.suppliers.len(), 2);
    }
}
