use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::agent::{AgentKind, Agents, CustomDirs};
use crate::supplier::Supplier;
use crate::sync::SyncConfig;

pub const CONFIG_DIR_NAME: &str = "cxc-v2";
pub const CONFIG_FILE_NAME: &str = "config.yaml";
pub const CONFIG_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    #[default]
    App,
    Wsl,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::App => "app",
            Source::Wsl => "wsl",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("app") {
            Source::App
        } else {
            Source::Wsl
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("找不到用户配置目录")]
    NoConfigDir,
    #[error("读取配置文件失败 {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("解析配置文件失败: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("序列化配置失败: {0}")]
    SerializeError(serde_yaml::Error),
    #[error("创建配置目录失败 {path}: {source}")]
    CreateDirError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("写入配置文件失败 {path}: {source}")]
    WriteError {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub suppliers: Vec<Supplier>,
    #[serde(default)]
    pub agents: Agents,
    #[serde(default)]
    pub custom_dirs: CustomDirs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<SyncConfig>,
    #[serde(default)]
    pub migrated_from_v1: bool,
}

fn default_version() -> u32 {
    CONFIG_VERSION
}

fn default_source() -> Source {
    if cfg!(target_os = "windows") {
        Source::App
    } else {
        Source::Wsl
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            source: default_source(),
            suppliers: Vec::new(),
            agents: Agents::default(),
            custom_dirs: CustomDirs::default(),
            sync: None,
            migrated_from_v1: false,
        }
    }
}

impl Config {
    pub fn supplier(&self, id: &str) -> Option<&Supplier> {
        self.suppliers.iter().find(|supplier| supplier.id == id)
    }

    pub fn supplier_mut(&mut self, id: &str) -> Option<&mut Supplier> {
        self.suppliers.iter_mut().find(|supplier| supplier.id == id)
    }

    pub fn supplier_by_name(&self, name: &str) -> Option<&Supplier> {
        self.suppliers.iter().find(|supplier| supplier.name == name)
    }

    pub fn agent(&self, kind: AgentKind) -> &crate::agent::AgentConfig {
        self.agents.get(kind)
    }

    pub fn agent_mut(&mut self, kind: AgentKind) -> &mut crate::agent::AgentConfig {
        self.agents.get_mut(kind)
    }
}

/// 配置文件路径。测试可用 `CXC_V2_CONFIG_DIR` 覆盖目录。
pub fn config_path() -> Result<PathBuf, ConfigError> {
    if let Ok(dir) = std::env::var("CXC_V2_CONFIG_DIR") {
        return Ok(PathBuf::from(dir).join(CONFIG_FILE_NAME));
    }
    let mut path = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    path.push(CONFIG_DIR_NAME);
    path.push(CONFIG_FILE_NAME);
    Ok(path)
}

pub fn load() -> Result<Config, ConfigError> {
    load_from(&config_path()?)
}

pub fn load_from(path: &Path) -> Result<Config, ConfigError> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = fs::read(path).map_err(|source| ConfigError::ReadError {
        path: path.to_path_buf(),
        source,
    })?;
    let mut cfg: Config = serde_yaml::from_slice(&data)?;
    if cfg.version == 0 {
        cfg.version = CONFIG_VERSION;
    }
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<(), ConfigError> {
    save_to(&config_path()?, cfg)
}

pub fn save_to(path: &Path, cfg: &Config) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDirError {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let data = serde_yaml::to_string(cfg).map_err(ConfigError::SerializeError)?;
    write_atomic(path, data.as_bytes()).map_err(|source| ConfigError::WriteError {
        path: path.to_path_buf(),
        source,
    })
}

fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("yaml.tmp");
    fs::write(&tmp, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&tmp)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&tmp, perms)?;
    }
    #[cfg(windows)]
    {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
    fs::rename(&tmp, path)
}

pub fn backup_config_file() -> Result<Option<PathBuf>, ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let mut backup_path = path.clone();
    backup_path.set_file_name(format!("{}.backup.{}", CONFIG_FILE_NAME, timestamp));
    fs::copy(&path, &backup_path).map_err(|source| ConfigError::WriteError {
        path: backup_path.clone(),
        source,
    })?;
    Ok(Some(backup_path))
}

pub fn get_sync_config(cfg: &Config) -> SyncConfig {
    cfg.sync.clone().unwrap_or_default()
}

pub fn update_sync_config(cfg: &mut Config, sync: SyncConfig) {
    cfg.sync = Some(sync);
}

/// 旧版配置路径，仅用于一次性导入。
pub fn legacy_config_path() -> Result<PathBuf, ConfigError> {
    if let Ok(dir) = std::env::var("CXC_LEGACY_CONFIG_DIR") {
        return Ok(PathBuf::from(dir).join(CONFIG_FILE_NAME));
    }
    let mut path = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    path.push("cxc");
    path.push(CONFIG_FILE_NAME);
    Ok(path)
}

#[allow(dead_code)]
pub fn now() -> DateTime<Local> {
    Local::now()
}
