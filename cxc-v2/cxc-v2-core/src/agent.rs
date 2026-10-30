use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

use crate::config::{Config, Source};
use crate::protocol::Protocol;
use crate::supplier::{self, Supplier};
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Binding {
    pub id: String,
    pub supplier_id: String,
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

impl Binding {
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
pub struct BindingInput {
    pub supplier_id: String,
    pub protocol: Protocol,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub plan_reasoning_effort: Option<String>,
    pub context_length: Option<String>,
    pub compact_ratio: Option<u8>,
    pub claude_models: Option<ClaudeModels>,
    pub grok_web_search: Option<GrokWebSearch>,
}

impl Default for BindingInput {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct AgentConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_wsl: Option<String>,
    #[serde(default)]
    pub bindings: Vec<Binding>,
}

impl AgentConfig {
    pub fn active(&self, source: Source) -> Option<&str> {
        match source {
            Source::App => self.active_app.as_deref(),
            Source::Wsl => self.active_wsl.as_deref(),
        }
    }

    pub fn set_active(&mut self, source: Source, id: Option<String>) {
        match source {
            Source::App => self.active_app = id,
            Source::Wsl => self.active_wsl = id,
        }
    }

    pub fn binding(&self, id: &str) -> Option<&Binding> {
        self.bindings.iter().find(|binding| binding.id == id)
    }

    pub fn binding_mut(&mut self, id: &str) -> Option<&mut Binding> {
        self.bindings.iter_mut().find(|binding| binding.id == id)
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.active_app.as_deref() == Some(id) || self.active_wsl.as_deref() == Some(id)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Agents {
    #[serde(default)]
    pub codex: AgentConfig,
    #[serde(default)]
    pub claude: AgentConfig,
    #[serde(default)]
    pub grok: AgentConfig,
    #[serde(default)]
    pub opencode: AgentConfig,
}

impl Agents {
    pub fn get(&self, kind: AgentKind) -> &AgentConfig {
        match kind {
            AgentKind::Codex => &self.codex,
            AgentKind::Claude => &self.claude,
            AgentKind::Grok => &self.grok,
            AgentKind::OpenCode => &self.opencode,
        }
    }

    pub fn get_mut(&mut self, kind: AgentKind) -> &mut AgentConfig {
        match kind {
            AgentKind::Codex => &mut self.codex,
            AgentKind::Claude => &mut self.claude,
            AgentKind::Grok => &mut self.grok,
            AgentKind::OpenCode => &mut self.opencode,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct CustomDirs {
    #[serde(default)]
    pub codex: String,
    #[serde(default)]
    pub claude: String,
    #[serde(default)]
    pub grok: String,
    #[serde(default)]
    pub opencode: String,
}

impl CustomDirs {
    pub fn get(&self, kind: AgentKind) -> Option<&Path> {
        let value = match kind {
            AgentKind::Codex => &self.codex,
            AgentKind::Claude => &self.claude,
            AgentKind::Grok => &self.grok,
            AgentKind::OpenCode => &self.opencode,
        };
        let value = value.trim();
        if value.is_empty() {
            None
        } else {
            Some(Path::new(value))
        }
    }

    pub fn set(&mut self, kind: AgentKind, value: String) {
        let value = value.trim().to_string();
        match kind {
            AgentKind::Codex => self.codex = value,
            AgentKind::Claude => self.claude = value,
            AgentKind::Grok => self.grok = value,
            AgentKind::OpenCode => self.opencode = value,
        }
    }
}

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent 不存在：{0}")]
    UnknownAgent(String),
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
    #[error("该绑定正在 {0} 来源启用，请先切换后再删除")]
    ActiveBinding(String),
    #[error("写入配置失败：{0}")]
    Write(String),
}

pub fn add_binding(
    cfg: &mut Config,
    kind: AgentKind,
    input: BindingInput,
) -> Result<Binding, AgentError> {
    let binding = normalize_binding(cfg, kind, input)?;
    cfg.agent_mut(kind).bindings.push(binding.clone());
    Ok(binding)
}

pub fn edit_binding(
    cfg: &mut Config,
    kind: AgentKind,
    id: &str,
    input: BindingInput,
) -> Result<Binding, AgentError> {
    let binding = normalize_binding(cfg, kind, input)?;
    let agent = cfg.agent_mut(kind);
    let target = agent
        .binding_mut(id)
        .ok_or_else(|| AgentError::BindingNotFound(id.to_string()))?;
    let mut updated = binding;
    updated.id = target.id.clone();
    *target = updated.clone();
    Ok(updated)
}

pub fn remove_binding(cfg: &mut Config, kind: AgentKind, id: &str) -> Result<(), AgentError> {
    let agent = cfg.agent_mut(kind);
    if agent.is_active(id) {
        let sources = [
            (Source::App, agent.active_app.as_deref() == Some(id)),
            (Source::Wsl, agent.active_wsl.as_deref() == Some(id)),
        ]
        .into_iter()
        .filter_map(|(source, active)| active.then_some(source.as_str()))
        .collect::<Vec<_>>()
        .join(" / ");
        return Err(AgentError::ActiveBinding(sources));
    }
    let before = agent.bindings.len();
    agent.bindings.retain(|binding| binding.id != id);
    if agent.bindings.len() == before {
        return Err(AgentError::BindingNotFound(id.to_string()));
    }
    Ok(())
}

pub fn activate_binding(
    cfg: &mut Config,
    kind: AgentKind,
    source: Source,
    id: &str,
    adapter: &dyn AgentAdapter,
) -> Result<(), AgentError> {
    let binding = cfg
        .agent(kind)
        .binding(id)
        .cloned()
        .ok_or_else(|| AgentError::BindingNotFound(id.to_string()))?;
    let supplier = cfg
        .supplier(&binding.supplier_id)
        .cloned()
        .ok_or_else(|| AgentError::SupplierNotFound(binding.supplier_id.clone()))?;

    validate_protocol(kind, &supplier, binding.protocol)?;
    adapter
        .apply(&supplier, &binding, cfg.custom_dirs.get(kind), source)
        .map_err(|error| AgentError::Write(error.to_string()))?;

    cfg.agent_mut(kind)
        .set_active(source, Some(binding.id.clone()));
    Ok(())
}

pub fn reload_active(
    cfg: &Config,
    kind: AgentKind,
    source: Source,
    adapter: &dyn AgentAdapter,
) -> Result<(), AgentError> {
    let id = cfg
        .agent(kind)
        .active(source)
        .ok_or_else(|| AgentError::BindingNotFound("当前来源没有启用的绑定".to_string()))?;
    let binding = cfg
        .agent(kind)
        .binding(id)
        .ok_or_else(|| AgentError::BindingNotFound(id.to_string()))?;
    let supplier = cfg
        .supplier(&binding.supplier_id)
        .ok_or_else(|| AgentError::SupplierNotFound(binding.supplier_id.clone()))?;
    validate_protocol(kind, supplier, binding.protocol)?;
    adapter
        .apply(supplier, binding, cfg.custom_dirs.get(kind), source)
        .map_err(|error| AgentError::Write(error.to_string()))
}

pub fn deactivate_binding(cfg: &mut Config, kind: AgentKind, source: Source) {
    cfg.agent_mut(kind).set_active(source, None);
}

fn normalize_binding(
    cfg: &Config,
    kind: AgentKind,
    input: BindingInput,
) -> Result<Binding, AgentError> {
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

    Ok(Binding {
        id: supplier::new_id(),
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

    fn input(supplier_id: &str, protocol: Protocol, model: &str) -> BindingInput {
        BindingInput {
            supplier_id: supplier_id.to_string(),
            protocol,
            model: model.to_string(),
            ..BindingInput::default()
        }
    }

    #[test]
    fn add_binding_normalizes_settings() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let binding = add_binding(
            &mut cfg,
            AgentKind::Codex,
            BindingInput {
                reasoning_effort: Some(" high ".to_string()),
                plan_reasoning_effort: Some("none".to_string()),
                context_length: Some("200000".to_string()),
                compact_ratio: Some(80),
                ..input(&supplier.id, Protocol::Responses, "gpt-5")
            },
        )
        .unwrap();
        assert_eq!(binding.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(binding.plan_reasoning_effort, None);
        assert_eq!(binding.context_length.as_deref(), Some("200k"));
        assert_eq!(binding.compact_ratio, Some(80));
    }

    #[test]
    fn claude_only_accepts_messages_and_1m() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Messages, Protocol::Responses]);
        assert!(matches!(
            add_binding(
                &mut cfg,
                AgentKind::Claude,
                input(&supplier.id, Protocol::Responses, "claude")
            ),
            Err(AgentError::UnsupportedProtocol { .. })
        ));
        let binding = add_binding(
            &mut cfg,
            AgentKind::Claude,
            BindingInput {
                context_length: Some("200k".to_string()),
                ..input(&supplier.id, Protocol::Messages, "claude")
            },
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
                input(&supplier.id, Protocol::Responses, "gpt-5")
            ),
            Err(AgentError::SupplierProtocolMissing(_))
        ));
    }

    #[test]
    fn active_binding_cannot_be_removed() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let binding = add_binding(
            &mut cfg,
            AgentKind::Codex,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
        )
        .unwrap();
        cfg.agent_mut(AgentKind::Codex)
            .set_active(Source::Wsl, Some(binding.id.clone()));
        assert!(matches!(
            remove_binding(&mut cfg, AgentKind::Codex, &binding.id),
            Err(AgentError::ActiveBinding(_))
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
            _source: Source,
            _custom_dir: Option<&std::path::Path>,
        ) -> Result<std::path::PathBuf, crate::target::TargetError> {
            Ok(std::path::PathBuf::from("/tmp/cxc-v2-test"))
        }

        fn apply(
            &self,
            _supplier: &Supplier,
            _binding: &Binding,
            _custom_dir: Option<&std::path::Path>,
            _source: Source,
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
    fn activation_updates_active_only_after_successful_write() {
        let mut cfg = Config::default();
        let supplier = supplier(&mut cfg, vec![Protocol::Responses]);
        let binding = add_binding(
            &mut cfg,
            AgentKind::Codex,
            input(&supplier.id, Protocol::Responses, "gpt-5"),
        )
        .unwrap();

        let failing = FakeAdapter { fail: true };
        assert!(
            activate_binding(
                &mut cfg,
                AgentKind::Codex,
                Source::Wsl,
                &binding.id,
                &failing
            )
            .is_err()
        );
        assert_eq!(cfg.agent(AgentKind::Codex).active_wsl, None);

        let working = FakeAdapter { fail: false };
        activate_binding(
            &mut cfg,
            AgentKind::Codex,
            Source::Wsl,
            &binding.id,
            &working,
        )
        .unwrap();
        assert_eq!(
            cfg.agent(AgentKind::Codex).active_wsl.as_deref(),
            Some(binding.id.as_str())
        );
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
