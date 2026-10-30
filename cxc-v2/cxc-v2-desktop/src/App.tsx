import { useCallback, useEffect, useMemo, useState } from "react";
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
import { BindingDialog } from "@/components/agents/BindingDialog";
import { SettingsDialog } from "@/components/settings/SettingsDialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, isTauri } from "@/lib/api";
import { messages, type Lang } from "@/lib/i18n";
import type {
  AgentKind,
  Binding,
  BindingInput,
  Config,
  CustomDirs,
  DetectedAgent,
  ModelTestResult,
  Protocol,
  Source,
  Supplier,
  SupplierInput,
  SyncConfig,
} from "@/lib/types";

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
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [section, setSection] = useState<Section>("suppliers");
  const [agent, setAgent] = useState<AgentKind>("codex");
  const [showAllAgents, setShowAllAgents] = useState(false);
  const [search, setSearch] = useState("");
  const [supplierDialog, setSupplierDialog] = useState<{
    open: boolean;
    supplier: Supplier | null;
  }>({ open: false, supplier: null });
  const [bindingDialog, setBindingDialog] = useState<{
    open: boolean;
    binding: Binding | null;
  }>({ open: false, binding: null });
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
      const [nextConfig, nextDetected, nextVersion, nextPath] = await Promise.all([
        api.getConfig(),
        api.detectAgents(),
        api.getAppVersion(),
        api.getPathEnv(),
      ]);
      setConfig(nextConfig);
      setDetected(nextDetected);
      setVersion(nextVersion);
      setPathEnv(nextPath);
    } catch (error) {
      setLoadError(String(error));
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

  const changeLang = (next: Lang) => {
    setLang(next);
  };

  const toggleSource = async () => {
    if (!config) return;
    const next: Source = config.source === "wsl" ? "app" : "wsl";
    try {
      const updated = await api.saveSettings(
        next,
        config.custom_dirs ?? { codex: "", claude: "", grok: "", opencode: "" },
      );
      setConfig(updated);
      const nextDetected = await api.detectAgents();
      setDetected(nextDetected);
      showToast(t.sourceSwitched, "success");
    } catch (error) {
      showToast(String(error), "error");
    }
  };

  const submitSupplier = async (input: SupplierInput, probeAfterSave: boolean) => {
    if (!config) return;
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
            ? result.report.added.map((item) => item).join(", ")
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
      showToast(String(error), "error");
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
      showToast(String(error), "error");
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
      showToast(String(error), "error");
    } finally {
      setBusy((current) => {
        const next = { ...current };
        delete next[supplier.id];
        return next;
      });
    }
  };

  const testModel = async (
    supplier: Supplier,
    model: string,
    protocol: Protocol,
  ): Promise<ModelTestResult> => {
    const result = await api.testSupplierModel(supplier.id, model, protocol);
    setConfig(result.config);
    showToast(
      `${model} · ${result.result.ok ? t.success : t.failed} · ${result.result.latency_ms} ms`,
      result.result.ok ? "success" : "error",
    );
    return result.result;
  };

  const submitBinding = async (input: BindingInput) => {
    if (!config) return;
    setSaving(true);
    try {
      const updated = bindingDialog.binding
        ? await api.editBinding(agent, bindingDialog.binding.id, input)
        : await api.addBinding(agent, input);
      setConfig(updated);
      setBindingDialog({ open: false, binding: null });
      showToast(bindingDialog.binding ? t.editBinding : t.addBinding, "success");
    } finally {
      setSaving(false);
    }
  };

  const deleteBinding = (binding: Binding) => {
    setConfirm({
      title: t.deleteBinding,
      description: t.bindingActive,
      confirmLabel: t.delete,
      onConfirm: async () => {
        const updated = await api.deleteBinding(agent, binding.id);
        setConfig(updated);
        showToast(t.deleteBinding, "success");
      },
    });
  };

  const activateBinding = async (binding: Binding) => {
    if (!config) return;
    setSaving(true);
    try {
      const updated = await api.activateBinding(agent, config.source, binding.id);
      setConfig(updated);
      showToast(t.active, "success");
    } catch (error) {
      showToast(String(error), "error");
    } finally {
      setSaving(false);
    }
  };

  const reloadActive = async () => {
    if (!config) return;
    setSaving(true);
    try {
      const updated = await api.reloadActive(agent, config.source);
      setConfig(updated);
      showToast(t.reload, "success");
    } catch (error) {
      showToast(String(error), "error");
    } finally {
      setSaving(false);
    }
  };

  const testBinding = async (binding: Binding) => {
    const supplier = config?.suppliers.find((item) => item.id === binding.supplier_id);
    if (!supplier) return;
    await testModel(supplier, binding.model, binding.protocol);
  };

  const saveSettings = async (source: Source, customDirs: CustomDirs) => {
    const updated = await api.saveSettings(source, customDirs);
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
    showToast(`${report.reason} · ${report.suppliers} / ${report.bindings}`, report.imported ? "success" : "info");
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
      showToast(String(error), "error");
    }
  };

  const activeAddLabel = section === "agents" ? t.addBinding : t.addSupplier;
  const handleAdd = () => {
    if (section === "agents") {
      setBindingDialog({ open: true, binding: null });
    } else {
      setSupplierDialog({ open: true, supplier: null });
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
          <p className="text-xs font-semibold">{t.errorPrefix}{loadError}</p>
          <Button size="sm" variant="outline" onClick={() => void reload()}>
            {t.reload}
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
          agent={agent}
          source={config.source}
          search={search}
          onAgent={setAgent}
          showAll={showAllAgents}
          onShowAll={setShowAllAgents}
          t={t}
          busy={saving}
          onAddBinding={() => setBindingDialog({ open: true, binding: null })}
          onEditBinding={(binding) => setBindingDialog({ open: true, binding })}
          onDeleteBinding={deleteBinding}
          onActivate={activateBinding}
          onReload={reloadActive}
          onTestBinding={testBinding}
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
        onTestModel={testModel}
      />
    );
  }, [
    loading,
    loadError,
    config,
    section,
    agent,
    detected,
    showAllAgents,
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
        source={config?.source ?? "wsl"}
        onToggleSource={() => void toggleSource()}
        search={search}
        onSearch={setSearch}
        onAdd={handleAdd}
        addLabel={activeAddLabel}
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

      <BindingDialog
        open={bindingDialog.open}
        agent={agent}
        binding={bindingDialog.binding}
        config={config ?? { version: 2, source: "wsl", suppliers: [], agents: { codex: { bindings: [] }, claude: { bindings: [] }, grok: { bindings: [] }, opencode: { bindings: [] } }, custom_dirs: { codex: "", claude: "", grok: "", opencode: "" }, migrated_from_v1: false }}
        t={t}
        saving={saving}
        onClose={() => setBindingDialog({ open: false, binding: null })}
        onSubmit={submitBinding}
        onTest={async (supplierId, model, protocol) => {
          const supplier = config?.suppliers.find((item) => item.id === supplierId);
          if (!supplier) throw new Error(t.selectSupplier);
          return testModel(supplier, model, protocol);
        }}
      />

      <SettingsDialog
        open={settingsOpen}
        config={config ?? { version: 2, source: "wsl", suppliers: [], agents: { codex: { bindings: [] }, claude: { bindings: [] }, grok: { bindings: [] }, opencode: { bindings: [] } }, custom_dirs: { codex: "", claude: "", grok: "", opencode: "" }, migrated_from_v1: false }}
        t={t}
        lang={lang}
        onLang={changeLang}
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
                  void action().catch((error) => showToast(String(error), "error"));
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
