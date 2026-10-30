use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::agent::{AgentConfig, AgentEnvs, AgentKind, AgentSlot, Agents, CustomDirs};
use crate::supplier::Supplier;
use crate::sync::SyncConfig;

pub const CONFIG_DIR_NAME: &str = "cxc-v2";
pub const CONFIG_FILE_NAME: &str = "config.yaml";
pub const CONFIG_VERSION: u32 = 4;

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

    pub fn slot_of(&self, kind: AgentKind, env: Env) -> Option<&AgentSlot> {
        self.agent(kind).get(env)
    }

    /// 该 `(工具, 环境)` 当前使用中的绑定。
    pub fn config_of(&self, kind: AgentKind, env: Env) -> Option<&AgentConfig> {
        self.slot_of(kind, env)?.active_binding()
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

    if version == 3 {
        let legacy: LegacyConfigV3 = serde_yaml::from_value(value)?;
        return Ok(legacy.into_config());
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
// 旧配置 → 版本 4
//   v3：每个 (工具, 环境) 一份配置 → 一条绑定，并设为使用中
//   v2：每个工具一份共享绑定列表 + 全局来源 → 主环境保留全部绑定，另一环境只保留激活项
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct LegacyConfigV3 {
    #[serde(default)]
    suppliers: Vec<Supplier>,
    #[serde(default)]
    agents: LegacyAgentsV3,
    #[serde(default)]
    custom_dirs: CustomDirs,
    #[serde(default)]
    sync: Option<SyncConfig>,
    #[serde(default)]
    migrated_from_v1: bool,
}

#[derive(Deserialize, Default)]
pub(crate) struct LegacyAgentsV3 {
    #[serde(default)]
    codex: LegacyAgentV3,
    #[serde(default)]
    claude: LegacyAgentV3,
    #[serde(default)]
    grok: LegacyAgentV3,
    #[serde(default)]
    opencode: LegacyAgentV3,
}

#[derive(Deserialize, Default)]
struct LegacyAgentV3 {
    #[serde(default)]
    wsl: Option<AgentConfig>,
    #[serde(default)]
    windows: Option<AgentConfig>,
}

impl LegacyConfigV3 {
    fn into_config(self) -> Config {
        Config {
            version: CONFIG_VERSION,
            suppliers: self.suppliers,
            agents: agents_from_v3(&self.agents),
            custom_dirs: self.custom_dirs,
            sync: self.sync,
            migrated_from_v1: self.migrated_from_v1,
        }
    }
}

impl LegacyAgentsV3 {
    fn get(&self, kind: AgentKind) -> &LegacyAgentV3 {
        match kind {
            AgentKind::Codex => &self.codex,
            AgentKind::Claude => &self.claude,
            AgentKind::Grok => &self.grok,
            AgentKind::OpenCode => &self.opencode,
        }
    }
}

/// 版本 3 的单份配置转成「一条绑定 + 使用中」。
pub(crate) fn agents_from_v3(legacy: &LegacyAgentsV3) -> Agents {
    let mut agents = Agents::default();
    for kind in AgentKind::ALL {
        let legacy = legacy.get(kind);
        let mut envs = AgentEnvs::default();
        for (env, config) in [
            (Env::Wsl, legacy.wsl.clone()),
            (Env::Windows, legacy.windows.clone()),
        ] {
            let Some(mut config) = config else {
                continue;
            };
            config.id = crate::agent::new_binding_id();
            envs.set(
                env,
                Some(AgentSlot {
                    active: Some(config.id.clone()),
                    bindings: vec![config],
                }),
            );
        }
        *agents.get_mut(kind) = envs;
    }
    agents
}

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
    bindings: Vec<AgentConfig>,
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
        let agents = agents_from_v2(&self.agents, self.source.as_deref());

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

/// 版本 2 的共享绑定列表：主环境（全局来源）保留全部绑定，另一环境只保留激活项。
pub(crate) fn agents_from_v2(legacy: &LegacyAgentsV2, source: Option<&str>) -> Agents {
    let primary = source.map(Env::parse).unwrap_or(Env::Wsl);
    let mut agents = Agents::default();
    for kind in AgentKind::ALL {
        let legacy = legacy.get(kind);
        let mut envs = AgentEnvs::default();
        for env in Env::ALL {
            let active = match env {
                Env::Wsl => legacy.active_wsl.as_deref(),
                Env::Windows => legacy.active_app.as_deref(),
            };
            let full_catalog = env == primary;
            if let Some(slot) = legacy.slot(active, full_catalog) {
                envs.set(env, Some(slot));
            }
        }
        *agents.get_mut(kind) = envs;
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
    /// `full_catalog` 为真时带上全部绑定，否则只保留激活项。
    fn slot(&self, active: Option<&str>, full_catalog: bool) -> Option<AgentSlot> {
        let active_binding =
            active.and_then(|id| self.bindings.iter().find(|binding| binding.id == id));

        let mut bindings: Vec<AgentConfig> = if full_catalog {
            self.bindings.clone()
        } else {
            active_binding.cloned().into_iter().collect()
        };
        if bindings.is_empty() {
            return None;
        }
        for binding in &mut bindings {
            if binding.id.is_empty() {
                binding.id = crate::agent::new_binding_id();
            }
        }

        let active_id = active_binding
            .map(|binding| binding.id.clone())
            .filter(|id| bindings.iter().any(|binding| &binding.id == id))
            .or_else(|| full_catalog.then(|| bindings[0].id.clone()));

        Some(AgentSlot {
            active: active_id,
            bindings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentConfig;
    use crate::protocol::Protocol;

    fn binding(id: &str, model: &str) -> AgentConfig {
        AgentConfig {
            id: id.to_string(),
            supplier_id: "s1".to_string(),
            protocol: Protocol::Responses,
            model: model.to_string(),
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
            claude_models: None,
            grok_web_search: None,
        }
    }

    #[test]
    fn env_parses_legacy_app_as_windows() {
        assert_eq!(Env::parse("app"), Env::Windows);
        assert_eq!(Env::parse("windows"), Env::Windows);
        assert_eq!(Env::parse("wsl"), Env::Wsl);
        assert_eq!(Env::parse(""), Env::Wsl);
    }

    #[test]
    fn upgrades_v3_single_config_into_one_binding() {
        let yaml = r#"
version: 3
suppliers:
  - id: s1
    name: relay
    base_url: https://api.example.com/v1
    api_key: sk-1
    protocols: [responses]
    models: []
agents:
  codex:
    wsl:
      supplier_id: s1
      protocol: responses
      model: gpt-5
      reasoning_effort: high
    windows: null
  claude: {}
  grok: {}
  opencode: {}
custom_dirs:
  codex: { wsl: /tmp/codex, windows: '' }
  claude: { wsl: '', windows: '' }
  grok: { wsl: '', windows: '' }
  opencode: { wsl: '', windows: '' }
migrated_from_v1: true
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        fs::write(&path, yaml).unwrap();

        let cfg = load_from(&path).unwrap();
        assert_eq!(cfg.version, CONFIG_VERSION);

        let slot = cfg.agent(AgentKind::Codex).get(Env::Wsl).unwrap();
        assert_eq!(slot.bindings.len(), 1);
        let binding = slot.active_binding().unwrap();
        assert_eq!(binding.model, "gpt-5");
        assert_eq!(binding.reasoning_effort.as_deref(), Some("high"));
        assert!(!binding.id.is_empty());
        assert!(cfg.agent(AgentKind::Codex).get(Env::Windows).is_none());
        assert_eq!(
            cfg.custom_dirs.get(AgentKind::Codex, Env::Wsl),
            Some(std::path::Path::new("/tmp/codex"))
        );
    }

    #[test]
    fn upgrades_v2_catalog_into_bindings() {
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

        let wsl = cfg.agent(AgentKind::Codex).get(Env::Wsl).unwrap();
        assert_eq!(wsl.bindings.len(), 2);
        assert_eq!(wsl.active_binding().unwrap().model, "gpt-5");

        let windows = cfg.agent(AgentKind::Codex).get(Env::Windows).unwrap();
        assert_eq!(windows.bindings.len(), 1);
        assert_eq!(windows.active_binding().unwrap().model, "gpt-5-mini");

        let claude = cfg.agent(AgentKind::Claude).get(Env::Wsl).unwrap();
        assert_eq!(claude.active_binding().unwrap().model, "claude-sonnet");
        assert!(cfg.agent(AgentKind::Claude).get(Env::Windows).is_none());
        assert_eq!(
            cfg.custom_dirs.get(AgentKind::Codex, Env::Wsl),
            Some(std::path::Path::new("/tmp/codex"))
        );
        assert!(cfg.migrated_from_v1);
    }

    #[test]
    fn roundtrips_v4_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        let mut cfg = Config::default();
        cfg.agents.codex.windows = Some(AgentSlot {
            active: Some("b1".to_string()),
            bindings: vec![binding("b1", "gpt-5")],
        });
        save_to(&path, &cfg).unwrap();

        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded, cfg);
    }
}
