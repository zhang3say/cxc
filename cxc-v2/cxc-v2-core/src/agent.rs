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

/// 某个 Agent（工具 + 环境）下的一条绑定。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AgentConfig {
    /// 创建时生成，之后不变。启用、编辑、删除都按它定位。
    #[serde(default)]
    pub id: String,
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

/// 一个 `(工具, 环境)` 下保存的绑定列表与当前使用中的那条。
/// `None` 表示该环境还没配置。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct AgentSlot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    #[serde(default)]
    pub bindings: Vec<AgentConfig>,
}

impl AgentSlot {
    pub fn binding(&self, id: &str) -> Option<&AgentConfig> {
        self.bindings.iter().find(|binding| binding.id == id)
    }

    pub fn binding_mut(&mut self, id: &str) -> Option<&mut AgentConfig> {
        self.bindings.iter_mut().find(|binding| binding.id == id)
    }

    /// 当前使用中的绑定。`active` 指向不存在的 id 时视为没有。
    pub fn active_binding(&self) -> Option<&AgentConfig> {
        self.active.as_deref().and_then(|id| self.binding(id))
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.active.as_deref() == Some(id)
    }
}

/// 一个工具在两个环境下的绑定。`None` 表示该环境还没配置。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct AgentEnvs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wsl: Option<AgentSlot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<AgentSlot>,
}

impl AgentEnvs {
    pub fn get(&self, env: Env) -> Option<&AgentSlot> {
        match env {
            Env::Wsl => self.wsl.as_ref(),
            Env::Windows => self.windows.as_ref(),
        }
    }

    pub fn get_mut(&mut self, env: Env) -> Option<&mut AgentSlot> {
        match env {
            Env::Wsl => self.wsl.as_mut(),
            Env::Windows => self.windows.as_mut(),
        }
    }

    pub fn set(&mut self, env: Env, slot: Option<AgentSlot>) {
        match env {
            Env::Wsl => self.wsl = slot,
            Env::Windows => self.windows = slot,
        }
    }

    /// 取出该环境的 slot，没有就建一个空 slot。
    pub fn slot_mut(&mut self, env: Env) -> &mut AgentSlot {
        match env {
            Env::Wsl => self.wsl.get_or_insert_with(AgentSlot::default),
            Env::Windows => self.windows.get_or_insert_with(AgentSlot::default),
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
    #[error("绑定不存在：{0}")]
    BindingNotFound(String),
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
    #[error("这条绑定正在使用中，请先启用别的绑定再删除")]
    ActiveBinding,
    #[error("写入配置失败：{0}")]
    Write(String),
}

/// 校验并生成一条新绑定（带新 id），不保存、不写目标工具。
pub fn build_binding(
    cfg: &Config,
    kind: AgentKind,
    input: AgentConfigInput,
) -> Result<AgentConfig, AgentError> {
    let mut config = normalize_config(cfg, kind, input)?;
    config.id = new_id();
    Ok(config)
}

/// 新增绑定。`activate` 为真时同时写入目标文件并设为使用中。
pub fn add_binding(
    cfg: &mut Config,
    kind: AgentKind,
    env: Env,
    input: AgentConfigInput,
    activate: bool,
    adapter: &dyn AgentAdapter,
) -> Result<AgentConfig, AgentError> {
    let binding = build_binding(cfg, kind, input)?;

    if activate {
        apply_binding(cfg, kind, env, &binding, adapter)?;
    }

    let slot = cfg.agent_mut(kind).slot_mut(env);
    slot.bindings.push(binding.clone());
    if activate {
        slot.active = Some(binding.id.clone());
    }
    Ok(binding)
}

/// 编辑绑定。改的是使用中的那条时，保存后按新参数重写目标文件。
pub fn edit_binding(
    cfg: &mut Config,
    kind: AgentKind,
    env: Env,
    id: &str,
    input: AgentConfigInput,
    adapter: &dyn AgentAdapter,
) -> Result<AgentConfig, AgentError> {
    let mut updated = normalize_config(cfg, kind, input)?;
    updated.id = id.to_string();

    let is_active = cfg
        .agent(kind)
        .get(env)
        .is_some_and(|slot| slot.is_active(id));

    if is_active {
        apply_binding(cfg, kind, env, &updated, adapter)?;
    }

    let slot = cfg
        .agent_mut(kind)
        .get_mut(env)
        .ok_or(AgentError::NotConfigured)?;
    let target = slot
        .binding_mut(id)
        .ok_or_else(|| AgentError::BindingNotFound(id.to_string()))?;
    *target = updated.clone();
    Ok(updated)
}

/// 删除绑定。使用中的绑定不能直接删。
pub fn delete_binding(
    cfg: &mut Config,
    kind: AgentKind,
    env: Env,
    id: &str,
) -> Result<(), AgentError> {
    let slot = cfg
        .agent_mut(kind)
        .get_mut(env)
        .ok_or(AgentError::NotConfigured)?;
    if slot.is_active(id) {
        return Err(AgentError::ActiveBinding);
    }
    let before = slot.bindings.len();
    slot.bindings.retain(|binding| binding.id != id);
    if slot.bindings.len() == before {
        return Err(AgentError::BindingNotFound(id.to_string()));
    }
    Ok(())
}

/// 启用一条绑定：先写目标文件，成功后再记为使用中。
pub fn activate_binding(
    cfg: &mut Config,
    kind: AgentKind,
    env: Env,
    id: &str,
    adapter: &dyn AgentAdapter,
) -> Result<(), AgentError> {
    let binding = cfg
        .agent(kind)
        .get(env)
        .and_then(|slot| slot.binding(id))
        .cloned()
        .ok_or_else(|| AgentError::BindingNotFound(id.to_string()))?;

    apply_binding(cfg, kind, env, &binding, adapter)?;

    cfg.agent_mut(kind)
        .get_mut(env)
        .expect("slot exists after apply")
        .active = Some(id.to_string());
    Ok(())
}

/// 按当前使用中的绑定重新写入目标工具。
pub fn rewrite(
    cfg: &Config,
    kind: AgentKind,
    env: Env,
    adapter: &dyn AgentAdapter,
) -> Result<(), AgentError> {
    let binding = cfg
        .agent(kind)
        .get(env)
        .and_then(AgentSlot::active_binding)
        .ok_or(AgentError::NotConfigured)?;
    apply_binding(cfg, kind, env, binding, adapter)
}

fn apply_binding(
    cfg: &Config,
    kind: AgentKind,
    env: Env,
    binding: &AgentConfig,
    adapter: &dyn AgentAdapter,
) -> Result<(), AgentError> {
    let supplier = cfg
        .supplier(&binding.supplier_id)
        .ok_or_else(|| AgentError::SupplierNotFound(binding.supplier_id.clone()))?;
    validate_protocol(kind, supplier, binding.protocol)?;
    adapter
        .apply(supplier, binding, cfg.custom_dirs.get(kind, env), env)
        .map_err(|error| AgentError::Write(error.to_string()))
}

fn new_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 迁移时给旧配置补一个绑定 id。
pub fn new_binding_id() -> String {
    new_id()
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
        id: String::new(),
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

    fn working() -> FakeAdapter {
        FakeAdapter { fail: false }
    }

    #[test]
    fn add_binding_normalizes_settings() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let binding = add_binding(
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
            false,
            &working(),
        )
        .unwrap();
        assert_eq!(binding.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(binding.plan_reasoning_effort, None);
        assert_eq!(binding.context_length.as_deref(), Some("200k"));
        assert_eq!(binding.compact_ratio, Some(80));
        assert!(!binding.id.is_empty());

        let slot = cfg.agent(AgentKind::Codex).get(Env::Wsl).unwrap();
        assert_eq!(slot.bindings.len(), 1);
        assert!(slot.active.is_none());
    }

    #[test]
    fn add_binding_can_activate() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let binding = add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
            true,
            &working(),
        )
        .unwrap();
        let slot = cfg.agent(AgentKind::Codex).get(Env::Wsl).unwrap();
        assert_eq!(slot.active.as_deref(), Some(binding.id.as_str()));
    }

    #[test]
    fn environments_are_independent() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
            true,
            &working(),
        )
        .unwrap();
        add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Windows,
            input(&supplier.id, Protocol::Responses, "gpt-5-mini"),
            true,
            &working(),
        )
        .unwrap();

        let wsl = cfg.agent(AgentKind::Codex).get(Env::Wsl).unwrap();
        let windows = cfg.agent(AgentKind::Codex).get(Env::Windows).unwrap();
        assert_eq!(wsl.active_binding().unwrap().model, "gpt-5");
        assert_eq!(windows.active_binding().unwrap().model, "gpt-5-mini");
    }

    #[test]
    fn activate_switches_active_binding() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let first = add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
            true,
            &working(),
        )
        .unwrap();
        let second = add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5-mini"),
            false,
            &working(),
        )
        .unwrap();

        activate_binding(&mut cfg, AgentKind::Codex, Env::Wsl, &second.id, &working()).unwrap();
        let slot = cfg.agent(AgentKind::Codex).get(Env::Wsl).unwrap();
        assert_eq!(slot.active.as_deref(), Some(second.id.as_str()));
        assert_ne!(slot.active.as_deref(), Some(first.id.as_str()));
    }

    #[test]
    fn failed_write_keeps_config_untouched() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let failing = FakeAdapter { fail: true };
        assert!(
            add_binding(
                &mut cfg,
                AgentKind::Codex,
                Env::Wsl,
                input(&supplier.id, Protocol::Responses, "gpt-5"),
                true,
                &failing,
            )
            .is_err()
        );
        assert!(cfg.agent(AgentKind::Codex).get(Env::Wsl).is_none());
    }

    #[test]
    fn active_binding_cannot_be_deleted() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let binding = add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
            true,
            &working(),
        )
        .unwrap();
        assert!(matches!(
            delete_binding(&mut cfg, AgentKind::Codex, Env::Wsl, &binding.id),
            Err(AgentError::ActiveBinding)
        ));
    }

    #[test]
    fn deleting_inactive_binding_works() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
            true,
            &working(),
        )
        .unwrap();
        let extra = add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5-mini"),
            false,
            &working(),
        )
        .unwrap();
        delete_binding(&mut cfg, AgentKind::Codex, Env::Wsl, &extra.id).unwrap();
        assert_eq!(
            cfg.agent(AgentKind::Codex)
                .get(Env::Wsl)
                .unwrap()
                .bindings
                .len(),
            1
        );
    }

    #[test]
    fn editing_active_binding_rewrites_and_keeps_id() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let binding = add_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
            true,
            &working(),
        )
        .unwrap();

        let updated = edit_binding(
            &mut cfg,
            AgentKind::Codex,
            Env::Wsl,
            &binding.id,
            input(&supplier.id, Protocol::Responses, "gpt-5-mini"),
            &working(),
        )
        .unwrap();
        assert_eq!(updated.id, binding.id);
        assert_eq!(updated.model, "gpt-5-mini");
        assert_eq!(
            cfg.agent(AgentKind::Codex)
                .get(Env::Wsl)
                .unwrap()
                .active_binding()
                .unwrap()
                .model,
            "gpt-5-mini"
        );
    }

    #[test]
    fn claude_only_accepts_messages_and_1m() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Messages, Protocol::Responses]);
        assert!(matches!(
            add_binding(
                &mut cfg,
                AgentKind::Claude,
                Env::Wsl,
                input(&supplier.id, Protocol::Responses, "claude"),
                false,
                &working(),
            ),
            Err(AgentError::UnsupportedProtocol { .. })
        ));
        let binding = add_binding(
            &mut cfg,
            AgentKind::Claude,
            Env::Wsl,
            AgentConfigInput {
                context_length: Some("200k".to_string()),
                ..input(&supplier.id, Protocol::Messages, "claude")
            },
            false,
            &working(),
        )
        .unwrap();
        assert_eq!(binding.context_length, None);
    }

    #[test]
    fn binding_requires_supplier_protocol() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::ChatCompletions]);
        assert!(matches!(
            add_binding(
                &mut cfg,
                AgentKind::Codex,
                Env::Wsl,
                input(&supplier.id, Protocol::Responses, "gpt-5"),
                false,
                &working(),
            ),
            Err(AgentError::SupplierProtocolMissing(_))
        ));
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
