export type Source = "app" | "wsl";
export type Protocol = "chat_completions" | "responses" | "messages";
export type AgentKind = "codex" | "claude" | "grok" | "opencode";

export interface SupplierModel {
  model: string;
  protocols: Protocol[];
  last_test?: string;
  latency_ms?: number;
  last_ok?: boolean;
  last_error?: string;
}

export interface Supplier {
  id: string;
  name: string;
  base_url: string;
  api_key: string;
  protocols: Protocol[];
  remark?: string;
  last_probe?: string;
  models: SupplierModel[];
}

export interface ClaudeModels {
  opus?: string;
  sonnet?: string;
  haiku?: string;
  subagent?: string;
}

export type GrokWebSearch =
  | { mode: "keep" }
  | { mode: "follow" }
  | { mode: "model"; model: string }
  | { mode: "unset" };

export interface Binding {
  id: string;
  supplier_id: string;
  protocol: Protocol;
  model: string;
  reasoning_effort?: string;
  plan_reasoning_effort?: string;
  context_length?: string;
  compact_ratio?: number;
  claude_models?: ClaudeModels;
  grok_web_search?: GrokWebSearch;
}

export interface AgentConfig {
  active_app?: string;
  active_wsl?: string;
  bindings: Binding[];
}

export interface Agents {
  codex: AgentConfig;
  claude: AgentConfig;
  grok: AgentConfig;
  opencode: AgentConfig;
}

export interface CustomDirs {
  codex: string;
  claude: string;
  grok: string;
  opencode: string;
}

export interface SyncConfig {
  gist_token: string;
  gist_id: string;
  secret_key?: string;
  last_synced_at?: string;
}

export interface Config {
  version: number;
  source: Source;
  suppliers: Supplier[];
  agents: Agents;
  custom_dirs: CustomDirs;
  sync?: SyncConfig;
  migrated_from_v1: boolean;
}

export interface DetectedAgent {
  kind: AgentKind;
  installed: boolean;
  binary?: string;
  config_dir?: string;
  configured: boolean;
  source: Source;
}

export interface AgentCapabilities {
  protocols: Protocol[];
  reasoning_effort: boolean;
  plan_reasoning_effort: boolean;
  context_length: boolean;
  compact_ratio: boolean;
  claude_models: boolean;
  grok_web_search: boolean;
}

export interface ProtocolProbeResult {
  protocol: Protocol;
  confirmed: boolean;
  status?: number;
  reason: string;
}

export interface ProbeReport {
  added: Protocol[];
  protocols: Protocol[];
  results: ProtocolProbeResult[];
}

export interface ModelTestResult {
  ok: boolean;
  latency_ms: number;
  status?: number;
  error?: string;
}

export interface ModelTestOutcome {
  model: string;
  protocol: Protocol;
  result: ModelTestResult;
}

export interface ImportReport {
  imported: boolean;
  reason: string;
  suppliers: number;
  bindings: number;
  skipped: string[];
}

export interface MergeReport {
  suppliers_added: number;
  suppliers_updated: number;
  bindings_added: number;
  bindings_updated: number;
}

export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  download_url?: string;
}

export interface SupplierInput {
  name: string;
  base_url: string;
  api_key: string;
  remark?: string;
  protocols?: Protocol[];
}

export interface BindingInput {
  supplier_id: string;
  protocol: Protocol;
  model: string;
  reasoning_effort?: string;
  plan_reasoning_effort?: string;
  context_length?: string;
  compact_ratio?: number;
  claude_models?: ClaudeModels;
  grok_web_search?: GrokWebSearch;
}

export const AGENT_KINDS: AgentKind[] = ["codex", "claude", "grok", "opencode"];

export const AGENT_LABELS: Record<AgentKind, string> = {
  codex: "Codex",
  claude: "Claude Code",
  grok: "Grok CLI",
  opencode: "OpenCode",
};

export const PROTOCOL_LABELS: Record<Protocol, string> = {
  chat_completions: "Chat",
  responses: "Responses",
  messages: "Messages",
};

export const PROTOCOL_FULL_LABELS: Record<Protocol, string> = {
  chat_completions: "OpenAI Chat Completions",
  responses: "OpenAI Responses",
  messages: "Anthropic Messages",
};

export const CAPABILITIES: Record<AgentKind, AgentCapabilities> = {
  codex: {
    protocols: ["responses", "chat_completions"],
    reasoning_effort: true,
    plan_reasoning_effort: true,
    context_length: true,
    compact_ratio: true,
    claude_models: false,
    grok_web_search: false,
  },
  claude: {
    protocols: ["messages"],
    reasoning_effort: true,
    plan_reasoning_effort: false,
    context_length: true,
    compact_ratio: true,
    claude_models: true,
    grok_web_search: false,
  },
  grok: {
    protocols: ["chat_completions", "responses", "messages"],
    reasoning_effort: true,
    plan_reasoning_effort: false,
    context_length: true,
    compact_ratio: true,
    claude_models: false,
    grok_web_search: true,
  },
  opencode: {
    protocols: ["chat_completions", "messages", "responses"],
    reasoning_effort: true,
    plan_reasoning_effort: false,
    context_length: true,
    compact_ratio: true,
    claude_models: false,
    grok_web_search: false,
  },
};

export const REASONING_OPTIONS: Record<AgentKind, string[]> = {
  codex: ["low", "medium", "high", "xhigh", "max", "ultra"],
  claude: ["low", "medium", "high", "xhigh", "max"],
  grok: ["minimal", "low", "medium", "high", "xhigh", "max"],
  opencode: ["minimal", "low", "medium", "high", "xhigh"],
};
