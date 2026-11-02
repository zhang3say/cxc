import { useEffect, useState } from "react";
import { Check, Loader2, Plus, RefreshCw, Star, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { errorText } from "@/lib/utils";
import { Label } from "@/components/ui/label";
import type { Messages } from "@/lib/i18n";
import type { Protocol, Supplier, SupplierInput, SupplierKeyInput } from "@/lib/types";
import { PROTOCOL_FULL_LABELS } from "@/lib/types";

interface SupplierDialogProps {
  open: boolean;
  supplier: Supplier | null;
  t: Messages;
  saving: boolean;
  onClose: () => void;
  onSubmit: (input: SupplierInput, probeAfterSave: boolean) => Promise<void>;
}

const protocols: Protocol[] = ["chat_completions", "responses", "messages"];

interface KeyDraft {
  id?: string;
  name: string;
  apiKey: string;
  remark: string;
  /** 已有账号的 Key 掩码，用于提示「留空保留」。 */
  masked: string;
}

function maskKey(key: string): string {
  const trimmed = key.trim();
  if (trimmed.length <= 8) return "••••";
  return `${trimmed.slice(0, 4)}…${trimmed.slice(-4)}`;
}

export function SupplierDialog({
  open,
  supplier,
  t,
  saving,
  onClose,
  onSubmit,
}: SupplierDialogProps) {
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [remark, setRemark] = useState("");
  const [selected, setSelected] = useState<Protocol[]>([]);
  const [keys, setKeys] = useState<KeyDraft[]>([]);
  const [defaultKeyId, setDefaultKeyId] = useState<string | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setName(supplier?.name ?? "");
    setBaseUrl(supplier?.base_url ?? "");
    setRemark(supplier?.remark ?? "");
    setSelected(supplier?.protocols ?? []);
    setKeys(
      supplier && supplier.keys.length
        ? supplier.keys.map((key) => ({
            id: key.id,
            name: key.name,
            apiKey: "",
            remark: key.remark ?? "",
            masked: maskKey(key.api_key),
          }))
        : [{ name: "默认", apiKey: "", remark: "", masked: "" }],
    );
    setDefaultKeyId(supplier?.default_key_id ?? undefined);
    setError(null);
  }, [open, supplier]);

  const toggle = (protocol: Protocol) => {
    setSelected((current) =>
      current.includes(protocol)
        ? current.filter((item) => item !== protocol)
        : [...current, protocol],
    );
  };

  const updateKey = (index: number, patch: Partial<KeyDraft>) => {
    setKeys((current) =>
      current.map((key, position) =>
        position === index ? { ...key, ...patch } : key,
      ),
    );
  };

  const addKey = () => {
    setKeys((current) => [
      ...current,
      { name: "", apiKey: "", remark: "", masked: "" },
    ]);
  };

  const removeKey = (index: number) => {
    setKeys((current) => {
      const next = current.filter((_, position) => position !== index);
      if (defaultKeyId && !next.some((key) => key.id === defaultKeyId)) {
        setDefaultKeyId(next[0]?.id);
      }
      return next;
    });
  };

  const submit = async (probeAfterSave: boolean) => {
    if (!name.trim()) {
      setError(t.supplierName);
      return;
    }
    if (!baseUrl.trim()) {
      setError(t.baseUrl);
      return;
    }
    if (keys.length === 0) {
      setError(t.atLeastOneKey);
      return;
    }
    const payloadKeys: SupplierKeyInput[] = keys.map((key) => ({
      id: key.id,
      name: key.name.trim(),
      api_key: key.apiKey.trim(),
      remark: key.remark.trim() || undefined,
    }));
    if (payloadKeys.some((key) => !key.name)) {
      setError(t.keyNameRequired);
      return;
    }
    if (payloadKeys.some((key, index) => !key.api_key && !keys[index].masked)) {
      setError(t.apiKey);
      return;
    }
    setError(null);
    try {
      await onSubmit(
        {
          name: name.trim(),
          base_url: baseUrl.trim(),
          remark: remark.trim() || undefined,
          protocols: selected,
          keys: payloadKeys,
          default_key_id: defaultKeyId,
        },
        probeAfterSave,
      );
    } catch (submitError) {
      setError(errorText(submitError));
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="flex max-h-[88vh] flex-col gap-0 overflow-hidden rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-lg">
        <DialogHeader className="mb-4 shrink-0 gap-1.5 border-b border-border/30 pb-4">
          <DialogTitle className="text-base font-bold tracking-tight">
            {supplier ? t.editSupplier : t.addSupplier}
          </DialogTitle>
          <DialogDescription className="text-xs text-muted-foreground">
            {t.supplierDialogHint}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 space-y-4 overflow-y-auto pr-1">
          <div className="space-y-1.5">
            <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              {t.supplierName}
            </Label>
            <Input
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="openrouter"
              className="h-9 rounded-md border-border/30 bg-muted/10 text-xs focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
            />
          </div>

          <div className="space-y-1.5">
            <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              {t.baseUrl}
            </Label>
            <Input
              value={baseUrl}
              onChange={(event) => setBaseUrl(event.target.value)}
              placeholder="https://api.example.com/v1"
              className="h-9 rounded-md border-border/30 bg-muted/10 font-mono text-xs focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
            />
          </div>

          <div className="space-y-1.5">
            <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              {t.remark}
            </Label>
            <Input
              value={remark}
              onChange={(event) => setRemark(event.target.value)}
              placeholder={t.remark}
              className="h-9 rounded-md border-border/30 bg-muted/10 text-xs focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
            />
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                {t.accounts} {keys.length}
              </Label>
              <Button size="xs" variant="outline" onClick={addKey} disabled={saving}>
                <Plus className="size-3" />
                {t.addAccount}
              </Button>
            </div>

            <div className="space-y-2">
              {keys.map((key, index) => {
                const isDefault = key.id != null && key.id === defaultKeyId;
                return (
                  <div
                    key={key.id ?? `new-${index}`}
                    data-account-index={index}
                    className="rounded-lg border border-border/40 bg-muted/10 p-2.5"
                  >
                    <div className="flex items-center gap-1.5">
                      <Input
                        value={key.name}
                        onChange={(event) =>
                          updateKey(index, { name: event.target.value })
                        }
                        placeholder={t.accountName}
                        className="h-8 flex-1 rounded-md border-border/30 bg-card text-[11px] focus:border-primary/60 focus-visible:ring-primary/40"
                      />
                      {isDefault ? (
                        <span className="flex items-center gap-1 rounded border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-primary">
                          <Check className="size-2.5" />
                          {t.defaultAccount}
                        </span>
                      ) : (
                        <Button
                          size="xs"
                          variant="ghost"
                          disabled={saving || key.id == null}
                          onClick={() => setDefaultKeyId(key.id)}
                          title={t.setDefault}
                        >
                          <Star className="size-3" />
                          {t.setDefault}
                        </Button>
                      )}
                      <Button
                        size="icon-sm"
                        variant="ghost"
                        disabled={saving || keys.length <= 1}
                        onClick={() => removeKey(index)}
                        className="text-red-500 hover:bg-red-500/10 hover:text-red-600"
                        title={t.delete}
                      >
                        <Trash2 className="size-3" />
                      </Button>
                    </div>

                    <div className="mt-1.5 flex items-center gap-1.5">
                      <Input
                        type="password"
                        value={key.apiKey}
                        onChange={(event) =>
                          updateKey(index, { apiKey: event.target.value })
                        }
                        placeholder={
                          key.masked ? `${key.masked}（${t.keepKeyHint}）` : "sk-..."
                        }
                        className="h-8 flex-1 rounded-md border-border/30 bg-card font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
                      />
                      <Input
                        value={key.remark}
                        onChange={(event) =>
                          updateKey(index, { remark: event.target.value })
                        }
                        placeholder={t.remark}
                        className="h-8 w-32 rounded-md border-border/30 bg-card text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="space-y-2">
            <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              {t.protocols}
            </Label>
            <div className="grid gap-2">
              {protocols.map((protocol) => {
                const active = selected.includes(protocol);
                return (
                  <button
                    key={protocol}
                    type="button"
                    onClick={() => toggle(protocol)}
                    className={`flex items-center justify-between rounded-lg border px-3 py-2 text-left transition-all ${
                      active
                        ? "border-primary/50 bg-primary/[0.04] text-primary"
                        : "border-border/40 bg-muted/10 text-foreground hover:bg-muted/20"
                    }`}
                  >
                    <span className="text-xs font-semibold">
                      {PROTOCOL_FULL_LABELS[protocol]}
                    </span>
                    <span
                      className={`size-2 rounded-full ${
                        active ? "bg-primary" : "bg-muted-foreground/25"
                      }`}
                    />
                  </button>
                );
              })}
            </div>
            <p className="text-[10px] leading-normal text-muted-foreground/70">
              {t.protocolHint}
            </p>
          </div>

        </div>

        {error ? (
          <p className="mt-3 shrink-0 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-[11px] text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400">
            {error}
          </p>
        ) : null}

        <DialogFooter className="mt-4 shrink-0 gap-2 border-t border-border/30 pt-4">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={saving}>
            {t.cancel}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => void submit(true)}
            disabled={saving}
          >
            {saving ? (
              <Loader2 className="size-3 animate-spin" />
            ) : (
              <RefreshCw className="size-3" />
            )}
            {t.save} + {t.probe}
          </Button>
          <Button size="sm" onClick={() => void submit(false)} disabled={saving}>
            {saving ? <Loader2 className="size-3 animate-spin" /> : null}
            {saving ? t.saving : t.save}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
