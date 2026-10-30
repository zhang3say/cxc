import { useMemo } from "react";
import { AlertCircle } from "lucide-react";
import type { Messages } from "@/lib/i18n";
import type {
  AgentConfigInput,
  AgentKind,
  Config,
  DetectedAgent,
  Env,
  ModelTestResult,
  Protocol,
} from "@/lib/types";
import { AGENT_KINDS, AGENT_LABELS, ENV_LABELS, configOf } from "@/lib/types";
import { AgentConfigCard } from "./AgentConfigCard";

interface AgentPageProps {
  config: Config;
  detected: DetectedAgent[];
  envs: Env[];
  agent: AgentKind;
  env: Env;
  search: string;
  onAgent: (agent: AgentKind) => void;
  onEnv: (env: Env) => void;
  t: Messages;
  busy: boolean;
  onSave: (input: AgentConfigInput) => Promise<void>;
  onRewrite: () => Promise<void>;
  onClear: () => Promise<void>;
  onTest: (
    supplierId: string,
    model: string,
    protocol: Protocol,
  ) => Promise<ModelTestResult>;
}

const agentAccent: Record<AgentKind, string> = {
  codex: "from-[#B1A7FF] to-[#3941FF]",
  claude: "from-[#D97757] to-[#B45309]",
  grok: "from-[#111827] to-[#4B5563]",
  opencode: "from-[#0EA5E9] to-[#0369A1]",
};

function AgentMark({ agent }: { agent: AgentKind }) {
  return (
    <span
      className={`flex size-6 items-center justify-center rounded-md bg-gradient-to-br ${agentAccent[agent]} text-[10px] font-black text-white shadow-sm`}
    >
      {AGENT_LABELS[agent].slice(0, 1)}
    </span>
  );
}

export function AgentPage({
  config,
  detected,
  envs,
  agent,
  env,
  search,
  onAgent,
  onEnv,
  t,
  busy,
  onSave,
  onRewrite,
  onClear,
  onTest,
}: AgentPageProps) {
  const detection = detected.find(
    (item) => item.kind === agent && item.env === env,
  );
  const agentConfig = configOf(config, agent, env);

  const matchesSearch = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return true;
    const supplier = config.suppliers.find(
      (item) => item.id === agentConfig?.supplier_id,
    );
    return [
      AGENT_LABELS[agent],
      ENV_LABELS[env],
      supplier?.name ?? "",
      agentConfig?.model ?? "",
    ]
      .join(" ")
      .toLowerCase()
      .includes(query);
  }, [search, agent, env, agentConfig, config.suppliers]);

  const visibleEnvs = envs.length ? envs : (["wsl"] as Env[]);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-1.5">
        {AGENT_KINDS.map((kind) => {
          const installed = detected.some(
            (item) => item.kind === kind && item.installed,
          );
          const selected = agent === kind;
          return (
            <button
              key={kind}
              onClick={() => onAgent(kind)}
              className={`flex items-center gap-2 rounded-lg border px-2.5 py-1.5 text-xs font-semibold transition-all ${
                selected
                  ? "border-primary/40 bg-primary/[0.05] text-primary shadow-sm"
                  : "border-border/50 bg-card text-muted-foreground hover:bg-muted/20 hover:text-foreground"
              }`}
            >
              <AgentMark agent={kind} />
              <span>{AGENT_LABELS[kind]}</span>
              <span
                className={`size-1.5 rounded-full ${
                  installed ? "bg-sticker-green" : "bg-muted-foreground/30"
                }`}
                title={installed ? t.agentInstalled : t.agentMissing}
              />
            </button>
          );
        })}
      </div>

      {visibleEnvs.length > 1 ? (
        <div className="flex items-center gap-1">
          <div className="flex items-center rounded-lg border border-border/60 bg-muted/30 p-0.5">
            {visibleEnvs.map((item) => (
              <button
                key={item}
                onClick={() => onEnv(item)}
                className={`rounded-md px-3 py-1 text-[11px] font-bold transition-all ${
                  env === item
                    ? "border border-border/10 bg-card text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {ENV_LABELS[item]}
              </button>
            ))}
          </div>
          <span className="text-[10px] text-muted-foreground/60">
            {AGENT_LABELS[agent]} · {ENV_LABELS[env]}
          </span>
        </div>
      ) : (
        <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground/60">
          <AlertCircle className="size-3" />
          {ENV_LABELS[env]}
        </div>
      )}

      {matchesSearch ? (
        <AgentConfigCard
          key={`${agent}:${env}:${JSON.stringify(agentConfig ?? null)}`}
          agent={agent}
          env={env}
          config={agentConfig}
          suppliers={config.suppliers}
          detection={detection}
          t={t}
          busy={busy}
          onSave={onSave}
          onRewrite={onRewrite}
          onClear={onClear}
          onTest={onTest}
        />
      ) : (
        <div className="rounded-xl border border-dashed border-border bg-card py-16 text-center text-xs text-muted-foreground">
          {t.emptySearch}
        </div>
      )}
    </div>
  );
}
