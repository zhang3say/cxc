pub mod crypto;
pub mod gist;
pub mod merge;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::agent::{AgentKind, Agents};
use crate::config::{self, Config};
use crate::supplier::{Supplier, SupplierModel};
use crypto::{decrypt_aes_gcm, encrypt_aes_gcm};
pub use gist::{GistClient, GistError};

pub const CURRENT_SYNC_VERSION: u32 = 3;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Gist 操作失败：{0}")]
    Gist(#[from] GistError),
    #[error("加解密失败：{0}")]
    Crypto(#[from] crypto::CryptoError),
    #[error("JSON 序列化或解析失败：{0}")]
    Json(#[from] serde_json::Error),
    #[error("同步载荷已加密，但没有提供口令")]
    MissingSecretKey,
    #[error("缺少 Gist Token")]
    MissingToken,
    #[error("缺少 Gist ID")]
    MissingGistId,
    #[error("配置错误：{0}")]
    Config(#[from] crate::config::ConfigError),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct SyncConfig {
    #[serde(default)]
    pub gist_token: String,
    #[serde(default)]
    pub gist_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<DateTime<Local>>,
}

/// 跨设备载荷：全局中转站 + 每个 Agent 每个环境的一份配置。
/// 不含自定义目录与本机测试结果。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SyncPayload {
    pub version: u32,
    pub updated_at: DateTime<Local>,
    #[serde(default)]
    pub suppliers: Vec<Supplier>,
    #[serde(default)]
    pub agents: Agents,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SyncEnvelope {
    pub version: u32,
    pub encrypted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

#[derive(Deserialize)]
struct LegacySyncPayloadV2 {
    #[serde(default)]
    updated_at: Option<DateTime<Local>>,
    #[serde(default)]
    suppliers: Vec<Supplier>,
    #[serde(default)]
    agents: config::LegacyAgentsV2,
}

#[derive(Deserialize)]
struct LegacySyncPayloadV1 {
    #[serde(default)]
    updated_at: Option<DateTime<Local>>,
    #[serde(default)]
    codex_providers: Vec<crate::migration::LegacyProvider>,
    #[serde(default)]
    claude_providers: Vec<crate::migration::LegacyProvider>,
    #[serde(default)]
    grok_providers: Vec<crate::migration::LegacyProvider>,
}

pub fn build_payload(cfg: &Config) -> SyncPayload {
    SyncPayload {
        version: CURRENT_SYNC_VERSION,
        updated_at: Local::now(),
        suppliers: cfg.suppliers.iter().map(sanitize_supplier).collect(),
        agents: cfg.agents.clone(),
    }
}

fn sanitize_supplier(supplier: &Supplier) -> Supplier {
    Supplier {
        id: supplier.id.clone(),
        name: supplier.name.clone(),
        base_url: supplier.base_url.clone(),
        api_key: supplier.api_key.clone(),
        protocols: supplier.protocols.clone(),
        remark: supplier.remark.clone(),
        last_probe: None,
        models: supplier
            .models
            .iter()
            .map(|model| SupplierModel {
                model: model.model.clone(),
                protocols: model.protocols.clone(),
                last_test: None,
                latency_ms: None,
                last_ok: None,
                last_error: None,
            })
            .collect(),
    }
}

pub fn pack_payload(payload: &SyncPayload, secret_key: Option<&str>) -> Result<String, SyncError> {
    let has_key = secret_key
        .map(|key| !key.trim().is_empty())
        .unwrap_or(false);
    if has_key {
        let json = serde_json::to_vec(payload)?;
        let (ciphertext, nonce) = encrypt_aes_gcm(secret_key.unwrap().trim(), &json)?;
        let envelope = SyncEnvelope {
            version: CURRENT_SYNC_VERSION,
            encrypted: true,
            nonce: Some(nonce),
            ciphertext: Some(ciphertext),
            payload: None,
        };
        Ok(serde_json::to_string_pretty(&envelope)?)
    } else {
        let envelope = SyncEnvelope {
            version: CURRENT_SYNC_VERSION,
            encrypted: false,
            nonce: None,
            ciphertext: None,
            payload: Some(serde_json::to_value(payload)?),
        };
        Ok(serde_json::to_string_pretty(&envelope)?)
    }
}

pub fn unpack_payload(raw: &str, secret_key: Option<&str>) -> Result<SyncPayload, SyncError> {
    let envelope: SyncEnvelope = serde_json::from_str(raw)?;
    let value = if envelope.encrypted {
        let key = secret_key
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or(SyncError::MissingSecretKey)?;
        let ciphertext = envelope.ciphertext.ok_or(SyncError::MissingSecretKey)?;
        let nonce = envelope.nonce.ok_or(SyncError::MissingSecretKey)?;
        let plaintext = decrypt_aes_gcm(key, &ciphertext, &nonce)?;
        serde_json::from_slice::<Value>(&plaintext)?
    } else {
        envelope.payload.ok_or_else(|| {
            SyncError::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "同步载荷为空",
            )))
        })?
    };

    parse_payload_value(value)
}

fn parse_payload_value(value: Value) -> Result<SyncPayload, SyncError> {
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(1) as u32;

    if version >= CURRENT_SYNC_VERSION {
        return Ok(serde_json::from_value(value)?);
    }

    if version == 2 {
        let legacy: LegacySyncPayloadV2 = serde_json::from_value(value)?;
        let cfg = Config {
            suppliers: legacy.suppliers,
            agents: config::agents_from_legacy(&legacy.agents),
            ..Config::default()
        };
        let mut payload = build_payload(&cfg);
        payload.updated_at = legacy.updated_at.unwrap_or_else(Local::now);
        return Ok(payload);
    }

    let legacy: LegacySyncPayloadV1 = serde_json::from_value(value)?;
    let mut cfg = Config::default();
    let mut report = crate::migration::ImportReport::default();
    let mut supplier_ids = std::collections::HashMap::new();
    crate::migration::import_agent(
        &mut cfg,
        AgentKind::Codex,
        &legacy.codex_providers,
        "",
        "",
        &mut supplier_ids,
        &mut report,
    );
    crate::migration::import_agent(
        &mut cfg,
        AgentKind::Claude,
        &legacy.claude_providers,
        "",
        "",
        &mut supplier_ids,
        &mut report,
    );
    crate::migration::import_agent(
        &mut cfg,
        AgentKind::Grok,
        &legacy.grok_providers,
        "",
        "",
        &mut supplier_ids,
        &mut report,
    );
    let mut payload = build_payload(&cfg);
    payload.updated_at = legacy.updated_at.unwrap_or_else(Local::now);
    Ok(payload)
}

pub async fn test_gist_connection(token: &str, gist_id: Option<&str>) -> Result<String, SyncError> {
    if token.trim().is_empty() {
        return Err(SyncError::MissingToken);
    }
    let client = GistClient::new(token.to_string());
    let login = client.validate_token().await?;
    if let Some(gist_id) = gist_id.map(str::trim).filter(|id| !id.is_empty()) {
        client.get_sync_gist(gist_id).await?;
    }
    Ok(login)
}

pub async fn push_to_gist(cfg: &Config, sync: &mut SyncConfig) -> Result<(), SyncError> {
    if sync.gist_token.trim().is_empty() {
        return Err(SyncError::MissingToken);
    }
    let client = GistClient::new(sync.gist_token.clone());
    let payload = build_payload(cfg);
    let content = pack_payload(&payload, sync.secret_key.as_deref())?;

    if sync.gist_id.trim().is_empty() {
        sync.gist_id = client.create_sync_gist(&content).await?;
    } else {
        client.update_sync_gist(&sync.gist_id, &content).await?;
    }
    sync.last_synced_at = Some(Local::now());
    Ok(())
}

pub async fn pull_from_gist(
    cfg: &mut Config,
    sync: &mut SyncConfig,
    mode: merge::SyncMode,
) -> Result<merge::MergeReport, SyncError> {
    if sync.gist_token.trim().is_empty() {
        return Err(SyncError::MissingToken);
    }
    if sync.gist_id.trim().is_empty() {
        return Err(SyncError::MissingGistId);
    }
    let client = GistClient::new(sync.gist_token.clone());
    let raw = client.get_sync_gist(&sync.gist_id).await?;
    let payload = unpack_payload(&raw, sync.secret_key.as_deref())?;
    let report = match mode {
        merge::SyncMode::Merge => merge::merge_payload(cfg, payload),
        merge::SyncMode::Overwrite => merge::overwrite_payload(cfg, payload),
    };
    sync.last_synced_at = Some(Local::now());
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentConfig, AgentKind};
    use crate::config::Env;
    use crate::protocol::Protocol;
    use crate::supplier::{SupplierInput, add_supplier};

    fn sample_config() -> Config {
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
        cfg.agent_mut(AgentKind::Codex).set(
            Env::Wsl,
            Some(AgentConfig {
                supplier_id: supplier.id,
                protocol: Protocol::Responses,
                model: "gpt-5".to_string(),
                reasoning_effort: Some("high".to_string()),
                plan_reasoning_effort: None,
                context_length: None,
                compact_ratio: None,
                claude_models: None,
                grok_web_search: None,
            }),
        );
        cfg
    }

    #[test]
    fn payload_roundtrip_without_encryption() {
        let cfg = sample_config();
        let payload = build_payload(&cfg);
        let raw = pack_payload(&payload, None).unwrap();
        let restored = unpack_payload(&raw, None).unwrap();
        assert_eq!(restored.version, 3);
        assert_eq!(restored.suppliers.len(), 1);
        assert_eq!(
            restored
                .agents
                .codex
                .wsl
                .as_ref()
                .map(|config| config.model.as_str()),
            Some("gpt-5")
        );
    }

    #[test]
    fn payload_roundtrip_with_encryption() {
        let cfg = sample_config();
        let payload = build_payload(&cfg);
        let raw = pack_payload(&payload, Some("secret")).unwrap();
        assert!(raw.contains("\"encrypted\": true"));
        let restored = unpack_payload(&raw, Some("secret")).unwrap();
        assert_eq!(restored.suppliers.len(), 1);
    }

    #[test]
    fn payload_excludes_test_results_and_custom_dirs() {
        let mut cfg = sample_config();
        cfg.suppliers[0].models.push(SupplierModel {
            model: "gpt-5".to_string(),
            last_ok: Some(true),
            latency_ms: Some(100),
            ..SupplierModel::default()
        });
        cfg.custom_dirs.codex.wsl = "/tmp/codex".to_string();
        let payload = build_payload(&cfg);
        assert!(payload.suppliers[0].models[0].last_ok.is_none());
        let raw = serde_json::to_string(&payload).unwrap();
        assert!(!raw.contains("/tmp/codex"));
    }

    #[test]
    fn reads_v2_payload_and_collapses_bindings() {
        let raw = r#"{
            "version": 2,
            "updated_at": "2026-10-30T12:00:00+08:00",
            "suppliers": [
                {
                    "id": "s1",
                    "name": "relay",
                    "base_url": "https://api.example.com/v1",
                    "api_key": "sk-1",
                    "protocols": ["responses"],
                    "models": []
                }
            ],
            "agents": {
                "codex": {
                    "active_wsl": "b1",
                    "bindings": [
                        {
                            "id": "b1",
                            "supplier_id": "s1",
                            "protocol": "responses",
                            "model": "gpt-5"
                        }
                    ]
                },
                "claude": { "bindings": [] },
                "grok": { "bindings": [] },
                "opencode": { "bindings": [] }
            }
        }"#;
        let payload = unpack_payload(
            &format!(r#"{{"version":2,"encrypted":false,"payload":{raw}}}"#),
            None,
        )
        .unwrap();
        assert_eq!(payload.version, 3);
        assert_eq!(
            payload
                .agents
                .codex
                .wsl
                .as_ref()
                .map(|config| config.model.as_str()),
            Some("gpt-5")
        );
    }
}
