use crate::target::{TargetAdapter, TargetConfig, TargetError};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::sync::OnceLock;
use toml_edit::{DocumentMut, Item, Table};

const CONFIG_DIR_NAME: &str = ".grok";
const CONFIG_FILE_NAME: &str = "config.toml";

pub struct GrokAdapter {
    grok_dir: PathBuf,
}

impl GrokAdapter {
    pub fn new() -> Result<Self, TargetError> {
        let grok_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_GROK_DIR") {
            PathBuf::from(test_dir)
        } else {
            let cfg = crate::config::load().ok();
            Self::resolve_grok_dir(cfg.as_ref())?
        };
        Ok(Self { grok_dir })
    }

    pub fn new_from_config(cfg: &crate::config::Config) -> Result<Self, TargetError> {
        let grok_dir = if let Ok(test_dir) = std::env::var("CXC_TEST_GROK_DIR") {
            PathBuf::from(test_dir)
        } else {
            Self::resolve_grok_dir(Some(cfg))?
        };
        Ok(Self { grok_dir })
    }

    fn resolve_grok_dir(cfg: Option<&crate::config::Config>) -> Result<PathBuf, TargetError> {
        let source = cfg
            .map(crate::config::effective_source)
            .unwrap_or(crate::config::SOURCE_WSL);

        let custom_dir = cfg.and_then(|c| {
            if !c.grok_custom_dir.is_empty() {
                let mut path_str = c.grok_custom_dir.clone();

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
            Self::default_grok_dir(source)
        }
    }

    /// Resolve the default Grok config directory based on global source.
    ///
    /// - `"wsl"` on Linux  → `~/.grok`
    /// - `"app"` on Linux  → `/mnt/c/Users/<username>/.grok`
    /// - `"app"` on Windows → `%USERPROFILE%\.grok`
    /// - `"wsl"` on Windows → `\\wsl.localhost\<distro>\home\<user>\.grok`
    fn default_grok_dir(source: &str) -> Result<PathBuf, TargetError> {
        #[cfg(target_os = "linux")]
        {
            if source == "app" {
                if let Some(win_dir) = Self::detect_windows_grok_dir() {
                    return Ok(win_dir);
                }
                return Err(TargetError::ParseError(
                    "Unable to locate Windows home directory automatically from within WSL. Please specify a custom directory.".to_string(),
                ));
            }
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            return Ok(home.join(CONFIG_DIR_NAME));
        }

        #[cfg(target_os = "windows")]
        {
            if source == "wsl" {
                if let Some(wsl_dir) = Self::cached_wsl_grok_dir() {
                    return Ok(wsl_dir);
                }
                return Err(TargetError::ParseError(
                    "Unable to locate WSL home directory automatically. Please specify a custom directory.".to_string(),
                ));
            }
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            return Ok(home.join(CONFIG_DIR_NAME));
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = source;
            let home = dirs::home_dir().ok_or(TargetError::NoHomeDir)?;
            Ok(home.join(CONFIG_DIR_NAME))
        }
    }

    #[cfg(target_os = "linux")]
    fn detect_windows_grok_dir() -> Option<PathBuf> {
        let username = std::env::var("USER").ok()?;
        let candidate = PathBuf::from(format!("/mnt/c/Users/{}/{}", username, CONFIG_DIR_NAME));
        if candidate.exists() {
            return Some(candidate);
        }
        let home = PathBuf::from(format!("/mnt/c/Users/{}", username));
        if home.exists() {
            return Some(candidate);
        }
        if let Ok(entries) = fs::read_dir("/mnt/c/Users") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name == username.to_lowercase() {
                    return Some(entry.path().join(CONFIG_DIR_NAME));
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    fn cached_wsl_grok_dir() -> Option<PathBuf> {
        static WSL_GROK_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
        WSL_GROK_DIR.get_or_init(Self::detect_wsl_grok_dir).clone()
    }

    #[cfg(target_os = "windows")]
    fn detect_wsl_grok_dir() -> Option<PathBuf> {
        let mut first_valid_home: Option<PathBuf> = None;

        if let Ok(output) = std::process::Command::new("wsl")
            .args(["-e", "sh", "-lc", "wslpath -w \"$HOME\""])
            .output()
        {
            if output.status.success() {
                if let Some(candidate) =
                    Self::wsl_home_output_to_config_dir(&output.stdout, CONFIG_DIR_NAME)
                {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                    first_valid_home = Some(candidate);
                }
            }
        }

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
                            let grok_dir = user_path.join(CONFIG_DIR_NAME);
                            if grok_dir.exists() {
                                return Some(grok_dir);
                            }
                            if first_valid_home.is_none() && user_path.exists() {
                                first_valid_home = Some(grok_dir);
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
                    "\\\\wsl.localhost\\{}\\home\\{}\\{}",
                    distro, username, CONFIG_DIR_NAME
                ));
                if candidate.exists() {
                    return Some(candidate);
                }
                let home =
                    PathBuf::from(format!("\\\\wsl.localhost\\{}\\home\\{}", distro, username));
                if first_valid_home.is_none() && home.exists() {
                    first_valid_home = Some(candidate);
                }

                let candidate = PathBuf::from(format!(
                    "\\\\wsl$\\{}\\home\\{}\\{}",
                    distro, username, CONFIG_DIR_NAME
                ));
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
            grok_dir: dir.as_ref().to_path_buf(),
        }
    }

    pub fn config_path(&self) -> PathBuf {
        self.grok_dir.join(CONFIG_FILE_NAME)
    }

    /// Map Provider.wire_api to Grok's api_backend values.
    pub fn map_wire_api_to_api_backend(wire_api: &str) -> String {
        match wire_api.trim() {
            "" | "chat" | "chat_completions" => "chat_completions".to_string(),
            "responses" => "responses".to_string(),
            "messages" => "messages".to_string(),
            other => other.to_string(),
        }
    }

    /// Map Grok api_backend back to Provider.wire_api for read().
    fn map_api_backend_to_wire_api(api_backend: &str) -> String {
        match api_backend.trim() {
            "" | "chat_completions" => "chat_completions".to_string(),
            other => other.to_string(),
        }
    }
}

impl TargetAdapter for GrokAdapter {
    fn name(&self) -> &'static str {
        "Grok"
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
            .get("models")
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("default"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut base_url = String::new();
        let mut api_key = String::new();
        let mut wire_api = "chat_completions".to_string();

        if !model.is_empty() {
            if let Some(model_table) = doc
                .get("model")
                .and_then(|m| m.as_table())
                .and_then(|t| t.get(model.as_str()))
                .and_then(|v| v.as_table())
            {
                if let Some(bu) = model_table.get("base_url").and_then(|v| v.as_str()) {
                    base_url = bu.to_string();
                }
                if let Some(key) = model_table.get("api_key").and_then(|v| v.as_str()) {
                    api_key = key.to_string();
                }
                if let Some(backend) = model_table.get("api_backend").and_then(|v| v.as_str()) {
                    wire_api = Self::map_api_backend_to_wire_api(backend);
                }
            }
        }

        Ok(TargetConfig {
            base_url,
            api_key,
            model,
            wire_api,
        })
    }

    fn write(&self, config: &TargetConfig) -> Result<(), TargetError> {
        if !self.grok_dir.exists() {
            fs::create_dir_all(&self.grok_dir).map_err(|err| TargetError::WriteError {
                path: self.grok_dir.clone(),
                source: err,
            })?;
        }

        let config_path = self.config_path();
        backup(&config_path)?;

        let toml_data = if config_path.exists() {
            fs::read_to_string(&config_path).map_err(|err| TargetError::ReadError {
                path: config_path.clone(),
                source: err,
            })?
        } else {
            // Minimal Grok config scaffold for first-time write
            String::from("[models]\ndefault = \"\"\n")
        };

        let mut doc = toml_data
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::ParseError(e.to_string()))?;

        // Ensure [models] table exists and set default
        if doc.get("models").and_then(|v| v.as_table()).is_none() {
            doc["models"] = Item::Table(Table::new());
        }
        doc["models"]["default"] = toml_edit::value(&config.model);

        // Ensure [model] outer table exists
        if doc.get("model").and_then(|v| v.as_table()).is_none() {
            let mut outer = Table::new();
            outer.set_implicit(true);
            doc["model"] = Item::Table(outer);
        }

        let api_backend = Self::map_wire_api_to_api_backend(&config.wire_api);
        let model_key = config.model.as_str();

        // Ensure the per-model table exists as a standard table (not inline).
        // Avoid Index panic when the model key is absent by using get() first.
        {
            let existing = doc
                .get("model")
                .and_then(|m| m.as_table())
                .and_then(|t| t.get(model_key))
                .cloned();

            match existing {
                Some(Item::Table(_)) => {
                    // already a proper table
                }
                Some(Item::Value(toml_edit::Value::InlineTable(it))) => {
                    let mut t = it.into_table();
                    t.set_implicit(false);
                    doc["model"][model_key] = Item::Table(t);
                }
                _ => {
                    let mut t = Table::new();
                    t.set_implicit(false);
                    doc["model"][model_key] = Item::Table(t);
                }
            }
        }

        doc["model"][model_key]["base_url"] = toml_edit::value(&config.base_url);
        doc["model"][model_key]["api_key"] = toml_edit::value(&config.api_key);
        doc["model"][model_key]["api_backend"] = toml_edit::value(&api_backend);
        // Keep model id field aligned with section key (harmless if already set)
        doc["model"][model_key]["model"] = toml_edit::value(&config.model);

        let out = doc.to_string();
        let _ = out
            .parse::<DocumentMut>()
            .map_err(|e| TargetError::SerializeError(format!("Written TOML is invalid: {}", e)))?;

        write_secure_file(&config_path, out.as_bytes()).map_err(|err| TargetError::WriteError {
            path: config_path.clone(),
            source: err,
        })?;

        Ok(())
    }
}

fn backup(path: &Path) -> Result<(), TargetError> {
    if !path.exists() {
        return Ok(());
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

    const SAMPLE_CONFIG: &str = r#"[cli]
installer = "internal"

[models]
default = "grok-4.5"
default_reasoning_effort = "high"

[model."grok-4.5"]
base_url = "https://old.example.com/v1"
api_key = "sk-old-key"
api_backend = "chat_completions"

[marketplace]
official_marketplace_auto_installed = true

[ui]
theme = "grokday"
yolo = false
"#;

    fn setup_grok_dir() -> (TempDir, GrokAdapter) {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), SAMPLE_CONFIG).unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());
        (dir, adapter)
    }

    #[test]
    fn test_name_returns_grok() {
        let (_dir, adapter) = setup_grok_dir();
        assert_eq!(adapter.name(), "Grok");
    }

    #[test]
    fn test_read() {
        let (_dir, adapter) = setup_grok_dir();
        let cfg = adapter.read().unwrap();
        assert_eq!(cfg.model, "grok-4.5");
        assert_eq!(cfg.base_url, "https://old.example.com/v1");
        assert_eq!(cfg.api_key, "sk-old-key");
        assert_eq!(cfg.wire_api, "chat_completions");
    }

    #[test]
    fn test_write() {
        let (dir, adapter) = setup_grok_dir();
        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new-key".to_string(),
            model: "grok-4.5".to_string(),
            wire_api: "chat_completions".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, new_cfg.base_url);
        assert_eq!(got.api_key, new_cfg.api_key);
        assert_eq!(got.model, new_cfg.model);
        assert_eq!(got.wire_api, "chat_completions");

        assert!(dir.path().join("config.toml.bak").exists());
    }

    #[test]
    fn test_write_preserves_unrelated_sections() {
        let (_dir, adapter) = setup_grok_dir();
        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new".to_string(),
            model: "grok-4.5".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        for check in ["installer", "marketplace", "theme", "yolo", "default_reasoning_effort"] {
            assert!(
                content.contains(check),
                "Unrelated key '{}' was lost:\n{}",
                check,
                content
            );
        }
    }

    #[test]
    fn test_write_maps_wire_api() {
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend(""),
            "chat_completions"
        );
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend("chat"),
            "chat_completions"
        );
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend("responses"),
            "responses"
        );
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend("messages"),
            "messages"
        );
        assert_eq!(
            GrokAdapter::map_wire_api_to_api_backend("custom"),
            "custom"
        );
    }

    #[test]
    fn test_write_sets_api_backend_responses() {
        let (_dir, adapter) = setup_grok_dir();
        let new_cfg = TargetConfig {
            base_url: "https://relay.example.com/v1".to_string(),
            api_key: "sk-relay".to_string(),
            model: "grok-4.5".to_string(),
            wire_api: "responses".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        assert!(
            content.contains("api_backend = \"responses\""),
            "expected api_backend responses, got:\n{}",
            content
        );

        let got = adapter.read().unwrap();
        assert_eq!(got.wire_api, "responses");
    }

    #[test]
    fn test_write_creates_files_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = GrokAdapter::new_with_dir(dir.path());

        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new-key".to_string(),
            model: "grok-build".to_string(),
            wire_api: "".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, new_cfg.base_url);
        assert_eq!(got.api_key, new_cfg.api_key);
        assert_eq!(got.model, new_cfg.model);
        assert_eq!(got.wire_api, "chat_completions");
        assert!(!dir.path().join("config.toml.bak").exists());
    }

    #[test]
    fn test_write_creates_directory_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("sub").join("grok");
        let adapter = GrokAdapter::new_with_dir(&nested);

        let new_cfg = TargetConfig {
            base_url: "https://example.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "grok-4.5".to_string(),
            wire_api: "chat_completions".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        assert!(nested.join(CONFIG_FILE_NAME).exists());
        let got = adapter.read().unwrap();
        assert_eq!(got.base_url, "https://example.com/v1");
    }

    #[test]
    fn test_write_switches_model_default() {
        let (_dir, adapter) = setup_grok_dir();
        let new_cfg = TargetConfig {
            base_url: "https://relay.example.com/v1".to_string(),
            api_key: "sk-relay".to_string(),
            model: "custom-model".to_string(),
            wire_api: "chat_completions".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let got = adapter.read().unwrap();
        assert_eq!(got.model, "custom-model");
        assert_eq!(got.base_url, "https://relay.example.com/v1");

        let content = fs::read_to_string(adapter.config_path()).unwrap();
        assert!(content.contains("default = \"custom-model\""));
        // Old model section should still exist (we only write the new one)
        assert!(content.contains("grok-4.5") || content.contains("old.example.com"));
    }

    #[test]
    fn test_new_from_config_uses_loaded_custom_dir() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = crate::config::Config {
            grok_source: Some("wsl".to_string()),
            grok_custom_dir: dir.path().to_string_lossy().to_string(),
            ..crate::config::Config::default()
        };

        let adapter = GrokAdapter::new_from_config(&cfg).unwrap();
        assert_eq!(adapter.config_path(), dir.path().join(CONFIG_FILE_NAME));
    }

    #[test]
    fn test_write_produces_valid_toml() {
        let (_dir, adapter) = setup_grok_dir();
        let new_cfg = TargetConfig {
            base_url: "https://new.example.com/v1".to_string(),
            api_key: "sk-new".to_string(),
            model: "grok-4.5".to_string(),
            wire_api: "chat_completions".to_string(),
        };
        adapter.write(&new_cfg).unwrap();

        let data = fs::read_to_string(adapter.config_path()).unwrap();
        let parsed = data.parse::<DocumentMut>();
        assert!(parsed.is_ok(), "Written TOML is invalid: {:?}", parsed.err());
    }
}
