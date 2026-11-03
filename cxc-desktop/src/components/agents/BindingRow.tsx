import { Activity, Check, Edit2, Loader2, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Messages } from "@/lib/i18n";
import type { AgentConfig, AgentKind, Supplier } from "@/lib/types";
import { PROTOCOL_LABELS } from "@/lib/types";

interface BindingRowProps {
  binding: AgentConfig;
  supplier: Supplier | undefined;
  agent: AgentKind;
  isActive: boolean;
  busy: boolean;
  testing: boolean;
  t: Messages;
  onActivate: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onTest: () => void;
}

function chips(binding: AgentConfig, t: Messages): string[] {
  const list: string[] = [];
  if (binding.reasoning_effort) list.push(`${t.reasoning}: ${binding.reasoning_effort}`);
  if (binding.plan_reasoning_effort) list.push(`plan: ${binding.plan_reasoning_effort}`);
  if (binding.context_length) list.push(`${t.context}: ${binding.context_length}`);
  if (binding.compact_ratio) list.push(`${t.compact}: ${binding.compact_ratio}%`);
  if (binding.claude_models) list.push(t.claudeModels);
  if (binding.grok_web_search) list.push(`web_search: ${binding.grok_web_search.mode}`);
  return list;
}

function latencyOf(supplier: Supplier | undefined, model: string) {
  const entry = supplier?.models.find((item) => item.model === model);
  if (!entry || entry.last_ok == null) return null;
  return entry;
}

export function BindingRow({
  binding,
  supplier,
  isActive,
  busy,
  testing,
  t,
  onActivate,
  onEdit,
  onDelete,
  onTest,
}: BindingRowProps) {
  const model = binding.model.trim();
  const test = latencyOf(supplier, model);
  // 只有显式指定账号时才显示账号名；跟随默认时不显示，避免和卡片重复。
  const accountName = binding.key_id
    ? supplier?.keys.find((key) => key.id === binding.key_id)?.name
    : undefined;

  return (
    <div
      className={`group relative flex flex-wrap items-start justify-between gap-3 px-4 py-3 transition-all duration-200 ${
        isActive
          ? "my-1 rounded-xl bg-primary/[0.06] shadow-[inset_0_0_0_1px_rgba(0,117,222,0.45),inset_0_0_12px_rgba(0,117,222,0.08)] dark:bg-primary/[0.09] dark:shadow-[inset_0_0_0_1px_rgba(46,170,220,0.4),inset_0_0_12px_rgba(46,170,220,0.1)]"
          : "hover:bg-white/25 dark:hover:bg-white/[0.04]"
      }`}
    >
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="truncate font-mono text-xs font-bold text-foreground">
            {model || t.notConfigured}
          </span>
          {isActive ? (
            <span className="flex items-center gap-1 rounded border border-primary/20 bg-primary/10 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-primary">
              <Check className="size-2.5" />
              {t.inUse}
            </span>
          ) : null}
          {test ? (
            <span
              className={`font-mono text-[9px] font-bold ${
                test.last_ok ? "text-sticker-green" : "text-red-500"
              }`}
              title={test.last_error}
            >
              {test.last_ok ? t.success : t.failed} · {test.latency_ms} ms
            </span>
          ) : null}
        </div>

        <p className="mt-0.5 truncate text-[10.5px] text-muted-foreground">
          {supplier?.name ?? t.unknownSupplier} · {PROTOCOL_LABELS[binding.protocol]}
          {accountName ? ` · ${accountName}` : ""}
        </p>

        {chips(binding, t).length ? (
          <div className="mt-1.5 flex flex-wrap gap-1">
            {chips(binding, t).map((chip) => (
              <span
                key={chip}
                className="rounded border border-border/40 bg-muted/20 px-1.5 py-0.5 text-[9px] text-muted-foreground/80"
              >
                {chip}
              </span>
            ))}
          </div>
        ) : null}
      </div>

      <div className="flex shrink-0 items-center gap-1">
        {!isActive ? (
          <Button
            size="xs"
            disabled={busy}
            onClick={onActivate}
            className="bg-primary/10 text-primary hover:bg-primary hover:text-primary-foreground"
          >
            {t.activate}
          </Button>
        ) : null}
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={busy || testing}
          onClick={onTest}
          title={t.test}
        >
          {testing ? (
            <Loader2 className="size-3 animate-spin" />
          ) : (
            <Activity className="size-3" />
          )}
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={busy}
          onClick={onEdit}
          title={t.editBinding}
        >
          <Edit2 className="size-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={busy || isActive}
          onClick={onDelete}
          className="text-red-500 hover:bg-red-500/10 hover:text-red-600 disabled:text-muted-foreground/30"
          title={isActive ? t.activeCannotDelete : t.delete}
        >
          <Trash2 className="size-3" />
        </Button>
      </div>
    </div>
  );
}
