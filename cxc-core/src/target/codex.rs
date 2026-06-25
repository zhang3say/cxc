use crate::target::{TargetAdapter, TargetConfig, TargetError};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::sync::OnceLock;
use toml_edit::DocumentMut;

pub struct CodexAdapter {
    codex_dir: PathBuf,
}

impl CodexAdapter {
    pub fn new() -> Result<Self, TargetError> {
        let codex_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_CODEX_DIR") {
            PathBuf::from(test_dir)
        } else {
            // Load global cxc config to check for custom codex directory and source settings
            let cfg = crate::config::load().ok();
            Self::resolve_codex_dir(cfg.as_ref())?
        };
        Ok(Self { codex_dir })
    }

    pub fn new_from_config(cfg: &crate::config::Config) -> Result<Self, TargetError> {
        let codex_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_CODEX_DIR") {
            PathBuf::from(test_dir)
        } else {
            Self::resolve_codex_dir(Some(cfg))?
        };
        Ok(Self { codex_dir })
    }

    fn resolve_codex_dir(cfg: Option<&crate::config::Config>) -> Result<PathBuf, TargetError> {
        let source = cfg
            .map(crate::config::effective_source)
            .unwrap_or(crate::config::SOURCE_WSL);

        let custom_dir = cfg.and_then(|c| {
            if !c.codex_custom_dir.is_empty() {
                let mut path_str = c.codex_custom_dir.clone();

                // If running on Linux (e.g., WSL) and user configured a Windows-style path (like C:\...),
                // automatically convert it to a WSL mount path (like /mnt/c/...)
                #[cfg(target_os = "linux")]
                {
                    if path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
                        let drive = path_str.chars().next().unwrap().to_ascii_lowercase();
                        let remaining = &path_str[2..];
                        let normalized = remaining.replace('\\', "/");
                        path_str = format!("/mnt/{}{}", drive, normalized);
                    } else {
                        path_str = path_str.replace('\\', "/");
                    }
                }

                return Some(PathBuf::from(path_str));
            }
            None
        });

        if let Some(dir) = custom_dir {
            Ok(dir)
        } else {
            // No custom dir: use smart defaults based on codex_source
            Self::default_codex_dir(source)
        }
    }

    /// Resolve the default Codex config directory based on `codex_source`.
    ///
    /// - `"wsl"` on Linux  → `~/.codex` (local WSL home)
    /// - `"app"` on Linux  → `/mnt/c/Users/<username>/.codex` (Windows host via WSL mount)
    /// - `"app"` on Windows → `%USERPROFILE%\.codex`
    /// - `"wsl"` on Windows → try `\\wsl.localhost\<distro>\home\<user>\.codex`
    fn default_codex_dir(source: &str) -> Result<PathBuf, TargetError> {
        #[cfg(target_os = "linux")]
        {
            if source == "app" {
                // Try to find the Windows home directory via common WSL mount points
                if let Some(win_dir) = Self::detect_windows_codex_dir() {
                    return Ok(win_dir);
                }
                // Don't fall back to local Linux home if app not reachable, otherwise it causes cross-contamination
                return Err(TargetError::ParseError("Unable to locate Windows home directory automatically from within WSL. Please specify a custom directory.".to_string()));
            }
            // source == "wsl" or fallback: use native Linux home
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            return Ok(home.join(".codex"));
        }

        #[cfg(target_os = "windows")]
        {
            if source == "wsl" {
                // Try to find WSL home directory via UNC path
                if let Some(wsl_dir) = Self::cached_wsl_codex_dir() {
                    return Ok(wsl_dir);
                }
                // Don't fall back to local Windows home if WSL not reachable, otherwise it causes cross-contamination
                return Err(TargetError::ParseError("Unable to locate WSL home directory automatically. Please specify a custom directory.".to_string()));
            }
            // source == "app" or fallback: use native Windows home
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            return Ok(home.join(".codex"));
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = source;
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            Ok(home.join(".codex"))
        }
    }

    /// Detect Windows Codex directory from within WSL via /mnt/c/Users/<username>/.codex
    #[cfg(target_os = "linux")]
    fn detect_windows_codex_dir() -> Option<PathBuf> {
        let username = std::env::var("USER").ok()?;
        // Try the most common mount point first
        let candidate = PathBuf::from(format!("/mnt/c/Users/{}/.codex", username));
        if candidate.exists() {
            return Some(candidate);
        }
        let home = PathBuf::from(format!("/mnt/c/Users/{}", username));
        if home.exists() {
            return Some(candidate);
        }
        // Try scanning /mnt/c/Users/ for a matching directory
        if let Ok(entries) = fs::read_dir("/mnt/c/Users") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name == username.to_lowercase() {
                    let codex_dir = entry.path().join(".codex");
                    return Some(codex_dir);
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    fn cached_wsl_codex_dir() -> Option<PathBuf> {
        static WSL_CODEX_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
        WSL_CODEX_DIR.get_or_init(Self::detect_wsl_codex_dir).clone()
    }

    /// Detect WSL Codex directory from Windows via \\wsl.localhost\<distro>\home\<user>\.codex
    #[cfg(target_os = "windows")]
    fn detect_wsl_codex_dir() -> Option<PathBuf> {
        let mut first_valid_home: Option<PathBuf> = None;

        // 0. Try using wsl.exe to get the home path dynamically
        if let Ok(output) = std::process::Command::new("wsl")
            .args(["-e", "sh", "-lc", "wslpath -w \"$HOME\""])
            .output()
        {
            if output.status.success() {
                if let Some(candidate) =
                    Self::wsl_home_output_to_config_dir(&output.stdout, ".codex")
                {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                    first_valid_home = Some(candidate);
                }
            }
        }

        // 1. Scan UNC roots instead of assuming Windows and WSL usernames match.
        let mut scan_unc_path = |base_path: &str| -> Option<PathBuf> {
            let base = Path::new(base_path);
            if !base.exists() {
                return None;
            }

            if let Ok(distro_entries) = fs::read_dir(base) {
                for distro_entry in distro_entries.flatten() {
                    let home_path = distro_entry.path().join("home");
                    if !home_path.exists() {
                        continue;
                    }

                    if let Ok(user_entries) = fs::read_dir(&home_path) {
                        for user_entry in user_entries.flatten() {
                            let user_path = user_entry.path();
                            let codex_dir = user_path.join(".codex");
                            if codex_dir.exists() {
                                return Some(codex_dir);
                            }
                            if first_valid_home.is_none() && user_path.exists() {
                                first_valid_home = Some(codex_dir);
                            }
                        }
                    }
                }
            }
            None
        };

        if let Some(path) = scan_unc_path("\\\\wsl.localhost") {
            return Some(path);
        }

        if let Some(path) = scan_unc_path("\\\\wsl$") {
            return Some(path);
        }

        // 2. Last fallback: try common distro names with the Windows username.
        if let Ok(username) = std::env::var("USERNAME") {
            let username = username.to_lowercase();
            let distros = [
                "Ubuntu",
                "Debian",
                "openSUSE-Leap",
                "kali-linux",
                "Ubuntu-22.04",
                "Ubuntu-24.04",
            ];
            for distro in &distros {
                let candidate = PathBuf::from(format!(
                    "\\\\wsl.localhost\\{}\\home\\{}\\.codex",
                    distro, username
                ));
                if candidate.exists() {
                    return Some(candidate);
                }
                let home =
                    PathBuf::from(format!("\\\\wsl.localhost\\{}\\home\\{}", distro, username));
                if first_valid_home.is_none() && home.exists() {
                    first_valid_home = Some(candidate);
                }

                let candidate =
                    PathBuf::from(format!("\\\\wsl$\\{}\\home\\{}\\.codex", distro, username));
                if candidate.exists() {
                    return Some(candidate);
                }
                let home = PathBuf::from(format!("\\\\wsl$\\{}\\home\\{}", distro, username));
                if first_valid_home.is_none() && home.exists() {
                    first_valid_home = Some(candidate);
                }
            }
        }

        first_valid_home
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn wsl_home_output_to_config_dir(output: &[u8], config_dir_name: &str) -> Option<PathBuf> {
        let path = String::from_utf8_lossy(output).trim().to_string();
        if path.is_empty() || path == "~" || path.contains("\\~") || path.contains("/~") {
            return None;
        }
        Some(PathBuf::from(path).join(config_dir_name))
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

        let doc = toml_data
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::ParseError(e.to_string()))?;

        let model = doc
            .get("model")
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

        let auth: serde_json::Value =
            serde_json::from_str(&auth_data).map_err(|e| TargetError::ParseError(e.to_string()))?;
        let api_key = auth
            .get("OPENAI_API_KEY")
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
        // Ensure the codex directory exists
        if !self.codex_dir.exists() {
            fs::create_dir_all(&self.codex_dir).map_err(|err| TargetError::WriteError {
                path: self.codex_dir.clone(),
                source: err,
            })?;
        }

        // Backup files first
        backup(&self.config_path())?;
        backup(&self.auth_path())?;

        // Update config.toml (read existing or use minimal scaffold)
        let config_path = self.config_path();
        let toml_data = if config_path.exists() {
            fs::read_to_string(&config_path).map_err(|err| TargetError::ReadError {
                path: config_path.clone(),
                source: err,
            })?
        } else {
            // Minimal Codex config scaffold for first-time write
            String::from(concat!(
                "model_provider = \"codex\"\n",
                "model = \"\"\n",
                "\n",
                "[model_providers.codex]\n",
                "base_url = \"\"\n",
                "name = \"codex\"\n",
                "requires_openai_auth = false\n",
                "wire_api = \"responses\"\n",
            ))
        };

        let mut doc = toml_data
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::ParseError(e.to_string()))?;

        doc["model"] = toml_edit::value(&config.model);
        // Point Codex at our fixed "codex" provider entry
        doc["model_provider"] = toml_edit::value("codex");

        // Promote any inline tables inside [model_providers] to standard TOML sections,
        // so the output uses `[model_providers.codex]` format instead of `codex = { ... }`.
        promote_model_providers_to_tables(&mut doc);

        // Ensure model_providers.codex path exists and update fields
        doc["model_providers"]["codex"]["base_url"] = toml_edit::value(&config.base_url);
        doc["model_providers"]["codex"]["wire_api"] = toml_edit::value(&config.wire_api);
        doc["model_providers"]["codex"]["name"] = toml_edit::value("codex");
        // false = disable OpenAI-native auth check; required for third-party relay gateways
        doc["model_providers"]["codex"]["requires_openai_auth"] = toml_edit::value(false);

        // Validate by re-parsing
        let out = doc.to_string();
        let _ = out
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::SerializeError(format!("Written TOML is invalid: {}", e)))?;

        write_secure_file(&config_path, out.as_bytes()).map_err(|err| TargetError::WriteError {
            path: config_path.clone(),
            source: err,
        })?;

        // Update auth.json (read existing or use minimal scaffold)
        let auth_path = self.auth_path();
        let auth_data = if auth_path.exists() {
            fs::read_to_string(&auth_path).map_err(|err| TargetError::ReadError {
                path: auth_path.clone(),
                source: err,
            })?
        } else {
            // Minimal auth scaffold
            String::from("{\"auth_mode\": \"apikey\", \"OPENAI_API_KEY\": \"\"}")
        };

        let mut auth: serde_json::Value =
            serde_json::from_str(&auth_data).map_err(|e| TargetError::ParseError(e.to_string()))?;
        auth["OPENAI_API_KEY"] = serde_json::Value::String(config.api_key.clone());

        let out_json = serde_json::to_string_pretty(&auth)
            .map_err(|e| TargetError::SerializeError(e.to_string()))?;
        write_secure_file(&auth_path, out_json.as_bytes()).map_err(|err| {
            TargetError::WriteError {
                path: auth_path.clone(),
                source: err,
            }
        })?;

        Ok(())
    }
}

/// Promote all inline tables inside `[model_providers]` to standard TOML table sections.
///
/// toml_edit preserves the original format when editing existing documents, so if the
/// existing config.toml has `codex = { ... }` (inline table), modifications stay in that
/// format. This function converts all inline-table entries under `model_providers` into
/// proper subtables, resulting in `[model_providers.codex]` section headers on output.
fn promote_model_providers_to_tables(doc: &mut DocumentMut) {
    use toml_edit::{Item, Table};

    // Extract existing model_providers item, if any
    let existing = match doc.remove("model_providers") {
        Some(item) => item,
        None => return, // nothing to promote
    };

    // Collect all provider entries as (key, Table) pairs
    let entries: Vec<(String, Table)> = match &existing {
        Item::Table(outer) => {
            outer
                .iter()
                .filter_map(|(k, v)| {
                    // Convert inline table to standard table
                    let tbl = match v {
                        Item::Value(toml_edit::Value::InlineTable(it)) => {
                            let mut t = it.clone().into_table();
                            t.set_implicit(false);
                            t
                        }
                        Item::Table(t) => t.clone(),
                        _ => return None,
                    };
                    Some((k.to_string(), tbl))
                })
                .collect()
        }
        _ => return, // unexpected shape; leave as-is
    };

    if entries.is_empty() {
        // Re-insert unchanged if nothing to promote
        doc["model_providers"] = existing;
        return;
    }

    // Build a new standard table for model_providers
    let mut outer = Table::new();
    outer.set_implicit(true); // don't emit `[model_providers]` header line itself
    for (key, tbl) in entries {
        outer.insert(&key, Item::Table(tbl));
    }
    doc.insert("model_providers", Item::Table(outer));
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

        // Verify model_provider is set to "codex"
        let content = fs::read_to_string(adapter.config_path()).unwrap();
        assert!(
            content.contains("model_provider = \"codex\""),
            "model_provider should be set to codex"
        );

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
            assert!(
                content.contains(check),
                "Unrelated section key '{}' was lost",
                check
            );
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

    #[test]
    fn test_write_creates_files_from_scratch() {
        // Test that write() works even when config.toml and auth.json don't exist
        let dir = tempfile::tempdir().unwrap();
        // Do NOT create config.toml or auth.json
        let adapter = CodexAdapter::new_with_dir(dir.path());

        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new-key".to_string(),
            model: "gpt-5".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        // Verify files were created and are readable
        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, new_cfg.base_url);
        assert_eq!(got.api_key, new_cfg.api_key);
        assert_eq!(got.model, new_cfg.model);
        assert_eq!(got.wire_api, new_cfg.wire_api);

        // No backups should exist since originals didn't exist
        assert!(!dir.path().join("config.toml.bak").exists());
        assert!(!dir.path().join("auth.json.bak").exists());
    }

    #[test]
    fn test_write_creates_directory_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("sub").join("codex");
        let adapter = CodexAdapter::new_with_dir(&nested);

        let new_cfg = TargetConfig {
            base_url: "https://example.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-4".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        assert!(nested.join("config.toml").exists());
        assert!(nested.join("auth.json").exists());
        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, "https://example.com/v1");
    }

    #[test]
    fn test_write_expands_inline_table_to_standard_toml() {
        // Simulate a config.toml written by Windows Codex Desktop App which uses inline format:
        // `codex = { base_url = "...", ... }` instead of `[model_providers.codex]`
        let inline_config = concat!(
            "model_provider = \"custom\"\n",
            "model = \"gpt-4o\"\n",
            "\n",
            "[model_providers]\n",
            "codex = { base_url = \"https://old.example.com/v1\", name = \"codex\", requires_openai_auth = true, wire_api = \"responses\" }\n",
        );

        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), inline_config).unwrap();
        fs::write(
            dir.path().join("auth.json"),
            r#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-old"}"#,
        )
        .unwrap();
        let adapter = CodexAdapter::new_with_dir(dir.path());

        let new_cfg = TargetConfig {
            base_url: "https://coderelay.cn/v1".to_string(),
            api_key: "sk-new".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let content = fs::read_to_string(adapter.config_path()).unwrap();

        // Should NOT contain inline table format
        assert!(
            !content.contains("codex = {"),
            "codex provider should not be written as inline table, got:\n{}",
            content
        );
        // Should use standard section header format
        assert!(
            content.contains("[model_providers.codex]"),
            "codex provider should use [model_providers.codex] section format, got:\n{}",
            content
        );
    }

    #[test]
    fn test_write_sets_requires_openai_auth_false() {
        let (_dir, adapter) = setup_codex_dir();
        let new_cfg = TargetConfig {
            base_url: "https://coderelay.cn/v1".to_string(),
            api_key: "sk-relay".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        assert!(
            content.contains("requires_openai_auth = false"),
            "requires_openai_auth should be false for relay gateway, got:\n{}",
            content
        );
    }

    #[test]
    fn test_wsl_home_output_ignores_unexpanded_tilde() {
        let path = CodexAdapter::wsl_home_output_to_config_dir(b"~\r\n", ".codex");
        assert!(
            path.is_none(),
            "unexpanded ~ must not become a relative .codex path"
        );
    }

    #[test]
    fn test_wsl_home_output_to_config_dir_accepts_unc_home() {
        let path = CodexAdapter::wsl_home_output_to_config_dir(
            br"\\wsl.localhost\Ubuntu-24.04\home\leezi
",
            ".codex",
        )
        .unwrap();

        assert_eq!(
            path,
            PathBuf::from(r"\\wsl.localhost\Ubuntu-24.04\home\leezi").join(".codex")
        );
    }

    #[test]
    fn test_new_from_config_uses_loaded_custom_dir() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = crate::config::Config {
            codex_source: Some("wsl".to_string()),
            codex_custom_dir: dir.path().to_string_lossy().to_string(),
            ..crate::config::Config::default()
        };

        let adapter = CodexAdapter::new_from_config(&cfg).unwrap();

        assert_eq!(adapter.config_path(), dir.path().join("config.toml"));
    }
}
