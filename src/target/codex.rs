use std::fs;
use std::path::{Path, PathBuf};
use crate::target::{TargetAdapter, TargetConfig, TargetError};
use toml_edit::DocumentMut;
use serde_json;

pub struct CodexAdapter {
    codex_dir: PathBuf,
}

impl CodexAdapter {
    pub fn new() -> Result<Self, TargetError> {
        let codex_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_CODEX_DIR") {
            PathBuf::from(test_dir)
        } else {
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            home.join(".codex")
        };
        Ok(Self { codex_dir })
    }

    pub fn new_with_dir<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            codex_dir: dir.as_ref().to_path_buf(),
        }
    }

    fn config_path(&self) -> PathBuf {
        self.codex_dir.join("config.toml")
    }

    fn auth_path(&self) -> PathBuf {
        self.codex_dir.join("auth.json")
    }
}

impl TargetAdapter for CodexAdapter {
    fn name(&self) -> &'static str {
        "Codex"
    }

    fn read(&self) -> Result<TargetConfig, TargetError> {
        let config_path = self.config_path();
        let toml_data = fs::read_to_string(&config_path).map_err(|err| TargetError::ReadError {
            path: config_path.clone(),
            source: err,
        })?;

        let doc = toml_data.parse::<DocumentMut>().map_err(|e| TargetError::ParseError(e.to_string()))?;

        let model = doc.get("model")
            .and_then(|item| item.as_str())
            .unwrap_or("")
            .to_string();

        let mut base_url = String::new();
        let mut wire_api = String::new();

        if let Some(providers) = doc.get("model_providers").and_then(|p| p.as_table()) {
            if let Some(codex) = providers.get("codex").and_then(|c| c.as_table()) {
                if let Some(bu) = codex.get("base_url").and_then(|b| b.as_str()) {
                    base_url = bu.to_string();
                }
                if let Some(wa) = codex.get("wire_api").and_then(|w| w.as_str()) {
                    wire_api = wa.to_string();
                }
            }
        }

        // Read API key from auth.json
        let auth_path = self.auth_path();
        let auth_data = fs::read_to_string(&auth_path).map_err(|err| TargetError::ReadError {
            path: auth_path.clone(),
            source: err,
        })?;

        let auth: serde_json::Value = serde_json::from_str(&auth_data).map_err(|e| TargetError::ParseError(e.to_string()))?;
        let api_key = auth.get("OPENAI_API_KEY")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(TargetConfig {
            base_url,
            api_key,
            model,
            wire_api,
        })
    }

    fn write(&self, config: &TargetConfig) -> Result<(), TargetError> {
        // Backup files first
        backup(&self.config_path())?;
        backup(&self.auth_path())?;

        // Update config.toml
        let config_path = self.config_path();
        let toml_data = fs::read_to_string(&config_path).map_err(|err| TargetError::ReadError {
            path: config_path.clone(),
            source: err,
        })?;

        let mut doc = toml_data.parse::<DocumentMut>().map_err(|e| TargetError::ParseError(e.to_string()))?;

        doc["model"] = toml_edit::value(&config.model);
        
        // Ensure model_providers.codex path exists and update fields
        doc["model_providers"]["codex"]["base_url"] = toml_edit::value(&config.base_url);
        doc["model_providers"]["codex"]["wire_api"] = toml_edit::value(&config.wire_api);
        doc["model_providers"]["codex"]["name"] = toml_edit::value("codex");
        doc["model_providers"]["codex"]["requires_openai_auth"] = toml_edit::value(true);

        // Validate by re-parsing
        let out = doc.to_string();
        let _ = out.parse::<DocumentMut>().map_err(|e| TargetError::SerializeError(format!("Written TOML is invalid: {}", e)))?;

        write_secure_file(&config_path, out.as_bytes()).map_err(|err| TargetError::WriteError {
            path: config_path.clone(),
            source: err,
        })?;

        // Update auth.json
        let auth_path = self.auth_path();
        let auth_data = fs::read_to_string(&auth_path).map_err(|err| TargetError::ReadError {
            path: auth_path.clone(),
            source: err,
        })?;

        let mut auth: serde_json::Value = serde_json::from_str(&auth_data).map_err(|e| TargetError::ParseError(e.to_string()))?;
        auth["OPENAI_API_KEY"] = serde_json::Value::String(config.api_key.clone());

        let out_json = serde_json::to_string_pretty(&auth).map_err(|e| TargetError::SerializeError(e.to_string()))?;
        write_secure_file(&auth_path, out_json.as_bytes()).map_err(|err| TargetError::WriteError {
            path: auth_path.clone(),
            source: err,
        })?;

        Ok(())
    }
}

fn backup(path: &Path) -> Result<(), TargetError> {
    if !path.exists() {
        return Ok(()); // nothing to backup
    }
    let data = fs::read(path).map_err(|err| TargetError::ReadError {
        path: path.to_path_buf(),
        source: err,
    })?;
    let mut backup_path = path.to_path_buf();
    let new_name = match path.file_name() {
        Some(name) => {
            let mut s = name.to_os_string();
            s.push(".bak");
            s
        }
        None => return Ok(()),
    };
    backup_path.set_file_name(new_name);

    write_secure_file(&backup_path, &data).map_err(|err| TargetError::WriteError {
        path: backup_path.clone(),
        source: err,
    })?;

    Ok(())
}

fn write_secure_file<P: AsRef<Path>>(path: P, contents: &[u8]) -> std::io::Result<()> {
    fs::write(&path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const MINIMAL_CONFIG_TOML: &str = r#"approval_policy = "never"
model = "gpt-4"
model_provider = "codex"

[model_providers.codex]
base_url = "https://old.example.com/v1"
name = "codex"
requires_openai_auth = true
wire_api = "responses"

[features]
guardian_approval = true
memories = true

[mcp_servers.context7]
args = ["-y", "@upstash/context7-mcp"]
command = "npx"

[projects."/home/user"]
trust_level = "trusted"
"#;

    const MINIMAL_AUTH_JSON: &str = r#"{
  "auth_mode": "apikey",
  "OPENAI_API_KEY": "sk-old-key"
}"#;

    fn setup_codex_dir() -> (TempDir, CodexAdapter) {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), MINIMAL_CONFIG_TOML).unwrap();
        fs::write(dir.path().join("auth.json"), MINIMAL_AUTH_JSON).unwrap();
        let adapter = CodexAdapter::new_with_dir(dir.path());
        (dir, adapter)
    }

    #[test]
    fn test_name_returns_codex() {
        let (_dir, adapter) = setup_codex_dir();
        assert_eq!(adapter.name(), "Codex");
    }

    #[test]
    fn test_read() {
        let (_dir, adapter) = setup_codex_dir();
        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.model, "gpt-4");
        assert_eq!(cfg.base_url, "https://old.example.com/v1");
        assert_eq!(cfg.api_key, "sk-old-key");
        assert_eq!(cfg.wire_api, "responses");
    }

    #[test]
    fn test_write() {
        let (dir, adapter) = setup_codex_dir();
        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new-key".to_string(),
            model: "gpt-5".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, new_cfg.base_url);
        assert_eq!(got.api_key, new_cfg.api_key);
        assert_eq!(got.model, new_cfg.model);

        // Verify backups created
        assert!(dir.path().join("config.toml.bak").exists());
        assert!(dir.path().join("auth.json.bak").exists());
    }

    #[test]
    fn test_write_preserves_unrelated_sections() {
        let (_dir, adapter) = setup_codex_dir();
        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new".to_string(),
            model: "gpt-5".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        let checks = ["guardian_approval", "memories", "context7", "trusted"];
        for check in checks.iter() {
            assert!(content.contains(check), "Unrelated section key '{}' was lost", check);
        }
    }

    #[test]
    fn test_write_creates_backup_with_original_content() {
        let (dir, adapter) = setup_codex_dir();
        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new".to_string(),
            model: "gpt-5".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let bak_data = fs::read_to_string(dir.path().join("auth.json.bak")).unwrap();
        let bak: serde_json::Value = serde_json::from_str(&bak_data).unwrap();
        assert_eq!(bak["OPENAI_API_KEY"], "sk-old-key");
    }

    #[test]
    fn test_write_produces_valid_toml() {
        let (_dir, adapter) = setup_codex_dir();
        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new".to_string(),
            model: "gpt-5".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let data = fs::read_to_string(adapter.config_path()).unwrap();
        let parsed = data.parse::<DocumentMut>();
        assert!(parsed.is_ok(), "Written TOML is invalid");
    }
}
