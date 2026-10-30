use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent::{AgentConfig, AgentKind, AgentSlot, Agents};
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
    pub bindings_added: usize,
    pub bindings_updated: usize,
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
            let Some(remote_slot) = payload.agents.get(kind).get(env).cloned() else {
                continue;
            };
            let mut remote_slot = remote_slot;
            for binding in &mut remote_slot.bindings {
                if let Some(supplier_id) = id_map.get(&binding.supplier_id) {
                    binding.supplier_id = supplier_id.clone();
                }
            }
            remote_slot
                .bindings
                .retain(|binding| cfg.supplier(&binding.supplier_id).is_some());
            if remote_slot.bindings.is_empty() {
                continue;
            }
            if remote_slot
                .active
                .as_deref()
                .is_some_and(|id| remote_slot.binding(id).is_none())
            {
                remote_slot.active = None;
            }

            let agent = cfg.agent_mut(kind);
            let local = agent.slot_mut(env);
            for remote in remote_slot.bindings {
                if let Some(existing) = local.binding_mut(&remote.id) {
                    *existing = remote;
                    report.bindings_updated += 1;
                } else {
                    local.bindings.push(remote);
                    report.bindings_added += 1;
                }
            }
            if local.active.is_none() {
                local.active = remote_slot.active;
            }
        }
    }

    report
}

pub fn overwrite_payload(cfg: &mut Config, payload: SyncPayload) -> MergeReport {
    let report = MergeReport {
        suppliers_added: payload.suppliers.len(),
        bindings_added: count_bindings(&payload.agents),
        ..MergeReport::default()
    };
    cfg.suppliers = payload.suppliers;
    cfg.agents = payload.agents;
    report
}

fn count_bindings(agents: &Agents) -> usize {
    AgentKind::ALL
        .iter()
        .map(|kind| {
            Env::ALL
                .iter()
                .filter_map(|env| agents.get(*kind).get(*env))
                .map(|slot: &AgentSlot| slot.bindings.len())
                .sum::<usize>()
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

#[allow(dead_code)]
fn _assert_binding_type(_: &AgentConfig) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Protocol;
    use crate::supplier::{SupplierInput, add_supplier};

    fn binding(id: &str, model: &str) -> AgentConfig {
        AgentConfig {
            id: id.to_string(),
            supplier_id: "local-supplier".to_string(),
            protocol: Protocol::Responses,
            model: model.to_string(),
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
            claude_models: None,
            grok_web_search: None,
        }
    }

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
        let mut first = binding("b1", "gpt-5");
        first.supplier_id = id.clone();
        cfg.agent_mut(AgentKind::Codex).set(
            Env::Wsl,
            Some(AgentSlot {
                active: Some("b1".to_string()),
                bindings: vec![first],
            }),
        );
        (cfg, id)
    }

    #[test]
    fn merge_remaps_supplier_ids_and_adds_bindings() {
        let (mut local, local_supplier) = config_with_supplier();
        let (remote_cfg, _) = config_with_supplier();
        let mut payload = super::super::build_payload(&remote_cfg);
        payload.suppliers[0].id = "remote-id".to_string();
        for binding in &mut payload.agents.codex.wsl.as_mut().unwrap().bindings {
            binding.supplier_id = "remote-id".to_string();
        }
        payload.agents.codex.wsl.as_mut().unwrap().bindings.push({
            let mut extra = binding("b2", "gpt-5-mini");
            extra.supplier_id = "remote-id".to_string();
            extra
        });

        let report = merge_payload(&mut local, payload);
        assert_eq!(local.suppliers.len(), 1);
        assert_eq!(report.suppliers_updated, 1);
        assert_eq!(report.bindings_added, 1);

        let slot = local.agent(AgentKind::Codex).get(Env::Wsl).unwrap();
        assert_eq!(slot.bindings.len(), 2);
        assert!(
            slot.bindings
                .iter()
                .all(|binding| binding.supplier_id == local_supplier)
        );
        assert_eq!(slot.active.as_deref(), Some("b1"));
    }

    #[test]
    fn overwrite_replaces_agents() {
        let (mut local, _) = config_with_supplier();
        let (remote_cfg, _) = config_with_supplier();
        let payload = super::super::build_payload(&remote_cfg);
        overwrite_payload(&mut local, payload);
        assert_eq!(local.suppliers.len(), 1);
        assert_eq!(
            local
                .agent(AgentKind::Codex)
                .get(Env::Wsl)
                .unwrap()
                .bindings
                .len(),
            1
        );
        assert!(local.agent(AgentKind::Codex).get(Env::Windows).is_none());
    }
}
