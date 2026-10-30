use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent::{AgentKind, Agents};
use crate::config::{Config, Env};
use crate::protocol::Protocol;
use crate::supplier::{Supplier, SupplierModel};

use super::SyncPayload;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    Merge,
    Overwrite,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct MergeReport {
    pub suppliers_added: usize,
    pub suppliers_updated: usize,
    pub configs_added: usize,
    pub configs_updated: usize,
}

pub fn merge_payload(cfg: &mut Config, payload: SyncPayload) -> MergeReport {
    let mut report = MergeReport::default();
    let mut id_map: HashMap<String, String> = HashMap::new();

    for remote in payload.suppliers {
        let local_id =
            if let Some(local) = cfg.suppliers.iter_mut().find(|local| local.id == remote.id) {
                merge_supplier(local, &remote);
                report.suppliers_updated += 1;
                local.id.clone()
            } else if let Some(local) = cfg
                .suppliers
                .iter_mut()
                .find(|local| same_endpoint(local, &remote))
            {
                merge_supplier(local, &remote);
                report.suppliers_updated += 1;
                local.id.clone()
            } else {
                cfg.suppliers.push(remote.clone());
                report.suppliers_added += 1;
                remote.id.clone()
            };
        id_map.insert(remote.id, local_id);
    }

    for kind in AgentKind::ALL {
        for env in Env::ALL {
            let Some(remote) = payload.agents.get(kind).get(env).cloned() else {
                continue;
            };
            let mut remote = remote;
            if let Some(supplier_id) = id_map.get(&remote.supplier_id) {
                remote.supplier_id = supplier_id.clone();
            }
            if cfg.supplier(&remote.supplier_id).is_none() {
                continue;
            }
            let agent = cfg.agent_mut(kind);
            if agent.get(env).is_some() {
                report.configs_updated += 1;
            } else {
                report.configs_added += 1;
            }
            agent.set(env, Some(remote));
        }
    }

    report
}

pub fn overwrite_payload(cfg: &mut Config, payload: SyncPayload) -> MergeReport {
    let report = MergeReport {
        suppliers_added: payload.suppliers.len(),
        configs_added: count_configs(&payload.agents),
        ..MergeReport::default()
    };
    cfg.suppliers = payload.suppliers;
    cfg.agents = payload.agents;
    report
}

fn count_configs(agents: &Agents) -> usize {
    AgentKind::ALL
        .iter()
        .map(|kind| {
            Env::ALL
                .iter()
                .filter(|env| agents.get(*kind).get(**env).is_some())
                .count()
        })
        .sum()
}

fn merge_supplier(local: &mut Supplier, remote: &Supplier) {
    local.name = remote.name.clone();
    local.base_url = remote.base_url.clone();
    local.api_key = remote.api_key.clone();
    local.remark = remote.remark.clone();
    local.protocols = Protocol::normalize_all(
        local
            .protocols
            .iter()
            .copied()
            .chain(remote.protocols.iter().copied()),
    );

    for remote_model in &remote.models {
        if let Some(local_model) = local
            .models
            .iter_mut()
            .find(|model| model.model == remote_model.model)
        {
            local_model.protocols = Protocol::normalize_all(
                local_model
                    .protocols
                    .iter()
                    .copied()
                    .chain(remote_model.protocols.iter().copied()),
            );
        } else {
            local.models.push(SupplierModel {
                model: remote_model.model.clone(),
                protocols: remote_model.protocols.clone(),
                ..SupplierModel::default()
            });
        }
    }
}

fn same_endpoint(left: &Supplier, right: &Supplier) -> bool {
    left.base_url.trim().trim_end_matches('/') == right.base_url.trim().trim_end_matches('/')
        && left.api_key.trim() == right.api_key.trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentConfig;
    use crate::protocol::Protocol;
    use crate::supplier::{SupplierInput, add_supplier};

    fn config_with_supplier() -> (Config, String) {
        let mut cfg = Config::default();
        let supplier = add_supplier(
            &mut cfg,
            SupplierInput {
                name: "relay".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "sk-1".to_string(),
                remark: None,
                protocols: Some(vec![Protocol::Responses]),
            },
        )
        .unwrap();
        let id = supplier.id.clone();
        cfg.agent_mut(AgentKind::Codex).set(
            Env::Wsl,
            Some(AgentConfig {
                supplier_id: id.clone(),
                protocol: Protocol::Responses,
                model: "gpt-5".to_string(),
                reasoning_effort: None,
                plan_reasoning_effort: None,
                context_length: None,
                compact_ratio: None,
                claude_models: None,
                grok_web_search: None,
            }),
        );
        (cfg, id)
    }

    #[test]
    fn merge_remaps_supplier_ids_and_updates_configs() {
        let (mut local, local_supplier) = config_with_supplier();
        let (remote_cfg, _) = config_with_supplier();
        let mut payload = super::super::build_payload(&remote_cfg);
        payload.suppliers[0].id = "remote-id".to_string();
        if let Some(config) = payload.agents.codex.wsl.as_mut() {
            config.supplier_id = "remote-id".to_string();
            config.model = "gpt-5-mini".to_string();
        }
        payload.agents.codex.windows = payload.agents.codex.wsl.clone();

        let report = merge_payload(&mut local, payload);
        assert_eq!(local.suppliers.len(), 1);
        assert_eq!(report.suppliers_updated, 1);
        assert_eq!(report.configs_updated, 1);
        assert_eq!(report.configs_added, 1);
        assert_eq!(
            local
                .config_of(AgentKind::Codex, Env::Wsl)
                .map(|config| config.model.as_str()),
            Some("gpt-5-mini")
        );
        assert!(
            local
                .agents
                .codex
                .wsl
                .as_ref()
                .is_some_and(|config| config.supplier_id == local_supplier)
        );
    }

    #[test]
    fn overwrite_replaces_agents() {
        let (mut local, _) = config_with_supplier();
        let (remote_cfg, _) = config_with_supplier();
        let payload = super::super::build_payload(&remote_cfg);
        overwrite_payload(&mut local, payload);
        assert_eq!(local.suppliers.len(), 1);
        assert!(local.agents.codex.wsl.is_some());
        assert!(local.agents.codex.windows.is_none());
    }
}
