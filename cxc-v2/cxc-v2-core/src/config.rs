use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::agent::{AgentConfig, AgentEnvs, AgentKind, Agents, CustomDirs};
use crate::protocol::Protocol;
use crate::supplier::Supplier;
use crate::sync::SyncConfig;

pub const CONFIG_DIR_NAME: &str = "cxc-v2";
pub const CONFIG_FILE_NAME: &str = "config.yaml";
pub const CONFIG_VERSION: u32 = 3;

/// Agent 所在的运行环境。环境是 Agent 身份的一部分：
/// `Grok (WSL)` 与 `Grok (Windows)` 是两个独立 Agent。
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum Env {
    #[default]
    Wsl,
    Windows,
}

impl Env {
    pub const ALL: [Env; 2] = [Env::Wsl, Env::Windows];

    pub fn as_str(self) -> &'static str {
        match self {
            Env::Wsl => "wsl",
            Env::Windows => "windows",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Env::Wsl => "WSL",
            Env::Windows => "Windows",
        }
    }

    /// 兼容旧字段：`app` 视为 Windows。
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "windows" | "app" | "win" => Env::Windows,
            _ => Env::Wsl,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
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

    pub fn agent(&self, kind: AgentKind) -> &AgentEnvs {
        self.agents.get(kind)
    }

    pub fn agent_mut(&mut self, kind: AgentKind) -> &mut AgentEnvs {
        self.agents.get_mut(kind)
    }

    pub fn config_of(&self, kind: AgentKind, env: Env) -> Option<&AgentConfig> {
        self.agent(kind).get(env)
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
    let path = config_path()?;
    let cfg = load_from(&path)?;
    if path.exists() && read_version(&path)? < CONFIG_VERSION {
        save_to(&path, &cfg)?;
    }
    Ok(cfg)
}

pub fn load_from(path: &Path) -> Result<Config, ConfigError> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = fs::read(path).map_err(|source| ConfigError::ReadError {
        path: path.to_path_buf(),
        source,
    })?;
    let value: serde_yaml::Value = serde_yaml::from_slice(&data)?;
    let version = value
        .get("version")
        .and_then(serde_yaml::Value::as_u64)
        .unwrap_or(0) as u32;

    if version >= CONFIG_VERSION {
        let mut cfg: Config = serde_yaml::from_value(value)?;
        cfg.version = CONFIG_VERSION;
        return Ok(cfg);
    }

    let legacy: LegacyConfigV2 = serde_yaml::from_value(value)?;
    Ok(legacy.into_config())
}

fn read_version(path: &Path) -> Result<u32, ConfigError> {
    let data = fs::read(path).map_err(|source| ConfigError::ReadError {
        path: path.to_path_buf(),
        source,
    })?;
    let value: serde_yaml::Value = serde_yaml::from_slice(&data)?;
    Ok(value
        .get("version")
        .and_then(serde_yaml::Value::as_u64)
        .unwrap_or(0) as u32)
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

pub fn now() -> DateTime<Local> {
    Local::now()
}

// ---------------------------------------------------------------------------
// 版本 2 配置（每个工具多份绑定 + 全局来源）→ 版本 3（每个工具每个环境一份配置）
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct LegacyConfigV2 {
    #[serde(default)]
    suppliers: Vec<Supplier>,
    #[serde(default)]
    agents: LegacyAgentsV2,
    #[serde(default)]
    custom_dirs: LegacyCustomDirsV2,
    #[serde(default)]
    sync: Option<SyncConfig>,
    #[serde(default)]
    migrated_from_v1: bool,
    #[serde(default)]
    source: Option<String>,
}

#[derive(Deserialize, Default)]
pub(crate) struct LegacyAgentsV2 {
    #[serde(default)]
    codex: LegacyAgentV2,
    #[serde(default)]
    claude: LegacyAgentV2,
    #[serde(default)]
    grok: LegacyAgentV2,
    #[serde(default)]
    opencode: LegacyAgentV2,
}

#[derive(Deserialize, Default)]
pub(crate) struct LegacyAgentV2 {
    #[serde(default)]
    active_app: Option<String>,
    #[serde(default)]
    active_wsl: Option<String>,
    #[serde(default)]
    bindings: Vec<LegacyBindingV2>,
}

#[derive(Deserialize, Clone, Default)]
struct LegacyBindingV2 {
    #[serde(default)]
    id: String,
    #[serde(default)]
    supplier_id: String,
    #[serde(default)]
    protocol: Protocol,
    #[serde(default)]
    model: String,
    #[serde(default)]
    reasoning_effort: Option<String>,
    #[serde(default)]
    plan_reasoning_effort: Option<String>,
    #[serde(default)]
    context_length: Option<String>,
    #[serde(default)]
    compact_ratio: Option<u8>,
    #[serde(default)]
    claude_models: Option<crate::agent::ClaudeModels>,
    #[serde(default)]
    grok_web_search: Option<crate::agent::GrokWebSearch>,
}

#[derive(Deserialize, Default)]
struct LegacyCustomDirsV2 {
    #[serde(default)]
    codex: String,
    #[serde(default)]
    claude: String,
    #[serde(default)]
    grok: String,
    #[serde(default)]
    opencode: String,
}

impl LegacyConfigV2 {
    fn into_config(self) -> Config {
        let agents = agents_from_legacy(&self.agents);

        let env = self.source.as_deref().map(Env::parse).unwrap_or(Env::Wsl);
        let mut custom_dirs = CustomDirs::default();
        for kind in AgentKind::ALL {
            let value = self.custom_dirs.get(kind).trim().to_string();
            if !value.is_empty() {
                custom_dirs.set(kind, env, value);
            }
        }

        Config {
            version: CONFIG_VERSION,
            suppliers: self.suppliers,
            agents,
            custom_dirs,
            sync: self.sync,
            migrated_from_v1: self.migrated_from_v1,
        }
    }
}

/// 把版本 2 的供应商列表 + 旧 Agent 结构收敛成版本 3 的 Agent 配置。
pub(crate) fn agents_from_legacy(legacy: &LegacyAgentsV2) -> Agents {
    let mut agents = Agents::default();
    for kind in AgentKind::ALL {
        let legacy = legacy.get(kind);
        *agents.get_mut(kind) = AgentEnvs {
            wsl: legacy.pick(legacy.active_wsl.as_deref(), true),
            windows: legacy.pick(legacy.active_app.as_deref(), false),
        };
    }
    agents
}

impl LegacyAgentsV2 {
    fn get(&self, kind: AgentKind) -> &LegacyAgentV2 {
        match kind {
            AgentKind::Codex => &self.codex,
            AgentKind::Claude => &self.claude,
            AgentKind::Grok => &self.grok,
            AgentKind::OpenCode => &self.opencode,
        }
    }
}

impl LegacyCustomDirsV2 {
    fn get(&self, kind: AgentKind) -> &str {
        match kind {
            AgentKind::Codex => &self.codex,
            AgentKind::Claude => &self.claude,
            AgentKind::Grok => &self.grok,
            AgentKind::OpenCode => &self.opencode,
        }
    }
}

impl LegacyAgentV2 {
    /// 优先取激活项；WSL 侧没有激活项时退回第一条绑定。
    fn pick(&self, active: Option<&str>, fallback_first: bool) -> Option<AgentConfig> {
        let binding = active
            .and_then(|id| self.bindings.iter().find(|binding| binding.id == id))
            .or_else(|| fallback_first.then(|| self.bindings.first()).flatten())?;
        Some(binding.to_config())
    }
}

impl LegacyBindingV2 {
    fn to_config(&self) -> AgentConfig {
        AgentConfig {
            supplier_id: self.supplier_id.clone(),
            protocol: self.protocol,
            model: self.model.clone(),
            reasoning_effort: self.reasoning_effort.clone(),
            plan_reasoning_effort: self.plan_reasoning_effort.clone(),
            context_length: self.context_length.clone(),
            compact_ratio: self.compact_ratio,
            claude_models: self.claude_models.clone(),
            grok_web_search: self.grok_web_search.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentConfig;

    #[test]
    fn env_parses_legacy_app_as_windows() {
        assert_eq!(Env::parse("app"), Env::Windows);
        assert_eq!(Env::parse("windows"), Env::Windows);
        assert_eq!(Env::parse("wsl"), Env::Wsl);
        assert_eq!(Env::parse(""), Env::Wsl);
    }

    #[test]
    fn upgrades_v2_config_into_per_env_configs() {
        let yaml = r#"
version: 2
source: wsl
suppliers:
  - id: s1
    name: relay
    base_url: https://api.example.com/v1
    api_key: sk-1
    protocols: [responses]
    models: []
agents:
  codex:
    active_wsl: b1
    active_app: b2
    bindings:
      - id: b1
        supplier_id: s1
        protocol: responses
        model: gpt-5
        reasoning_effort: high
      - id: b2
        supplier_id: s1
        protocol: responses
        model: gpt-5-mini
  claude:
    bindings:
      - id: c1
        supplier_id: s1
        protocol: messages
        model: claude-sonnet
  grok:
    bindings: []
  opencode:
    bindings: []
custom_dirs:
  codex: /tmp/codex
  claude: ''
  grok: ''
  opencode: ''
migrated_from_v1: true
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(&path, yaml).unwrap();

        let cfg = load_from(&path).unwrap();
        assert_eq!(cfg.version, CONFIG_VERSION);
        assert_eq!(
            cfg.config_of(AgentKind::Codex, Env::Wsl)
                .map(|config| config.model.as_str()),
            Some("gpt-5")
        );
        assert_eq!(
            cfg.config_of(AgentKind::Codex, Env::Windows)
                .map(|config| config.model.as_str()),
            Some("gpt-5-mini")
        );
        assert_eq!(
            cfg.config_of(AgentKind::Claude, Env::Wsl)
                .map(|config| config.model.as_str()),
            Some("claude-sonnet")
        );
        assert!(cfg.config_of(AgentKind::Claude, Env::Windows).is_none());
        assert_eq!(
            cfg.custom_dirs.get(AgentKind::Codex, Env::Wsl),
            Some(std::path::Path::new("/tmp/codex"))
        );
        assert!(cfg.migrated_from_v1);
    }

    #[test]
    fn roundtrips_v3_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        let mut cfg = Config::default();
        cfg.agents.codex.windows = Some(AgentConfig {
            supplier_id: "s1".to_string(),
            protocol: Protocol::Responses,
            model: "gpt-5".to_string(),
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
            claude_models: None,
            grok_web_search: None,
        });
        save_to(&path, &cfg).unwrap();

        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded, cfg);
    }
}
