import type {
  AgentKind,
  Binding,
  BindingInput,
  ClaudeModels,
  Config,
  DetectedAgent,
  GrokWebSearch,
  MergeReport,
  ModelTestOutcome,
  ProbeReport,
  Protocol,
  Source,
  Supplier,
  SupplierInput,
  SyncConfig,
} from "./types";

const STORAGE_KEY = "cxc-v2-mock-config";

function seed(): Config {
  return {
    version: 2,
    source: "wsl",
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
            model: "deepseek/deepseek-v4",
            protocols: ["chat_completions"],
            latency_ms: 1200,
            last_ok: false,
            last_error: "HTTP 429: rate limited",
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
        active_wsl: "bind-codex-1",
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
      claude: {
        active_wsl: "bind-claude-1",
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
      grok: {
        active_wsl: "bind-grok-1",
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
        ],
      },
      opencode: {
        active_wsl: "bind-opencode-1",
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
    custom_dirs: { codex: "", claude: "", grok: "", opencode: "" },
    migrated_from_v1: true,
  };
}

function load(): Config {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as Config;
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
    Math.floor(Math.random() * 256).toString(16).padStart(2, "0"),
  ).join("");
}

function delay(ms = 450) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function agentKey(agent: string): AgentKind {
  return agent as AgentKind;
}

function supplierOf(config: Config, supplierId: string): Supplier {
  const supplier = config.suppliers.find((item) => item.id === supplierId);
  if (!supplier) throw new Error("供应商不存在");
  return supplier;
}

function references(config: Config, supplierId: string): string[] {
  const labels: Record<AgentKind, string> = {
    codex: "Codex",
    claude: "Claude Code",
    grok: "Grok CLI",
    opencode: "OpenCode",
  };
  return (Object.keys(config.agents) as AgentKind[])
    .filter((kind) =>
      config.agents[kind].bindings.some((binding) => binding.supplier_id === supplierId),
    )
    .map((kind) => labels[kind]);
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

    case "get_path_env":
      return "/home/leezi/.local/bin:/home/leezi/.grok/bin:/usr/local/bin:/usr/bin:/bin" as T;

    case "detect_agents": {
      const detected: DetectedAgent[] = [
        {
          kind: "codex",
          installed: true,
          binary: "/home/leezi/.local/bin/codex",
          config_dir: "/home/leezi/.codex",
          configured: true,
          source: config.source,
        },
        {
          kind: "claude",
          installed: true,
          binary: "/home/leezi/.local/bin/claude",
          config_dir: "/home/leezi/.claude",
          configured: true,
          source: config.source,
        },
        {
          kind: "grok",
          installed: true,
          binary: "/home/leezi/.grok/bin/grok",
          config_dir: "/home/leezi/.grok",
          configured: true,
          source: config.source,
        },
        {
          kind: "opencode",
          installed: true,
          binary: "/usr/local/bin/opencode",
          config_dir: "/home/leezi/.config/opencode",
          configured: true,
          source: config.source,
        },
      ];
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
        throw new Error(`供应商仍被以下绑定引用：${refs.join("、")}`);
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
      const input = payload.input as BindingInput;
      supplierOf(config, input.supplier_id);
      const binding: Binding = {
        id: `bind-${id().slice(0, 8)}`,
        supplier_id: input.supplier_id,
        protocol: input.protocol,
        model: input.model.trim(),
        reasoning_effort: input.reasoning_effort,
        plan_reasoning_effort: input.plan_reasoning_effort,
        context_length: input.context_length,
        compact_ratio: input.compact_ratio,
        claude_models: normalizeClaudeModels(input.claude_models),
        grok_web_search: normalizeWebSearch(input.grok_web_search),
      };
      config.agents[agent].bindings.push(binding);
      save(config);
      return clone(config) as T;
    }

    case "edit_binding": {
      const agent = agentKey(payload.agent as string);
      const binding = config.agents[agent].bindings.find(
        (item) => item.id === (payload.id as string),
      );
      if (!binding) throw new Error("绑定不存在");
      const input = payload.input as BindingInput;
      Object.assign(binding, {
        supplier_id: input.supplier_id,
        protocol: input.protocol,
        model: input.model.trim(),
        reasoning_effort: input.reasoning_effort,
        plan_reasoning_effort: input.plan_reasoning_effort,
        context_length: input.context_length,
        compact_ratio: input.compact_ratio,
        claude_models: normalizeClaudeModels(input.claude_models),
        grok_web_search: normalizeWebSearch(input.grok_web_search),
      });
      save(config);
      return clone(config) as T;
    }

    case "delete_binding": {
      const agent = agentKey(payload.agent as string);
      const bindingId = payload.id as string;
      const agentConfig = config.agents[agent];
      if (agentConfig.active_app === bindingId || agentConfig.active_wsl === bindingId) {
        throw new Error("该绑定正在启用，请先切换后再删除");
      }
      agentConfig.bindings = agentConfig.bindings.filter((item) => item.id !== bindingId);
      save(config);
      return clone(config) as T;
    }

    case "activate_binding": {
      const agent = agentKey(payload.agent as string);
      const source = payload.source as Source;
      const bindingId = payload.id as string;
      if (!config.agents[agent].bindings.some((item) => item.id === bindingId)) {
        throw new Error("绑定不存在");
      }
      if (source === "app") config.agents[agent].active_app = bindingId;
      else config.agents[agent].active_wsl = bindingId;
      save(config);
      return clone(config) as T;
    }

    case "reload_active": {
      return clone(config) as T;
    }

    case "save_settings": {
      config.source = payload.source as Source;
      const dirs = (payload.customDirs ?? payload.custom_dirs) as
        | Config["custom_dirs"]
        | undefined;
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
        bindings: 0,
        skipped: [],
      } as T;

    default:
      throw new Error(`未实现的演示命令：${command}`);
  }
}

export function resetMockConfig() {
  localStorage.removeItem(STORAGE_KEY);
}
