import { useMemo, useState } from "react";
import { Activity, Loader2, RefreshCw, Save, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
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
  DetectedAgent,
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
  PROTOCOL_LABELS,
  REASONING_OPTIONS,
} from "@/lib/types";

interface AgentConfigCardProps {
  agent: AgentKind;
  env: Env;
  config: AgentConfig | undefined;
  suppliers: Config["suppliers"];
  detection: DetectedAgent | undefined;
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

export function AgentConfigCard({
  agent,
  env,
  config,
  suppliers,
  detection,
  t,
  busy,
  onSave,
  onRewrite,
  onClear,
  onTest,
}: AgentConfigCardProps) {
  const capabilities = CAPABILITIES[agent];
  const [protocol, setProtocol] = useState<Protocol>(
    config?.protocol ?? capabilities.protocols[0],
  );
  const [supplierId, setSupplierId] = useState(config?.supplier_id ?? "");
  const [model, setModel] = useState(config?.model ?? "");
  const [reasoning, setReasoning] = useState(config?.reasoning_effort ?? "");
  const [planReasoning, setPlanReasoning] = useState(
    config?.plan_reasoning_effort ?? "",
  );
  const [context, setContext] = useState(config?.context_length ?? "");
  const [compact, setCompact] = useState(
    config?.compact_ratio != null ? String(config.compact_ratio) : "",
  );
  const [opus, setOpus] = useState(config?.claude_models?.opus ?? "");
  const [sonnet, setSonnet] = useState(config?.claude_models?.sonnet ?? "");
  const [haiku, setHaiku] = useState(config?.claude_models?.haiku ?? "");
  const [subagent, setSubagent] = useState(config?.claude_models?.subagent ?? "");
  const [webSearchMode, setWebSearchMode] = useState<GrokWebSearch["mode"]>(
    config?.grok_web_search?.mode ?? "follow",
  );
  const [webSearchModel, setWebSearchModel] = useState(
    config?.grok_web_search?.mode === "model" ? config.grok_web_search.model : "",
  );
  const [error, setError] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ModelTestResult | null>(null);

  const compatibleSuppliers = useMemo(
    () => suppliers.filter((supplier) => supplier.protocols.includes(protocol)),
    [suppliers, protocol],
  );
  const supplier = suppliers.find((item) => item.id === supplierId);
  const supplierName = supplier?.name;

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
      setError("压缩比例需要是 70–100 的整数");
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

  const submit = async () => {
    const input = buildInput();
    if (!input) return;
    try {
      await onSave(input);
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

  const title = model.trim() || t.notConfigured;
  const subtitle = supplierName
    ? `${supplierName} · ${PROTOCOL_LABELS[protocol]}`
    : t.selectSupplier;

  return (
    <div className="overflow-hidden rounded-xl border border-border/70 bg-card shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
      <div className="flex flex-wrap items-start justify-between gap-3 border-b border-border/40 px-4 py-3.5">
        <div className="min-w-0">
          <h2 className="truncate text-sm font-bold text-foreground">{title}</h2>
          <p className="mt-0.5 truncate text-[11px] text-muted-foreground">
            {subtitle}
          </p>
          <p className="mt-1 truncate font-mono text-[10px] text-muted-foreground/60">
            {detection?.config_dir ?? "—"}
          </p>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-1.5">
          <span
            className={`rounded border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide ${
              detection?.installed
                ? "border-emerald-200 bg-emerald-50 text-emerald-700"
                : "border-border bg-muted/30 text-muted-foreground"
            }`}
          >
            {detection?.installed ? t.agentInstalled : t.agentMissing}
          </span>
          <span
            className={`rounded border px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide ${
              config
                ? "border-sky-200 bg-sky-50 text-sky-700"
                : "border-border bg-muted/30 text-muted-foreground"
            }`}
          >
            {config ? t.agentConfigured : t.agentUnconfigured}
          </span>
          <Button
            size="sm"
            variant="outline"
            disabled={testing || busy || !supplierId || !model.trim()}
            onClick={() => void runTest()}
          >
            {testing ? (
              <Loader2 className="size-3 animate-spin" />
            ) : (
              <Activity className="size-3" />
            )}
            {t.test}
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={busy || !config}
            onClick={() => void onRewrite()}
          >
            <RefreshCw className="size-3" />
            {t.rewrite}
          </Button>
          <Button size="sm" disabled={busy} onClick={() => void submit()}>
            {busy ? <Loader2 className="size-3 animate-spin" /> : <Save className="size-3" />}
            {t.saveAndWrite}
          </Button>
          {config ? (
            <Button
              size="icon-sm"
              variant="ghost"
              disabled={busy}
              onClick={() => void onClear()}
              className="text-red-500 hover:bg-red-500/10 hover:text-red-600"
              title={t.clearConfig}
            >
              <Trash2 className="size-3" />
            </Button>
          ) : null}
        </div>
      </div>

      <div className="space-y-3 px-4 py-3.5">
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
            <p className="text-[10px] text-muted-foreground/70">留空则写入主模型。</p>
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

      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border/40 bg-background/40 px-4 py-2 text-[10px] text-muted-foreground/70">
        <span>
          {AGENT_LABELS[agent]} · {ENV_LABELS[env]}
        </span>
        <span className="font-mono">
          {capabilities.protocols.map((item) => PROTOCOL_LABELS[item]).join(" / ")}
        </span>
      </div>
    </div>
  );
}
