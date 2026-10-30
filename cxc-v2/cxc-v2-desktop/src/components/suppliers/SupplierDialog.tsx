import { useEffect, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
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
import type { Protocol, Supplier, SupplierInput } from "@/lib/types";
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
  const [apiKey, setApiKey] = useState("");
  const [remark, setRemark] = useState("");
  const [selected, setSelected] = useState<Protocol[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setName(supplier?.name ?? "");
    setBaseUrl(supplier?.base_url ?? "");
    setApiKey("");
    setRemark(supplier?.remark ?? "");
    setSelected(supplier?.protocols ?? []);
    setError(null);
  }, [open, supplier]);

  const toggle = (protocol: Protocol) => {
    setSelected((current) =>
      current.includes(protocol)
        ? current.filter((item) => item !== protocol)
        : [...current, protocol],
    );
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
    if (!supplier && !apiKey.trim()) {
      setError(t.apiKey);
      return;
    }
    setError(null);
    try {
      await onSubmit(
        {
          name: name.trim(),
          base_url: baseUrl.trim(),
          api_key: apiKey.trim(),
          remark: remark.trim() || undefined,
          protocols: selected,
        },
        probeAfterSave,
      );
    } catch (submitError) {
      setError(errorText(submitError));
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="flex max-h-[85vh] flex-col gap-0 overflow-hidden rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-md">
        <DialogHeader className="mb-4 shrink-0 gap-1.5 border-b border-border/30 pb-4">
          <DialogTitle className="text-base font-bold tracking-tight">
            {supplier ? t.editSupplier : t.addSupplier}
          </DialogTitle>
          <DialogDescription className="text-xs text-muted-foreground">
            {t.baseUrl}
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
              {t.apiKey}
            </Label>
            <Input
              type="password"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
              placeholder={supplier ? "••••••••（留空保留原密钥）" : "sk-..."}
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
                    <span className="text-xs font-semibold">{PROTOCOL_FULL_LABELS[protocol]}</span>
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
              保存后可以点「探测协议」自动识别；手选会保留。
            </p>
          </div>

          {error ? (
            <p className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-[11px] text-red-600">
              {error}
            </p>
          ) : null}
        </div>

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
            {saving ? <Loader2 className="size-3 animate-spin" /> : <RefreshCw className="size-3" />}
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
