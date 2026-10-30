import { useMemo } from "react";
import {
  Activity,
  Check,
  ChevronRight,
  Edit2,
  Loader2,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Messages } from "@/lib/i18n";
import type {
  AgentKind,
  Binding,
  Config,
  DetectedAgent,
  Source,
} from "@/lib/types";
import { AGENT_KINDS, AGENT_LABELS, CAPABILITIES, PROTOCOL_LABELS } from "@/lib/types";
import { ProtocolBadges } from "@/components/suppliers/SupplierPage";

interface AgentPageProps {
  config: Config;
  detected: DetectedAgent[];
  agent: AgentKind;
  source: Source;
  search: string;
  onAgent: (agent: AgentKind) => void;
  showAll: boolean;
  onShowAll: (value: boolean) => void;
  t: Messages;
  busy: boolean;
  onAddBinding: () => void;
  onEditBinding: (binding: Binding) => void;
  onDeleteBinding: (binding: Binding) => void;
  onActivate: (binding: Binding) => void;
  onReload: () => void;
  onTestBinding: (binding: Binding) => void;
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

function paramChips(binding: Binding, t: Messages) {
  const chips: string[] = [];
  if (binding.reasoning_effort) chips.push(`${t.reasoning}: ${binding.reasoning_effort}`);
  if (binding.plan_reasoning_effort) chips.push(`plan: ${binding.plan_reasoning_effort}`);
  if (binding.context_length) chips.push(`${t.context}: ${binding.context_length}`);
  if (binding.compact_ratio) chips.push(`${t.compact}: ${binding.compact_ratio}%`);
  if (binding.claude_models) chips.push(t.claudeModels);
  if (binding.grok_web_search) chips.push(`web_search: ${binding.grok_web_search.mode}`);
  return chips;
}

export function AgentPage({
  config,
  detected,
  agent,
  source,
  search,
  onAgent,
  showAll,
  onShowAll,
  t,
  busy,
  onAddBinding,
  onEditBinding,
  onDeleteBinding,
  onActivate,
  onReload,
  onTestBinding,
}: AgentPageProps) {
  const agentConfig = config.agents[agent];
  const activeId = source === "app" ? agentConfig.active_app : agentConfig.active_wsl;
  const active = agentConfig.bindings.find((binding) => binding.id === activeId) ?? null;
  const detection = detected.find((item) => item.kind === agent);
  const visibleAgents = useMemo(
    () =>
      AGENT_KINDS.filter(
        (kind) => showAll || detected.find((item) => item.kind === kind)?.installed,
      ),
    [detected, showAll],
  );

  const supplierName = (supplierId: string) =>
    config.suppliers.find((supplier) => supplier.id === supplierId)?.name ?? "—";

  const visibleBindings = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return agentConfig.bindings;
    return agentConfig.bindings.filter((binding) => {
      const supplier = supplierName(binding.supplier_id).toLowerCase();
      return supplier.includes(query) || binding.model.toLowerCase().includes(query);
    });
  }, [agentConfig.bindings, search, config.suppliers]);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-1.5">
          {visibleAgents.map((kind) => {
            const item = detected.find((entry) => entry.kind === kind);
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
                    item?.installed ? "bg-sticker-green" : "bg-muted-foreground/30"
                  }`}
                  title={item?.installed ? t.agentDetected : t.agentMissing}
                />
              </button>
            );
          })}
        </div>
        <button
          onClick={() => onShowAll(!showAll)}
          className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground/70 transition-colors hover:text-primary"
        >
          {showAll ? t.hideAll : t.showAll}
        </button>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-border/60 bg-card px-4 py-3">
        <div className="flex min-w-0 items-center gap-3">
          <AgentMark agent={agent} />
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <h2 className="text-sm font-bold text-foreground">{AGENT_LABELS[agent]}</h2>
              <span
                className={`rounded border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide ${
                  detection?.installed
                    ? "border-emerald-200 bg-emerald-50 text-emerald-700"
                    : "border-border bg-muted/30 text-muted-foreground"
                }`}
              >
                {detection?.installed ? t.agentDetected : t.agentMissing}
              </span>
              <span
                className={`rounded border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide ${
                  detection?.configured
                    ? "border-sky-200 bg-sky-50 text-sky-700"
                    : "border-border bg-muted/30 text-muted-foreground"
                }`}
              >
                {detection?.configured ? t.agentConfigured : t.agentUnconfigured}
              </span>
            </div>
            <p className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground/70">
              {detection?.config_dir ?? t.noDetection}
            </p>
          </div>
        </div>
        <Button size="sm" onClick={onAddBinding} disabled={busy}>
          <Plus className="size-3.5" />
          {t.addBinding}
        </Button>
      </div>

      <section className="space-y-2">
        <h3 className="text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground/70">
          {t.currentBinding}
        </h3>
        {active ? (
          <div className="rounded-xl border border-primary/20 bg-primary/[0.03] p-4 shadow-[0_0_0_1px_rgba(0,117,222,0.04)]">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="rounded bg-primary/10 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-primary">
                    {t.active}
                  </span>
                  <span className="truncate text-sm font-bold text-foreground">
                    {supplierName(active.supplier_id)}
                  </span>
                  <ProtocolBadges protocols={[active.protocol]} compact />
                </div>
                <p className="mt-1 truncate font-mono text-[11px] text-muted-foreground">
                  {active.model}
                </p>
                <div className="mt-2 flex flex-wrap gap-1">
                  {paramChips(active, t).map((chip) => (
                    <span
                      key={chip}
                      className="rounded border border-border/50 bg-card px-1.5 py-0.5 text-[9px] font-semibold text-muted-foreground"
                    >
                      {chip}
                    </span>
                  ))}
                </div>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busy}
                  onClick={() => onTestBinding(active)}
                >
                  <Activity className="size-3" />
                  {t.testModel}
                </Button>
                <Button size="sm" variant="outline" disabled={busy} onClick={onReload}>
                  {busy ? (
                    <Loader2 className="size-3 animate-spin" />
                  ) : (
                    <RefreshCw className="size-3" />
                  )}
                  {t.reload}
                </Button>
              </div>
            </div>
          </div>
        ) : (
          <div className="rounded-xl border border-dashed border-border bg-card py-8 text-center text-xs text-muted-foreground">
            {t.noBindings}
          </div>
        )}
      </section>

      <section className="space-y-2">
        <div className="flex items-center justify-between">
          <h3 className="text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground/70">
            {t.bindingCatalog}
          </h3>
          <span className="text-[10px] text-muted-foreground/60">
            {visibleBindings.length}
          </span>
        </div>
        {agentConfig.bindings.length === 0 ? (
          <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed border-border bg-card py-10 text-center">
            <p className="text-xs text-muted-foreground">{t.noBindings}</p>
            <Button size="sm" onClick={onAddBinding}>
              <Plus className="size-3.5" />
              {t.addFirstBinding}
            </Button>
          </div>
        ) : visibleBindings.length === 0 ? (
          <div className="rounded-xl border border-dashed border-border bg-card py-10 text-center text-xs text-muted-foreground">
            {t.emptySearch}
          </div>
        ) : (
          <div className="overflow-hidden rounded-xl border border-border/60 bg-card">
            {visibleBindings.map((binding, index) => {
              const isActive = binding.id === active?.id;
              return (
                <div
                  key={binding.id}
                  className={`flex flex-wrap items-center justify-between gap-3 px-4 py-3 transition-colors hover:bg-muted/10 ${
                    index > 0 ? "border-t border-border/40" : ""
                  }`}
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      {isActive ? (
                        <Check className="size-3.5 text-primary" />
                      ) : (
                        <ChevronRight className="size-3.5 text-muted-foreground/30" />
                      )}
                      <span className="truncate text-xs font-bold text-foreground">
                        {supplierName(binding.supplier_id)}
                      </span>
                      <span className="text-muted-foreground/30">/</span>
                      <span className="truncate font-mono text-[11px] text-muted-foreground">
                        {binding.model}
                      </span>
                      <ProtocolBadges protocols={[binding.protocol]} compact />
                    </div>
                    <div className="mt-1 flex flex-wrap gap-1 pl-5">
                      {paramChips(binding, t).map((chip) => (
                        <span
                          key={chip}
                          className="rounded border border-border/40 bg-muted/20 px-1.5 py-0.5 text-[9px] text-muted-foreground/80"
                        >
                          {chip}
                        </span>
                      ))}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    {!isActive ? (
                      <Button
                        size="xs"
                        disabled={busy}
                        onClick={() => onActivate(binding)}
                        className="bg-primary/10 text-primary hover:bg-primary hover:text-primary-foreground"
                      >
                        {t.activate}
                      </Button>
                    ) : null}
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      disabled={busy}
                      onClick={() => onEditBinding(binding)}
                      title={t.editBinding}
                    >
                      <Edit2 className="size-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      disabled={busy}
                      onClick={() => onDeleteBinding(binding)}
                      className="text-red-500 hover:bg-red-500/10 hover:text-red-600"
                      title={t.deleteBinding}
                    >
                      <Trash2 className="size-3" />
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </section>

      <div className="rounded-xl border border-border/50 bg-muted/10 px-4 py-3 text-[10px] leading-relaxed text-muted-foreground/80">
        <span className="font-bold text-foreground/70">{t.agentProtocols}: </span>
        {CAPABILITIES[agent].protocols.map((protocol) => PROTOCOL_LABELS[protocol]).join(" / ")}
        <span className="mx-2 text-border">|</span>
        <span className="font-bold text-foreground/70">{t.configPath}: </span>
        <span className="font-mono">{detection?.config_dir ?? "—"}</span>
      </div>
    </div>
  );
}
