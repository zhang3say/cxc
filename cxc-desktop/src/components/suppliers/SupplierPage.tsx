import { useMemo, useState } from "react";
import {
  Activity,
  AlertCircle,
  CheckCircle2,
  Download,
  Edit2,
  LayoutGrid,
  List,
  Loader2,
  Plus,
  RefreshCw,
  Server,
  Trash2,
  XCircle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { Messages } from "@/lib/i18n";
import type { Config, Protocol, Supplier, SupplierModel } from "@/lib/types";
import { PROTOCOL_LABELS, effectiveKey } from "@/lib/types";
import { CxcSelect } from "@/components/ui/select";

export const PROTOCOL_BADGE_CLASSES: Record<Protocol, string> = {
  chat_completions: "border-sky-200 bg-sky-50 text-sky-700",
  responses: "border-purple-200 bg-purple-50 text-purple-700",
  messages: "border-orange-200 bg-orange-50 text-orange-700",
};

export function ProtocolBadges({
  protocols,
  compact = false,
}: {
  protocols: Protocol[];
  compact?: boolean;
}) {
  if (protocols.length === 0) {
    return (
      <span className="rounded border border-border/50 bg-muted/30 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-muted-foreground/60">
        unknown
      </span>
    );
  }
  return (
    <div className="flex flex-wrap items-center gap-1">
      {protocols.map((protocol) => (
        <span
          key={protocol}
          className={`rounded border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide ${PROTOCOL_BADGE_CLASSES[protocol]} ${compact ? "" : "px-2"}`}
        >
          {PROTOCOL_LABELS[protocol]}
        </span>
      ))}
    </div>
  );
}

function latencyClass(latency?: number) {
  if (latency == null) return "text-muted-foreground/60";
  if (latency < 200) return "text-sticker-green";
  if (latency < 600) return "text-sticker-orange";
  return "text-red-500";
}

function statusOf(supplier: Supplier): "ok" | "fail" | "unknown" {
  const tested = supplier.models.filter((model) => model.last_ok != null);
  if (tested.length === 0) return "unknown";
  return tested.some((model) => model.last_ok) ? "ok" : "fail";
}

function statusDotClass(status: "ok" | "fail" | "unknown") {
  if (status === "ok") return "bg-sticker-green shadow-[0_0_6px_rgba(26,174,57,0.35)]";
  if (status === "fail") return "bg-red-500";
  return "bg-muted-foreground/30";
}

function lastLatencyOf(supplier: Supplier): number | undefined {
  // 测试失败时不再展示延迟毫秒，仅以成功结果作为端点延迟。
  return supplier.models.find(
    (model) => model.last_ok === true && model.latency_ms != null,
  )?.latency_ms;
}

/** 中转站延迟列文案：有成功结果显示毫秒，测试过但全失败显示「失败」。 */
function latencyText(supplier: Supplier, t: Messages): string {
  const latency = lastLatencyOf(supplier);
  if (latency != null) return `${latency} ms`;
  return supplier.models.some((model) => model.last_ok != null)
    ? t.failed
    : t.never;
}

/** 中转站延迟列配色：成功按延迟分档，全部失败标红，未测试置灰。 */
function latencyColorClass(supplier: Supplier): string {
  const latency = lastLatencyOf(supplier);
  if (latency != null) return latencyClass(latency);
  return supplier.models.some((model) => model.last_ok != null)
    ? "text-red-500"
    : "text-muted-foreground/60";
}

function formatTime(value?: string) {
  if (!value) return "—";
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

type ViewMode = "list" | "cards";
const VIEW_STORAGE_KEY = "cxc-supplier-view";

function initialView(): ViewMode {
  const stored = localStorage.getItem(VIEW_STORAGE_KEY);
  return stored === "cards" || stored === "list" ? stored : "list";
}

interface SupplierActionsProps {
  supplier: Supplier;
  action?: string;
  t: Messages;
  onProbe: () => void;
  onFetchModels: () => void;
  onTestAll: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

function SupplierActions({
  action,
  t,
  onProbe,
  onFetchModels,
  onTestAll,
  onEdit,
  onDelete,
}: SupplierActionsProps) {
  return (
    <div className="flex items-center gap-1">
      <Button
        variant="ghost"
        size="icon-sm"
        disabled={Boolean(action)}
        onClick={onProbe}
        title={t.probe}
      >
        {action === "probe" ? (
          <Loader2 className="size-3 animate-spin" />
        ) : (
          <RefreshCw className="size-3" />
        )}
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        disabled={Boolean(action)}
        onClick={onFetchModels}
        title={t.fetchModels}
      >
        {action === "models" ? (
          <Loader2 className="size-3 animate-spin" />
        ) : (
          <Download className="size-3" />
        )}
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        disabled={Boolean(action)}
        onClick={onTestAll}
        title={t.testAll}
      >
        {action === "test" ? (
          <Loader2 className="size-3 animate-spin" />
        ) : (
          <Activity className="size-3" />
        )}
      </Button>
      <span className="mx-0.5 h-4 w-px bg-border/60" />
      <Button
        variant="ghost"
        size="icon-sm"
        disabled={Boolean(action)}
        onClick={onEdit}
        title={t.editSupplier}
      >
        <Edit2 className="size-3" />
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        disabled={Boolean(action)}
        onClick={onDelete}
        className="text-red-500 hover:bg-red-500/10 hover:text-red-600"
        title={t.delete}
      >
        <Trash2 className="size-3" />
      </Button>
    </div>
  );
}

function SupplierCard({
  supplier,
  action,
  t,
  onOpen,
  onProbe,
  onFetchModels,
  onTestAll,
  onEdit,
  onDelete,
}: {
  supplier: Supplier;
  action?: string;
  t: Messages;
  onOpen: () => void;
  onProbe: () => void;
  onFetchModels: () => void;
  onTestAll: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const status = statusOf(supplier);

  return (
    <div
      className="group flex cursor-pointer flex-col overflow-hidden rounded-xl border border-white/60 bg-card/30 shadow-[0_8px_30px_-12px_rgba(0,0,0,0.08)] backdrop-blur-xl transition-all hover:border-white/80 hover:bg-card/40 dark:border-white/[0.06] dark:hover:border-white/10"
      onClick={onOpen}
    >
      <div className="flex items-start justify-between gap-3 p-4 pb-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className={`size-2 shrink-0 rounded-full ${statusDotClass(status)}`} />
            <h3 className="truncate text-sm font-bold text-foreground">{supplier.name}</h3>
          </div>
          {supplier.remark ? (
            <p className="mt-1 line-clamp-1 text-[11px] text-muted-foreground">
              {supplier.remark}
            </p>
          ) : null}
          <p className="mt-0.5 text-[10px] text-muted-foreground/70">
            {supplier.keys.length} {t.accountsCount}
          </p>
        </div>
        <ProtocolBadges protocols={supplier.protocols} />
      </div>

      <div className="space-y-2 px-4 pb-3">
        <p className="truncate font-mono text-[11px] text-muted-foreground/80">
          {supplier.base_url}
        </p>
        <div className="flex items-center justify-between text-[10.5px]">
          <span className="font-medium text-muted-foreground/80">
            {supplier.models.length} {t.modelsCount}
          </span>
          <span className={`font-bold ${latencyColorClass(supplier)}`}>
            {latencyText(supplier, t)}
          </span>
        </div>
        <div className="flex items-center justify-between text-[10px] text-muted-foreground/60">
          <span>{t.lastProbe}</span>
          <span>{formatTime(supplier.last_probe)}</span>
        </div>
      </div>

      <div
        className="mt-auto flex items-center justify-end border-t border-white/40 bg-white/10 px-3 py-2 dark:border-white/[0.05] dark:bg-white/[0.02]"
        onClick={(event) => event.stopPropagation()}
      >
        <SupplierActions
          supplier={supplier}
          action={action}
          t={t}
          onProbe={onProbe}
          onFetchModels={onFetchModels}
          onTestAll={onTestAll}
          onEdit={onEdit}
          onDelete={onDelete}
        />
      </div>
    </div>
  );
}

function SupplierListRow({
  supplier,
  action,
  t,
  onOpen,
  onProbe,
  onFetchModels,
  onTestAll,
  onEdit,
  onDelete,
}: {
  supplier: Supplier;
  action?: string;
  t: Messages;
  onOpen: () => void;
  onProbe: () => void;
  onFetchModels: () => void;
  onTestAll: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const status = statusOf(supplier);

  return (
    <tr
      className="group cursor-pointer border-t border-border/25 transition-colors first:border-t-0 hover:bg-white/30 dark:hover:bg-white/[0.04]"
      onClick={onOpen}
    >
      <td className="border-l-2 border-transparent px-4 py-2.5 transition-colors group-hover:border-primary/60">
        <div className="flex items-center gap-2">
          <span className={`size-2 shrink-0 rounded-full ${statusDotClass(status)}`} />
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="truncate text-xs font-bold text-foreground">
                {supplier.name}
              </span>
              {supplier.remark ? (
                <span className="hidden truncate text-[10px] text-muted-foreground/70 lg:inline">
                  {supplier.remark}
                </span>
              ) : null}
            </div>
            <p className="truncate font-mono text-[10px] text-muted-foreground/70">
              {supplier.base_url}
            </p>
          </div>
        </div>
      </td>
      <td className="px-3 py-2.5">
        <ProtocolBadges protocols={supplier.protocols} compact />
      </td>
      <td className="px-3 py-2.5 text-center font-mono text-[10.5px] text-muted-foreground">
        {supplier.keys.length}
      </td>
      <td className="px-3 py-2.5 text-center font-mono text-[10.5px] text-muted-foreground">
        {supplier.models.length}
      </td>
      <td className={`px-3 py-2.5 text-right font-mono text-[10.5px] font-bold ${latencyColorClass(supplier)}`}>
        {latencyText(supplier, t)}
      </td>
      <td className="px-3 py-2.5" onClick={(event) => event.stopPropagation()}>
        <div className="flex justify-end">
          <SupplierActions
            supplier={supplier}
            action={action}
            t={t}
            onProbe={onProbe}
            onFetchModels={onFetchModels}
            onTestAll={onTestAll}
            onEdit={onEdit}
            onDelete={onDelete}
          />
        </div>
      </td>
    </tr>
  );
}

interface SupplierPageProps {
  config: Config;
  search: string;
  t: Messages;
  busy: Record<string, string>;
  onAdd: () => void;
  onEdit: (supplier: Supplier) => void;
  onDelete: (supplier: Supplier) => void;
  onProbe: (supplier: Supplier) => void;
  onFetchModels: (supplier: Supplier) => void;
  onTestAll: (supplier: Supplier, keyId?: string) => void;
  onTestModelAll: (
    supplier: Supplier,
    model: string,
    protocols: Protocol[],
    keyId?: string,
  ) => Promise<void>;
}

export function SupplierPage({
  config,
  search,
  t,
  busy,
  onAdd,
  onEdit,
  onDelete,
  onProbe,
  onFetchModels,
  onTestAll,
  onTestModelAll,
}: SupplierPageProps) {
  const [detailId, setDetailId] = useState<string | null>(null);
  const [testKeyId, setTestKeyId] = useState<string | undefined>(undefined);
  const [view, setView] = useState<ViewMode>(initialView);
  const detail = config.suppliers.find((supplier) => supplier.id === detailId) ?? null;
  const testAccount = effectiveKey(detail ?? undefined, testKeyId);

  const protocolsFor = (model: SupplierModel): Protocol[] => {
    if (model.protocols.length) return model.protocols;
    if (detail?.protocols.length) return detail.protocols;
    return ["chat_completions", "responses", "messages"];
  };

  const changeView = (next: ViewMode) => {
    setView(next);
    localStorage.setItem(VIEW_STORAGE_KEY, next);
  };

  const suppliers = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return config.suppliers;
    return config.suppliers.filter((supplier) => {
      const haystack = [
        supplier.name,
        supplier.base_url,
        supplier.remark ?? "",
        ...supplier.models.map((model) => model.model),
      ]
        .join(" ")
        .toLowerCase();
      return haystack.includes(query);
    });
  }, [config.suppliers, search]);

  if (config.suppliers.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-border/40 bg-card/10 py-20">
        <Server className="size-9 text-muted-foreground/40" />
        <div className="text-center">
          <p className="text-sm font-bold text-foreground/85">{t.noSuppliers}</p>
          <p className="mt-1.5 text-xs text-muted-foreground">{t.addFirstSupplier}</p>
        </div>
        <Button onClick={onAdd} className="mt-3 h-8 rounded-full px-4 text-xs font-semibold">
          <Plus className="size-3.5" />
          {t.addSupplier}
        </Button>
      </div>
    );
  }

  const rowProps = (supplier: Supplier) => ({
    supplier,
    action: busy[supplier.id],
    t,
    onOpen: () => setDetailId(supplier.id),
    onProbe: () => onProbe(supplier),
    onFetchModels: () => onFetchModels(supplier),
    onTestAll: () => onTestAll(supplier),
    onEdit: () => onEdit(supplier),
    onDelete: () => onDelete(supplier),
  });

  return (
    <>
      <div className="mb-2.5 flex items-center justify-between">
        <span className="text-[10px] font-extrabold uppercase tracking-wider text-muted-foreground/70">
          {suppliers.length} {t.suppliersCount}
        </span>
        <div className="flex h-7 items-center rounded-lg border border-border/60 bg-muted/30 p-0.5">
          {(
            [
              { key: "list" as ViewMode, icon: List, label: t.viewList },
              { key: "cards" as ViewMode, icon: LayoutGrid, label: t.viewCards },
            ]
          ).map(({ key, icon: Icon, label }) => (
            <button
              key={key}
              onClick={() => changeView(key)}
              title={label}
              aria-label={label}
              aria-pressed={view === key}
              className={`flex h-full items-center gap-1 rounded-md px-2 text-[10px] font-bold transition-all ${
                view === key
                  ? "border border-border/10 bg-card/70 text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Icon className="size-3" />
              <span className="hidden sm:inline">{label}</span>
            </button>
          ))}
        </div>
      </div>

      {suppliers.length === 0 ? (
        <div className="rounded-xl border border-dashed border-border/40 bg-card/10 py-12 text-center text-xs text-muted-foreground">
          {t.emptySearch}
        </div>
      ) : view === "cards" ? (
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
          {suppliers.map((supplier) => (
            <SupplierCard key={supplier.id} {...rowProps(supplier)} />
          ))}
        </div>
      ) : (
        <div className="overflow-hidden rounded-xl">
          <table className="w-full text-left">
            <thead className="border-b border-border/30 bg-transparent text-[9px] uppercase tracking-wider text-muted-foreground">
              <tr>
                <th className="px-4 py-2 font-bold">{t.supplierName}</th>
                <th className="px-3 py-2 font-bold">{t.protocols}</th>
                <th className="px-3 py-2 text-center font-bold">{t.accounts}</th>
                <th className="px-3 py-2 text-center font-bold">{t.models}</th>
                <th className="px-3 py-2 text-right font-bold">{t.latency}</th>
                <th className="px-3 py-2 text-right font-bold">{t.actions}</th>
              </tr>
            </thead>
            <tbody>
              {suppliers.map((supplier) => (
                <SupplierListRow key={supplier.id} {...rowProps(supplier)} />
              ))}
            </tbody>
          </table>
        </div>
      )}

      <Dialog open={detail != null} onOpenChange={(open) => !open && setDetailId(null)}>
        <DialogContent className="flex max-h-[85vh] flex-col gap-0 overflow-hidden rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-3xl">
          <DialogHeader className="mb-4 shrink-0 gap-1.5 border-b border-border/30 pb-4">
            <DialogTitle className="flex items-center gap-2 text-base font-bold tracking-tight">
              <Server className="size-4 text-primary" />
              {detail?.name}
            </DialogTitle>
            <DialogDescription className="truncate font-mono text-[11px]">
              {detail?.base_url}
            </DialogDescription>
          </DialogHeader>

          {detail ? (
            <div className="min-h-0 overflow-y-auto pr-1">
              <div className="mb-3 space-y-2">
                <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                  <ProtocolBadges protocols={detail.protocols} />
                  <span className="whitespace-nowrap text-[10px] text-muted-foreground">
                    {detail.models.length} {t.modelsCount}
                  </span>
                  <span className="text-[10px] text-muted-foreground/40">·</span>
                  <span className="whitespace-nowrap text-[10px] text-muted-foreground">
                    {detail.keys.length} {t.accountsCount}
                  </span>
                </div>

                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex items-center gap-1">
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={Boolean(busy[detail.id])}
                      onClick={() => onProbe(detail)}
                    >
                      {busy[detail.id] === "probe" ? (
                        <Loader2 className="size-3 animate-spin" />
                      ) : (
                        <RefreshCw className="size-3" />
                      )}
                      {t.probe}
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={Boolean(busy[detail.id])}
                      onClick={() => onFetchModels(detail)}
                    >
                      {busy[detail.id] === "models" ? (
                        <Loader2 className="size-3 animate-spin" />
                      ) : (
                        <Download className="size-3" />
                      )}
                      {t.fetchModels}
                    </Button>
                  </div>

                  <div className="flex items-center gap-1.5">
                    <span className="whitespace-nowrap text-[9px] font-bold uppercase tracking-wider text-muted-foreground/70">
                      {t.testAccount}
                    </span>
                    <div className="w-24">
                      <CxcSelect
                        value={testAccount?.id ?? ""}
                        onValueChange={(value) => setTestKeyId(value)}
                        options={detail.keys.map((key) => ({
                          value: key.id,
                          label: key.name,
                        }))}
                        size="xs"
                      />
                    </div>
                    <Button
                      size="sm"
                      disabled={Boolean(busy[detail.id])}
                      onClick={() => onTestAll(detail, testAccount?.id)}
                    >
                      {busy[detail.id] === "test" ? (
                        <Loader2 className="size-3 animate-spin" />
                      ) : (
                        <Activity className="size-3" />
                      )}
                      {t.testAll}
                    </Button>
                  </div>
                </div>
              </div>

              {detail.models.length === 0 ? (
                <div className="rounded-xl border border-dashed border-border py-10 text-center text-xs text-muted-foreground">
                  {t.noModels}
                </div>
              ) : (
                <>
                  <div className="overflow-hidden rounded-xl border border-border/60">
                    <table className="w-full table-fixed text-left text-[11px]">
                      <thead className="bg-muted/30 text-[9px] uppercase tracking-wider text-muted-foreground">
                        <tr>
                          <th className="px-3 py-2 font-bold">{t.model}</th>
                          <th className="w-[152px] px-3 py-2 font-bold">{t.status}</th>
                          <th className="w-11 px-2 py-2 text-right font-bold">
                            <span className="sr-only">{t.test}</span>
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {detail.models.map((model) => (
                          <ModelRow
                            key={model.model}
                            model={model}
                            protocols={protocolsFor(model)}
                            busy={Boolean(busy[detail.id])}
                            keyName={
                              detail.keys.find((key) => key.id === model.tested_key_id)?.name
                            }
                            t={t}
                            onTestAll={() =>
                              onTestModelAll(
                                detail,
                                model.model,
                                protocolsFor(model),
                                testAccount?.id,
                              )
                            }
                          />
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              )}
            </div>
          ) : null}
        </DialogContent>
      </Dialog>
    </>
  );
}

function ModelRow({
  model,
  protocols,
  busy,
  keyName,
  t,
  onTestAll,
}: {
  model: SupplierModel;
  protocols: Protocol[];
  /** 中转站级操作（探测 / 拉取 / 全部测试）进行中时禁用本行。 */
  busy: boolean;
  keyName?: string;
  t: Messages;
  onTestAll: () => Promise<void>;
}) {
  // 测试状态按行独立：只让被点的那一行转圈。
  const [testing, setTesting] = useState(false);
  const locked = busy || testing;

  const runTest = async () => {
    if (locked) return;
    setTesting(true);
    try {
      await onTestAll();
    } finally {
      setTesting(false);
    }
  };

  return (
    <tr className="border-t border-border/40 align-middle">
      <td className="px-3 py-2">
        <span
          className="block truncate font-mono text-[10.5px] text-foreground/85"
          title={model.model}
        >
          {model.model}
        </span>
        <div className="mt-1">
          <ProtocolBadges protocols={protocols} compact />
        </div>
      </td>
      <td className="px-3 py-2">
        <div
          className="flex items-center gap-1 whitespace-nowrap"
          title={model.last_ok === false ? model.last_error : undefined}
        >
          {model.last_ok == null ? (
            <AlertCircle className="size-3 shrink-0 text-muted-foreground/40" />
          ) : model.last_ok ? (
            <CheckCircle2 className="size-3 shrink-0 text-sticker-green" />
          ) : (
            <XCircle className="size-3 shrink-0 text-red-500" />
          )}
          <span className="text-[10.5px] text-muted-foreground">
            {model.last_ok == null ? t.never : model.last_ok ? t.success : t.failed}
          </span>
          {model.last_ok === true && model.latency_ms != null ? (
            <span
              className={`font-mono text-[10px] font-bold tabular-nums ${latencyClass(model.latency_ms)}`}
            >
              {model.latency_ms} ms
            </span>
          ) : null}
        </div>
        {keyName ? (
          <span className="mt-0.5 block truncate text-[9px] text-muted-foreground/60">
            {keyName}
          </span>
        ) : null}
      </td>
      <td className="px-2 py-2 text-right">
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={locked}
          onClick={() => void runTest()}
          title={t.testThisModel}
        >
          {testing ? (
            <Loader2 className="size-3 animate-spin" />
          ) : (
            <Activity className="size-3" />
          )}
        </Button>
      </td>
    </tr>
  );
}
