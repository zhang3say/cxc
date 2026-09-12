pub mod crypto;
pub mod gist;
pub mod merge;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{backup_config_file, save, Config, Provider};
use crypto::{decrypt_aes_gcm, encrypt_aes_gcm, CryptoError};
pub use gist::{GistClient, GistError};
use merge::merge_providers;

pub const CURRENT_SYNC_VERSION: u32 = 1;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Gist operation failed: {0}")]
    Gist(#[from] GistError),
    #[error("Crypto operation failed: {0}")]
    Crypto(#[from] CryptoError),
    #[error("JSON serialization or deserialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Sync payload is encrypted, but no secret key was provided")]
    MissingSecretKey,
    #[error("Missing Gist Token: Please provide a valid GitHub Personal Access Token")]
    MissingToken,
    #[error("Missing Gist ID: Please provide an existing Gist ID or choose to create one")]
    MissingGistId,
    #[error("Config error: {0}")]
    Config(#[from] crate::config::ConfigError),
}

/// User's persistent sync settings stored in config.yaml
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

/// The actual payload exchanged between devices across GitHub Gist.
/// Note: strictly contains cross-platform Provider definitions.
/// Local platform paths (WSL UNC paths), environment sources, and active selections remain on the local machine.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SyncPayload {
    pub version: u32,
    pub updated_at: DateTime<Local>,
    #[serde(default)]
    pub codex_providers: Vec<Provider>,
    #[serde(default)]
    pub claude_providers: Vec<Provider>,
    #[serde(default)]
    pub grok_providers: Vec<Provider>,
}

/// The JSON envelope stored in `cxc_sync.json` on GitHub Gist.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncEnvelope {
    pub version: u32,
    pub encrypted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ciphertext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<SyncPayload>,
}

/// Builds a SyncPayload from current Config (extracts all Providers).
pub fn build_payload(cfg: &Config) -> SyncPayload {
    SyncPayload {
        version: CURRENT_SYNC_VERSION,
        updated_at: Local::now(),
        codex_providers: cfg.codex_providers.clone(),
        claude_providers: cfg.claude_providers.clone(),
        grok_providers: cfg.grok_providers.clone(),
    }
}

/// Packs a SyncPayload into a JSON string for cloud upload.
/// If `secret_key` is provided and non-empty, payload is encrypted with AES-256-GCM.
pub fn pack_payload(payload: &SyncPayload, secret_key: Option<&str>) -> Result<String, SyncError> {
    let has_key = secret_key.map(|k| !k.trim().is_empty()).unwrap_or(false);

    if has_key {
        let key = secret_key.unwrap().trim();
        let json_bytes = serde_json::to_vec(payload)?;
        let (ciphertext, nonce) = encrypt_aes_gcm(key, &json_bytes)?;

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
            payload: Some(payload.clone()),
        };
        Ok(serde_json::to_string_pretty(&envelope)?)
    }
}

/// Unpacks raw JSON string from cloud Gist into a SyncPayload.
/// Decrypts if the envelope was encrypted.
pub fn unpack_payload(raw_json: &str, secret_key: Option<&str>) -> Result<SyncPayload, SyncError> {
    let envelope: SyncEnvelope = serde_json::from_str(raw_json)?;

    if envelope.encrypted {
        let key = match secret_key {
            Some(k) if !k.trim().is_empty() => k.trim(),
            _ => return Err(SyncError::MissingSecretKey),
        };

        let ciphertext = envelope
            .ciphertext
            .ok_or(CryptoError::DecryptError)?;
        let nonce = envelope
            .nonce
            .ok_or(CryptoError::DecryptError)?;

        let decrypted_bytes = decrypt_aes_gcm(key, &ciphertext, &nonce)?;
        let payload: SyncPayload = serde_json::from_slice(&decrypted_bytes)?;
        Ok(payload)
    } else {
        // Plaintext envelope
        envelope
            .payload
            .ok_or_else(|| SyncError::Json(serde::de::Error::custom("Missing payload in unencrypted envelope")))
    }
}

/// Overwrites local providers with the downloaded payload providers.
/// Note: Active selections and custom dirs are preserved.
pub fn apply_payload_overwrite(cfg: &mut Config, payload: SyncPayload) {
    cfg.codex_providers = payload.codex_providers;
    cfg.claude_providers = payload.claude_providers;
    cfg.grok_providers = payload.grok_providers;
}

/// Intelligently merges downloaded payload providers into local providers.
pub fn apply_payload_merge(cfg: &mut Config, payload: SyncPayload) {
    cfg.codex_providers = merge_providers(&cfg.codex_providers, &payload.codex_providers);
    cfg.claude_providers = merge_providers(&cfg.claude_providers, &payload.claude_providers);
    cfg.grok_providers = merge_providers(&cfg.grok_providers, &payload.grok_providers);
}

/// Validates GitHub Token and optionally verifies accessibility of an existing Gist.
/// Returns the authenticated GitHub username on success.
pub async fn test_gist_connection(token: &str, gist_id: Option<&str>) -> Result<String, SyncError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(SyncError::MissingToken);
    }
    let client = GistClient::new(token.to_string());
    let username = client.validate_token().await?;

    if let Some(id) = gist_id {
        let id = id.trim();
        if !id.is_empty() {
            // Check if the gist can be fetched
            let _ = client.get_sync_gist(id).await?;
        }
    }

    Ok(username)
}

/// Uploads the current local configuration to GitHub Gist.
/// If `gist_id` is empty, a new secret Gist is created and the returned ID is saved into `sync_config`.
pub async fn push_to_gist(
    cfg: &mut Config,
    sync_config: &mut SyncConfig,
) -> Result<String, SyncError> {
    let token = sync_config.gist_token.trim();
    if token.is_empty() {
        return Err(SyncError::MissingToken);
    }

    let client = GistClient::new(token.to_string());
    let payload = build_payload(cfg);
    let content = pack_payload(&payload, sync_config.secret_key.as_deref())?;

    let final_gist_id = if sync_config.gist_id.trim().is_empty() {
        // Create new Secret Gist
        let id = client.create_sync_gist(&content).await?;
        sync_config.gist_id = id.clone();
        id
    } else {
        let id = sync_config.gist_id.trim().to_string();
        client.update_sync_gist(&id, &content).await?;
        id
    };

    sync_config.last_synced_at = Some(Local::now());
    cfg.sync = Some(sync_config.clone());
    save(cfg)?;

    Ok(final_gist_id)
}

/// Pulls payload from GitHub Gist and overwrites local providers.
/// Backs up the local config before overwriting.
pub async fn pull_from_gist(
    cfg: &mut Config,
    sync_config: &mut SyncConfig,
) -> Result<SyncPayload, SyncError> {
    let token = sync_config.gist_token.trim();
    if token.is_empty() {
        return Err(SyncError::MissingToken);
    }
    let gist_id = sync_config.gist_id.trim();
    if gist_id.is_empty() {
        return Err(SyncError::MissingGistId);
    }

    let client = GistClient::new(token.to_string());
    let raw_content = client.get_sync_gist(gist_id).await?;
    let payload = unpack_payload(&raw_content, sync_config.secret_key.as_deref())?;

    // Create a local backup before overwriting
    backup_config_file()?;

    apply_payload_overwrite(cfg, payload.clone());
    sync_config.last_synced_at = Some(Local::now());
    cfg.sync = Some(sync_config.clone());
    save(cfg)?;

    Ok(payload)
}

/// Bidirectionally merges local and remote providers, then pushes merged result back to Gist.
/// Backs up the local config before merging.
pub async fn sync_and_merge(
    cfg: &mut Config,
    sync_config: &mut SyncConfig,
) -> Result<String, SyncError> {
    let token = sync_config.gist_token.trim();
    if token.is_empty() {
        return Err(SyncError::MissingToken);
    }

    let client = GistClient::new(token.to_string());

    if sync_config.gist_id.trim().is_empty() {
        // No gist ID yet: directly push local to newly created gist
        return push_to_gist(cfg, sync_config).await;
    }

    let gist_id = sync_config.gist_id.trim();
    // Pull remote content
    let raw_content = client.get_sync_gist(gist_id).await?;
    let remote_payload = unpack_payload(&raw_content, sync_config.secret_key.as_deref())?;

    // Backup local config before merging
    backup_config_file()?;

    // Perform merge locally
    apply_payload_merge(cfg, remote_payload);

    // Build merged payload and push to Gist
    let merged_payload = build_payload(cfg);
    let packed = pack_payload(&merged_payload, sync_config.secret_key.as_deref())?;
    client.update_sync_gist(gist_id, &packed).await?;

    sync_config.last_synced_at = Some(Local::now());
    cfg.sync = Some(sync_config.clone());
    save(cfg)?;

    Ok(gist_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_and_unpack_plaintext() {
        let payload = SyncPayload {
            version: 1,
            updated_at: Local::now(),
            codex_providers: vec![],
            claude_providers: vec![],
            grok_providers: vec![],
        };

        let packed = pack_payload(&payload, None).unwrap();
        let unpacked = unpack_payload(&packed, None).unwrap();
        assert_eq!(payload.version, unpacked.version);
    }

    #[test]
    fn test_pack_and_unpack_encrypted() {
        let payload = SyncPayload {
            version: 1,
            updated_at: Local::now(),
            codex_providers: vec![Provider {
                name: "test-p".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-12345678".to_string(),
                model: "gpt-4o".to_string(),
                wire_api: "responses".to_string(),
                remark: Some("primary".to_string()),
                last_test: None,
                latency_ms: None,
                last_ok: None,
                claude_models: None,
            }],
            claude_providers: vec![],
            grok_providers: vec![],
        };

        let password = "my_safe_password";
        let packed = pack_payload(&payload, Some(password)).unwrap();

        // Ensure sensitive key is not exposed in plaintext json
        assert!(!packed.contains("sk-12345678"));
        assert!(!packed.contains("test-p"));

        // Unpack with correct password
        let unpacked = unpack_payload(&packed, Some(password)).unwrap();
        assert_eq!(unpacked.codex_providers.len(), 1);
        assert_eq!(unpacked.codex_providers[0].api_key, "sk-12345678");

        // Unpack without password fails
        assert!(matches!(
            unpack_payload(&packed, None),
            Err(SyncError::MissingSecretKey)
        ));

        // Unpack with wrong password fails
        assert!(matches!(
            unpack_payload(&packed, Some("wrong_pass")),
            Err(SyncError::Crypto(CryptoError::DecryptError))
        ));
    }
}
