use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

use crate::config::{Config, Env};
use crate::protocol::Protocol;
use crate::supplier::Supplier;
use crate::target::AgentAdapter;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Codex,
    Claude,
    Grok,
    OpenCode,
}

impl AgentKind {
    pub const ALL: [AgentKind; 4] = [
        AgentKind::Codex,
        AgentKind::Claude,
        AgentKind::Grok,
        AgentKind::OpenCode,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AgentKind::Codex => "codex",
            AgentKind::Claude => "claude",
            AgentKind::Grok => "grok",
            AgentKind::OpenCode => "opencode",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            AgentKind::Codex => "Codex",
            AgentKind::Claude => "Claude Code",
            AgentKind::Grok => "Grok CLI",
            AgentKind::OpenCode => "OpenCode",
        }
    }

    pub fn binary_name(self) -> &'static str {
        match self {
            AgentKind::Codex => "codex",
            AgentKind::Claude => "claude",
            AgentKind::Grok => "grok",
            AgentKind::OpenCode => "opencode",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "codex" => Some(AgentKind::Codex),
            "claude" => Some(AgentKind::Claude),
            "grok" => Some(AgentKind::Grok),
            "opencode" => Some(AgentKind::OpenCode),
            _ => None,
        }
    }

    pub fn capabilities(self) -> AgentCapabilities {
        match self {
            AgentKind::Codex => AgentCapabilities {
                protocols: vec![Protocol::Responses, Protocol::ChatCompletions],
                reasoning_effort: true,
                plan_reasoning_effort: true,
                context_length: true,
                compact_ratio: true,
                claude_models: false,
                grok_web_search: false,
            },
            AgentKind::Claude => AgentCapabilities {
                protocols: vec![Protocol::Messages],
                reasoning_effort: true,
                plan_reasoning_effort: false,
                context_length: true,
                compact_ratio: true,
                claude_models: true,
                grok_web_search: false,
            },
            AgentKind::Grok => AgentCapabilities {
                protocols: vec![
                    Protocol::ChatCompletions,
                    Protocol::Responses,
                    Protocol::Messages,
                ],
                reasoning_effort: true,
                plan_reasoning_effort: false,
                context_length: true,
                compact_ratio: true,
                claude_models: false,
                grok_web_search: true,
            },
            AgentKind::OpenCode => AgentCapabilities {
                protocols: vec![
                    Protocol::ChatCompletions,
                    Protocol::Messages,
                    Protocol::Responses,
                ],
                reasoning_effort: true,
                plan_reasoning_effort: false,
                context_length: true,
                compact_ratio: true,
                claude_models: false,
                grok_web_search: false,
            },
        }
    }

    pub fn reasoning_options(self) -> Vec<&'static str> {
        match self {
            AgentKind::Codex => vec!["low", "medium", "high", "xhigh", "max", "ultra"],
            AgentKind::Claude => vec!["low", "medium", "high", "xhigh", "max"],
            AgentKind::Grok => vec!["minimal", "low", "medium", "high", "xhigh", "max"],
            AgentKind::OpenCode => vec!["minimal", "low", "medium", "high", "xhigh"],
        }
    }

    /// 该 agent 需要的协议。Claude 固定 messages，Codex 优先 responses。
    pub fn preferred_protocol(self) -> Protocol {
        match self {
            AgentKind::Codex => Protocol::Responses,
            AgentKind::Claude => Protocol::Messages,
            AgentKind::Grok => Protocol::ChatCompletions,
            AgentKind::OpenCode => Protocol::ChatCompletions,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AgentCapabilities {
    pub protocols: Vec<Protocol>,
    pub reasoning_effort: bool,
    pub plan_reasoning_effort: bool,
    pub context_length: bool,
    pub compact_ratio: bool,
    pub claude_models: bool,
    pub grok_web_search: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ClaudeModels {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opus: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sonnet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub haiku: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent: Option<String>,
}

impl ClaudeModels {
    pub fn normalize(mut self) -> Option<Self> {
        self.opus = normalize_optional(self.opus);
        self.sonnet = normalize_optional(self.sonnet);
        self.haiku = normalize_optional(self.haiku);
        self.subagent = normalize_optional(self.subagent);
        if self.opus.is_none()
            && self.sonnet.is_none()
            && self.haiku.is_none()
            && self.subagent.is_none()
        {
            None
        } else {
            Some(self)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum GrokWebSearch {
    Keep,
    #[default]
    Follow,
    Model {
        model: String,
    },
    Unset,
}

impl GrokWebSearch {
    fn normalize(self) -> Option<Self> {
        match self {
            GrokWebSearch::Model { model } => {
                let model = model.trim().to_string();
                if model.is_empty() {
                    None
                } else {
                    Some(GrokWebSearch::Model { model })
                }
            }
            other => Some(other),
        }
    }
}

/// 某个 Agent（工具 + 环境）唯一的一份配置。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AgentConfig {
    pub supplier_id: String,
    #[serde(default)]
    pub protocol: Protocol,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_reasoning_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_length: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compact_ratio: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_models: Option<ClaudeModels>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grok_web_search: Option<GrokWebSearch>,
}

impl AgentConfig {
    pub fn reasoning_effort_level(&self) -> Option<&str> {
        trimmed_level(self.reasoning_effort.as_deref())
    }

    pub fn plan_reasoning_effort_level(&self) -> Option<&str> {
        trimmed_level(self.plan_reasoning_effort.as_deref())
    }

    pub fn context_tokens(&self) -> Option<u64> {
        self.context_length
            .as_deref()
            .and_then(parse_context_tokens)
    }

    pub fn compact_ratio_percent(&self) -> Option<u8> {
        self.compact_ratio
            .filter(|ratio| (70..=100).contains(ratio))
    }

    pub fn compact_token_limit(&self) -> Option<u64> {
        let tokens = self.context_tokens()?;
        let ratio = self.compact_ratio_percent()?;
        Some(clamp_compact_tokens(tokens, ratio))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentConfigInput {
    pub supplier_id: String,
    pub protocol: Protocol,
    pub model: String,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub plan_reasoning_effort: Option<String>,
    #[serde(default)]
    pub context_length: Option<String>,
    #[serde(default)]
    pub compact_ratio: Option<u8>,
    #[serde(default)]
    pub claude_models: Option<ClaudeModels>,
    #[serde(default)]
    pub grok_web_search: Option<GrokWebSearch>,
}

impl Default for AgentConfigInput {
    fn default() -> Self {
        Self {
            supplier_id: String::new(),
            protocol: Protocol::ChatCompletions,
            model: String::new(),
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
            claude_models: None,
            grok_web_search: None,
        }
    }
}

/// 一个工具在两个环境下的配置。`None` 表示该环境还没配置。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct AgentEnvs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wsl: Option<AgentConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<AgentConfig>,
}

impl AgentEnvs {
    pub fn get(&self, env: Env) -> Option<&AgentConfig> {
        match env {
            Env::Wsl => self.wsl.as_ref(),
            Env::Windows => self.windows.as_ref(),
        }
    }

    pub fn set(&mut self, env: Env, config: Option<AgentConfig>) {
        match env {
            Env::Wsl => self.wsl = config,
            Env::Windows => self.windows = config,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.wsl.is_none() && self.windows.is_none()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Agents {
    #[serde(default)]
    pub codex: AgentEnvs,
    #[serde(default)]
    pub claude: AgentEnvs,
    #[serde(default)]
    pub grok: AgentEnvs,
    #[serde(default)]
    pub opencode: AgentEnvs,
}

impl Agents {
    pub fn get(&self, kind: AgentKind) -> &AgentEnvs {
        match kind {
            AgentKind::Codex => &self.codex,
            AgentKind::Claude => &self.claude,
            AgentKind::Grok => &self.grok,
            AgentKind::OpenCode => &self.opencode,
        }
    }

    pub fn get_mut(&mut self, kind: AgentKind) -> &mut AgentEnvs {
        match kind {
            AgentKind::Codex => &mut self.codex,
            AgentKind::Claude => &mut self.claude,
            AgentKind::Grok => &mut self.grok,
            AgentKind::OpenCode => &mut self.opencode,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct EnvDirs {
    #[serde(default)]
    pub wsl: String,
    #[serde(default)]
    pub windows: String,
}

impl EnvDirs {
    pub fn get(&self, env: Env) -> Option<&Path> {
        let value = match env {
            Env::Wsl => &self.wsl,
            Env::Windows => &self.windows,
        };
        let value = value.trim();
        if value.is_empty() {
            None
        } else {
            Some(Path::new(value))
        }
    }

    pub fn set(&mut self, env: Env, value: String) {
        let value = value.trim().to_string();
        match env {
            Env::Wsl => self.wsl = value,
            Env::Windows => self.windows = value,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct CustomDirs {
    #[serde(default)]
    pub codex: EnvDirs,
    #[serde(default)]
    pub claude: EnvDirs,
    #[serde(default)]
    pub grok: EnvDirs,
    #[serde(default)]
    pub opencode: EnvDirs,
}

impl CustomDirs {
    pub fn dirs(&self, kind: AgentKind) -> &EnvDirs {
        match kind {
            AgentKind::Codex => &self.codex,
            AgentKind::Claude => &self.claude,
            AgentKind::Grok => &self.grok,
            AgentKind::OpenCode => &self.opencode,
        }
    }

    pub fn dirs_mut(&mut self, kind: AgentKind) -> &mut EnvDirs {
        match kind {
            AgentKind::Codex => &mut self.codex,
            AgentKind::Claude => &mut self.claude,
            AgentKind::Grok => &mut self.grok,
            AgentKind::OpenCode => &mut self.opencode,
        }
    }

    pub fn get(&self, kind: AgentKind, env: Env) -> Option<&Path> {
        self.dirs(kind).get(env)
    }

    pub fn set(&mut self, kind: AgentKind, env: Env, value: String) {
        self.dirs_mut(kind).set(env, value);
    }
}

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent 不存在：{0}")]
    UnknownAgent(String),
    #[error("该 Agent 还没有配置")]
    NotConfigured,
    #[error("供应商不存在：{0}")]
    SupplierNotFound(String),
    #[error("{agent} 不支持 {protocol} 协议")]
    UnsupportedProtocol {
        agent: &'static str,
        protocol: &'static str,
    },
    #[error("供应商尚未确认支持 {0} 协议，请先探测或勾选协议")]
    SupplierProtocolMissing(&'static str),
    #[error("模型不能为空")]
    EmptyModel,
    #[error("写入配置失败：{0}")]
    Write(String),
}

/// 校验并生成配置，不保存、不写目标工具。
pub fn build_config(
    cfg: &Config,
    kind: AgentKind,
    input: AgentConfigInput,
) -> Result<AgentConfig, AgentError> {
    normalize_config(cfg, kind, input)
}

/// 只保存到 CXC 配置，不写目标工具（同步、迁移用）。
pub fn set_config(
    cfg: &mut Config,
    kind: AgentKind,
    env: Env,
    input: AgentConfigInput,
) -> Result<AgentConfig, AgentError> {
    let config = normalize_config(cfg, kind, input)?;
    cfg.agent_mut(kind).set(env, Some(config.clone()));
    Ok(config)
}

/// 校验 → 写目标工具 → 保存到 CXC。写入失败时 CXC 配置保持不变。
pub fn save_and_apply(
    cfg: &mut Config,
    kind: AgentKind,
    env: Env,
    input: AgentConfigInput,
    adapter: &dyn AgentAdapter,
) -> Result<AgentConfig, AgentError> {
    let config = normalize_config(cfg, kind, input)?;
    let supplier = cfg
        .supplier(&config.supplier_id)
        .cloned()
        .ok_or_else(|| AgentError::SupplierNotFound(config.supplier_id.clone()))?;

    adapter
        .apply(&supplier, &config, cfg.custom_dirs.get(kind, env), env)
        .map_err(|error| AgentError::Write(error.to_string()))?;

    cfg.agent_mut(kind).set(env, Some(config.clone()));
    Ok(config)
}

/// 按已保存的配置重新写入目标工具。
pub fn rewrite(
    cfg: &Config,
    kind: AgentKind,
    env: Env,
    adapter: &dyn AgentAdapter,
) -> Result<(), AgentError> {
    let config = cfg.config_of(kind, env).ok_or(AgentError::NotConfigured)?;
    let supplier = cfg
        .supplier(&config.supplier_id)
        .ok_or_else(|| AgentError::SupplierNotFound(config.supplier_id.clone()))?;
    adapter
        .apply(supplier, config, cfg.custom_dirs.get(kind, env), env)
        .map_err(|error| AgentError::Write(error.to_string()))
}

pub fn clear_config(cfg: &mut Config, kind: AgentKind, env: Env) {
    cfg.agent_mut(kind).set(env, None);
}

fn normalize_config(
    cfg: &Config,
    kind: AgentKind,
    input: AgentConfigInput,
) -> Result<AgentConfig, AgentError> {
    let supplier = cfg
        .supplier(&input.supplier_id)
        .ok_or_else(|| AgentError::SupplierNotFound(input.supplier_id.clone()))?;
    validate_protocol(kind, supplier, input.protocol)?;

    let model = input.model.trim();
    if model.is_empty() {
        return Err(AgentError::EmptyModel);
    }

    let capabilities = kind.capabilities();
    let reasoning_effort = if capabilities.reasoning_effort {
        trimmed_level(input.reasoning_effort.as_deref()).map(str::to_string)
    } else {
        None
    };
    let plan_reasoning_effort = if capabilities.plan_reasoning_effort {
        trimmed_level(input.plan_reasoning_effort.as_deref()).map(str::to_string)
    } else {
        None
    };
    let context_length = if capabilities.context_length {
        input
            .context_length
            .as_deref()
            .and_then(canonical_context_length)
            .filter(|length| kind != AgentKind::Claude || length == "1m")
    } else {
        None
    };
    let compact_ratio = if capabilities.compact_ratio {
        input
            .compact_ratio
            .filter(|ratio| (70..=100).contains(ratio))
    } else {
        None
    };
    let claude_models = if capabilities.claude_models {
        input.claude_models.and_then(ClaudeModels::normalize)
    } else {
        None
    };
    let grok_web_search = if capabilities.grok_web_search {
        input.grok_web_search.and_then(GrokWebSearch::normalize)
    } else {
        None
    };

    Ok(AgentConfig {
        supplier_id: input.supplier_id,
        protocol: input.protocol,
        model: model.to_string(),
        reasoning_effort,
        plan_reasoning_effort,
        context_length,
        compact_ratio,
        claude_models,
        grok_web_search,
    })
}

fn validate_protocol(
    kind: AgentKind,
    supplier: &Supplier,
    protocol: Protocol,
) -> Result<(), AgentError> {
    if !kind.capabilities().protocols.contains(&protocol) {
        return Err(AgentError::UnsupportedProtocol {
            agent: kind.display_name(),
            protocol: protocol.as_str(),
        });
    }
    if !supplier.protocols.contains(&protocol) {
        return Err(AgentError::SupplierProtocolMissing(protocol.as_str()));
    }
    Ok(())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn trimmed_level(value: Option<&str>) -> Option<&str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("none"))
}

const COMPACT_TOKEN_MIN: u64 = 100_000;
const COMPACT_TOKEN_MAX: u64 = 1_000_000;

pub fn parse_context_tokens(raw: &str) -> Option<u64> {
    let text = raw.trim().to_ascii_lowercase();
    if text.is_empty() || text == "none" {
        return None;
    }
    let (coefficient, multiplier) = if let Some(rest) = text.strip_suffix('k') {
        (rest, 1_000u64)
    } else if let Some(rest) = text.strip_suffix('m') {
        (rest, 1_000_000u64)
    } else {
        (text.as_str(), 1u64)
    };
    if coefficient.is_empty() || !coefficient.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number: u64 = coefficient.parse().ok()?;
    if number == 0 {
        return None;
    }
    number.checked_mul(multiplier)
}

pub fn canonical_context_length(raw: &str) -> Option<String> {
    parse_context_tokens(raw).map(format_context_length)
}

pub fn format_context_length(tokens: u64) -> String {
    if tokens.is_multiple_of(1_000_000) {
        format!("{}m", tokens / 1_000_000)
    } else if tokens.is_multiple_of(1_000) {
        format!("{}k", tokens / 1_000)
    } else {
        tokens.to_string()
    }
}

pub fn clamp_compact_tokens(context_tokens: u64, ratio: u8) -> u64 {
    let raw = context_tokens.saturating_mul(u64::from(ratio)) / 100;
    raw.clamp(COMPACT_TOKEN_MIN, COMPACT_TOKEN_MAX)
}

pub fn strip_trailing_1m(model: &str) -> String {
    model.strip_suffix("[1m]").unwrap_or(model).to_string()
}

pub fn claude_model_for_context(model: &str, context_tokens: Option<u64>) -> String {
    let base = strip_trailing_1m(model);
    if context_tokens == Some(1_000_000) {
        format!("{base}[1m]")
    } else {
        base
    }
}

pub fn connectivity_model(model: &str, is_claude: bool) -> String {
    if is_claude {
        strip_trailing_1m(model)
    } else {
        model.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supplier::{SupplierInput, add_supplier};
    use std::path::PathBuf;

    fn supplier(cfg: &mut Config, protocols: Vec<Protocol>) -> Supplier {
        add_supplier(
            cfg,
            SupplierInput {
                name: "relay".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "sk-1".to_string(),
                remark: None,
                protocols: Some(protocols),
            },
        )
        .unwrap()
    }

    fn input(supplier_id: &str, protocol: Protocol, model: &str) -> AgentConfigInput {
        AgentConfigInput {
            supplier_id: supplier_id.to_string(),
            protocol,
            model: model.to_string(),
            ..AgentConfigInput::default()
        }
    }

    #[test]
    fn set_config_normalizes_settings() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let config = set_config(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            AgentConfigInput {
                reasoning_effort: Some(" high ".to_string()),
                plan_reasoning_effort: Some("none".to_string()),
                context_length: Some("200000".to_string()),
                compact_ratio: Some(80),
                ..input(&supplier.id, Protocol::Responses, "gpt-5")
            },
        )
        .unwrap();
        assert_eq!(config.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(config.plan_reasoning_effort, None);
        assert_eq!(config.context_length.as_deref(), Some("200k"));
        assert_eq!(config.compact_ratio, Some(80));
        assert_eq!(
            cfg.config_of(AgentKind::Codex, Env::Wsl)
                .map(|config| config.model.as_str()),
            Some("gpt-5")
        );
    }

    #[test]
    fn environments_are_independent() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        set_config(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
        )
        .unwrap();
        set_config(
            &mut cfg,
            AgentKind::Codex,
            Env::Windows,
            input(&supplier.id, Protocol::Responses, "gpt-5-mini"),
        )
        .unwrap();
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
    }

    #[test]
    fn claude_only_accepts_messages_and_1m() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Messages, Protocol::Responses]);
        assert!(matches!(
            set_config(
                &mut cfg,
                AgentKind::Claude,
                Env::Wsl,
                input(&supplier.id, Protocol::Responses, "claude")
            ),
            Err(AgentError::UnsupportedProtocol { .. })
        ));
        let config = set_config(
            &mut cfg,
            AgentKind::Claude,
            Env::Wsl,
            AgentConfigInput {
                context_length: Some("200k".to_string()),
                ..input(&supplier.id, Protocol::Messages, "claude")
            },
        )
        .unwrap();
        assert_eq!(config.context_length, None);
    }

    #[test]
    fn config_requires_supplier_protocol() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::ChatCompletions]);
        assert!(matches!(
            set_config(
                &mut cfg,
                AgentKind::Codex,
                Env::Wsl,
                input(&supplier.id, Protocol::Responses, "gpt-5")
            ),
            Err(AgentError::SupplierProtocolMissing(_))
        ));
    }

    struct FakeAdapter {
        fail: bool,
    }

    impl AgentAdapter for FakeAdapter {
        fn kind(&self) -> AgentKind {
            AgentKind::Codex
        }

        fn config_path(
            &self,
            _env: Env,
            _custom_dir: Option<&Path>,
        ) -> Result<PathBuf, crate::target::TargetError> {
            Ok(PathBuf::from("/tmp/cxc-v2-test"))
        }

        fn apply(
            &self,
            _supplier: &Supplier,
            _config: &AgentConfig,
            _custom_dir: Option<&Path>,
            _env: Env,
        ) -> Result<(), crate::target::TargetError> {
            if self.fail {
                Err(crate::target::TargetError::ParseError(
                    "fake write failure".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn failed_write_leaves_config_untouched() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let failing = FakeAdapter { fail: true };
        assert!(
            save_and_apply(
                &mut cfg,
                AgentKind::Codex,
                Env::Wsl,
                input(&supplier.id, Protocol::Responses, "gpt-5"),
                &failing,
            )
            .is_err()
        );
        assert!(cfg.config_of(AgentKind::Codex, Env::Wsl).is_none());

        let working = FakeAdapter { fail: false };
        save_and_apply(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
            &working,
        )
        .unwrap();
        assert!(cfg.config_of(AgentKind::Codex, Env::Wsl).is_some());
    }

    #[test]
    fn context_helpers() {
        assert_eq!(parse_context_tokens("1m"), Some(1_000_000));
        assert_eq!(parse_context_tokens("200k"), Some(200_000));
        assert_eq!(canonical_context_length("1000000"), Some("1m".to_string()));
        assert_eq!(clamp_compact_tokens(200_000, 80), 160_000);
        assert_eq!(claude_model_for_context("x[1m]", Some(1_000_000)), "x[1m]");
        assert_eq!(claude_model_for_context("x[1m]", None), "x");
    }
}
