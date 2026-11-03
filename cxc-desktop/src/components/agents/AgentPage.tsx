import { useMemo, useState } from "react";
import { AlertCircle, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Messages } from "@/lib/i18n";
import type {
  AgentConfig,
  AgentConfigInput,
  AgentKind,
  Config,
  DetectedAgent,
  Env,
  ModelTestResult,
  Protocol,
} from "@/lib/types";
import { AGENT_KINDS, AGENT_LABELS, ENV_LABELS, slotOf } from "@/lib/types";
import { BindingRow } from "./BindingRow";
import { BindingDialog } from "./BindingDialog";

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
  onAdd: (input: AgentConfigInput, activate: boolean) => Promise<void>;
  onEdit: (id: string, input: AgentConfigInput) => Promise<void>;
  onDelete: (binding: AgentConfig) => Promise<void>;
  onActivate: (binding: AgentConfig) => Promise<void>;
  onTest: (
    supplierId: string,
    model: string,
    protocol: Protocol,
    keyId?: string,
  ) => Promise<ModelTestResult>;
  onRefreshModels: (supplierId: string) => Promise<void>;
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
  onAdd,
  onEdit,
  onDelete,
  onActivate,
  onTest,
  onRefreshModels,
}: AgentPageProps) {
  const [dialog, setDialog] = useState<{ open: boolean; binding: AgentConfig | null }>({
    open: false,
    binding: null,
  });
  const [saving, setSaving] = useState(false);
  const [testingId, setTestingId] = useState<string | null>(null);

  const slot = slotOf(config, agent, env);
  const bindings = slot?.bindings ?? [];
  const activeId = slot?.active ?? null;
  const detection = detected.find((item) => item.kind === agent && item.env === env);

  const visible = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return bindings;
    return bindings.filter((binding) => {
      const supplier = config.suppliers.find((item) => item.id === binding.supplier_id);
      return [binding.model, supplier?.name ?? "", AGENT_LABELS[agent], ENV_LABELS[env]]
        .join(" ")
        .toLowerCase()
        .includes(query);
    });
  }, [bindings, search, config.suppliers, agent, env]);

  const visibleEnvs = envs.length ? envs : (["wsl"] as Env[]);

  const submit = async (input: AgentConfigInput, activate: boolean) => {
    setSaving(true);
    try {
      if (dialog.binding) {
        await onEdit(dialog.binding.id, input);
      } else {
        await onAdd(input, activate);
      }
      setDialog({ open: false, binding: null });
    } finally {
      setSaving(false);
    }
  };

  const test = async (binding: AgentConfig) => {
    setTestingId(binding.id);
    try {
      await onTest(
        binding.supplier_id,
        binding.model,
        binding.protocol,
        binding.key_id,
      );
    } finally {
      setTestingId(null);
    }
  };

  return (
    <div className="space-y-3">
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
                  : "border-border/40 bg-card/40 text-muted-foreground backdrop-blur-md hover:bg-card/60 hover:text-foreground"
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

      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          {visibleEnvs.length > 1 ? (
            <div className="flex items-center rounded-lg border border-border/60 bg-muted/30 p-0.5">
              {visibleEnvs.map((item) => (
                <button
                  key={item}
                  onClick={() => onEnv(item)}
                  className={`rounded-md px-3 py-1 text-[11px] font-bold transition-all ${
                    env === item
                      ? "border border-border/10 bg-card/70 text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {ENV_LABELS[item]}
                </button>
              ))}
            </div>
          ) : (
            <span className="flex items-center gap-1.5 text-[10px] text-muted-foreground/60">
              <AlertCircle className="size-3" />
              {ENV_LABELS[env]}
            </span>
          )}
          <span className="hidden truncate font-mono text-[10px] text-muted-foreground/60 sm:inline">
            {detection?.config_dir ?? ""}
          </span>
        </div>
        <span
          className={`rounded border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide ${
            detection?.installed
              ? "border-emerald-200 bg-emerald-50 text-emerald-700"
              : "border-border bg-muted/30 text-muted-foreground"
          }`}
        >
          {detection?.installed ? t.agentInstalled : t.agentMissing}
        </span>
      </div>

      <div className="overflow-hidden rounded-xl">
        <div className="flex items-center justify-between border-b border-border/30 px-4 py-2.5">
          <span className="text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground/70">
            {t.bindings} {bindings.length}
          </span>
          <Button
            size="xs"
            onClick={() => setDialog({ open: true, binding: null })}
            disabled={busy || saving}
            className={bindings.length === 0 ? "invisible" : ""}
          >
            <Plus className="size-3" />
            {t.newBinding}
          </Button>
        </div>

        {bindings.length === 0 ? (
          <div className="flex flex-col items-center gap-2 py-12 text-center">
            <p className="text-xs text-muted-foreground">{t.noBindings}</p>
            <Button size="sm" onClick={() => setDialog({ open: true, binding: null })}>
              <Plus className="size-3.5" />
              {t.newBinding}
            </Button>
          </div>
        ) : visible.length === 0 ? (
          <div className="py-12 text-center text-xs text-muted-foreground">
            {t.emptySearch}
          </div>
        ) : (
          visible.map((binding, index) => (
            <div
              key={binding.id}
              className={index > 0 ? "border-t border-border/30" : ""}
            >
              <BindingRow
                binding={binding}
                supplier={config.suppliers.find(
                  (item) => item.id === binding.supplier_id,
                )}
                agent={agent}
                isActive={binding.id === activeId}
                busy={busy || saving}
                testing={testingId === binding.id}
                t={t}
                onActivate={() => void onActivate(binding)}
                onEdit={() => setDialog({ open: true, binding })}
                onDelete={() => void onDelete(binding)}
                onTest={() => void test(binding)}
              />
            </div>
          ))
        )}
      </div>

      <BindingDialog
        open={dialog.open}
        agent={agent}
        env={env}
        binding={dialog.binding}
        suppliers={config.suppliers}
        isActive={dialog.binding?.id === activeId}
        t={t}
        saving={saving}
        onClose={() => setDialog({ open: false, binding: null })}
        onSubmit={submit}
        onTest={onTest}
        onRefreshModels={onRefreshModels}
      />
    </div>
  );
}
