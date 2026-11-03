import { useMemo, useState } from "react";
import { CircleCheck, CircleSlash, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Messages } from "@/lib/i18n";
import type {
  AgentConfig,
  AgentConfigInput,
  AgentKind,
  Config,
  DetectedAgent,
  Env,
  EnvInfo,
  ModelTestResult,
  Protocol,
} from "@/lib/types";
import { AGENT_KINDS, AGENT_LABELS, slotOf } from "@/lib/types";
import { AgentMark } from "./AgentMark";
import { BindingRow } from "./BindingRow";
import { BindingDialog } from "./BindingDialog";

interface AgentPageProps {
  config: Config;
  detected: DetectedAgent[];
  envInfos: EnvInfo[];
  agent: AgentKind;
  env: Env;
  search: string;
  onSelect: (agent: AgentKind, env: Env) => void;
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

/** 宿主环境不加后缀；WSL 等间接环境带上发行版，避免同名 Agent 混在一起。 */
function agentTitle(kind: AgentKind, info: EnvInfo): string {
  return info.host ? AGENT_LABELS[kind] : `${AGENT_LABELS[kind]} ${info.label}`;
}

export function AgentPage({
  config,
  detected,
  envInfos,
  agent,
  env,
  search,
  onSelect,
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
  const orderedInfos = useMemo(
    () => [...envInfos].sort((a, b) => Number(b.host) - Number(a.host)),
    [envInfos],
  );
  const selectedInfo = orderedInfos.find((info) => info.env === env);
  const selectedTitle = selectedInfo
    ? agentTitle(agent, selectedInfo)
    : AGENT_LABELS[agent];

  const navigation = useMemo(
    () =>
      AGENT_KINDS.flatMap((kind) =>
        orderedInfos.map((info) => {
          const slot = slotOf(config, kind, info.env);
          const itemDetection = detected.find(
            (item) => item.kind === kind && item.env === info.env,
          );
          return {
            key: `${kind}:${info.env}`,
            kind,
            info,
            count: slot?.bindings.length ?? 0,
            installed: itemDetection?.installed ?? false,
          };
        }),
      ),
    [config, detected, orderedInfos],
  );

  const visible = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return bindings;
    const agentTerms = `${AGENT_LABELS[agent]} ${selectedInfo?.label ?? ""}`;
    return bindings.filter((binding) => {
      const supplier = config.suppliers.find((item) => item.id === binding.supplier_id);
      return [binding.model, supplier?.name ?? "", agentTerms]
        .join(" ")
        .toLowerCase()
        .includes(query);
    });
  }, [bindings, search, config.suppliers, agent, selectedInfo]);

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
    <div className="flex flex-col gap-4 md:flex-row md:items-start">
      <nav aria-label={t.sectionAgents} className="shrink-0 md:sticky md:top-0 md:w-56">
        <div className="space-y-1 rounded-xl border border-border/25 bg-card/10 p-1.5 backdrop-blur-sm [scrollbar-width:thin] md:max-h-[calc(100vh-6.5rem)] md:overflow-y-auto md:overscroll-contain">
          {navigation.map((item) => {
            const selected = item.kind === agent && item.info.env === env;
            const itemName = item.info.host
              ? AGENT_LABELS[item.kind]
              : `${AGENT_LABELS[item.kind]} ${item.info.label}`;
            return (
              <button
                key={item.key}
                type="button"
                onClick={() => onSelect(item.kind, item.info.env)}
                aria-label={`${itemName}, ${
                  item.installed ? t.agentInstalled : t.agentMissing
                }, ${t.bindings} ${item.count}`}
                aria-current={selected ? "true" : undefined}
                className={`flex w-full items-center gap-2.5 rounded-lg border px-2.5 py-2 text-left transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 ${
                  selected
                    ? "border-primary/35 bg-primary/[0.06] shadow-[inset_0_0_0_1px_rgba(0,117,222,0.06)]"
                    : "border-transparent text-muted-foreground hover:border-border/30 hover:bg-card/40 hover:text-foreground"
                }`}
              >
                <AgentMark agent={item.kind} />
                <span className="min-w-0 flex-1 leading-tight">
                  <span className="block truncate text-xs font-semibold">
                    {AGENT_LABELS[item.kind]}
                  </span>
                  {item.info.host ? null : (
                    <span
                      className="mt-0.5 block truncate text-[10px] font-medium text-muted-foreground/70"
                      title={item.info.label}
                    >
                      {item.info.label}
                    </span>
                  )}
                </span>
                <span
                  className="shrink-0 font-mono text-[10px] text-muted-foreground/55"
                  title={`${t.bindings} ${item.count}`}
                >
                  {item.count}
                </span>
                <span
                  className="flex shrink-0 items-center"
                  title={item.installed ? t.agentInstalled : t.agentMissing}
                >
                  {item.installed ? (
                    <CircleCheck className="size-3.5 text-sticker-green" aria-hidden="true" />
                  ) : (
                    <CircleSlash
                      className="size-3.5 text-muted-foreground/35"
                      aria-hidden="true"
                    />
                  )}
                </span>
              </button>
            );
          })}
        </div>
      </nav>

      <div className="min-w-0 flex-1 space-y-3">
        <div className="px-1">
          <div className="flex flex-wrap items-center gap-2">
            <AgentMark agent={agent} />
            <h2 className="truncate text-sm font-bold tracking-tight">
              {selectedTitle}
            </h2>
          </div>
          <p className="mt-1 truncate font-mono text-[10px] text-muted-foreground/60">
            {detection?.config_dir ?? ""}
          </p>
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
          visible.map((binding, index) => {
            const isActive = binding.id === activeId;
            const previousActive =
              index > 0 && visible[index - 1].id === activeId;
            return (
              <div
                key={binding.id}
                className={
                  index > 0 && !isActive && !previousActive
                    ? "border-t border-border/30"
                    : ""
                }
              >
                <BindingRow
                  binding={binding}
                  supplier={config.suppliers.find(
                    (item) => item.id === binding.supplier_id,
                  )}
                  agent={agent}
                  isActive={isActive}
                  busy={busy || saving}
                  testing={testingId === binding.id}
                  t={t}
                  onActivate={() => void onActivate(binding)}
                  onEdit={() => setDialog({ open: true, binding })}
                  onDelete={() => void onDelete(binding)}
                  onTest={() => void test(binding)}
                />
              </div>
            );
          })
        )}
        </div>
      </div>

      <BindingDialog
        open={dialog.open}
        agent={agent}
        env={env}
        envLabel={selectedInfo?.label}
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
