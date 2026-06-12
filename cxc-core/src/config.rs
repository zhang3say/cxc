use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use chrono::{DateTime, Local};
use std::cell::RefCell;

const CONFIG_FILE_NAME: &str = "config.yaml";
const CONFIG_DIR_NAME: &str = "cxc";

thread_local! {
    static TEST_CONFIG_DIR: RefCell<Option<PathBuf>> = RefCell::new(None);
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to find user config directory")]
    NoConfigDir,
    #[error("Failed to read config file at {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("Failed to serialize config: {0}")]
    SerializeError(serde_yaml::Error),
    #[error("Failed to create config directory at {path}: {source}")]
    CreateDirError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to write config file to {path}: {source}")]
    WriteError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Provider '{0}' already exists")]
    ProviderExists(String),
    #[error("Provider '{0}' not found")]
    ProviderNotFound(String),
    #[error("Cannot remove the active provider '{0}' — switch to another provider first")]
    CannotRemoveActive(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Provider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_wire_api")]
    pub wire_api: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_test: Option<DateTime<Local>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ok: Option<bool>,
}

fn default_wire_api() -> String {
    "responses".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub active: String,
    #[serde(default)]
    pub providers: Vec<Provider>,
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    let mut path = if let Some(test_dir) = TEST_CONFIG_DIR.with(|dir| dir.borrow().clone()) {
        test_dir
    } else if let Ok(env_dir) = std::env::var("CXC_TEST_CONFIG_DIR") {
        PathBuf::from(env_dir)
    } else {
        dirs::config_dir().ok_or(ConfigError::NoConfigDir)?
    };
    path.push(CONFIG_DIR_NAME);
    path.push(CONFIG_FILE_NAME);
    Ok(path)
}

pub fn load() -> Result<Config, ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }

    let data = fs::read(&path).map_err(|err| ConfigError::ReadError {
        path: path.clone(),
        source: err,
    })?;

    let cfg: Config = serde_yaml::from_slice(&data)?;
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<(), ConfigError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|err| ConfigError::CreateDirError {
                path: parent.to_path_buf(),
                source: err,
            })?;
        }
    }

    let data = serde_yaml::to_string(cfg).map_err(ConfigError::SerializeError)?;

    write_secure_file(&path, data.as_bytes()).map_err(|err| ConfigError::WriteError {
        path: path.clone(),
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

pub fn add_provider(cfg: &mut Config, mut p: Provider) -> Result<(), ConfigError> {
    if cfg.providers.iter().any(|prov| prov.name == p.name) {
        return Err(ConfigError::ProviderExists(p.name));
    }
    if p.wire_api.is_empty() {
        p.wire_api = default_wire_api();
    }
    cfg.providers.push(p.clone());
    if cfg.active.is_empty() {
        cfg.active = p.name;
    }
    save(cfg)
}

pub fn edit_provider(cfg: &mut Config, old_name: &str, mut updated: Provider) -> Result<(), ConfigError> {
    let idx = cfg
        .providers
        .iter()
        .position(|prov| prov.name == old_name)
        .ok_or_else(|| ConfigError::ProviderNotFound(old_name.to_string()))?;

    if updated.name != old_name && cfg.providers.iter().any(|prov| prov.name == updated.name) {
        return Err(ConfigError::ProviderExists(updated.name));
    }

    if updated.wire_api.is_empty() {
        updated.wire_api = default_wire_api();
    }

    let existing = &cfg.providers[idx];
    if updated.last_test.is_none() {
        updated.last_test = existing.last_test;
    }
    if updated.latency_ms.is_none() {
        updated.latency_ms = existing.latency_ms;
    }
    if updated.last_ok.is_none() {
        updated.last_ok = existing.last_ok;
    }

    cfg.providers[idx] = updated.clone();

    if cfg.active == old_name {
        cfg.active = updated.name;
    }

    save(cfg)
}

pub fn remove_provider(cfg: &mut Config, name: &str) -> Result<(), ConfigError> {
    if cfg.active == name {
        return Err(ConfigError::CannotRemoveActive(name.to_string()));
    }
    let idx = cfg
        .providers
        .iter()
        .position(|prov| prov.name == name)
        .ok_or_else(|| ConfigError::ProviderNotFound(name.to_string()))?;

    cfg.providers.remove(idx);
    save(cfg)
}

pub fn set_active(cfg: &mut Config, name: &str) -> Result<(), ConfigError> {
    if !cfg.providers.iter().any(|prov| prov.name == name) {
        return Err(ConfigError::ProviderNotFound(name.to_string()));
    }
    cfg.active = name.to_string();
    save(cfg)
}

pub fn get_provider<'a>(cfg: &'a Config, name: &str) -> Option<&'a Provider> {
    cfg.providers.iter().find(|prov| prov.name == name)
}

pub fn get_active<'a>(cfg: &'a Config) -> Option<&'a Provider> {
    get_provider(cfg, &cfg.active)
}

pub fn update_test_result(cfg: &mut Config, name: &str, latency_ms: i64, ok: bool) -> Result<(), ConfigError> {
    let idx = cfg
        .providers
        .iter()
        .position(|prov| prov.name == name)
        .ok_or_else(|| ConfigError::ProviderNotFound(name.to_string()))?;

    cfg.providers[idx].last_test = Some(Local::now());
    cfg.providers[idx].latency_ms = Some(latency_ms);
    cfg.providers[idx].last_ok = Some(ok);

    save(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test() -> (TempDir, PathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        TEST_CONFIG_DIR.with(|dir| {
            *dir.borrow_mut() = Some(temp_dir.path().to_path_buf());
        });
        let path = config_path().unwrap();
        (temp_dir, path)
    }

    #[test]
    fn test_add_provider() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        let p = Provider {
            name: "test".to_string(),
            base_url: "https://example.com/v1".to_string(),
            api_key: "sk-xxx".to_string(),
            model: "gpt-4".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        };
        add_provider(&mut cfg, p).unwrap();

        assert_eq!(cfg.providers.len(), 1);
        assert_eq!(cfg.active, "test");
        assert_eq!(cfg.providers[0].wire_api, "responses");
    }

    #[test]
    fn test_add_provider_duplicate() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        let p = Provider {
            name: "test".to_string(),
            base_url: "https://example.com/v1".to_string(),
            api_key: "sk-xxx".to_string(),
            model: "gpt-4".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        };
        add_provider(&mut cfg, p.clone()).unwrap();
        let err = add_provider(&mut cfg, p);
        assert!(err.is_err());
    }

    #[test]
    fn test_first_provider_becomes_active() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();
        add_provider(&mut cfg, Provider {
            name: "b".to_string(),
            base_url: "https://b.com/v1".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        assert_eq!(cfg.active, "a");
    }

    #[test]
    fn test_remove_provider() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();
        add_provider(&mut cfg, Provider {
            name: "b".to_string(),
            base_url: "https://b.com/v1".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();
        set_active(&mut cfg, "b").unwrap();
        remove_provider(&mut cfg, "a").unwrap();

        assert_eq!(cfg.providers.len(), 1);
    }

    #[test]
    fn test_remove_active_provider_fails() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        let err = remove_provider(&mut cfg, "a");
        assert!(err.is_err());
    }

    #[test]
    fn test_remove_nonexistent_provider_fails() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        let err = remove_provider(&mut cfg, "nonexistent");
        assert!(err.is_err());
    }

    #[test]
    fn test_set_active() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();
        add_provider(&mut cfg, Provider {
            name: "b".to_string(),
            base_url: "https://b.com/v1".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        set_active(&mut cfg, "b").unwrap();
        assert_eq!(cfg.active, "b");
    }

    #[test]
    fn test_persistence() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "sk-abc".to_string(),
            model: "gpt-4".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        let loaded = load().unwrap();
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.providers[0].api_key, "sk-abc");
    }

    #[test]
    #[cfg(unix)]
    fn test_config_file_permissions() {
        let (_dir, path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn test_update_test_result() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        update_test_result(&mut cfg, "a", 123, true).unwrap();
        let p = get_provider(&cfg, "a").unwrap();
        assert_eq!(p.latency_ms, Some(123));
        assert_eq!(p.last_ok, Some(true));
        assert!(p.last_test.is_some());
    }

    #[test]
    fn test_remark_persistence() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: Some("My backup endpoint".to_string()),
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        let loaded = load().unwrap();
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.providers[0].remark, Some("My backup endpoint".to_string()));
    }

    #[test]
    fn test_edit_provider() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: Some("old".to_string()),
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        let updated = Provider {
            name: "a".to_string(),
            base_url: "https://new-a.com".to_string(),
            api_key: "new-k1".to_string(),
            model: "new-m1".to_string(),
            wire_api: "".to_string(),
            remark: Some("new".to_string()),
            last_test: None,
            latency_ms: None,
            last_ok: None,
        };
        edit_provider(&mut cfg, "a", updated).unwrap();

        let p = get_provider(&cfg, "a").unwrap();
        assert_eq!(p.base_url, "https://new-a.com");
        assert_eq!(p.api_key, "new-k1");
        assert_eq!(p.model, "new-m1");
        assert_eq!(p.remark, Some("new".to_string()));
    }

    #[test]
    fn test_edit_provider_rename_active() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();
        add_provider(&mut cfg, Provider {
            name: "b".to_string(),
            base_url: "https://b.com".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();
        set_active(&mut cfg, "a").unwrap();

        let updated = Provider {
            name: "new-a".to_string(),
            base_url: "https://a.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        };
        edit_provider(&mut cfg, "a", updated).unwrap();

        assert_eq!(cfg.active, "new-a");
        assert!(get_provider(&cfg, "a").is_none());
        assert!(get_provider(&cfg, "new-a").is_some());
    }

    #[test]
    fn test_edit_provider_duplicate_name() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, Provider {
            name: "a".to_string(),
            base_url: "https://a.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();
        add_provider(&mut cfg, Provider {
            name: "b".to_string(),
            base_url: "https://b.com".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        }).unwrap();

        let updated = Provider {
            name: "b".to_string(),
            base_url: "https://a.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
        };
        let err = edit_provider(&mut cfg, "a", updated);
        assert!(err.is_err());
    }
}
