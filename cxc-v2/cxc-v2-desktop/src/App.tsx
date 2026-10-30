import { useCallback, useEffect, useMemo, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertCircle, CheckCircle2, Loader2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Header, type Section } from "@/components/layout/Header";
import { SupplierPage } from "@/components/suppliers/SupplierPage";
import { SupplierDialog } from "@/components/suppliers/SupplierDialog";
import { AgentPage } from "@/components/agents/AgentPage";
import { SettingsDialog } from "@/components/settings/SettingsDialog";
import { api, isTauri } from "@/lib/api";
import { messages, type Lang } from "@/lib/i18n";
import type {
  AgentConfigInput,
  AgentKind,
  Config,
  CustomDirs,
  DetectedAgent,
  Env,
  ModelTestResult,
  Protocol,
  Supplier,
  SupplierInput,
  SyncConfig,
} from "@/lib/types";
import { emptyDirs } from "@/lib/types";
import { errorText } from "@/lib/utils";

type ToastKind = "success" | "error" | "info";
interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
}

interface ConfirmState {
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => Promise<void>;
}

export default function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [detected, setDetected] = useState<DetectedAgent[]>([]);
  const [envs, setEnvs] = useState<Env[]>(["wsl"]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [section, setSection] = useState<Section>("suppliers");
  const [agent, setAgent] = useState<AgentKind>("codex");
  const [env, setEnv] = useState<Env>("wsl");
  const [search, setSearch] = useState("");
  const [supplierDialog, setSupplierDialog] = useState<{
    open: boolean;
    supplier: Supplier | null;
  }>({ open: false, supplier: null });
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [busy, setBusy] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [confirm, setConfirm] = useState<ConfirmState | null>(null);
  const [version, setVersion] = useState("0.1.0");
  const [pathEnv, setPathEnv] = useState("");
  const [lang, setLang] = useState<Lang>(() => {
    const stored = localStorage.getItem("cxc-v2-lang");
    return stored === "zh" || stored === "en" ? stored : "zh";
  });
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    const stored = localStorage.getItem("cxc-v2-theme");
    if (stored === "light" || stored === "dark") return stored;
    return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  });

  const t = messages[lang];

  const showToast = useCallback((message: string, kind: ToastKind = "info") => {
    const id = Date.now() + Math.random();
    setToasts((current) => [...current, { id, message, kind }]);
    window.setTimeout(() => {
      setToasts((current) => current.filter((toast) => toast.id !== id));
    }, 4000);
  }, []);

  const reload = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const [nextConfig, nextEnvs, nextDetected, nextVersion, nextPath] =
        await Promise.all([
          api.getConfig(),
          api.getEnvs(),
          api.detectAgents(),
          api.getAppVersion(),
          api.getPathEnv(),
        ]);
      setConfig(nextConfig);
      setEnvs(nextEnvs);
      setDetected(nextDetected);
      setVersion(nextVersion);
      setPathEnv(nextPath);
      setEnv((current) => (nextEnvs.includes(current) ? current : (nextEnvs[0] ?? "wsl")));
    } catch (error) {
      setLoadError(errorText(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    localStorage.setItem("cxc-v2-lang", lang);
  }, [lang]);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem("cxc-v2-theme", theme);
  }, [theme]);

  const submitSupplier = async (input: SupplierInput, probeAfterSave: boolean) => {
    setSaving(true);
    try {
      const updated = supplierDialog.supplier
        ? await api.editSupplier(supplierDialog.supplier.id, input)
        : await api.addSupplier(input);
      setConfig(updated);
      setSupplierDialog({ open: false, supplier: null });
      showToast(supplierDialog.supplier ? t.editSupplier : t.addSupplier, "success");

      if (probeAfterSave) {
        const saved = updated.suppliers.find((supplier) => supplier.name === input.name.trim());
        if (saved) {
          const result = await api.probeSupplier(saved.id);
          setConfig(result.config);
          const added = result.report.added.length
            ? result.report.added.join(", ")
            : "—";
          showToast(`${t.probeDone}: +${added}`, "success");
        }
      }
    } finally {
      setSaving(false);
    }
  };

  const deleteSupplier = (supplier: Supplier) => {
    setConfirm({
      title: `${t.delete} ${supplier.name}`,
      description: t.supplierReferenced,
      confirmLabel: t.delete,
      onConfirm: async () => {
        const updated = await api.deleteSupplier(supplier.id);
        setConfig(updated);
        showToast(t.delete, "success");
      },
    });
  };

  const probeSupplier = async (supplier: Supplier) => {
    setBusy((current) => ({ ...current, [supplier.id]: "probe" }));
    try {
      const result = await api.probeSupplier(supplier.id);
      setConfig(result.config);
      const summary = result.report.results
        .map((item) => `${item.protocol}: ${item.confirmed ? "✓" : "×"}`)
        .join(" · ");
      showToast(`${t.probeDone} · ${summary}`, "success");
    } catch (error) {
      showToast(errorText(error), "error");
    } finally {
      setBusy((current) => {
        const next = { ...current };
        delete next[supplier.id];
        return next;
      });
    }
  };

  const fetchModels = async (supplier: Supplier) => {
    setBusy((current) => ({ ...current, [supplier.id]: "models" }));
    try {
      const result = await api.fetchSupplierModels(supplier.id);
      setConfig(result.config);
      showToast(`${t.modelsFetched}: ${result.models.length}`, "success");
    } catch (error) {
      showToast(errorText(error), "error");
    } finally {
      setBusy((current) => {
        const next = { ...current };
        delete next[supplier.id];
        return next;
      });
    }
  };

  const testAll = async (supplier: Supplier) => {
    setBusy((current) => ({ ...current, [supplier.id]: "test" }));
    try {
      const result = await api.testSupplierModels(supplier.id);
      setConfig(result.config);
      const ok = result.outcomes.filter((outcome) => outcome.result.ok).length;
      showToast(`${t.testDone}: ${ok}/${result.outcomes.length}`, "success");
    } catch (error) {
      showToast(errorText(error), "error");
    } finally {
      setBusy((current) => {
        const next = { ...current };
        delete next[supplier.id];
        return next;
      });
    }
  };

  const refreshModels = async (supplierId: string) => {
    const result = await api.fetchSupplierModels(supplierId);
    setConfig(result.config);
    showToast(`${t.modelsFetched}: ${result.models.length}`, "success");
  };

  const testModel = async (
    supplierId: string,
    model: string,
    protocol: Protocol,
  ): Promise<ModelTestResult> => {
    const result = await api.testSupplierModel(supplierId, model, protocol);
    setConfig(result.config);
    showToast(
      `${model} · ${result.result.ok ? t.success : t.failed} · ${result.result.latency_ms} ms`,
      result.result.ok ? "success" : "error",
    );
    return result.result;
  };

  const addBinding = async (input: AgentConfigInput, activate: boolean) => {
    setSaving(true);
    try {
      const updated = await api.addBinding(agent, env, input, activate);
      setConfig(updated);
      showToast(activate ? t.bindingActivated : t.bindingSaved, "success");
    } catch (error) {
      showToast(errorText(error), "error");
      throw error;
    } finally {
      setSaving(false);
    }
  };

  const editBinding = async (id: string, input: AgentConfigInput) => {
    setSaving(true);
    try {
      const updated = await api.editBinding(agent, env, id, input);
      setConfig(updated);
      showToast(t.bindingSaved, "success");
    } catch (error) {
      showToast(errorText(error), "error");
      throw error;
    } finally {
      setSaving(false);
    }
  };

  const activateBinding = async (binding: { id: string }) => {
    setSaving(true);
    try {
      const updated = await api.activateBinding(agent, env, binding.id);
      setConfig(updated);
      showToast(t.bindingActivated, "success");
    } catch (error) {
      showToast(errorText(error), "error");
    } finally {
      setSaving(false);
    }
  };

  const deleteBinding = async (binding: { id: string }) => {
    setConfirm({
      title: t.delete,
      description: `${agent} · ${env}`,
      confirmLabel: t.delete,
      onConfirm: async () => {
        const updated = await api.deleteBinding(agent, env, binding.id);
        setConfig(updated);
        showToast(t.bindingDeleted, "success");
      },
    });
  };

  const saveSettings = async (customDirs: CustomDirs) => {
    const updated = await api.saveSettings(customDirs);
    setConfig(updated);
    const nextDetected = await api.detectAgents();
    setDetected(nextDetected);
    showToast(t.settingsSaved, "success");
  };

  const saveSync = async (sync: SyncConfig) => {
    const updated = await api.saveSyncConfig(sync);
    setConfig(updated);
  };

  const syncAction = async (action: "push" | "pull" | "merge") => {
    const result =
      action === "push"
        ? await api.syncPush()
        : action === "pull"
          ? await api.syncPull()
          : await api.syncMerge();
    setConfig(result.config);
    showToast(
      `${t.syncDone} · +${result.report.suppliers_added} / ~${result.report.bindings_updated}`,
      "success",
    );
  };

  const importLegacy = async () => {
    const report = await api.importLegacyConfig();
    showToast(
      `${report.reason} · ${report.suppliers} / ${report.configs}`,
      report.imported ? "success" : "info",
    );
    await reload();
  };

  const checkUpdate = async () => {
    try {
      const info = await api.checkUpdate();
      showToast(
        info.has_update
          ? `${t.update}: ${info.latest_version}`
          : `${t.version} ${info.current_version}`,
        "info",
      );
      if (info.has_update && info.download_url) {
        if (isTauri) {
          await openUrl(info.download_url);
        } else {
          window.open(info.download_url, "_blank");
        }
      }
    } catch (error) {
      showToast(errorText(error), "error");
    }
  };

  const content = useMemo(() => {
    if (loading) {
      return (
        <div className="flex h-64 items-center justify-center gap-2 text-xs text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          {t.loading}
        </div>
      );
    }
    if (loadError) {
      return (
        <div className="flex h-64 flex-col items-center justify-center gap-3 rounded-xl border border-red-200 bg-red-50 text-red-600">
          <AlertCircle className="size-6" />
          <p className="text-xs font-semibold">
            {t.errorPrefix}
            {loadError}
          </p>
          <Button size="sm" variant="outline" onClick={() => void reload()}>
            {t.redetect}
          </Button>
        </div>
      );
    }
    if (!config) return null;
    if (section === "agents") {
      return (
        <AgentPage
          config={config}
          detected={detected}
          envs={envs}
          agent={agent}
          env={env}
          search={search}
          onAgent={setAgent}
          onEnv={setEnv}
          t={t}
          busy={saving}
          onAdd={addBinding}
          onEdit={editBinding}
          onDelete={deleteBinding}
          onActivate={activateBinding}
          onTest={testModel}
          onRefreshModels={refreshModels}
        />
      );
    }
    return (
      <SupplierPage
        config={config}
        search={search}
        t={t}
        busy={busy}
        onAdd={() => setSupplierDialog({ open: true, supplier: null })}
        onEdit={(supplier) => setSupplierDialog({ open: true, supplier })}
        onDelete={deleteSupplier}
        onProbe={probeSupplier}
        onFetchModels={fetchModels}
        onTestAll={testAll}
        onTestModel={(supplier, model, protocol) =>
          testModel(supplier.id, model, protocol)
        }
      />
    );
  }, [
    loading,
    loadError,
    config,
    section,
    agent,
    env,
    envs,
    detected,
    search,
    busy,
    saving,
    t,
    reload,
  ]);

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background/55 text-foreground backdrop-blur-[24px]">
      <Header
        section={section}
        onSection={setSection}
        search={search}
        onSearch={setSearch}
        onAdd={() => setSupplierDialog({ open: true, supplier: null })}
        addLabel={t.addSupplier}
        onSettings={() => setSettingsOpen(true)}
        theme={theme}
        onToggleTheme={() => setTheme((current) => (current === "dark" ? "light" : "dark"))}
        t={t}
      />

      <main className="mx-auto w-full max-w-[1200px] flex-1 overflow-y-auto px-5 py-5">
        {content}
      </main>

      <SupplierDialog
        open={supplierDialog.open}
        supplier={supplierDialog.supplier}
        t={t}
        saving={saving}
        onClose={() => setSupplierDialog({ open: false, supplier: null })}
        onSubmit={submitSupplier}
      />

      <SettingsDialog
        open={settingsOpen}
        config={
          config ?? {
            version: 3,
            suppliers: [],
            agents: {
              codex: {},
              claude: {},
              grok: {},
              opencode: {},
            },
            custom_dirs: emptyDirs(),
            migrated_from_v1: false,
          }
        }
        envs={envs}
        t={t}
        lang={lang}
        onLang={setLang}
        onClose={() => setSettingsOpen(false)}
        onSave={saveSettings}
        onSyncSave={saveSync}
        onTestGist={(token, gistId) => api.testGistConnection(token, gistId || undefined)}
        onSyncAction={syncAction}
        onImport={importLegacy}
        version={version}
        onCheckUpdate={checkUpdate}
        pathEnv={pathEnv}
        onRedetect={reload}
      />

      <Dialog open={confirm != null} onOpenChange={(open) => !open && setConfirm(null)}>
        <DialogContent className="rounded-2xl border-border/50 bg-card/95 p-6 shadow-2xl backdrop-blur-xl sm:max-w-sm">
          <DialogHeader className="gap-1.5">
            <DialogTitle className="text-base font-bold">{confirm?.title}</DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              {confirm?.description}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="mt-4 gap-2">
            <Button variant="ghost" size="sm" onClick={() => setConfirm(null)}>
              {t.cancel}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => {
                const action = confirm?.onConfirm;
                setConfirm(null);
                if (action) {
                  void action().catch((error) => showToast(errorText(error), "error"));
                }
              }}
            >
              {confirm?.confirmLabel}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <div className="pointer-events-none fixed bottom-4 right-4 z-[100] flex w-80 flex-col gap-2">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`pointer-events-auto flex items-start gap-2 rounded-lg border bg-card/95 px-3 py-2 text-[11px] shadow-lg backdrop-blur-xl ${
              toast.kind === "error"
                ? "border-red-200 text-red-600"
                : toast.kind === "success"
                  ? "border-emerald-200 text-emerald-700"
                  : "border-border text-foreground"
            }`}
          >
            {toast.kind === "error" ? (
              <AlertCircle className="mt-0.5 size-3.5 shrink-0" />
            ) : (
              <CheckCircle2 className="mt-0.5 size-3.5 shrink-0" />
            )}
            <span className="flex-1 leading-snug">{toast.message}</span>
            <button
              onClick={() =>
                setToasts((current) => current.filter((item) => item.id !== toast.id))
              }
              className="text-muted-foreground/50 hover:text-foreground"
            >
              <X className="size-3" />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
