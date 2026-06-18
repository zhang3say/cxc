use crate::config::Provider;
use crate::target::{TargetAdapter, TargetConfig, TargetError};
use serde_json::{self, json};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::sync::OnceLock;

pub struct ClaudeAdapter {
    claude_dir: PathBuf,
}

impl ClaudeAdapter {
    pub fn new() -> Result<Self, TargetError> {
        let claude_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_CLAUDE_DIR") {
            PathBuf::from(test_dir)
        } else {
            // Load global cxc config to check for custom claude directory and source settings
            let cfg = crate::config::load().ok();
            Self::resolve_claude_dir(cfg.as_ref())?
        };
        Ok(Self { claude_dir })
    }

    pub fn new_from_config(cfg: &crate::config::Config) -> Result<Self, TargetError> {
        let claude_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_CLAUDE_DIR") {
            PathBuf::from(test_dir)
        } else {
            Self::resolve_claude_dir(Some(cfg))?
        };
        Ok(Self { claude_dir })
    }

    fn resolve_claude_dir(cfg: Option<&crate::config::Config>) -> Result<PathBuf, TargetError> {
        let source = cfg
            .and_then(|c| c.claude_source.as_deref())
            .unwrap_or("wsl");

        let custom_dir = cfg.and_then(|c| {
            if !c.claude_custom_dir.is_empty() {
                let mut path_str = c.claude_custom_dir.clone();

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
            // No custom dir: use smart defaults based on claude_source
            Self::default_claude_dir(source)
        }
    }

    /// Resolve the default Claude config directory based on `claude_source`.
    ///
    /// - `"wsl"` on Linux  → `~/.claude` (local WSL home)
    /// - `"app"` on Linux  → `/mnt/c/Users/<username>/.claude` (Windows host via WSL mount)
    /// - `"app"` on Windows → `%USERPROFILE%\.claude`
    /// - `"wsl"` on Windows → try `\\wsl.localhost\<distro>\home\<user>\.claude`
    fn default_claude_dir(source: &str) -> Result<PathBuf, TargetError> {
        #[cfg(target_os = "linux")]
        {
            if source == "app" {
                // Try to find the Windows home directory via common WSL mount points
                if let Some(win_dir) = Self::detect_windows_claude_dir() {
                    return Ok(win_dir);
                }
                return Err(TargetError::ParseError("Unable to locate Windows home directory automatically from within WSL. Please specify a custom directory.".to_string()));
            }
            // source == "wsl" or fallback: use native Linux home
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            return Ok(home.join(".claude"));
        }

        #[cfg(target_os = "windows")]
        {
            if source == "wsl" {
                // Try to find WSL home directory via UNC path
                if let Some(wsl_dir) = Self::cached_wsl_claude_dir() {
                    return Ok(wsl_dir);
                }
                return Err(TargetError::ParseError("Unable to locate WSL home directory automatically. Please specify a custom directory.".to_string()));
            }
            // source == "app" or fallback: use native Windows home
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            return Ok(home.join(".claude"));
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = source;
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            Ok(home.join(".claude"))
        }
    }

    /// Detect Windows Claude directory from within WSL via /mnt/c/Users/<username>/.claude
    #[cfg(target_os = "linux")]
    fn detect_windows_claude_dir() -> Option<PathBuf> {
        let username = std::env::var("USER").ok()?;
        // Try the most common mount point first
        let candidate = PathBuf::from(format!("/mnt/c/Users/{}/.claude", username));
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
                    let claude_dir = entry.path().join(".claude");
                    return Some(claude_dir);
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    fn cached_wsl_claude_dir() -> Option<PathBuf> {
        static WSL_CLAUDE_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
        WSL_CLAUDE_DIR.get_or_init(Self::detect_wsl_claude_dir).clone()
    }

    /// Detect WSL Claude directory from Windows via \\wsl.localhost\<distro>\home\<user>\.claude
    #[cfg(target_os = "windows")]
    fn detect_wsl_claude_dir() -> Option<PathBuf> {
        let mut first_valid_home: Option<PathBuf> = None;

        // 0. Try using wsl.exe to get the home path dynamically
        if let Ok(output) = std::process::Command::new("wsl")
            .args(["-e", "sh", "-lc", "wslpath -w \"$HOME\""])
            .output()
        {
            if output.status.success() {
                if let Some(candidate) =
                    Self::wsl_home_output_to_config_dir(&output.stdout, ".claude")
                {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                    first_valid_home = Some(candidate);
                }
            }
        }

        // Helper function to scan a base UNC path (like \\wsl.localhost or \\wsl$)
        let mut scan_unc_path = |base_path: &str| -> Option<PathBuf> {
            let base = Path::new(base_path);
            if !base.exists() {
                return None;
            }

            // Read all distros (subdirectories of the UNC base)
            if let Ok(distro_entries) = fs::read_dir(base) {
                for distro_entry in distro_entries.flatten() {
                    let distro_path = distro_entry.path();
                    let home_path = distro_path.join("home");
                    if home_path.exists() {
                        // Read all users in /home/
                        if let Ok(user_entries) = fs::read_dir(&home_path) {
                            for user_entry in user_entries.flatten() {
                                let user_path = user_entry.path();
                                let claude_dir = user_path.join(".claude");
                                if claude_dir.exists() {
                                    return Some(claude_dir);
                                }
                                if first_valid_home.is_none() && user_path.exists() {
                                    first_valid_home = Some(claude_dir);
                                }
                            }
                        }
                    }
                }
            }
            None
        };

        // 1. Try modern wsl.localhost path first
        if let Some(path) = scan_unc_path("\\\\wsl.localhost") {
            return Some(path);
        }

        // 2. Fall back to legacy wsl$ path
        if let Some(path) = scan_unc_path("\\\\wsl$") {
            return Some(path);
        }

        // 3. Last fallback: try hardcoded approach using USERNAME env
        if let Some(username) = std::env::var("USERNAME").ok() {
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
                let candidate1 = PathBuf::from(format!(
                    "\\\\wsl.localhost\\{}\\home\\{}\\.claude",
                    distro, username
                ));
                if candidate1.exists() {
                    return Some(candidate1);
                }
                let home1 =
                    PathBuf::from(format!("\\\\wsl.localhost\\{}\\home\\{}", distro, username));
                if first_valid_home.is_none() && home1.exists() {
                    first_valid_home = Some(candidate1);
                }
                let candidate2 =
                    PathBuf::from(format!("\\\\wsl$\\{}\\home\\{}\\.claude", distro, username));
                if candidate2.exists() {
                    return Some(candidate2);
                }
                let home2 = PathBuf::from(format!("\\\\wsl$\\{}\\home\\{}", distro, username));
                if first_valid_home.is_none() && home2.exists() {
                    first_valid_home = Some(candidate2);
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
            claude_dir: dir.as_ref().to_path_buf(),
        }
    }

    fn settings_path(&self) -> PathBuf {
        self.claude_dir.join("settings.json")
    }

    /// Write a Provider's configuration to Claude CLI settings.json
    pub fn write_provider(&self, provider: &Provider) -> Result<(), TargetError> {
        let config = TargetConfig {
            base_url: provider.base_url.clone(),
            api_key: provider.api_key.clone(),
            model: provider.model.clone(),
            wire_api: String::new(), // Claude doesn't use this field
        };
        self.write_with_provider(&config, provider)
    }

    /// Internal write method that has access to full Provider for claude_models
    fn write_with_provider(
        &self,
        config: &TargetConfig,
        provider: &Provider,
    ) -> Result<(), TargetError> {
        // Check if .claude directory exists
        if !self.claude_dir.exists() {
            let reason = if cfg!(target_os = "linux") && self.claude_dir.starts_with("/mnt/c") {
                "Windows 主机上未安装 Claude CLI".to_string()
            } else if cfg!(target_os = "windows")
                && self.claude_dir.to_string_lossy().contains("wsl")
            {
                "WSL 中未安装 Claude CLI".to_string()
            } else {
                "Claude CLI 未安装或未初始化".to_string()
            };

            let suggestion = if cfg!(target_os = "linux") && self.claude_dir.starts_with("/mnt/c") {
                "在 Windows 上安装 Claude CLI，或修改 CXC 配置：claude_source = \"wsl\"".to_string()
            } else if cfg!(target_os = "windows")
                && self.claude_dir.to_string_lossy().contains("wsl")
            {
                "在 WSL 中安装 Claude CLI，或修改 CXC 配置：claude_source = \"app\"".to_string()
            } else {
                "安装 Claude CLI：https://claude.ai/download".to_string()
            };

            return Err(TargetError::ClaudeConfigDirNotFound {
                path: self.claude_dir.clone(),
                reason,
                suggestion,
            });
        }

        let settings_path = self.settings_path();

        // Read existing settings or create empty object
        let mut settings: serde_json::Value = if settings_path.exists() {
            // Backup existing file
            backup(&settings_path)?;

            let data =
                fs::read_to_string(&settings_path).map_err(|err| TargetError::ReadError {
                    path: settings_path.clone(),
                    source: err,
                })?;

            serde_json::from_str(&data).map_err(|e| TargetError::ClaudeConfigInvalid {
                path: settings_path.clone(),
                details: format!("JSON 解析失败: {}", e),
            })?
        } else {
            json!({})
        };

        // Ensure env object exists
        if !settings.get("env").map(|v| v.is_object()).unwrap_or(false) {
            settings["env"] = json!({});
        }

        let env = settings["env"].as_object_mut().unwrap();

        // Update base_url
        env.insert("ANTHROPIC_BASE_URL".to_string(), json!(config.base_url));

        // Update authentication field
        // Force write to ANTHROPIC_AUTH_TOKEN and remove ANTHROPIC_API_KEY
        env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(config.api_key));
        env.remove("ANTHROPIC_API_KEY");

        // Update model fields
        // Use claude_models if configured, otherwise fall back to model field
        let opus = provider
            .claude_models
            .as_ref()
            .and_then(|m| m.opus.as_ref())
            .unwrap_or(&provider.model);
        let sonnet = provider
            .claude_models
            .as_ref()
            .and_then(|m| m.sonnet.as_ref())
            .unwrap_or(&provider.model);
        let haiku = provider
            .claude_models
            .as_ref()
            .and_then(|m| m.haiku.as_ref())
            .unwrap_or(&provider.model);
        let fable = provider
            .claude_models
            .as_ref()
            .and_then(|m| m.fable.as_ref())
            .unwrap_or(&provider.model);

        env.insert("ANTHROPIC_MODEL".to_string(), json!(config.model));
        env.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), json!(opus));
        env.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), json!(sonnet));
        env.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), json!(haiku));
        env.insert("ANTHROPIC_DEFAULT_FABLE_MODEL".to_string(), json!(fable));

        if let Some(sub) = provider
            .claude_models
            .as_ref()
            .and_then(|m| m.subagent.as_ref())
            .filter(|s| !s.is_empty())
        {
            env.insert("CLAUDE_CODE_SUBAGENT_MODEL".to_string(), json!(sub));
        } else {
            env.remove("CLAUDE_CODE_SUBAGENT_MODEL");
        }

        // Write settings
        let out = serde_json::to_string_pretty(&settings)
            .map_err(|e| TargetError::SerializeError(e.to_string()))?;

        write_secure_file(&settings_path, out.as_bytes()).map_err(|err| {
            TargetError::WriteError {
                path: settings_path.clone(),
                source: err,
            }
        })?;

        Ok(())
    }
}

impl TargetAdapter for ClaudeAdapter {
    fn name(&self) -> &'static str {
        "Claude CLI"
    }

    fn read(&self) -> Result<TargetConfig, TargetError> {
        let settings_path = self.settings_path();
        let data = fs::read_to_string(&settings_path).map_err(|err| TargetError::ReadError {
            path: settings_path.clone(),
            source: err,
        })?;

        let settings: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| TargetError::ParseError(e.to_string()))?;

        let env = settings
            .get("env")
            .and_then(|v| v.as_object())
            .ok_or_else(|| TargetError::ParseError("Missing or invalid env field".to_string()))?;

        // Read base_url
        let base_url = env
            .get("ANTHROPIC_BASE_URL")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Read api_key from ANTHROPIC_AUTH_TOKEN
        let api_key = env
            .get("ANTHROPIC_AUTH_TOKEN")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Read model (prefer ANTHROPIC_MODEL, then OPUS, SONNET, FABLE, HAIKU)
        let model = env
            .get("ANTHROPIC_MODEL")
            .or_else(|| env.get("ANTHROPIC_DEFAULT_OPUS_MODEL"))
            .or_else(|| env.get("ANTHROPIC_DEFAULT_SONNET_MODEL"))
            .or_else(|| env.get("ANTHROPIC_DEFAULT_FABLE_MODEL"))
            .or_else(|| env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(TargetConfig {
            base_url,
            api_key,
            model,
            wire_api: String::new(), // Claude doesn't use this field
        })
    }

    fn write(&self, config: &TargetConfig) -> Result<(), TargetError> {
        // Create a minimal provider from TargetConfig
        // This is used when write() is called directly without a Provider
        let provider = Provider {
            name: String::new(),
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            wire_api: String::new(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        };
        self.write_with_provider(config, &provider)
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

    fn setup_claude_dir() -> (TempDir, ClaudeAdapter) {
        let dir = tempfile::tempdir().unwrap();
        // Create minimal settings.json
        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://old.example.com/v1",
                "ANTHROPIC_AUTH_TOKEN": "sk-old-key",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-6"
            },
            "model": "opus"
        });
        fs::write(
            dir.path().join("settings.json"),
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .unwrap();
        let adapter = ClaudeAdapter::new_with_dir(dir.path());
        (dir, adapter)
    }

    #[test]
    fn test_name_returns_claude_cli() {
        let (_dir, adapter) = setup_claude_dir();
        assert_eq!(adapter.name(), "Claude CLI");
    }

    #[test]
    fn test_read() {
        let (_dir, adapter) = setup_claude_dir();
        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.base_url, "https://old.example.com/v1");
        assert_eq!(cfg.api_key, "sk-old-key");
        assert_eq!(cfg.model, "claude-opus-4-6");
        assert_eq!(cfg.wire_api, "");
    }

    #[test]
    fn test_write() {
        let (dir, adapter) = setup_claude_dir();
        let provider = Provider {
            name: "test".to_string(),
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new-key".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: String::new(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        };
        adapter.write_provider(&provider).unwrap();

        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, "https://new.example.com/v1");
        assert_eq!(got.api_key, "sk-new-key");
        assert_eq!(got.model, "gpt-4o");

        // Verify backup created
        assert!(dir.path().join("settings.json.bak").exists());
    }

    #[test]
    fn test_write_with_claude_models() {
        let (_dir, adapter) = setup_claude_dir();
        let provider = Provider {
            name: "deepseek".to_string(),
            base_url: "https://api.deepseek.com/anthropic".to_string(),
            api_key: "sk-deepseek".to_string(),
            model: "deepseek-v4-pro".to_string(),
            wire_api: String::new(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: Some(crate::config::ClaudeModels {
                opus: Some("deepseek-v4-pro[1m]".to_string()),
                sonnet: Some("deepseek-v4-pro[1m]".to_string()),
                haiku: Some("deepseek-v4-flash".to_string()),
                fable: Some("deepseek-v4-pro[1m]".to_string()),
                subagent: None,
            }),
        };
        adapter.write_provider(&provider).unwrap();

        // Read back and check model fields
        let settings_path = adapter.settings_path();
        let data = fs::read_to_string(&settings_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&data).unwrap();
        let env = settings["env"].as_object().unwrap();

        assert_eq!(env["ANTHROPIC_MODEL"], "deepseek-v4-pro");
        assert_eq!(env["ANTHROPIC_DEFAULT_OPUS_MODEL"], "deepseek-v4-pro[1m]");
        assert_eq!(env["ANTHROPIC_DEFAULT_SONNET_MODEL"], "deepseek-v4-pro[1m]");
        assert_eq!(env["ANTHROPIC_DEFAULT_HAIKU_MODEL"], "deepseek-v4-flash");
        assert_eq!(env["ANTHROPIC_DEFAULT_FABLE_MODEL"], "deepseek-v4-pro[1m]");
    }

    #[test]
    fn test_write_preserves_env_fields() {
        let dir = tempfile::tempdir().unwrap();
        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://old.example.com",
                "ANTHROPIC_AUTH_TOKEN": "sk-old",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-6",
                "CLAUDE_CODE_EFFORT_LEVEL": "max",
                "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "75"
            }
        });
        fs::write(
            dir.path().join("settings.json"),
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .unwrap();
        let adapter = ClaudeAdapter::new_with_dir(dir.path());

        let provider = Provider {
            name: "test".to_string(),
            base_url: "https://new.example.com".to_string(),
            api_key: "sk-new".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: String::new(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        };
        adapter.write_provider(&provider).unwrap();

        let data = fs::read_to_string(adapter.settings_path()).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&data).unwrap();
        let env = settings["env"].as_object().unwrap();

        // Check preserved fields
        assert_eq!(env["CLAUDE_CODE_EFFORT_LEVEL"], "max");
        assert_eq!(env["CLAUDE_AUTOCOMPACT_PCT_OVERRIDE"], "75");
    }

    #[test]
    fn test_auth_field_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://old.example.com",
                "ANTHROPIC_API_KEY": "sk-api-key",
                "ANTHROPIC_AUTH_TOKEN": "sk-auth-token",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-6"
            }
        });
        fs::write(
            dir.path().join("settings.json"),
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .unwrap();
        let adapter = ClaudeAdapter::new_with_dir(dir.path());

        let provider = Provider {
            name: "test".to_string(),
            base_url: "https://new.example.com".to_string(),
            api_key: "sk-new".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: String::new(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        };
        adapter.write_provider(&provider).unwrap();

        let data = fs::read_to_string(adapter.settings_path()).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&data).unwrap();
        let env = settings["env"].as_object().unwrap();

        // Should keep AUTH_TOKEN and remove API_KEY
        assert!(env.contains_key("ANTHROPIC_AUTH_TOKEN"));
        assert!(!env.contains_key("ANTHROPIC_API_KEY"));
        assert_eq!(env["ANTHROPIC_AUTH_TOKEN"], "sk-new");
    }

    #[test]
    fn test_dir_not_exists() {
        let dir = tempfile::tempdir().unwrap();
        let non_existent = dir.path().join("non-existent");
        let adapter = ClaudeAdapter::new_with_dir(&non_existent);

        let provider = Provider {
            name: "test".to_string(),
            base_url: "https://example.com".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: String::new(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        };

        let result = adapter.write_provider(&provider);
        assert!(result.is_err());
        match result {
            Err(TargetError::ClaudeConfigDirNotFound { .. }) => (),
            _ => panic!("Expected ClaudeConfigDirNotFound error"),
        }
    }

    #[test]
    fn test_create_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        // Don't create settings.json
        let adapter = ClaudeAdapter::new_with_dir(dir.path());

        let provider = Provider {
            name: "test".to_string(),
            base_url: "https://example.com".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: String::new(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        };
        adapter.write_provider(&provider).unwrap();

        // Verify file was created
        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, "https://example.com");
        assert_eq!(got.api_key, "sk-test");

        // No backup should exist
        assert!(!dir.path().join("settings.json.bak").exists());
    }
}
