use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent::AgentKind;
use crate::config::Config;
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
        let remote_bindings = payload.agents.bindings(kind).to_vec();
        for mut binding in remote_bindings {
            if let Some(supplier_id) = id_map.get(&binding.supplier_id) {
                binding.supplier_id = supplier_id.clone();
            }
            if cfg.supplier(&binding.supplier_id).is_none() {
                continue;
            }
            let agent = cfg.agent_mut(kind);
            if let Some(local) = agent
                .bindings
                .iter_mut()
                .find(|local| local.id == binding.id)
            {
                *local = binding;
                report.bindings_updated += 1;
            } else {
                agent.bindings.push(binding);
                report.bindings_added += 1;
            }
        }
    }

    clear_invalid_active(cfg);
    report
}

pub fn overwrite_payload(cfg: &mut Config, payload: SyncPayload) -> MergeReport {
    let report = MergeReport {
        suppliers_added: payload.suppliers.len(),
        bindings_added: AgentKind::ALL
            .iter()
            .map(|kind| payload.agents.bindings(*kind).len())
            .sum(),
        ..MergeReport::default()
    };
    cfg.suppliers = payload.suppliers;
    payload.agents.apply_to(&mut cfg.agents);
    clear_invalid_active(cfg);
    report
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

fn clear_invalid_active(cfg: &mut Config) {
    for kind in AgentKind::ALL {
        let agent = cfg.agent_mut(kind);
        if let Some(id) = agent.active_app.clone()
            && agent.binding(&id).is_none()
        {
            agent.active_app = None;
        }
        if let Some(id) = agent.active_wsl.clone()
            && agent.binding(&id).is_none()
        {
            agent.active_wsl = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{BindingInput, add_binding};
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
        add_binding(
            &mut cfg,
            AgentKind::Codex,
            BindingInput {
                supplier_id: id.clone(),
                protocol: Protocol::Responses,
                model: "gpt-5".to_string(),
                ..BindingInput::default()
            },
        )
        .unwrap();
        (cfg, id)
    }

    #[test]
    fn merge_is_additive_and_remaps_supplier_ids() {
        let (mut local, local_supplier) = config_with_supplier();
        let (remote_cfg, _) = config_with_supplier();
        let mut payload = super::super::build_payload(&remote_cfg);
        payload.suppliers[0].id = "remote-id".to_string();
        for binding in &mut payload.agents.codex {
            binding.supplier_id = "remote-id".to_string();
            binding.model = "gpt-5-mini".to_string();
        }
        let report = merge_payload(&mut local, payload);
        assert_eq!(local.suppliers.len(), 1);
        assert_eq!(report.suppliers_updated, 1);
        assert_eq!(local.agents.codex.bindings.len(), 2);
        assert!(
            local
                .agents
                .codex
                .bindings
                .iter()
                .all(|binding| binding.supplier_id == local_supplier)
        );
    }

    #[test]
    fn overwrite_replaces_bindings_and_clears_active() {
        let (mut local, _) = config_with_supplier();
        local.agent_mut(AgentKind::Codex).active_app =
            Some(local.agents.codex.bindings[0].id.clone());
        let (remote_cfg, _) = config_with_supplier();
        let payload = super::super::build_payload(&remote_cfg);
        overwrite_payload(&mut local, payload);
        assert_eq!(local.agents.codex.bindings.len(), 1);
        assert!(local.agents.codex.active_app.is_none());
    }
}
