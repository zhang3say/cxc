use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::agent::AgentKind;
use crate::config::Config;
use crate::protocol::Protocol;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub protocols: Vec<Protocol>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_probe: Option<DateTime<Local>>,
    #[serde(default)]
    pub models: Vec<SupplierModel>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SupplierModel {
    pub model: String,
    #[serde(default)]
    pub protocols: Vec<Protocol>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_test: Option<DateTime<Local>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SupplierInput {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub remark: Option<String>,
    /// `None` 表示编辑时保留原协议集合；`Some` 表示整份替换。
    pub protocols: Option<Vec<Protocol>>,
}

#[derive(Error, Debug)]
pub enum SupplierError {
    #[error("供应商不存在：{0}")]
    NotFound(String),
    #[error("供应商名称已存在：{0}")]
    DuplicateName(String),
    #[error("供应商名称不能为空")]
    EmptyName,
    #[error("中转站地址无效：{0}")]
    InvalidUrl(String),
    #[error("API Key 不能为空")]
    EmptyKey,
    #[error("供应商仍被以下绑定引用：{0}")]
    Referenced(String),
}

pub fn new_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn short_id(id: &str) -> &str {
    &id[..id.len().min(8)]
}

pub fn normalize_name(raw: &str) -> Result<String, SupplierError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(SupplierError::EmptyName);
    }
    Ok(name.to_string())
}

pub fn normalize_base_url(raw: &str) -> Result<String, SupplierError> {
    let value = raw.trim();
    let parsed = Url::parse(value).map_err(|_| SupplierError::InvalidUrl(value.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(SupplierError::InvalidUrl(value.to_string()));
    }
    Ok(value.to_string())
}

pub fn normalize_remark(raw: Option<String>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn add_supplier(cfg: &mut Config, input: SupplierInput) -> Result<Supplier, SupplierError> {
    let name = normalize_name(&input.name)?;
    if cfg.suppliers.iter().any(|supplier| supplier.name == name) {
        return Err(SupplierError::DuplicateName(name));
    }
    let base_url = normalize_base_url(&input.base_url)?;
    let api_key = input.api_key.trim();
    if api_key.is_empty() {
        return Err(SupplierError::EmptyKey);
    }

    let supplier = Supplier {
        id: new_id(),
        name,
        base_url,
        api_key: api_key.to_string(),
        protocols: Protocol::normalize_all(input.protocols.unwrap_or_default()),
        remark: normalize_remark(input.remark),
        last_probe: None,
        models: Vec::new(),
    };
    cfg.suppliers.push(supplier.clone());
    Ok(supplier)
}

pub fn edit_supplier(
    cfg: &mut Config,
    id: &str,
    input: SupplierInput,
) -> Result<Supplier, SupplierError> {
    let name = normalize_name(&input.name)?;
    if cfg
        .suppliers
        .iter()
        .any(|supplier| supplier.id != id && supplier.name == name)
    {
        return Err(SupplierError::DuplicateName(name));
    }
    let base_url = normalize_base_url(&input.base_url)?;

    let supplier = cfg
        .supplier_mut(id)
        .ok_or_else(|| SupplierError::NotFound(id.to_string()))?;

    let mut endpoint_changed = supplier.base_url != base_url;
    supplier.name = name;
    supplier.base_url = base_url;
    let key = input.api_key.trim();
    if !key.is_empty() {
        if supplier.api_key != key {
            endpoint_changed = true;
        }
        supplier.api_key = key.to_string();
    }
    if endpoint_changed {
        supplier.models.iter_mut().for_each(clear_model_test);
        supplier.last_probe = None;
    }
    if let Some(protocols) = input.protocols {
        supplier.protocols = Protocol::normalize_all(protocols);
    }
    supplier.remark = normalize_remark(input.remark);
    Ok(supplier.clone())
}

fn clear_model_test(model: &mut SupplierModel) {
    model.last_test = None;
    model.latency_ms = None;
    model.last_ok = None;
    model.last_error = None;
}

pub fn remove_supplier(cfg: &mut Config, id: &str) -> Result<(), SupplierError> {
    if cfg.supplier(id).is_none() {
        return Err(SupplierError::NotFound(id.to_string()));
    }

    let mut references = Vec::new();
    for kind in AgentKind::ALL {
        let agent = cfg.agent(kind);
        if crate::config::Env::ALL.iter().any(|env| {
            agent.get(*env).is_some_and(|slot| {
                slot.bindings
                    .iter()
                    .any(|binding| binding.supplier_id == id)
            })
        }) {
            references.push(kind.display_name().to_string());
        }
    }
    if !references.is_empty() {
        return Err(SupplierError::Referenced(references.join("、")));
    }

    cfg.suppliers.retain(|supplier| supplier.id != id);
    Ok(())
}

pub fn get_supplier<'a>(cfg: &'a Config, id: &str) -> Option<&'a Supplier> {
    cfg.supplier(id)
}

pub fn merge_discovered_models(supplier: &mut Supplier, models: Vec<String>) {
    for model in models {
        let model = model.trim();
        if model.is_empty() {
            continue;
        }
        if !supplier.models.iter().any(|entry| entry.model == model) {
            supplier.models.push(SupplierModel {
                model: model.to_string(),
                ..SupplierModel::default()
            });
        }
    }
}

/// 用接口返回的模型列表替换现有目录：下架模型连同测试结果一起移除，
/// 仍然存在的模型保留原有测试结果，新出现的模型结果为空。
pub fn replace_discovered_models(supplier: &mut Supplier, models: Vec<String>) {
    let previous = std::mem::take(&mut supplier.models);
    let mut next: Vec<SupplierModel> = Vec::new();

    for model in models {
        let model = model.trim();
        if model.is_empty() || next.iter().any(|entry| entry.model == model) {
            continue;
        }
        let mut entry = previous
            .iter()
            .find(|entry| entry.model == model)
            .cloned()
            .unwrap_or_else(|| SupplierModel {
                model: model.to_string(),
                ..SupplierModel::default()
            });
        entry.model = model.to_string();
        next.push(entry);
    }

    supplier.models = next;
}

pub fn record_model_test(
    cfg: &mut Config,
    supplier_id: &str,
    model: &str,
    protocol: Protocol,
    ok: bool,
    latency_ms: i64,
    error: Option<String>,
) -> Result<(), SupplierError> {
    let supplier = cfg
        .supplier_mut(supplier_id)
        .ok_or_else(|| SupplierError::NotFound(supplier_id.to_string()))?;
    let model = model.trim();
    if model.is_empty() {
        return Ok(());
    }

    if let Some(entry) = supplier
        .models
        .iter_mut()
        .find(|entry| entry.model == model)
    {
        entry.last_test = Some(Local::now());
        entry.latency_ms = Some(latency_ms);
        entry.last_ok = Some(ok);
        entry.last_error = error;
        if ok && !entry.protocols.contains(&protocol) {
            entry.protocols.push(protocol);
            entry.protocols = Protocol::normalize_all(entry.protocols.iter().copied());
        }
    } else {
        supplier.models.push(SupplierModel {
            model: model.to_string(),
            protocols: if ok { vec![protocol] } else { Vec::new() },
            last_test: Some(Local::now()),
            latency_ms: Some(latency_ms),
            last_ok: Some(ok),
            last_error: error,
        });
    }
    Ok(())
}

pub fn find_model<'a>(supplier: &'a Supplier, model: &str) -> Option<&'a SupplierModel> {
    supplier.models.iter().find(|entry| entry.model == model)
}

pub fn supports(supplier: &Supplier, protocol: Protocol) -> bool {
    supplier.protocols.contains(&protocol)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, url: &str, key: &str) -> SupplierInput {
        SupplierInput {
            name: name.to_string(),
            base_url: url.to_string(),
            api_key: key.to_string(),
            remark: None,
            protocols: None,
        }
    }

    #[test]
    fn add_and_edit_supplier() {
        let mut cfg = Config::default();
        let supplier = add_supplier(
            &mut cfg,
            input("relay", "https://api.example.com/v1", "sk-1"),
        )
        .unwrap();
        assert_eq!(cfg.suppliers.len(), 1);
        assert_eq!(supplier.base_url, "https://api.example.com/v1");

        let edited = edit_supplier(
            &mut cfg,
            &supplier.id,
            SupplierInput {
                name: "relay-2".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                api_key: String::new(),
                remark: Some("  ".to_string()),
                protocols: Some(vec![Protocol::Messages]),
            },
        )
        .unwrap();
        assert_eq!(edited.name, "relay-2");
        assert_eq!(edited.api_key, "sk-1");
        assert_eq!(edited.remark, None);
        assert_eq!(edited.protocols, vec![Protocol::Messages]);
    }

    #[test]
    fn rejects_duplicate_name_and_bad_url() {
        let mut cfg = Config::default();
        add_supplier(
            &mut cfg,
            input("relay", "https://api.example.com/v1", "sk-1"),
        )
        .unwrap();
        assert!(matches!(
            add_supplier(
                &mut cfg,
                input("relay", "https://api.example.com/v1", "sk-2")
            ),
            Err(SupplierError::DuplicateName(_))
        ));
        assert!(matches!(
            add_supplier(&mut cfg, input("other", "ftp://api.example.com", "sk-2")),
            Err(SupplierError::InvalidUrl(_))
        ));
    }

    #[test]
    fn refuses_to_remove_referenced_supplier() {
        let mut cfg = Config::default();
        let supplier = add_supplier(
            &mut cfg,
            input("relay", "https://api.example.com/v1", "sk-1"),
        )
        .unwrap();
        cfg.agent_mut(AgentKind::Codex).set(
            crate::config::Env::Wsl,
            Some(crate::agent::AgentSlot {
                active: Some("b1".to_string()),
                bindings: vec![crate::agent::AgentConfig {
                    id: "b1".to_string(),
                    supplier_id: supplier.id.clone(),
                    protocol: Protocol::Responses,
                    model: "gpt-5".to_string(),
                    reasoning_effort: None,
                    plan_reasoning_effort: None,
                    context_length: None,
                    compact_ratio: None,
                    claude_models: None,
                    grok_web_search: None,
                }],
            }),
        );
        assert!(matches!(
            remove_supplier(&mut cfg, &supplier.id),
            Err(SupplierError::Referenced(_))
        ));
    }

    #[test]
    fn merges_models_without_losing_results() {
        let mut supplier = Supplier {
            id: "s1".to_string(),
            name: "relay".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "sk-1".to_string(),
            protocols: vec![],
            remark: None,
            last_probe: None,
            models: vec![SupplierModel {
                model: "a".to_string(),
                last_ok: Some(true),
                ..SupplierModel::default()
            }],
        };
        merge_discovered_models(&mut supplier, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(supplier.models.len(), 2);
        assert_eq!(supplier.models[0].last_ok, Some(true));
    }

    fn supplier_with_models(models: Vec<SupplierModel>) -> Supplier {
        Supplier {
            id: "s1".to_string(),
            name: "relay".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "sk-1".to_string(),
            protocols: vec![],
            remark: None,
            last_probe: None,
            models,
        }
    }

    #[test]
    fn replace_drops_retired_models_and_keeps_surviving_results() {
        let mut supplier = supplier_with_models(vec![
            SupplierModel {
                model: "keep".to_string(),
                last_ok: Some(true),
                latency_ms: Some(120),
                ..SupplierModel::default()
            },
            SupplierModel {
                model: "retired".to_string(),
                last_ok: Some(false),
                last_error: Some("gone".to_string()),
                ..SupplierModel::default()
            },
        ]);

        replace_discovered_models(&mut supplier, vec!["keep".to_string(), "fresh".to_string()]);

        assert_eq!(supplier.models.len(), 2);
        assert_eq!(supplier.models[0].model, "keep");
        assert_eq!(supplier.models[0].last_ok, Some(true));
        assert_eq!(supplier.models[0].latency_ms, Some(120));
        assert_eq!(supplier.models[1].model, "fresh");
        assert_eq!(supplier.models[1].last_ok, None);
        assert!(!supplier.models.iter().any(|entry| entry.model == "retired"));
    }

    #[test]
    fn replace_dedupes_and_ignores_blank_models() {
        let mut supplier = supplier_with_models(vec![SupplierModel {
            model: "a".to_string(),
            ..SupplierModel::default()
        }]);
        replace_discovered_models(
            &mut supplier,
            vec![
                "a".to_string(),
                " a ".to_string(),
                "  ".to_string(),
                "b".to_string(),
            ],
        );
        assert_eq!(supplier.models.len(), 2);
        assert_eq!(supplier.models[0].model, "a");
        assert_eq!(supplier.models[1].model, "b");
    }

    #[test]
    fn replace_with_empty_list_clears_models() {
        let mut supplier = supplier_with_models(vec![SupplierModel {
            model: "a".to_string(),
            last_ok: Some(true),
            ..SupplierModel::default()
        }]);
        replace_discovered_models(&mut supplier, Vec::new());
        assert!(supplier.models.is_empty());
    }
}
