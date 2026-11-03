export type Env = "wsl" | "windows";
export type Protocol = "chat_completions" | "responses" | "messages";
export type AgentKind = "codex" | "claude" | "grok" | "opencode";

export interface SupplierModel {
  model: string;
  protocols: Protocol[];
  last_test?: string;
  latency_ms?: number;
  last_ok?: boolean;
  last_error?: string;
  tested_key_id?: string;
}

export interface SupplierKey {
  id: string;
  name: string;
  api_key: string;
  remark?: string;
}

export interface Supplier {
  id: string;
  name: string;
  base_url: string;
  keys: SupplierKey[];
  default_key_id?: string;
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

/** 某个 Agent（工具 + 环境）下的一条绑定。 */
export interface AgentConfig {
  id: string;
  supplier_id: string;
  /** 不设置表示跟随中转站的默认账号。 */
  key_id?: string;
  protocol: Protocol;
  model: string;
  reasoning_effort?: string;
  plan_reasoning_effort?: string;
  context_length?: string;
  compact_ratio?: number;
  claude_models?: ClaudeModels;
  grok_web_search?: GrokWebSearch;
}

/** 一个 (工具, 环境) 下保存的绑定与当前使用中的那条。 */
export interface AgentSlot {
  active?: string | null;
  bindings: AgentConfig[];
}

export interface AgentEnvs {
  wsl?: AgentSlot | null;
  windows?: AgentSlot | null;
}

export interface Agents {
  codex: AgentEnvs;
  claude: AgentEnvs;
  grok: AgentEnvs;
  opencode: AgentEnvs;
}

export interface EnvDirs {
  wsl: string;
  windows: string;
}

export interface CustomDirs {
  codex: EnvDirs;
  claude: EnvDirs;
  grok: EnvDirs;
  opencode: EnvDirs;
}

export interface SyncConfig {
  gist_token: string;
  gist_id: string;
  secret_key?: string;
  last_synced_at?: string;
}

export interface Config {
  version: number;
  suppliers: Supplier[];
  agents: Agents;
  custom_dirs: CustomDirs;
  sync?: SyncConfig;
  migrated_from_v1: boolean;
}

export interface DetectedAgent {
  kind: AgentKind;
  env: Env;
  installed: boolean;
  binary?: string;
  config_dir?: string;
  configured: boolean;
}

export interface EnvInfo {
  env: Env;
  label: string;
  /** 当前进程直接运行在这个环境里。 */
  host: boolean;
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

export interface SupplierKeyInput {
  id?: string;
  name: string;
  api_key: string;
  remark?: string;
}

export interface SupplierInput {
  name: string;
  base_url: string;
  remark?: string;
  protocols?: Protocol[];
  keys: SupplierKeyInput[];
  default_key_id?: string;
}

export interface AgentConfigInput {
  supplier_id: string;
  key_id?: string;
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
export const ENVS: Env[] = ["wsl", "windows"];

export const AGENT_LABELS: Record<AgentKind, string> = {
  codex: "Codex",
  claude: "Claude Code",
  grok: "Grok CLI",
  opencode: "OpenCode",
};

export const ENV_LABELS: Record<Env, string> = {
  wsl: "WSL",
  windows: "Windows",
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

/** 解析绑定实际使用的账号：显式指定 → 中转站默认 → 第一个账号。 */
export function effectiveKey(
  supplier: Supplier | undefined,
  keyId?: string,
): SupplierKey | undefined {
  if (!supplier) return undefined;
  const explicit = keyId ? supplier.keys.find((key) => key.id === keyId) : undefined;
  if (explicit) return explicit;
  const fallback = supplier.default_key_id
    ? supplier.keys.find((key) => key.id === supplier.default_key_id)
    : undefined;
  return fallback ?? supplier.keys[0];
}

export function slotOf(
  config: Config,
  kind: AgentKind,
  env: Env,
): AgentSlot | undefined {
  return config.agents[kind]?.[env] ?? undefined;
}

/** 当前使用中的绑定。`active` 指向不存在的 id 时视为没有。 */
export function activeBinding(
  config: Config,
  kind: AgentKind,
  env: Env,
): AgentConfig | undefined {
  const slot = slotOf(config, kind, env);
  if (!slot?.active) return undefined;
  return slot.bindings.find((binding) => binding.id === slot.active);
}

export function emptyDirs(): CustomDirs {
  return {
    codex: { wsl: "", windows: "" },
    claude: { wsl: "", windows: "" },
    grok: { wsl: "", windows: "" },
    opencode: { wsl: "", windows: "" },
  };
}
