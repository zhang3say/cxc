use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::agent::AgentKind;
use crate::config::Config;
use crate::protocol::Protocol;

/// 中转站下的一个账号。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SupplierKey {
    /// 创建时生成，之后不变。绑定按它引用账号。
    pub id: String,
    pub name: String,
    pub api_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub keys: Vec<SupplierKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_key_id: Option<String>,
    #[serde(default)]
    pub protocols: Vec<Protocol>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_probe: Option<DateTime<Local>>,
    #[serde(default)]
    pub models: Vec<SupplierModel>,
}

impl Supplier {
    pub fn key(&self, id: &str) -> Option<&SupplierKey> {
        self.keys.iter().find(|key| key.id == id)
    }

    /// 中转站默认账号：`default_key_id` 有效时用它，否则退回第一个账号。
    pub fn default_key(&self) -> Option<&SupplierKey> {
        self.default_key_id
            .as_deref()
            .and_then(|id| self.key(id))
            .or_else(|| self.keys.first())
    }
}

/// 解析绑定实际使用的账号：显式指定 → 中转站默认 → 第一个账号。
pub fn effective_key<'a>(supplier: &'a Supplier, key_id: Option<&str>) -> Option<&'a SupplierKey> {
    key_id
        .and_then(|id| supplier.key(id))
        .or_else(|| supplier.default_key())
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
    /// 最近一次测试使用的账号；`None` 表示迁移前的结果。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tested_key_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SupplierKeyInput {
    /// `None` 表示新增账号；`Some` 表示编辑现有账号。
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    /// 编辑现有账号时留空表示保留原 Key。
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SupplierInput {
    pub name: String,
    pub base_url: String,
    pub remark: Option<String>,
    /// `None` 表示编辑时保留原协议集合；`Some` 表示整份替换。
    pub protocols: Option<Vec<Protocol>>,
    #[serde(default)]
    pub keys: Vec<SupplierKeyInput>,
    #[serde(default)]
    pub default_key_id: Option<String>,
}

impl SupplierInput {
    /// 迁移与旧版导入用：单个 Key 建成一个名为「默认」的账号。
    pub fn single_key(
        name: String,
        base_url: String,
        api_key: String,
        remark: Option<String>,
        protocols: Option<Vec<Protocol>>,
    ) -> Self {
        Self {
            name,
            base_url,
            remark,
            protocols,
            keys: vec![SupplierKeyInput {
                id: None,
                name: "默认".to_string(),
                api_key,
                remark: None,
            }],
            default_key_id: None,
        }
    }
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
    #[error("账号名称不能为空")]
    EmptyKeyName,
    #[error("账号名称已存在：{0}")]
    DuplicateKeyName(String),
    #[error("中转站至少要有一个账号")]
    NoKeys,
    #[error("账号不存在：{0}")]
    KeyNotFound(String),
    #[error("账号「{0}」仍被以下 Agent 的绑定引用：{1}")]
    KeyReferenced(String, String),
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

/// 校验并生成账号列表。编辑现有账号时 `api_key` 留空表示保留原 Key。
fn resolve_keys(
    existing: &[SupplierKey],
    inputs: Vec<SupplierKeyInput>,
) -> Result<Vec<SupplierKey>, SupplierError> {
    let mut keys: Vec<SupplierKey> = Vec::new();

    for input in inputs {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(SupplierError::EmptyKeyName);
        }
        if keys.iter().any(|key| key.name == name) {
            return Err(SupplierError::DuplicateKeyName(name.to_string()));
        }

        let existing_key = input
            .id
            .as_deref()
            .and_then(|id| existing.iter().find(|key| key.id == id));
        let api_key = {
            let trimmed = input.api_key.trim();
            if trimmed.is_empty() {
                existing_key
                    .map(|key| key.api_key.clone())
                    .ok_or(SupplierError::EmptyKey)?
            } else {
                trimmed.to_string()
            }
        };

        keys.push(SupplierKey {
            id: existing_key
                .map(|key| key.id.clone())
                .unwrap_or_else(new_id),
            name: name.to_string(),
            api_key,
            remark: normalize_remark(input.remark),
        });
    }

    if keys.is_empty() {
        return Err(SupplierError::NoKeys);
    }
    Ok(keys)
}

fn resolve_default_key_id(keys: &[SupplierKey], requested: Option<&str>) -> Option<String> {
    requested
        .and_then(|id| keys.iter().find(|key| key.id == id))
        .or_else(|| keys.first())
        .map(|key| key.id.clone())
}

pub fn add_supplier(cfg: &mut Config, input: SupplierInput) -> Result<Supplier, SupplierError> {
    let name = normalize_name(&input.name)?;
    if cfg.suppliers.iter().any(|supplier| supplier.name == name) {
        return Err(SupplierError::DuplicateName(name));
    }
    let base_url = normalize_base_url(&input.base_url)?;
    let keys = resolve_keys(&[], input.keys)?;
    let default_key_id = resolve_default_key_id(&keys, input.default_key_id.as_deref());

    let supplier = Supplier {
        id: new_id(),
        name,
        base_url,
        keys,
        default_key_id,
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

    let existing = cfg
        .supplier(id)
        .ok_or_else(|| SupplierError::NotFound(id.to_string()))?
        .clone();
    let keys = resolve_keys(&existing.keys, input.keys)?;

    // 被绑定显式引用的账号不能删。
    let removed: Vec<&SupplierKey> = existing
        .keys
        .iter()
        .filter(|key| !keys.iter().any(|next| next.id == key.id))
        .collect();
    if !removed.is_empty() {
        let removed_ids: Vec<&str> = removed.iter().map(|key| key.id.as_str()).collect();
        if let Some((key_name, agents)) = referencing_agents(cfg, id, &removed_ids) {
            return Err(SupplierError::KeyReferenced(key_name, agents));
        }
    }

    let default_key_id = resolve_default_key_id(&keys, input.default_key_id.as_deref());
    let endpoint_changed = existing.base_url != base_url;

    let supplier = cfg
        .supplier_mut(id)
        .ok_or_else(|| SupplierError::NotFound(id.to_string()))?;
    supplier.name = name;
    supplier.base_url = base_url;
    supplier.keys = keys;
    supplier.default_key_id = default_key_id;
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

/// 找出显式引用这些账号的 Agent 名称，返回 `(账号名, Agent 列表)`。
fn referencing_agents(
    cfg: &Config,
    supplier_id: &str,
    key_ids: &[&str],
) -> Option<(String, String)> {
    for key_id in key_ids {
        let mut agents: Vec<String> = Vec::new();
        for kind in AgentKind::ALL {
            let agent = cfg.agent(kind);
            let referenced = crate::config::Env::ALL.iter().any(|env| {
                agent.get(*env).is_some_and(|slot| {
                    slot.bindings.iter().any(|binding| {
                        binding.supplier_id == supplier_id
                            && binding.key_id.as_deref() == Some(*key_id)
                    })
                })
            });
            if referenced {
                agents.push(kind.display_name().to_string());
            }
        }
        if !agents.is_empty() {
            let name = cfg
                .supplier(supplier_id)
                .and_then(|supplier| supplier.key(key_id))
                .map(|key| key.name.clone())
                .unwrap_or_else(|| key_id.to_string());
            return Some((name, agents.join("、")));
        }
    }
    None
}

fn clear_model_test(model: &mut SupplierModel) {
    model.last_test = None;
    model.latency_ms = None;
    model.last_ok = None;
    model.last_error = None;
    model.tested_key_id = None;
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

#[allow(clippy::too_many_arguments)]
pub fn record_model_test(
    cfg: &mut Config,
    supplier_id: &str,
    model: &str,
    protocol: Protocol,
    key_id: Option<&str>,
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
    let tested_key_id = key_id.map(str::to_string);

    if let Some(entry) = supplier
        .models
        .iter_mut()
        .find(|entry| entry.model == model)
    {
        entry.last_test = Some(Local::now());
        entry.latency_ms = Some(latency_ms);
        entry.last_ok = Some(ok);
        entry.last_error = error;
        entry.tested_key_id = tested_key_id;
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
            tested_key_id,
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

// ---------------------------------------------------------------------------
// 版本 4 及更早：单个 api_key
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub(crate) struct LegacySupplier {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) protocols: Vec<Protocol>,
    #[serde(default)]
    pub(crate) remark: Option<String>,
    #[serde(default)]
    pub(crate) last_probe: Option<DateTime<Local>>,
    #[serde(default)]
    pub(crate) models: Vec<SupplierModel>,
}

impl LegacySupplier {
    /// 单 Key 变成名为「默认」的账号，并设为默认账号。
    pub(crate) fn into_supplier(self) -> Supplier {
        let key = SupplierKey {
            id: new_id(),
            name: "默认".to_string(),
            api_key: self.api_key,
            remark: None,
        };
        Supplier {
            id: if self.id.is_empty() {
                new_id()
            } else {
                self.id
            },
            name: self.name,
            base_url: self.base_url,
            keys: vec![key.clone()],
            default_key_id: Some(key.id),
            protocols: self.protocols,
            remark: self.remark,
            last_probe: self.last_probe,
            models: self.models,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentConfig, AgentKind, AgentSlot};
    use crate::config::Env;

    fn key_input(name: &str, api_key: &str) -> SupplierKeyInput {
        SupplierKeyInput {
            id: None,
            name: name.to_string(),
            api_key: api_key.to_string(),
            remark: None,
        }
    }

    fn supplier_input(name: &str, url: &str) -> SupplierInput {
        SupplierInput {
            name: name.to_string(),
            base_url: url.to_string(),
            remark: None,
            protocols: None,
            keys: vec![key_input("个人", "sk-1")],
            default_key_id: None,
        }
    }

    #[test]
    fn add_supplier_creates_first_key_as_default() {
        let mut cfg = Config::default();
        let supplier = add_supplier(
            &mut cfg,
            supplier_input("relay", "https://api.example.com/v1"),
        )
        .unwrap();
        assert_eq!(supplier.keys.len(), 1);
        assert_eq!(supplier.keys[0].name, "个人");
        assert_eq!(supplier.keys[0].api_key, "sk-1");
        assert_eq!(
            supplier.default_key_id.as_deref(),
            Some(supplier.keys[0].id.as_str())
        );
    }

    #[test]
    fn add_supplier_requires_at_least_one_key() {
        let mut cfg = Config::default();
        let mut input = supplier_input("relay", "https://api.example.com/v1");
        input.keys = Vec::new();
        assert!(matches!(
            add_supplier(&mut cfg, input),
            Err(SupplierError::NoKeys)
        ));
    }

    #[test]
    fn rejects_duplicate_and_empty_key_names() {
        let mut cfg = Config::default();
        let mut input = supplier_input("relay", "https://api.example.com/v1");
        input.keys = vec![key_input("个人", "sk-1"), key_input("个人", "sk-2")];
        assert!(matches!(
            add_supplier(&mut cfg, input),
            Err(SupplierError::DuplicateKeyName(_))
        ));

        let mut input = supplier_input("relay", "https://api.example.com/v1");
        input.keys = vec![key_input("  ", "sk-1")];
        assert!(matches!(
            add_supplier(&mut cfg, input),
            Err(SupplierError::EmptyKeyName)
        ));
    }

    #[test]
    fn edit_keeps_existing_key_when_api_key_blank() {
        let mut cfg = Config::default();
        let supplier = add_supplier(
            &mut cfg,
            supplier_input("relay", "https://api.example.com/v1"),
        )
        .unwrap();
        let key_id = supplier.keys[0].id.clone();

        let updated = edit_supplier(
            &mut cfg,
            &supplier.id,
            SupplierInput {
                name: "relay".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                remark: None,
                protocols: None,
                keys: vec![SupplierKeyInput {
                    id: Some(key_id.clone()),
                    name: "个人".to_string(),
                    api_key: String::new(),
                    remark: None,
                }],
                default_key_id: None,
            },
        )
        .unwrap();
        assert_eq!(updated.keys[0].id, key_id);
        assert_eq!(updated.keys[0].api_key, "sk-1");
    }

    #[test]
    fn edit_adds_and_removes_keys() {
        let mut cfg = Config::default();
        let supplier = add_supplier(
            &mut cfg,
            supplier_input("relay", "https://api.example.com/v1"),
        )
        .unwrap();
        let first_id = supplier.keys[0].id.clone();

        let updated = edit_supplier(
            &mut cfg,
            &supplier.id,
            SupplierInput {
                name: "relay".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                remark: None,
                protocols: None,
                keys: vec![
                    SupplierKeyInput {
                        id: Some(first_id.clone()),
                        name: "个人".to_string(),
                        api_key: String::new(),
                        remark: None,
                    },
                    key_input("团队", "sk-2"),
                ],
                default_key_id: None,
            },
        )
        .unwrap();
        assert_eq!(updated.keys.len(), 2);
        assert_eq!(updated.keys[1].name, "团队");
    }

    #[test]
    fn cannot_remove_key_referenced_by_binding() {
        let mut cfg = Config::default();
        let supplier = add_supplier(
            &mut cfg,
            supplier_input("relay", "https://api.example.com/v1"),
        )
        .unwrap();
        let key_id = supplier.keys[0].id.clone();
        cfg.agent_mut(AgentKind::Codex).set(
            Env::Wsl,
            Some(AgentSlot {
                active: Some("b1".to_string()),
                bindings: vec![AgentConfig {
                    id: "b1".to_string(),
                    supplier_id: supplier.id.clone(),
                    key_id: Some(key_id.clone()),
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

        let result = edit_supplier(
            &mut cfg,
            &supplier.id,
            SupplierInput {
                name: "relay".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                remark: None,
                protocols: None,
                keys: vec![key_input("团队", "sk-2")],
                default_key_id: None,
            },
        );
        assert!(matches!(result, Err(SupplierError::KeyReferenced(_, _))));
    }

    #[test]
    fn deleting_default_key_falls_back_to_first_remaining() {
        let mut cfg = Config::default();
        let mut start = supplier_input("relay", "https://api.example.com/v1");
        start.keys = vec![key_input("个人", "sk-1"), key_input("团队", "sk-2")];
        let supplier = add_supplier(&mut cfg, start).unwrap();
        let first_id = supplier.keys[0].id.clone();
        let second_id = supplier.keys[1].id.clone();

        let updated = edit_supplier(
            &mut cfg,
            &supplier.id,
            SupplierInput {
                name: "relay".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                remark: None,
                protocols: None,
                keys: vec![SupplierKeyInput {
                    id: Some(second_id.clone()),
                    name: "团队".to_string(),
                    api_key: String::new(),
                    remark: None,
                }],
                default_key_id: None,
            },
        )
        .unwrap();
        assert_eq!(updated.keys.len(), 1);
        assert_ne!(updated.keys[0].id, first_id);
        assert_eq!(updated.default_key_id.as_deref(), Some(second_id.as_str()));
    }

    #[test]
    fn effective_key_prefers_explicit_then_default() {
        let mut cfg = Config::default();
        let mut start = supplier_input("relay", "https://api.example.com/v1");
        start.keys = vec![key_input("个人", "sk-1"), key_input("团队", "sk-2")];
        let supplier = add_supplier(&mut cfg, start).unwrap();
        let first_id = supplier.keys[0].id.clone();
        let second_id = supplier.keys[1].id.clone();

        assert_eq!(
            effective_key(&supplier, Some(&second_id)).map(|key| key.name.as_str()),
            Some("团队")
        );
        assert_eq!(
            effective_key(&supplier, None).map(|key| key.name.as_str()),
            Some("个人")
        );
        // 指向已删除的账号时回退到默认。
        assert_eq!(
            effective_key(&supplier, Some("gone")).map(|key| key.name.as_str()),
            Some("个人")
        );
        assert_ne!(first_id, second_id);
    }

    #[test]
    fn refuses_to_remove_referenced_supplier() {
        let mut cfg = Config::default();
        let supplier = add_supplier(
            &mut cfg,
            supplier_input("relay", "https://api.example.com/v1"),
        )
        .unwrap();
        cfg.agent_mut(AgentKind::Codex).set(
            Env::Wsl,
            Some(AgentSlot {
                active: Some("b1".to_string()),
                bindings: vec![AgentConfig {
                    id: "b1".to_string(),
                    supplier_id: supplier.id.clone(),
                    key_id: None,
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

    fn supplier_with_models(models: Vec<SupplierModel>) -> Supplier {
        Supplier {
            id: "s1".to_string(),
            name: "relay".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            keys: vec![SupplierKey {
                id: "k1".to_string(),
                name: "个人".to_string(),
                api_key: "sk-1".to_string(),
                remark: None,
            }],
            default_key_id: Some("k1".to_string()),
            protocols: vec![],
            remark: None,
            last_probe: None,
            models,
        }
    }

    #[test]
    fn merges_models_without_losing_results() {
        let mut supplier = supplier_with_models(vec![SupplierModel {
            model: "a".to_string(),
            last_ok: Some(true),
            ..SupplierModel::default()
        }]);
        merge_discovered_models(&mut supplier, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(supplier.models.len(), 2);
        assert_eq!(supplier.models[0].last_ok, Some(true));
    }

    #[test]
    fn replace_drops_retired_models_and_keeps_surviving_results() {
        let mut supplier = supplier_with_models(vec![
            SupplierModel {
                model: "keep".to_string(),
                last_ok: Some(true),
                latency_ms: Some(120),
                tested_key_id: Some("k1".to_string()),
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
        assert_eq!(supplier.models[0].tested_key_id.as_deref(), Some("k1"));
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

    #[test]
    fn legacy_supplier_becomes_single_default_key() {
        let legacy = LegacySupplier {
            id: "s1".to_string(),
            name: "relay".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "sk-old".to_string(),
            protocols: vec![Protocol::Responses],
            remark: None,
            last_probe: None,
            models: Vec::new(),
        };
        let supplier = legacy.into_supplier();
        assert_eq!(supplier.keys.len(), 1);
        assert_eq!(supplier.keys[0].name, "默认");
        assert_eq!(supplier.keys[0].api_key, "sk-old");
        assert_eq!(
            supplier.default_key_id.as_deref(),
            Some(supplier.keys[0].id.as_str())
        );
    }
}
