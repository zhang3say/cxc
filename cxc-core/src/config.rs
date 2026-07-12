use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use chrono::{DateTime, Local};
use std::cell::RefCell;

const CONFIG_FILE_NAME: &str = "config.yaml";
const CONFIG_DIR_NAME: &str = "cxc";
pub const SOURCE_APP: &str = "app";
pub const SOURCE_WSL: &str = "wsl";

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
    #[error("Cannot remove provider '{name}' for {target_tool}: active in {sources} — switch it first")]
    CannotRemoveActive {
        name: String,
        target_tool: String,
        sources: String,
    },
    #[error("Unknown target tool: '{0}' (expected codex, claude, or grok)")]
    UnknownTargetTool(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ClaudeModels {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sonnet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haiku: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_models: Option<ClaudeModels>,
}

fn default_wire_api() -> String {
    "responses".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    // Codex 配置
    #[serde(default)]
    pub codex_active_app: String,
    #[serde(default)]
    pub codex_active_wsl: String,
    #[serde(default)]
    pub codex_providers: Vec<Provider>,
    #[serde(default)]
    pub codex_source: Option<String>,
    #[serde(default)]
    pub codex_custom_dir: String,

    // Claude CLI 配置
    #[serde(default)]
    pub claude_active_app: String,
    #[serde(default)]
    pub claude_active_wsl: String,
    #[serde(default)]
    pub claude_providers: Vec<Provider>,
    #[serde(default)]
    pub claude_source: Option<String>,
    #[serde(default)]
    pub claude_custom_dir: String,

    // Grok CLI 配置
    #[serde(default)]
    pub grok_active_app: String,
    #[serde(default)]
    pub grok_active_wsl: String,
    #[serde(default)]
    pub grok_providers: Vec<Provider>,
    #[serde(default)]
    pub grok_source: Option<String>,
    #[serde(default)]
    pub grok_custom_dir: String,

    // 旧字段（向后兼容，迁移后不再保存）
    #[serde(default, skip_serializing)]
    pub codex_active: String,
    #[serde(default, skip_serializing)]
    pub claude_active: String,
    #[serde(default, skip_serializing)]
    pub active: String,
    #[serde(default, skip_serializing)]
    pub providers: Vec<Provider>,
}

fn normalize_source(source: &str) -> &'static str {
    if source == SOURCE_APP {
        SOURCE_APP
    } else {
        SOURCE_WSL
    }
}

pub fn effective_source(cfg: &Config) -> &'static str {
    if let Some(source) = cfg.codex_source.as_deref() {
        return normalize_source(source);
    }
    if let Some(source) = cfg.claude_source.as_deref() {
        return normalize_source(source);
    }
    if let Some(source) = cfg.grok_source.as_deref() {
        return normalize_source(source);
    }
    SOURCE_WSL
}

pub fn set_global_source(cfg: &mut Config, source: &str) {
    let source = normalize_source(source);
    cfg.codex_source = Some(source.to_string());
    cfg.claude_source = Some(source.to_string());
    cfg.grok_source = Some(source.to_string());
}

fn sync_global_source_fields(cfg: &mut Config) -> bool {
    if cfg.codex_source.is_none() && cfg.claude_source.is_none() && cfg.grok_source.is_none() {
        return false;
    }

    let source = effective_source(cfg);
    let changed = cfg.codex_source.as_deref() != Some(source)
        || cfg.claude_source.as_deref() != Some(source)
        || cfg.grok_source.as_deref() != Some(source);
    if changed {
        set_global_source(cfg, source);
    }
    changed
}

fn providers_mut<'a>(
    cfg: &'a mut Config,
    target_tool: &str,
) -> Result<&'a mut Vec<Provider>, ConfigError> {
    match target_tool {
        "codex" => Ok(&mut cfg.codex_providers),
        "claude" => Ok(&mut cfg.claude_providers),
        "grok" => Ok(&mut cfg.grok_providers),
        other => Err(ConfigError::UnknownTargetTool(other.to_string())),
    }
}

fn providers_ref<'a>(
    cfg: &'a Config,
    target_tool: &str,
) -> Result<&'a Vec<Provider>, ConfigError> {
    match target_tool {
        "codex" => Ok(&cfg.codex_providers),
        "claude" => Ok(&cfg.claude_providers),
        "grok" => Ok(&cfg.grok_providers),
        other => Err(ConfigError::UnknownTargetTool(other.to_string())),
    }
}

fn active_fields_mut<'a>(
    cfg: &'a mut Config,
    target_tool: &str,
) -> Result<(&'a mut String, &'a mut String), ConfigError> {
    match target_tool {
        "codex" => Ok((&mut cfg.codex_active_app, &mut cfg.codex_active_wsl)),
        "claude" => Ok((&mut cfg.claude_active_app, &mut cfg.claude_active_wsl)),
        "grok" => Ok((&mut cfg.grok_active_app, &mut cfg.grok_active_wsl)),
        other => Err(ConfigError::UnknownTargetTool(other.to_string())),
    }
}

fn active_fields_ref<'a>(
    cfg: &'a Config,
    target_tool: &str,
) -> Result<(&'a String, &'a String), ConfigError> {
    match target_tool {
        "codex" => Ok((&cfg.codex_active_app, &cfg.codex_active_wsl)),
        "claude" => Ok((&cfg.claude_active_app, &cfg.claude_active_wsl)),
        "grok" => Ok((&cfg.grok_active_app, &cfg.grok_active_wsl)),
        other => Err(ConfigError::UnknownTargetTool(other.to_string())),
    }
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

    let mut cfg: Config = serde_yaml::from_slice(&data)?;
    let mut changed = false;

    // 自动迁移 1：极早期版本的旧字段，迁移到 Codex 配置
    if cfg.codex_providers.is_empty() && !cfg.providers.is_empty() {
        cfg.codex_providers = cfg.providers.clone();
        cfg.codex_active_app = cfg.active.clone(); // 默认给 app
        cfg.providers.clear();
        cfg.active.clear();
        changed = true;
    }

    // 自动迁移 2：单状态到多源状态的迁移
    if !cfg.codex_active.is_empty() {
        if cfg.codex_source.as_deref() == Some("wsl") {
            cfg.codex_active_wsl = cfg.codex_active.clone();
        } else {
            cfg.codex_active_app = cfg.codex_active.clone();
        }
        cfg.codex_active.clear();
        changed = true;
    }
    if !cfg.claude_active.is_empty() {
        if cfg.claude_source.as_deref() == Some("app") {
            cfg.claude_active_app = cfg.claude_active.clone();
        } else {
            cfg.claude_active_wsl = cfg.claude_active.clone();
        }
        cfg.claude_active.clear();
        changed = true;
    }

    if sync_global_source_fields(&mut cfg) {
        changed = true;
    }

    if changed {
        // 立即保存迁移后的配置
        save(&cfg)?;
    }

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

pub fn add_provider(cfg: &mut Config, target_tool: &str, mut p: Provider) -> Result<(), ConfigError> {
    let providers = providers_mut(cfg, target_tool)?;

    if providers.iter().any(|prov| prov.name == p.name) {
        return Err(ConfigError::ProviderExists(p.name));
    }
    if p.wire_api.is_empty() {
        // Grok 默认 chat_completions，其余工具默认 responses
        p.wire_api = if target_tool == "grok" {
            "chat_completions".to_string()
        } else {
            default_wire_api()
        };
    }
    providers.push(p.clone());
    save(cfg)
}

pub fn edit_provider(cfg: &mut Config, target_tool: &str, old_name: &str, mut updated: Provider) -> Result<(), ConfigError> {
    {
        let providers = providers_mut(cfg, target_tool)?;

        let idx = providers
            .iter()
            .position(|prov| prov.name == old_name)
            .ok_or_else(|| ConfigError::ProviderNotFound(old_name.to_string()))?;

        if updated.name != old_name && providers.iter().any(|prov| prov.name == updated.name) {
            return Err(ConfigError::ProviderExists(updated.name));
        }

        if updated.wire_api.is_empty() {
            updated.wire_api = if target_tool == "grok" {
                "chat_completions".to_string()
            } else {
                default_wire_api()
            };
        }

        let existing = &providers[idx];
        if updated.last_test.is_none() {
            updated.last_test = existing.last_test;
        }
        if updated.latency_ms.is_none() {
            updated.latency_ms = existing.latency_ms;
        }
        if updated.last_ok.is_none() {
            updated.last_ok = existing.last_ok;
        }

        providers[idx] = updated.clone();
    }

    {
        let (active_app, active_wsl) = active_fields_mut(cfg, target_tool)?;
        if *active_app == old_name {
            *active_app = updated.name.clone();
        }
        if *active_wsl == old_name {
            *active_wsl = updated.name.clone();
        }
    }

    save(cfg)
}

pub fn remove_provider(cfg: &mut Config, target_tool: &str, name: &str) -> Result<(), ConfigError> {
    let (active_app, active_wsl) = active_fields_ref(cfg, target_tool)?;

    let mut active_sources = Vec::new();
    if *active_app == name {
        active_sources.push("app");
    }
    if *active_wsl == name {
        active_sources.push("wsl");
    }

    if !active_sources.is_empty() {
        return Err(ConfigError::CannotRemoveActive {
            name: name.to_string(),
            target_tool: target_tool.to_string(),
            sources: active_sources.join(", "),
        });
    }

    let providers = providers_mut(cfg, target_tool)?;
    let idx = providers
        .iter()
        .position(|prov| prov.name == name)
        .ok_or_else(|| ConfigError::ProviderNotFound(name.to_string()))?;

    providers.remove(idx);
    save(cfg)
}

pub fn set_active(cfg: &mut Config, target_tool: &str, source: &str, name: &str) -> Result<(), ConfigError> {
    {
        let providers = providers_ref(cfg, target_tool)?;
        if !providers.iter().any(|prov| prov.name == name) {
            return Err(ConfigError::ProviderNotFound(name.to_string()));
        }
    }

    let (active_app, active_wsl) = active_fields_mut(cfg, target_tool)?;
    if source == "wsl" {
        *active_wsl = name.to_string();
    } else {
        *active_app = name.to_string();
    }
    save(cfg)
}

pub fn get_provider<'a>(cfg: &'a Config, target_tool: &str, name: &str) -> Option<&'a Provider> {
    let providers = providers_ref(cfg, target_tool).ok()?;
    providers.iter().find(|prov| prov.name == name)
}

pub fn get_active<'a>(cfg: &'a Config, target_tool: &str, source: &str) -> Option<&'a Provider> {
    let (active_app, active_wsl) = active_fields_ref(cfg, target_tool).ok()?;
    let active_name = if source == "wsl" { active_wsl } else { active_app };
    get_provider(cfg, target_tool, active_name)
}

pub fn update_test_result(cfg: &mut Config, target_tool: &str, name: &str, latency_ms: i64, ok: bool) -> Result<(), ConfigError> {
    let providers = providers_mut(cfg, target_tool)?;

    let idx = providers
        .iter()
        .position(|prov| prov.name == name)
        .ok_or_else(|| ConfigError::ProviderNotFound(name.to_string()))?;

    providers[idx].last_test = Some(Local::now());
    providers[idx].latency_ms = Some(latency_ms);
    providers[idx].last_ok = Some(ok);

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
            claude_models: None,
        };
        add_provider(&mut cfg, "codex", p).unwrap();

        assert_eq!(cfg.codex_providers.len(), 1);
        assert_eq!(cfg.codex_active_app, "");
        assert_eq!(cfg.codex_providers[0].wire_api, "responses");
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
            claude_models: None,
        };
        add_provider(&mut cfg, "codex", p.clone()).unwrap();
        let err = add_provider(&mut cfg, "codex", p);
        assert!(err.is_err());
    }

    #[test]
    fn test_first_provider_does_not_become_active() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        add_provider(&mut cfg, "codex", Provider {
            name: "b".to_string(),
            base_url: "https://b.com/v1".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();

        assert_eq!(cfg.codex_active_app, "");
    }

    #[test]
    fn test_remove_provider() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        add_provider(&mut cfg, "codex", Provider {
            name: "b".to_string(),
            base_url: "https://b.com/v1".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        set_active(&mut cfg, "codex", "app", "b").unwrap();
        remove_provider(&mut cfg, "codex", "a").unwrap();

        assert_eq!(cfg.codex_providers.len(), 1);
    }

    #[test]
    fn test_remove_active_provider_fails() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        set_active(&mut cfg, "codex", "app", "a").unwrap();

        let err = remove_provider(&mut cfg, "codex", "a");
        match err {
            Err(ConfigError::CannotRemoveActive { name, target_tool, sources }) => {
                assert_eq!(name, "a");
                assert_eq!(target_tool, "codex");
                assert_eq!(sources, "app");
            }
            other => panic!("expected CannotRemoveActive, got {:?}", other),
        }
    }

    #[test]
    fn test_remove_provider_active_in_both_sources_fails_with_both_sources() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "claude", Provider {
            name: "shared".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        set_active(&mut cfg, "claude", "app", "shared").unwrap();
        set_active(&mut cfg, "claude", "wsl", "shared").unwrap();

        let err = remove_provider(&mut cfg, "claude", "shared");
        match err {
            Err(ConfigError::CannotRemoveActive { name, target_tool, sources }) => {
                assert_eq!(name, "shared");
                assert_eq!(target_tool, "claude");
                assert_eq!(sources, "app, wsl");
            }
            other => panic!("expected CannotRemoveActive, got {:?}", other),
        }
    }

    #[test]
    fn test_remove_nonexistent_provider_fails() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();

        let err = remove_provider(&mut cfg, "codex", "nonexistent");
        assert!(err.is_err());
    }

    #[test]
    fn test_set_active() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        add_provider(&mut cfg, "codex", Provider {
            name: "b".to_string(),
            base_url: "https://b.com/v1".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();

        set_active(&mut cfg, "codex", "app", "b").unwrap();
        assert_eq!(cfg.codex_active_app, "b");
    }

    #[test]
    fn test_persistence() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "sk-abc".to_string(),
            model: "gpt-4".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();

        let loaded = load().unwrap();
        assert_eq!(loaded.codex_providers.len(), 1);
        assert_eq!(loaded.codex_providers[0].api_key, "sk-abc");
    }

    #[test]
    fn test_multi_source_active_persistence() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "node-a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "sk-abc".to_string(),
            model: "gpt-4".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        add_provider(&mut cfg, "codex", Provider {
            name: "node-b".to_string(),
            base_url: "https://b.com/v1".to_string(),
            api_key: "sk-def".to_string(),
            model: "gpt-4".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();

        set_active(&mut cfg, "codex", "app", "node-a").unwrap();
        set_active(&mut cfg, "codex", "wsl", "node-b").unwrap();

        let loaded = load().unwrap();
        assert_eq!(loaded.codex_active_app, "node-a");
        assert_eq!(loaded.codex_active_wsl, "node-b");
    }

    #[test]
    fn test_effective_source_defaults_to_wsl() {
        let cfg = Config::default();
        assert_eq!(effective_source(&cfg), SOURCE_WSL);
    }

    #[test]
    fn test_load_syncs_global_source_fields() {
        let (_dir, _path) = setup_test();
        let cfg = Config {
            codex_source: Some("app".to_string()),
            claude_source: Some("wsl".to_string()),
            grok_source: Some("wsl".to_string()),
            ..Config::default()
        };
        save(&cfg).unwrap();

        let loaded = load().unwrap();
        assert_eq!(loaded.codex_source.as_deref(), Some(SOURCE_APP));
        assert_eq!(loaded.claude_source.as_deref(), Some(SOURCE_APP));
        assert_eq!(loaded.grok_source.as_deref(), Some(SOURCE_APP));
    }

    #[test]
    fn test_grok_provider_crud_and_default_wire_api() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(
            &mut cfg,
            "grok",
            Provider {
                name: "relay".to_string(),
                base_url: "https://example.com/v1".to_string(),
                api_key: "sk-xxx".to_string(),
                model: "grok-4.5".to_string(),
                wire_api: "".to_string(),
                remark: None,
                last_test: None,
                latency_ms: None,
                last_ok: None,
                claude_models: None,
            },
        )
        .unwrap();

        assert_eq!(cfg.grok_providers.len(), 1);
        assert_eq!(cfg.grok_providers[0].wire_api, "chat_completions");

        set_active(&mut cfg, "grok", "wsl", "relay").unwrap();
        assert_eq!(cfg.grok_active_wsl, "relay");
        assert_eq!(get_active(&cfg, "grok", "wsl").unwrap().name, "relay");
    }

    #[test]
    fn test_unknown_target_tool_errors() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        let err = add_provider(
            &mut cfg,
            "unknown",
            Provider {
                name: "x".to_string(),
                base_url: "https://x.com".to_string(),
                api_key: "k".to_string(),
                model: "m".to_string(),
                wire_api: "".to_string(),
                remark: None,
                last_test: None,
                latency_ms: None,
                last_ok: None,
                claude_models: None,
            },
        );
        assert!(matches!(err, Err(ConfigError::UnknownTargetTool(_))));
    }

    #[test]
    #[cfg(unix)]
    fn test_config_file_permissions() {
        let (_dir, path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();

        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn test_update_test_result() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();

        update_test_result(&mut cfg, "codex", "a", 123, true).unwrap();
        let p = get_provider(&cfg, "codex", "a").unwrap();
        assert_eq!(p.latency_ms, Some(123));
        assert_eq!(p.last_ok, Some(true));
        assert!(p.last_test.is_some());
    }

    #[test]
    fn test_remark_persistence() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com/v1".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: Some("My backup endpoint".to_string()),
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();

        let loaded = load().unwrap();
        assert_eq!(loaded.codex_providers.len(), 1);
        assert_eq!(loaded.codex_providers[0].remark, Some("My backup endpoint".to_string()));
    }

    #[test]
    fn test_edit_provider() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: Some("old".to_string()),
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
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
            claude_models: None,
        };
        edit_provider(&mut cfg, "codex", "a", updated).unwrap();

        let p = get_provider(&cfg, "codex", "a").unwrap();
        assert_eq!(p.base_url, "https://new-a.com");
        assert_eq!(p.api_key, "new-k1");
        assert_eq!(p.model, "new-m1");
        assert_eq!(p.remark, Some("new".to_string()));
    }

    #[test]
    fn test_edit_provider_rename_active() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        add_provider(&mut cfg, "codex", Provider {
            name: "b".to_string(),
            base_url: "https://b.com".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        set_active(&mut cfg, "codex", "app", "a").unwrap();

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
            claude_models: None,
        };
        edit_provider(&mut cfg, "codex", "a", updated).unwrap();

        assert_eq!(cfg.codex_active_app, "new-a");
        assert!(get_provider(&cfg, "codex", "a").is_none());
        assert!(get_provider(&cfg, "codex", "new-a").is_some());
    }

    #[test]
    fn test_edit_provider_duplicate_name() {
        let (_dir, _path) = setup_test();
        let mut cfg = Config::default();
        add_provider(&mut cfg, "codex", Provider {
            name: "a".to_string(),
            base_url: "https://a.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }).unwrap();
        add_provider(&mut cfg, "codex", Provider {
            name: "b".to_string(),
            base_url: "https://b.com".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
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
            claude_models: None,
        };
        let err = edit_provider(&mut cfg, "codex", "a", updated);
        assert!(err.is_err());
    }
}
