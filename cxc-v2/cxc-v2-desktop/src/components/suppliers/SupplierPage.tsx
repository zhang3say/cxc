import { useMemo, useState } from "react";
import {
  Activity,
  AlertCircle,
  CheckCircle2,
  Download,
  Edit2,
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
  const colors: Record<Protocol, string> = {
    chat_completions: "border-sky-200 bg-sky-50 text-sky-700",
    responses: "border-purple-200 bg-purple-50 text-purple-700",
    messages: "border-orange-200 bg-orange-50 text-orange-700",
  };
  return (
    <div className="flex flex-wrap items-center gap-1">
      {protocols.map((protocol) => (
        <span
          key={protocol}
          className={`rounded border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide ${colors[protocol]} ${compact ? "" : "px-2"}`}
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

function formatTime(value?: string) {
  if (!value) return "—";
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
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
  onTestModel: (
    supplier: Supplier,
    model: string,
    protocol: Protocol,
    keyId?: string,
  ) => void;
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
  onTestModel,
}: SupplierPageProps) {
  const [detailId, setDetailId] = useState<string | null>(null);
  const [testKeyId, setTestKeyId] = useState<string | undefined>(undefined);
  const detail = config.suppliers.find((supplier) => supplier.id === detailId) ?? null;
  const testAccount = effectiveKey(detail ?? undefined, testKeyId);

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
      <div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-border bg-card py-20">
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

  return (
    <>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        {suppliers.map((supplier) => {
          const action = busy[supplier.id];
          const status = statusOf(supplier);
          const lastLatency = supplier.models.find((model) => model.latency_ms != null)?.latency_ms;
          return (
            <div
              key={supplier.id}
              className="group flex cursor-pointer flex-col overflow-hidden rounded-xl border border-border/70 bg-card shadow-[0_1px_2px_rgba(0,0,0,0.02)] transition-all hover:border-border hover:shadow-sm"
              onClick={() => setDetailId(supplier.id)}
            >
              <div className="flex items-start justify-between gap-3 p-4 pb-3">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span
                      className={`size-2 shrink-0 rounded-full ${
                        status === "ok"
                          ? "bg-sticker-green shadow-[0_0_6px_rgba(26,174,57,0.35)]"
                          : status === "fail"
                            ? "bg-red-500"
                            : "bg-muted-foreground/30"
                      }`}
                    />
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
                  <span className={`font-bold ${latencyClass(lastLatency)}`}>
                    {lastLatency != null ? `${lastLatency} ms` : t.never}
                  </span>
                </div>
                <div className="flex items-center justify-between text-[10px] text-muted-foreground/60">
                  <span>{t.lastProbe}</span>
                  <span>{formatTime(supplier.last_probe)}</span>
                </div>
              </div>

              <div
                className="mt-auto flex items-center justify-between border-t border-border/50 bg-background/40 px-3 py-2"
                onClick={(event) => event.stopPropagation()}
              >
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    disabled={Boolean(action)}
                    onClick={() => onProbe(supplier)}
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
                    onClick={() => onFetchModels(supplier)}
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
                    onClick={() => onTestAll(supplier)}
                    title={t.testAll}
                  >
                    {action === "test" ? (
                      <Loader2 className="size-3 animate-spin" />
                    ) : (
                      <Activity className="size-3" />
                    )}
                  </Button>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    disabled={Boolean(action)}
                    onClick={() => onEdit(supplier)}
                    title={t.editSupplier}
                  >
                    <Edit2 className="size-3" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    disabled={Boolean(action)}
                    onClick={() => onDelete(supplier)}
                    className="text-red-500 hover:bg-red-500/10 hover:text-red-600"
                    title={t.delete}
                  >
                    <Trash2 className="size-3" />
                  </Button>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {suppliers.length === 0 ? (
        <div className="mt-6 rounded-xl border border-dashed border-border bg-card py-12 text-center text-xs text-muted-foreground">
          {t.emptySearch}
        </div>
      ) : null}

      <Dialog open={detail != null} onOpenChange={(open) => !open && setDetailId(null)}>
        <DialogContent className="flex max-h-[85vh] flex-col gap-0 overflow-hidden rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-2xl">
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
              <div className="mb-3 flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <ProtocolBadges protocols={detail.protocols} />
                  <span className="text-[10px] text-muted-foreground">
                    {detail.models.length} {t.modelsCount}
                  </span>
                </div>
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
                  <div className="flex items-center gap-1.5">
                    <span className="text-[9px] font-bold uppercase tracking-wider text-muted-foreground/70">
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

              {detail.models.length === 0 ? (
                <div className="rounded-xl border border-dashed border-border py-10 text-center text-xs text-muted-foreground">
                  {t.noModels}
                </div>
              ) : (
                <div className="overflow-hidden rounded-xl border border-border/60">
                  <table className="w-full text-left text-[11px]">
                    <thead className="bg-muted/30 text-[9px] uppercase tracking-wider text-muted-foreground">
                      <tr>
                        <th className="px-3 py-2 font-bold">{t.model}</th>
                        <th className="px-3 py-2 font-bold">{t.protocols}</th>
                        <th className="px-3 py-2 font-bold">{t.latency}</th>
                        <th className="px-3 py-2 font-bold">{t.status}</th>
                        <th className="px-3 py-2 text-right font-bold">{t.test}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {detail.models.map((model) => (
                        <ModelRow
                          key={model.model}
                          model={model}
                          protocols={
                            detail.protocols.length ? detail.protocols : (["chat_completions", "responses", "messages"] as Protocol[])
                          }
                          busy={Boolean(busy[detail.id])}
                          keyName={
                            detail.keys.find((key) => key.id === model.tested_key_id)
                              ?.name
                          }
                          t={t}
                          onTest={(protocol) =>
                            onTestModel(detail, model.model, protocol, testAccount?.id)
                          }
                        />
                      ))}
                    </tbody>
                  </table>
                </div>
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
  onTest,
}: {
  model: SupplierModel;
  protocols: Protocol[];
  busy: boolean;
  keyName?: string;
  t: Messages;
  onTest: (protocol: Protocol) => void;
}) {
  return (
    <tr className="border-t border-border/40 align-top">
      <td className="max-w-56 px-3 py-2 font-mono text-[10.5px] text-foreground/85">
        <span className="line-clamp-2">{model.model}</span>
      </td>
      <td className="px-3 py-2">
        <ProtocolBadges protocols={model.protocols} compact />
      </td>
      <td className={`px-3 py-2 font-bold ${latencyClass(model.latency_ms)}`}>
        {model.latency_ms != null ? `${model.latency_ms} ms` : "—"}
      </td>
      <td className="px-3 py-2">
        {model.last_ok == null ? (
          <span className="flex items-center gap-1 text-muted-foreground/50">
            <AlertCircle className="size-3" />
            {t.never}
          </span>
        ) : model.last_ok ? (
          <span className="flex items-center gap-1 text-sticker-green">
            <CheckCircle2 className="size-3" />
            {t.success}
          </span>
        ) : (
          <span className="flex items-center gap-1 text-red-500" title={model.last_error}>
            <XCircle className="size-3" />
            {t.failed}
          </span>
        )}
        {keyName ? (
          <span className="mt-0.5 block text-[9px] text-muted-foreground/60">
            {keyName}
          </span>
        ) : null}
      </td>
      <td className="px-3 py-2">
        <div className="flex flex-wrap justify-end gap-1">
          {protocols.map((protocol) => (
            <Button
              key={protocol}
              size="xs"
              variant="outline"
              disabled={busy}
              onClick={() => onTest(protocol)}
              title={`${t.test} ${PROTOCOL_LABELS[protocol]}`}
            >
              {PROTOCOL_LABELS[protocol]}
            </Button>
          ))}
        </div>
      </td>
    </tr>
  );
}
