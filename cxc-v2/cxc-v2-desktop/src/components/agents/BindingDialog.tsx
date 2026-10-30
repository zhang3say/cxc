import { useEffect, useMemo, useState } from "react";
import { Activity, Loader2 } from "lucide-react";
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
import { Label } from "@/components/ui/label";
import { CxcSelect } from "@/components/ui/select";
import { errorText } from "@/lib/utils";
import type { Messages } from "@/lib/i18n";
import type {
  AgentConfig,
  AgentConfigInput,
  AgentKind,
  Config,
  Env,
  GrokWebSearch,
  ModelTestResult,
  Protocol,
} from "@/lib/types";
import {
  AGENT_LABELS,
  CAPABILITIES,
  ENV_LABELS,
  PROTOCOL_FULL_LABELS,
  REASONING_OPTIONS,
} from "@/lib/types";

interface BindingDialogProps {
  open: boolean;
  agent: AgentKind;
  env: Env;
  binding: AgentConfig | null;
  suppliers: Config["suppliers"];
  isActive: boolean;
  t: Messages;
  saving: boolean;
  onClose: () => void;
  onSubmit: (input: AgentConfigInput, activate: boolean) => Promise<void>;
  onTest: (
    supplierId: string,
    model: string,
    protocol: Protocol,
  ) => Promise<ModelTestResult>;
}

export function BindingDialog({
  open,
  agent,
  env,
  binding,
  suppliers,
  isActive,
  t,
  saving,
  onClose,
  onSubmit,
  onTest,
}: BindingDialogProps) {
  const capabilities = CAPABILITIES[agent];
  const [protocol, setProtocol] = useState<Protocol>(capabilities.protocols[0]);
  const [supplierId, setSupplierId] = useState("");
  const [model, setModel] = useState("");
  const [reasoning, setReasoning] = useState("");
  const [planReasoning, setPlanReasoning] = useState("");
  const [context, setContext] = useState("");
  const [compact, setCompact] = useState("");
  const [opus, setOpus] = useState("");
  const [sonnet, setSonnet] = useState("");
  const [haiku, setHaiku] = useState("");
  const [subagent, setSubagent] = useState("");
  const [webSearchMode, setWebSearchMode] = useState<GrokWebSearch["mode"]>("follow");
  const [webSearchModel, setWebSearchModel] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ModelTestResult | null>(null);
  const [initializedKey, setInitializedKey] = useState<string | null>(null);

  const compatibleSuppliers = useMemo(
    () => suppliers.filter((supplier) => supplier.protocols.includes(protocol)),
    [suppliers, protocol],
  );
  const supplier = suppliers.find((item) => item.id === supplierId);

  useEffect(() => {
    if (!open) {
      setInitializedKey(null);
      return;
    }
    const key = `${agent}:${env}:${binding?.id ?? "new"}`;
    if (initializedKey === key) return;

    const initialProtocol = binding?.protocol ?? capabilities.protocols[0];
    const initialSupplier =
      suppliers.find((item) => item.id === binding?.supplier_id) ??
      suppliers.find((item) => item.protocols.includes(initialProtocol));

    setProtocol(initialProtocol);
    setSupplierId(initialSupplier?.id ?? "");
    setModel(binding?.model ?? initialSupplier?.models[0]?.model ?? "");
    setReasoning(binding?.reasoning_effort ?? "");
    setPlanReasoning(binding?.plan_reasoning_effort ?? "");
    setContext(binding?.context_length ?? "");
    setCompact(binding?.compact_ratio != null ? String(binding.compact_ratio) : "");
    setOpus(binding?.claude_models?.opus ?? "");
    setSonnet(binding?.claude_models?.sonnet ?? "");
    setHaiku(binding?.claude_models?.haiku ?? "");
    setSubagent(binding?.claude_models?.subagent ?? "");
    setWebSearchMode(binding?.grok_web_search?.mode ?? "follow");
    setWebSearchModel(
      binding?.grok_web_search?.mode === "model" ? binding.grok_web_search.model : "",
    );
    setError(null);
    setTestResult(null);
    setInitializedKey(key);
  }, [open, agent, env, binding, suppliers, capabilities.protocols, initializedKey]);

  const chooseProtocol = (next: Protocol) => {
    setProtocol(next);
    if (!supplier?.protocols.includes(next)) {
      const fallback = suppliers.find((item) => item.protocols.includes(next));
      setSupplierId(fallback?.id ?? "");
      if (fallback && !fallback.models.some((item) => item.model === model)) {
        setModel(fallback.models[0]?.model ?? "");
      }
    }
  };

  const chooseSupplier = (nextId: string) => {
    setSupplierId(nextId);
    const next = suppliers.find((item) => item.id === nextId);
    if (next && !next.models.some((item) => item.model === model)) {
      setModel(next.models[0]?.model ?? "");
    }
  };

  const buildInput = (): AgentConfigInput | null => {
    if (!supplierId) {
      setError(t.selectSupplier);
      return null;
    }
    if (!supplier?.protocols.includes(protocol)) {
      setError(t.protocolMissing);
      return null;
    }
    if (!model.trim()) {
      setError(t.selectModel);
      return null;
    }
    const compactValue = compact.trim() ? Number(compact) : undefined;
    if (
      compactValue != null &&
      (Number.isNaN(compactValue) || compactValue < 70 || compactValue > 100)
    ) {
      setError(t.compactRange);
      return null;
    }
    setError(null);
    return {
      supplier_id: supplierId,
      protocol,
      model: model.trim(),
      reasoning_effort: reasoning || undefined,
      plan_reasoning_effort: capabilities.plan_reasoning_effort
        ? planReasoning || undefined
        : undefined,
      context_length: context.trim() || undefined,
      compact_ratio: compactValue,
      claude_models: capabilities.claude_models
        ? {
            opus: opus.trim() || undefined,
            sonnet: sonnet.trim() || undefined,
            haiku: haiku.trim() || undefined,
            subagent: subagent.trim() || undefined,
          }
        : undefined,
      grok_web_search: capabilities.grok_web_search
        ? webSearchMode === "model"
          ? { mode: "model", model: webSearchModel.trim() }
          : { mode: webSearchMode }
        : undefined,
    };
  };

  const submit = async (activate: boolean) => {
    const input = buildInput();
    if (!input) return;
    try {
      await onSubmit(input, activate);
    } catch (submitError) {
      setError(errorText(submitError));
    }
  };

  const runTest = async () => {
    if (!supplierId || !model.trim()) return;
    setTesting(true);
    setTestResult(null);
    try {
      setTestResult(await onTest(supplierId, model.trim(), protocol));
    } catch (testError) {
      setTestResult({ ok: false, latency_ms: 0, error: errorText(testError) });
    } finally {
      setTesting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="flex max-h-[88vh] flex-col gap-0 overflow-hidden rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-lg">
        <DialogHeader className="mb-4 shrink-0 gap-1.5 border-b border-border/30 pb-4">
          <DialogTitle className="text-base font-bold tracking-tight">
            {binding ? t.editBinding : t.newBinding}
          </DialogTitle>
          <DialogDescription className="text-xs text-muted-foreground">
            {AGENT_LABELS[agent]} · {ENV_LABELS[env]}
            {isActive ? ` · ${t.inUse}` : ""}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 space-y-4 overflow-y-auto pr-1">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                {t.protocols}
              </Label>
              <CxcSelect
                value={protocol}
                onValueChange={(value) => chooseProtocol(value as Protocol)}
                options={capabilities.protocols.map((item) => ({
                  value: item,
                  label: PROTOCOL_FULL_LABELS[item],
                }))}
              />
            </div>
            <div className="space-y-1.5">
              <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                {t.selectSupplier}
              </Label>
              <CxcSelect
                value={supplierId}
                onValueChange={chooseSupplier}
                placeholder={t.selectSupplier}
                options={compatibleSuppliers.map((item) => ({
                  value: item.id,
                  label: item.name,
                }))}
              />
            </div>
          </div>

          {compatibleSuppliers.length === 0 ? (
            <p className="rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-[11px] text-amber-700">
              {t.protocolMissing}
            </p>
          ) : null}

          <div className="space-y-1.5">
            <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
              {t.model}
            </Label>
            <Input
              value={model}
              onChange={(event) => setModel(event.target.value)}
              placeholder={t.manualModel}
              className="h-9 rounded-md border-border/30 bg-muted/10 font-mono text-xs focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
            />
            {supplier?.models.length ? (
              <div className="flex flex-wrap gap-1 pt-1">
                {supplier.models.slice(0, 14).map((entry) => (
                  <button
                    key={entry.model}
                    type="button"
                    onClick={() => setModel(entry.model)}
                    className={`rounded border px-1.5 py-0.5 font-mono text-[9px] transition-colors ${
                      model === entry.model
                        ? "border-primary/40 bg-primary/10 text-primary"
                        : "border-border/40 bg-muted/20 text-muted-foreground hover:text-foreground"
                    }`}
                  >
                    {entry.model}
                  </button>
                ))}
              </div>
            ) : null}
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                {t.reasoningEffort}
              </Label>
              <CxcSelect
                value={reasoning}
                onValueChange={setReasoning}
                options={[
                  { value: "", label: t.none },
                  ...REASONING_OPTIONS[agent].map((item) => ({ value: item, label: item })),
                ]}
              />
            </div>
            {capabilities.plan_reasoning_effort ? (
              <div className="space-y-1.5">
                <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                  {t.planReasoningEffort}
                </Label>
                <CxcSelect
                  value={planReasoning}
                  onValueChange={setPlanReasoning}
                  options={[
                    { value: "", label: t.none },
                    ...REASONING_OPTIONS[agent].map((item) => ({ value: item, label: item })),
                  ]}
                />
              </div>
            ) : null}
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                {t.contextLength}
              </Label>
              {agent === "claude" ? (
                <CxcSelect
                  value={context}
                  onValueChange={setContext}
                  options={[
                    { value: "", label: t.none },
                    { value: "1m", label: "1m" },
                  ]}
                />
              ) : (
                <Input
                  value={context}
                  onChange={(event) => setContext(event.target.value)}
                  placeholder="200k / 500k / 1m"
                  className="h-9 rounded-md border-border/30 bg-muted/10 font-mono text-xs focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
                />
              )}
            </div>
            <div className="space-y-1.5">
              <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                {t.compactRatio}
              </Label>
              <Input
                type="number"
                min={70}
                max={100}
                value={compact}
                onChange={(event) => setCompact(event.target.value)}
                placeholder="70–100"
                className="h-9 rounded-md border-border/30 bg-muted/10 font-mono text-xs focus:border-primary/60 focus:bg-background/80 focus-visible:ring-primary/40"
              />
            </div>
          </div>

          {capabilities.claude_models ? (
            <div className="space-y-2 rounded-lg border border-border/40 bg-muted/10 p-3">
              <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                {t.claudeModels}
              </Label>
              <div className="grid grid-cols-2 gap-2">
                {[
                  { label: t.opus, value: opus, set: setOpus },
                  { label: t.sonnet, value: sonnet, set: setSonnet },
                  { label: t.haiku, value: haiku, set: setHaiku },
                  { label: t.subagent, value: subagent, set: setSubagent },
                ].map((item) => (
                  <Input
                    key={item.label}
                    value={item.value}
                    onChange={(event) => item.set(event.target.value)}
                    placeholder={item.label}
                    className="h-8 rounded-md border-border/30 bg-card font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
                  />
                ))}
              </div>
              <p className="text-[10px] text-muted-foreground/70">{t.claudeModelsHint}</p>
            </div>
          ) : null}

          {capabilities.grok_web_search ? (
            <div className="space-y-2 rounded-lg border border-border/40 bg-muted/10 p-3">
              <Label className="text-[11px] font-extrabold uppercase tracking-wider text-muted-foreground/80">
                {t.webSearch}
              </Label>
              <CxcSelect
                value={webSearchMode}
                onValueChange={(value) => setWebSearchMode(value as GrokWebSearch["mode"])}
                options={[
                  { value: "follow", label: t.webSearchFollow },
                  { value: "keep", label: t.webSearchKeep },
                  { value: "unset", label: t.webSearchUnset },
                  { value: "model", label: t.webSearchModel },
                ]}
              />
              {webSearchMode === "model" ? (
                <Input
                  value={webSearchModel}
                  onChange={(event) => setWebSearchModel(event.target.value)}
                  placeholder={t.model}
                  className="h-8 rounded-md border-border/30 bg-card font-mono text-[10px] focus:border-primary/60 focus-visible:ring-primary/40"
                />
              ) : null}
            </div>
          ) : null}

          {testResult ? (
            <div
              className={`rounded-md border px-3 py-2 text-[11px] ${
                testResult.ok
                  ? "border-emerald-200 bg-emerald-50 text-emerald-700"
                  : "border-red-200 bg-red-50 text-red-600"
              }`}
            >
              {testResult.ok ? t.success : t.failed} · {testResult.latency_ms} ms
              {testResult.error ? ` · ${testResult.error}` : ""}
            </div>
          ) : null}

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
            onClick={() => void runTest()}
            disabled={testing || saving || !supplierId || !model.trim()}
          >
            {testing ? <Loader2 className="size-3 animate-spin" /> : <Activity className="size-3" />}
            {t.test}
          </Button>
          <Button size="sm" onClick={() => void submit(false)} disabled={saving}>
            {saving ? <Loader2 className="size-3 animate-spin" /> : null}
            {saving ? t.saving : t.save}
          </Button>
          {!binding ? (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void submit(true)}
              disabled={saving}
            >
              {t.saveAndActivate}
            </Button>
          ) : null}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
