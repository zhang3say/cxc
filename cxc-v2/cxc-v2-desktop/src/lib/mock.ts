import type {
  AgentConfig,
  AgentConfigInput,
  AgentKind,
  ClaudeModels,
  Config,
  CustomDirs,
  DetectedAgent,
  Env,
  GrokWebSearch,
  MergeReport,
  ModelTestOutcome,
  ProbeReport,
  Protocol,
  Supplier,
  SupplierInput,
  SyncConfig,
} from "./types";
import { AGENT_KINDS, ENVS, emptyDirs } from "./types";

const STORAGE_KEY = "cxc-v2-mock-config";

function seed(): Config {
  return {
    version: 4,
    suppliers: [
      {
        id: "a1b2c3d4e5f60718",
        name: "openrouter",
        base_url: "https://openrouter.ai/api/v1",
        api_key: "sk-or-v1-demo",
        protocols: ["chat_completions", "messages"],
        remark: "主力中转，覆盖 Claude 与通用模型",
        last_probe: new Date().toISOString(),
        models: [
          {
            model: "anthropic/claude-sonnet-4.5",
            protocols: ["chat_completions", "messages"],
            last_test: new Date().toISOString(),
            latency_ms: 820,
            last_ok: true,
          },
          {
            model: "openai/gpt-5",
            protocols: ["chat_completions"],
            last_test: new Date().toISOString(),
            latency_ms: 640,
            last_ok: true,
          },
          {
            model: "x-ai/grok-4.5",
            protocols: ["chat_completions"],
            last_test: new Date().toISOString(),
            latency_ms: 520,
            last_ok: true,
          },
        ],
      },
      {
        id: "b2c3d4e5f6071829",
        name: "codex-relay",
        base_url: "https://codex.example.com/v1",
        api_key: "sk-codex-demo",
        protocols: ["responses", "chat_completions"],
        remark: "Codex 专用 Responses 端点",
        last_probe: new Date().toISOString(),
        models: [
          {
            model: "gpt-5",
            protocols: ["responses", "chat_completions"],
            last_test: new Date().toISOString(),
            latency_ms: 410,
            last_ok: true,
          },
          {
            model: "gpt-5-mini",
            protocols: ["responses"],
            latency_ms: 380,
            last_ok: true,
          },
        ],
      },
      {
        id: "c3d4e5f60718293a",
        name: "anthropic-direct",
        base_url: "https://api.anthropic.com",
        api_key: "sk-ant-demo",
        protocols: ["messages"],
        remark: "官方 Anthropic 端点",
        last_probe: new Date().toISOString(),
        models: [
          {
            model: "claude-sonnet-4-5",
            protocols: ["messages"],
            last_test: new Date().toISOString(),
            latency_ms: 950,
            last_ok: true,
          },
          {
            model: "claude-haiku-4-5",
            protocols: ["messages"],
            latency_ms: 520,
            last_ok: true,
          },
        ],
      },
    ],
    agents: {
      codex: {
        wsl: {
          active: "bind-codex-1",
          bindings: [
            {
              id: "bind-codex-1",
              supplier_id: "b2c3d4e5f6071829",
              protocol: "responses",
              model: "gpt-5",
              reasoning_effort: "high",
              plan_reasoning_effort: "xhigh",
              context_length: "200k",
              compact_ratio: 80,
            },
            {
              id: "bind-codex-2",
              supplier_id: "a1b2c3d4e5f60718",
              protocol: "chat_completions",
              model: "openai/gpt-5",
              reasoning_effort: "medium",
            },
          ],
        },
        windows: {
          active: "bind-codex-3",
          bindings: [
            {
              id: "bind-codex-3",
              supplier_id: "b2c3d4e5f6071829",
              protocol: "responses",
              model: "gpt-5-mini",
              reasoning_effort: "medium",
              context_length: "200k",
              compact_ratio: 80,
            },
          ],
        },
      },
      claude: {
        wsl: {
          active: "bind-claude-1",
          bindings: [
            {
              id: "bind-claude-1",
              supplier_id: "c3d4e5f60718293a",
              protocol: "messages",
              model: "claude-sonnet-4-5",
              reasoning_effort: "high",
              context_length: "1m",
              compact_ratio: 85,
              claude_models: {
                opus: "claude-opus-4-5",
                haiku: "claude-haiku-4-5",
              },
            },
          ],
        },
      },
      grok: {
        wsl: {
          active: "bind-grok-1",
          bindings: [
            {
              id: "bind-grok-1",
              supplier_id: "a1b2c3d4e5f60718",
              protocol: "chat_completions",
              model: "x-ai/grok-4.5",
              reasoning_effort: "high",
              context_length: "500k",
              compact_ratio: 75,
              grok_web_search: { mode: "follow" },
            },
            {
              id: "bind-grok-2",
              supplier_id: "a1b2c3d4e5f60718",
              protocol: "chat_completions",
              model: "openai/gpt-5",
              reasoning_effort: "minimal",
              context_length: "200k",
            },
          ],
        },
      },
      opencode: {
        wsl: {
          active: "bind-opencode-1",
          bindings: [
            {
              id: "bind-opencode-1",
              supplier_id: "a1b2c3d4e5f60718",
              protocol: "chat_completions",
              model: "openai/gpt-5",
              reasoning_effort: "medium",
              context_length: "200k",
              compact_ratio: 80,
            },
          ],
        },
      },
    },
    custom_dirs: emptyDirs(),
    migrated_from_v1: true,
  };
}

function load(): Config {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Config;
      if (parsed.version === 4) return parsed;
    }
  } catch {
    // ignore
  }
  const config = seed();
  save(config);
  return config;
}

function save(config: Config) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function id(): string {
  return Array.from({ length: 16 }, () =>
    Math.floor(Math.random() * 256)
      .toString(16)
      .padStart(2, "0"),
  ).join("");
}

function delay(ms = 450) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function agentKey(agent: string): AgentKind {
  return agent as AgentKind;
}

function envKey(env: string): Env {
  return env === "windows" ? "windows" : "wsl";
}

function supplierOf(config: Config, supplierId: string): Supplier {
  const supplier = config.suppliers.find((item) => item.id === supplierId);
  if (!supplier) throw new Error("供应商不存在");
  return supplier;
}

function references(config: Config, supplierId: string): string[] {
  return AGENT_KINDS.filter((kind) =>
    ENVS.some((env) =>
      (config.agents[kind]?.[env]?.bindings ?? []).some(
        (binding) => binding.supplier_id === supplierId,
      ),
    ),
  ).map((kind) => kind);
}

function probeReport(supplier: Supplier): ProbeReport {
  const confirmed = supplier.protocols.length
    ? supplier.protocols
    : (["chat_completions", "responses", "messages"] as Protocol[]);
  const results = (["chat_completions", "responses", "messages"] as Protocol[]).map(
    (protocol) => ({
      protocol,
      confirmed: confirmed.includes(protocol),
      status: confirmed.includes(protocol) ? 200 : 404,
      reason: confirmed.includes(protocol)
        ? "端点返回了 JSON 响应"
        : "端点返回 404，正文不是 JSON",
    }),
  );
  return {
    added: confirmed.filter((protocol) => !supplier.protocols.includes(protocol)),
    protocols: confirmed,
    results,
  };
}

function normalizeClaudeModels(models?: ClaudeModels): ClaudeModels | undefined {
  if (!models) return undefined;
  const entries = Object.entries(models).filter(
    ([, value]) => typeof value === "string" && value.trim().length > 0,
  );
  return entries.length ? (Object.fromEntries(entries) as ClaudeModels) : undefined;
}

function normalizeWebSearch(policy?: GrokWebSearch): GrokWebSearch | undefined {
  if (!policy) return undefined;
  if (policy.mode === "model" && !policy.model.trim()) return { mode: "follow" };
  return policy;
}

function buildConfig(
  config: Config,
  input: AgentConfigInput,
  id: string,
): AgentConfig {
  const supplier = supplierOf(config, input.supplier_id);
  if (!supplier.protocols.includes(input.protocol)) {
    throw new Error(`供应商尚未确认支持 ${input.protocol} 协议`);
  }
  const model = input.model.trim();
  if (!model) throw new Error("模型不能为空");
  return {
    id,
    supplier_id: input.supplier_id,
    protocol: input.protocol,
    model,
    reasoning_effort: input.reasoning_effort || undefined,
    plan_reasoning_effort: input.plan_reasoning_effort || undefined,
    context_length: input.context_length?.trim() || undefined,
    compact_ratio: input.compact_ratio,
    claude_models: normalizeClaudeModels(input.claude_models),
    grok_web_search: normalizeWebSearch(input.grok_web_search),
  };
}

function modelList(supplier: Supplier): string[] {
  const base = supplier.models.map((model) => model.model);
  const extras = ["gpt-5", "gpt-5-mini", "claude-sonnet-4-5", "claude-opus-4-5", "grok-4.5"];
  return Array.from(new Set([...base, ...extras]));
}

export async function mockInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const config = load();
  const payload = args ?? {};

  switch (command) {
    case "get_config":
      return clone(config) as T;

    case "get_envs":
      return ["wsl", "windows"] as T;

    case "get_path_env":
      return "/home/leezi/.local/bin:/home/leezi/.grok/bin:/usr/local/bin:/usr/bin:/bin" as T;

    case "detect_agents": {
      const detected: DetectedAgent[] = [];
      for (const env of ENVS) {
        for (const kind of AGENT_KINDS) {
          detected.push({
            kind,
            env,
            installed: true,
            binary:
              env === "wsl"
                ? `/home/leezi/.local/bin/${kind}`
                : `C:\\Users\\leezi\\AppData\\Roaming\\npm\\${kind}.cmd`,
            config_dir:
              env === "wsl"
                ? kind === "opencode"
                  ? "/home/leezi/.config/opencode"
                  : `/home/leezi/.${kind}`
                : kind === "opencode"
                  ? "C:\\Users\\leezi\\AppData\\Roaming\\opencode"
                  : `C:\\Users\\leezi\\.${kind}`,
            configured: true,
          });
        }
      }
      return detected as T;
    }

    case "add_supplier": {
      const input = payload.input as SupplierInput;
      if (config.suppliers.some((supplier) => supplier.name === input.name.trim())) {
        throw new Error(`供应商名称已存在：${input.name}`);
      }
      const supplier: Supplier = {
        id: id(),
        name: input.name.trim(),
        base_url: input.base_url.trim(),
        api_key: input.api_key.trim(),
        protocols: input.protocols ?? [],
        remark: input.remark?.trim() || undefined,
        models: [],
      };
      config.suppliers.push(supplier);
      save(config);
      return clone(config) as T;
    }

    case "edit_supplier": {
      const supplier = supplierOf(config, payload.id as string);
      const input = payload.input as SupplierInput;
      supplier.name = input.name.trim();
      supplier.base_url = input.base_url.trim();
      if (input.api_key.trim()) supplier.api_key = input.api_key.trim();
      supplier.remark = input.remark?.trim() || undefined;
      if (input.protocols) supplier.protocols = input.protocols;
      save(config);
      return clone(config) as T;
    }

    case "delete_supplier": {
      const supplierId = payload.id as string;
      const refs = references(config, supplierId);
      if (refs.length) {
        throw new Error(`供应商仍被以下 Agent 引用：${refs.join("、")}`);
      }
      config.suppliers = config.suppliers.filter((supplier) => supplier.id !== supplierId);
      save(config);
      return clone(config) as T;
    }

    case "probe_supplier": {
      await delay(900);
      const supplier = supplierOf(config, payload.id as string);
      const report = probeReport(supplier);
      supplier.protocols = report.protocols;
      supplier.last_probe = new Date().toISOString();
      save(config);
      return { config: clone(config), report } as T;
    }

    case "fetch_supplier_models": {
      await delay(700);
      const supplier = supplierOf(config, payload.id as string);
      const models = modelList(supplier);
      for (const model of models) {
        if (!supplier.models.some((entry) => entry.model === model)) {
          supplier.models.push({ model, protocols: [] });
        }
      }
      save(config);
      return { config: clone(config), models } as T;
    }

    case "test_supplier_model": {
      await delay(800);
      const supplier = supplierOf(config, payload.id as string);
      const model = payload.model as string;
      const protocol = payload.protocol as Protocol;
      const ok = !model.toLowerCase().includes("bad");
      const result = {
        ok,
        latency_ms: Math.floor(Math.random() * 260) + 40,
        status: ok ? 200 : 429,
        error: ok ? undefined : "HTTP 429: rate limited",
      };
      let entry = supplier.models.find((item) => item.model === model);
      if (!entry) {
        entry = { model, protocols: [] };
        supplier.models.push(entry);
      }
      entry.last_test = new Date().toISOString();
      entry.latency_ms = result.latency_ms;
      entry.last_ok = ok;
      entry.last_error = result.error;
      if (ok && !entry.protocols.includes(protocol)) entry.protocols.push(protocol);
      save(config);
      return { config: clone(config), result } as T;
    }

    case "test_supplier_models": {
      await delay(1400);
      const supplier = supplierOf(config, payload.id as string);
      const outcomes: ModelTestOutcome[] = [];
      for (const entry of supplier.models) {
        const protocols = entry.protocols.length ? entry.protocols : supplier.protocols;
        for (const protocol of protocols) {
          const ok = !entry.model.toLowerCase().includes("bad");
          const result = {
            ok,
            latency_ms: Math.floor(Math.random() * 260) + 40,
            status: ok ? 200 : 429,
            error: ok ? undefined : "HTTP 429: rate limited",
          };
          entry.last_test = new Date().toISOString();
          entry.latency_ms = result.latency_ms;
          entry.last_ok = ok;
          entry.last_error = result.error;
          outcomes.push({ model: entry.model, protocol, result });
        }
      }
      save(config);
      return { config: clone(config), outcomes } as T;
    }

    case "add_binding": {
      const agent = agentKey(payload.agent as string);
      const env = envKey(payload.env as string);
      const input = payload.input as AgentConfigInput;
      const activate = Boolean(payload.activate);
      const built = buildConfig(config, input, `bind-${id().slice(0, 8)}`);
      const slot = config.agents[agent]?.[env] ?? { bindings: [] };
      slot.bindings = [...(slot.bindings ?? []), built];
      if (activate || !slot.active) slot.active = built.id;
      config.agents[agent] = { ...config.agents[agent], [env]: slot };
      save(config);
      return clone(config) as T;
    }

    case "edit_binding": {
      const agent = agentKey(payload.agent as string);
      const env = envKey(payload.env as string);
      const bindingId = payload.id as string;
      const input = payload.input as AgentConfigInput;
      const slot = config.agents[agent]?.[env];
      if (!slot) throw new Error("绑定不存在");
      const index = slot.bindings.findIndex((item) => item.id === bindingId);
      if (index < 0) throw new Error("绑定不存在");
      const built = buildConfig(config, input, bindingId);
      slot.bindings[index] = built;
      save(config);
      return clone(config) as T;
    }

    case "delete_binding": {
      const agent = agentKey(payload.agent as string);
      const env = envKey(payload.env as string);
      const bindingId = payload.id as string;
      const slot = config.agents[agent]?.[env];
      if (!slot) throw new Error("绑定不存在");
      if (slot.active === bindingId) {
        throw new Error("这条绑定正在使用中，请先启用别的绑定再删除");
      }
      slot.bindings = slot.bindings.filter((item) => item.id !== bindingId);
      save(config);
      return clone(config) as T;
    }

    case "activate_binding": {
      const agent = agentKey(payload.agent as string);
      const env = envKey(payload.env as string);
      const bindingId = payload.id as string;
      const slot = config.agents[agent]?.[env];
      if (!slot?.bindings.some((item) => item.id === bindingId)) {
        throw new Error("绑定不存在");
      }
      slot.active = bindingId;
      save(config);
      return clone(config) as T;
    }

    case "rewrite_agent":
      await delay(300);
      return clone(config) as T;

    case "save_settings": {
      const dirs = (payload.customDirs ?? payload.custom_dirs) as CustomDirs | undefined;
      if (dirs) config.custom_dirs = dirs;
      save(config);
      return clone(config) as T;
    }

    case "get_sync_config": {
      return (config.sync ?? {
        gist_token: "",
        gist_id: "",
        secret_key: undefined,
        last_synced_at: undefined,
      }) as T;
    }

    case "save_sync_config": {
      config.sync = payload.sync as SyncConfig;
      save(config);
      return clone(config) as T;
    }

    case "test_gist_connection":
      await delay(600);
      return "demo-user" as T;

    case "sync_push":
    case "sync_pull":
    case "sync_merge": {
      await delay(800);
      const report: MergeReport = {
        suppliers_added: 0,
        suppliers_updated: config.suppliers.length,
        bindings_added: 0,
        bindings_updated: 4,
      };
      if (config.sync) config.sync.last_synced_at = new Date().toISOString();
      save(config);
      return { config: clone(config), report } as T;
    }

    case "get_app_version":
      return "0.1.0" as T;

    case "check_update":
      return {
        current_version: "0.1.0",
        latest_version: "0.1.0",
        has_update: false,
      } as T;

    case "import_legacy_config":
      return {
        imported: false,
        reason: "浏览器演示模式不读取本机旧配置",
        suppliers: 0,
        configs: 0,
        skipped: [],
      } as T;

    default:
      throw new Error(`未实现的演示命令：${command}`);
  }
}

export function resetMockConfig() {
  localStorage.removeItem(STORAGE_KEY);
}
