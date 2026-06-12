import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import {
  Plus,
  RefreshCw,
  Activity,
  Trash2,
  Edit2,
  Search,
  Sun,
  Moon,
  Check,
  Compass,
  AlertTriangle,
  Loader2,
  List,
  LayoutGrid,
} from "lucide-react";

interface Provider {
  name: string;
  base_url: string;
  api_key: string;
  model: string;
  wire_api: string;
  remark?: string;
  last_test?: string;
  latency_ms?: number;
  last_ok?: boolean;
}

interface Config {
  active: string;
  providers: Provider[];
}

const initialFormValues = {
  name: "",
  base_url: "",
  api_key: "",
  model: "",
  wire_api: "responses",
  remark: "",
};

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [switching, setSwitching] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>("");

  // View mode state (list or card, default to list)
  const [viewMode, setViewMode] = useState<"list" | "card">(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("cxc-view-mode");
      if (saved === "list" || saved === "card") return saved;
    }
    return "list";
  });

  useEffect(() => {
    localStorage.setItem("cxc-view-mode", viewMode);
  }, [viewMode]);

  // Theme state
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("cxc-theme");
      if (saved === "light" || saved === "dark") return saved;
      return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "dark";
    }
    return "dark";
  });

  // Form State
  const [showForm, setShowForm] = useState<"add" | "edit" | null>(null);
  const [editingName, setEditingName] = useState<string | null>(null);
  const [formValues, setFormValues] = useState(initialFormValues);

  // Model Discovery State
  const [fetchingModels, setFetchingModels] = useState<boolean>(false);
  const [fetchedModels, setFetchedModels] = useState<string[]>([]);
  const [fetchError, setFetchError] = useState<string | null>(null);

  // Connectivity Test State
  const [testingProvider, setTestingProvider] = useState<string | null>(null);
  const [testingAll, setTestingAll] = useState<boolean>(false);

  // Apply theme
  useEffect(() => {
    const root = window.document.documentElement;
    if (theme === "dark") {
      root.classList.add("dark");
    } else {
      root.classList.remove("dark");
    }
    localStorage.setItem("cxc-theme", theme);
  }, [theme]);

  // Load config & subscribe to tray events
  useEffect(() => {
    loadConfig();

    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      unlisten = await listen<Config>("config-updated", (event) => {
        setConfig(event.payload);
      });
    };
    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  async function loadConfig() {
    try {
      setLoading(true);
      setError(null);
      const cfg = await invoke<Config>("get_config");
      setConfig(cfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setLoading(false);
    }
  }

  async function handleSwitch(name: string) {
    try {
      setSwitching(name);
      setError(null);
      const updatedCfg = await invoke<Config>("switch_provider", { name });
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setSwitching(null);
    }
  }

  function openAddForm() {
    setFormValues(initialFormValues);
    setFetchedModels([]);
    setFetchError(null);
    setShowForm("add");
    setEditingName(null);
  }

  function openEditForm(p: Provider) {
    setFormValues({
      name: p.name,
      base_url: p.base_url,
      api_key: p.api_key,
      model: p.model,
      wire_api: p.wire_api,
      remark: p.remark || "",
    });
    setFetchedModels([]);
    setFetchError(null);
    setShowForm("edit");
    setEditingName(p.name);
  }

  async function handleFetchModels() {
    if (!formValues.base_url || !formValues.api_key) {
      setFetchError("Please fill in Base URL and API Key first");
      return;
    }
    try {
      setFetchingModels(true);
      setFetchError(null);
      const models = await invoke<string[]>("fetch_models", {
        baseUrl: formValues.base_url,
        apiKey: formValues.api_key,
      });
      if (models.length === 0) {
        setFetchError("No models returned from endpoint");
      } else {
        setFetchedModels(models);
      }
    } catch (e: any) {
      setFetchError(e.toString());
    } finally {
      setFetchingModels(false);
    }
  }

  async function handleSubmitForm(e: React.FormEvent) {
    e.preventDefault();
    if (!formValues.name || !formValues.base_url || !formValues.api_key || !formValues.model) {
      setError("Please fill in all required fields (Name, Base URL, API Key, Model)");
      return;
    }

    try {
      setError(null);
      let updatedCfg: Config;
      if (showForm === "add") {
        updatedCfg = await invoke<Config>("add_provider", { provider: formValues });
      } else {
        updatedCfg = await invoke<Config>("edit_provider", {
          oldName: editingName,
          updated: formValues,
        });
      }
      setConfig(updatedCfg);
      setShowForm(null);
    } catch (e: any) {
      setError(e.toString());
    }
  }

  async function handleDeleteProvider(name: string) {
    if (!confirm(`Are you sure you want to remove provider "${name}"?`)) {
      return;
    }
    try {
      setError(null);
      const updatedCfg = await invoke<Config>("delete_provider", { name });
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    }
  }

  async function handleTestProvider(name: string) {
    try {
      setTestingProvider(name);
      setError(null);
      const updatedCfg = await invoke<Config>("test_provider", { name });
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setTestingProvider(null);
    }
  }

  async function handleTestAllProviders() {
    try {
      setTestingAll(true);
      setError(null);
      const updatedCfg = await invoke<Config>("test_all_providers");
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setTestingAll(false);
    }
  }

  function formatDate(isoStr?: string) {
    if (!isoStr) return "";
    try {
      const d = new Date(isoStr);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    } catch {
      return isoStr;
    }
  }

  const filteredProviders = config?.providers.filter(
    (p) =>
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.model.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (p.remark && p.remark.toLowerCase().includes(searchQuery.toLowerCase()))
  ) || [];

  return (
    <div className="min-h-screen bg-background text-foreground transition-colors duration-200">
      {/* Notion Document Navigation Chrome */}
      <header className="sticky top-0 z-40 w-full border-b border-border bg-card/90 backdrop-blur-md px-6 py-4 flex items-center justify-between shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className="flex items-center gap-3.5">
          {/* Logo with Notion decorative palette style */}
          <div className="relative flex items-center justify-center size-10 rounded-xl bg-card border border-border text-primary font-bold text-lg shadow-sm transition-transform hover:rotate-3 duration-200">
            C
            <span className="text-[9px] absolute bottom-1 right-1.5 font-bold text-muted-foreground">XC</span>
            {/* Playful sticker dot */}
            <span className="absolute -top-0.5 -right-0.5 size-2 rounded-full bg-sticker-purple shadow-sm"></span>
          </div>
          <div>
            <h1 className="text-base font-bold tracking-tight text-foreground/95 flex items-center gap-1.5">
              CXC Cross-Connect
            </h1>
            <p className="text-xs text-muted-foreground font-medium">Relay Configuration Manager</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Theme switcher styled as utility button */}
          <Button
            variant="outline"
            size="icon"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            className="size-8 rounded-md border-border hover:bg-muted text-muted-foreground hover:text-foreground shadow-sm transition-all duration-200"
            title="Toggle theme"
          >
            {theme === "dark" ? <Sun className="size-4 text-amber-500" /> : <Moon className="size-4 text-primary" />}
          </Button>

          {/* Refresh config utility button */}
          <Button
            variant="outline"
            size="icon"
            onClick={loadConfig}
            disabled={loading}
            className="size-8 rounded-md border-border hover:bg-muted text-muted-foreground hover:text-foreground shadow-sm transition-all duration-200"
            title="Refresh configuration"
          >
            <RefreshCw className={`size-4 ${loading ? "animate-spin text-muted-foreground" : ""}`} />
          </Button>

          {/* Primary CTA: Pill-shaped in Notion Blue */}
          <Button
            onClick={openAddForm}
            className="h-8 px-4 rounded-full bg-primary text-primary-foreground hover:bg-primary/90 hover:scale-[1.02] active:scale-95 shadow-sm font-medium text-xs transition-all duration-200"
          >
            <Plus className="size-3.5 mr-1" />
            Add Provider
          </Button>
        </div>
      </header>

      {/* Main Container */}
      <main className="max-w-6xl mx-auto p-6 space-y-6">
        {/* Error Alert */}
        {error && (
          <div className="rounded-xl border border-red-200/60 bg-red-500/5 p-4 dark:border-red-900/40 dark:bg-red-950/10 text-red-600 dark:text-red-400 flex items-start gap-3 animate-in fade-in slide-in-from-top-2 duration-200">
            <AlertTriangle className="size-5 shrink-0 mt-0.5 text-sticker-orange" />
            <div className="flex-1">
              <h5 className="font-bold text-xs tracking-tight uppercase text-sticker-orange">System Alert</h5>
              <p className="text-xs mt-1 leading-normal opacity-90">{error}</p>
            </div>
            <button
              onClick={() => setError(null)}
              className="text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300 font-bold text-lg leading-none"
            >
              ×
            </button>
          </div>
        )}

        {/* Toolbar */}
        <div className="flex flex-col sm:flex-row gap-4 items-center justify-between">
          <div className="relative w-full sm:w-80">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/60 pointer-events-none" />
            <Input
              type="text"
              placeholder="Search by name, model, remark..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 h-9 w-full bg-card border-border rounded-[4px] placeholder-muted-foreground/50 focus-visible:ring-primary focus-visible:border-primary text-sm shadow-sm transition-all"
            />
          </div>

          <div className="flex items-center gap-3 w-full sm:w-auto justify-end">
            {/* View Mode Toggle Controls */}
            <div className="flex items-center border border-border rounded-md p-0.5 bg-card shadow-sm h-9">
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setViewMode("list")}
                className={`size-7 rounded-[4px] p-0 transition-all ${
                  viewMode === "list"
                    ? "bg-muted text-foreground font-semibold shadow-inner border border-border/10"
                    : "text-muted-foreground hover:text-foreground"
                }`}
                title="List View"
              >
                <List className="size-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setViewMode("card")}
                className={`size-7 rounded-[4px] p-0 transition-all ${
                  viewMode === "card"
                    ? "bg-muted text-foreground font-semibold shadow-inner border border-border/10"
                    : "text-muted-foreground hover:text-foreground"
                }`}
                title="Card View"
              >
                <LayoutGrid className="size-3.5" />
              </Button>
            </div>

            {config && config.providers.length > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleTestAllProviders}
                disabled={testingAll || testingProvider !== null}
                className="h-9 border-border bg-card text-xs font-semibold rounded-md hover:bg-muted text-foreground shadow-sm transition-all duration-200 hover:scale-[1.01]"
              >
                {testingAll ? (
                  <>
                    <Loader2 className="size-3.5 mr-1.5 animate-spin" />
                    Testing All...
                  </>
                ) : (
                  <>
                    <Activity className="size-3.5 mr-1.5 text-muted-foreground" />
                    Test All Connections
                  </>
                )}
              </Button>
            )}
          </div>
        </div>

        {/* Loading State */}
        {loading && !config ? (
          <div className="flex flex-col items-center justify-center py-24 gap-4">
            <Loader2 className="size-7 animate-spin text-muted-foreground/50" />
            <p className="text-xs text-muted-foreground font-medium">Fetching active configuration...</p>
          </div>
        ) : (
          <section className="space-y-4">
            {/* Conditional list/card rendering */}
            {filteredProviders.length > 0 ? (
              viewMode === "list" ? (
                /* List View Layout */
                <div className="border border-border rounded-xl bg-card overflow-hidden divide-y divide-border shadow-[0_1px_3px_rgba(0,0,0,0.02)]">
                  {filteredProviders.map((p) => {
                    const isActive = config?.active === p.name;
                    const isThisSwitching = switching === p.name;
                    const isTesting = testingProvider === p.name || (testingAll && !isActive);

                    // Latency styling using Notion's decorative sticker palette
                    let latencyBadgeStyle = "bg-muted text-muted-foreground border-border";
                    if (p.last_ok) {
                      if (p.latency_ms !== undefined) {
                        if (p.latency_ms < 150) {
                          latencyBadgeStyle = "bg-sticker-green/10 dark:bg-sticker-green/20 text-sticker-green dark:text-emerald-400 border-sticker-green/20";
                        } else if (p.latency_ms < 400) {
                          latencyBadgeStyle = "bg-sticker-orange/10 dark:bg-sticker-orange/20 text-sticker-orange dark:text-orange-400 border-sticker-orange/20";
                        } else {
                          latencyBadgeStyle = "bg-sticker-orange/15 dark:bg-sticker-orange/25 text-sticker-orange dark:text-orange-400 border-sticker-orange/30";
                        }
                      }
                    } else if (p.last_ok === false) {
                      latencyBadgeStyle = "bg-red-500/10 dark:bg-red-950/20 text-red-600 dark:text-red-400 border-red-500/20 dark:border-red-900/30";
                    }

                    return (
                      <div
                        key={p.name}
                        className={`flex flex-col sm:flex-row sm:items-center justify-between p-4 gap-4 transition-colors hover:bg-muted/30 ${
                          isActive ? "bg-primary/[0.02] dark:bg-primary/[0.04]" : ""
                        } ${isTesting ? "opacity-80 animate-pulse" : ""}`}
                      >
                        {/* Left: Name & remark */}
                        <div className="flex items-center gap-3 min-w-[200px] max-w-[260px] w-full sm:w-auto">
                          <div className="flex-shrink-0 flex items-center justify-center size-5">
                            {isActive ? (
                              <span className="size-2 rounded-full bg-primary animate-pulse" />
                            ) : (
                              <span className="size-2 rounded-full bg-muted-foreground/30" />
                            )}
                          </div>
                          <div className="truncate">
                            <span className="text-sm font-bold text-foreground/90 tracking-tight" title={p.name}>
                              {p.name}
                            </span>
                            {p.remark && (
                              <p className="text-xs text-muted-foreground/75 truncate mt-0.5" title={p.remark}>
                                {p.remark}
                              </p>
                            )}
                          </div>
                        </div>

                        {/* Middle: Base URL & Model */}
                        <div className="flex-1 flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-6 text-xs w-full sm:w-auto">
                          <div className="flex items-center gap-1.5 min-w-[150px]">
                            <span className="text-muted-foreground/60 text-[10px] uppercase font-bold tracking-wider sm:hidden">Endpoint:</span>
                            <span className="font-mono text-foreground/70 truncate max-w-[220px]" title={p.base_url}>
                              {p.base_url}
                            </span>
                          </div>
                          <div className="flex items-center gap-1.5">
                            <span className="text-muted-foreground/60 text-[10px] uppercase font-bold tracking-wider sm:hidden">Model:</span>
                            <span className="font-semibold text-foreground/80 bg-background border border-border px-1.5 py-0.5 rounded text-[11px]">
                              {p.model}
                            </span>
                          </div>
                        </div>

                        {/* Right: Latency & Actions */}
                        <div className="flex items-center justify-between sm:justify-end gap-4 flex-shrink-0 w-full sm:w-auto">
                          {/* Latency */}
                          <div className="flex items-center gap-1.5 min-w-[90px] justify-end">
                            {isTesting ? (
                              <div className="flex items-center gap-1 text-[11px] text-sticker-orange font-semibold">
                                <Loader2 className="size-3 animate-spin text-sticker-orange" />
                                testing...
                              </div>
                            ) : p.latency_ms !== undefined && p.last_ok !== undefined ? (
                              <Badge className={`px-2 py-0.5 rounded-full text-[10px] font-bold border ${latencyBadgeStyle}`}>
                                {p.last_ok ? `${p.latency_ms} ms` : "Offline"}
                              </Badge>
                            ) : (
                              <span className="text-[10px] text-muted-foreground/60 font-medium italic">
                                Never tested
                              </span>
                            )}
                          </div>

                          {/* Quiet action buttons */}
                          <div className="flex items-center gap-1 border-l border-border pl-3">
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleTestProvider(p.name)}
                              disabled={testingProvider !== null || testingAll || switching !== null}
                              className="size-7 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title="Test connection"
                            >
                              <Activity className="size-3.5" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => openEditForm(p)}
                              disabled={testingProvider !== null || testingAll || switching !== null}
                              className="size-7 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title="Edit provider"
                            >
                              <Edit2 className="size-3.5" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleDeleteProvider(p.name)}
                              disabled={isActive || switching !== null || testingAll || testingProvider !== null}
                              className="size-7 rounded-md text-red-500 hover:text-red-600 hover:bg-red-500/10 dark:hover:bg-red-950/20 disabled:opacity-40 transition-colors"
                              title={isActive ? "Active provider cannot be deleted" : "Delete provider"}
                            >
                              <Trash2 className="size-3.5" />
                            </Button>
                          </div>

                          {/* Switch Active Trigger */}
                          <div className="w-24 flex justify-end">
                            {!isActive ? (
                              <Button
                                size="sm"
                                onClick={() => handleSwitch(p.name)}
                                disabled={switching !== null || testingAll || testingProvider !== null}
                                className="w-full text-[11px] h-7 bg-card text-foreground border border-border rounded-md hover:bg-muted font-medium shadow-sm transition-all duration-200 active:scale-95"
                              >
                                {isThisSwitching ? (
                                  <Loader2 className="size-3 animate-spin text-muted-foreground" />
                                ) : (
                                  "Switch"
                                )}
                              </Button>
                            ) : (
                              <div className="flex items-center justify-center gap-1 h-7 border border-primary/20 bg-primary/5 text-primary rounded-full text-[10px] font-bold px-3 py-0.5">
                                <Check className="size-3 shrink-0" />
                                Active
                              </div>
                            )}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              ) : (
                /* Card Grid View Layout */
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                  {filteredProviders.map((p) => {
                    const isActive = config?.active === p.name;
                    const isThisSwitching = switching === p.name;
                    const isTesting = testingProvider === p.name || (testingAll && !isActive);

                    // Latency styling using Notion's decorative sticker palette
                    let latencyBadgeStyle = "bg-muted text-muted-foreground border-border";
                    if (p.last_ok) {
                      if (p.latency_ms !== undefined) {
                        if (p.latency_ms < 150) {
                          // Fast: Green sticker
                          latencyBadgeStyle = "bg-sticker-green/10 dark:bg-sticker-green/20 text-sticker-green dark:text-emerald-400 border-sticker-green/20";
                        } else if (p.latency_ms < 400) {
                          // Moderate: Orange sticker
                          latencyBadgeStyle = "bg-sticker-orange/10 dark:bg-sticker-orange/20 text-sticker-orange dark:text-orange-400 border-sticker-orange/20";
                        } else {
                          // Slow: Orange / deep orange sticker
                          latencyBadgeStyle = "bg-sticker-orange/15 dark:bg-sticker-orange/25 text-sticker-orange dark:text-orange-400 border-sticker-orange/30";
                        }
                      }
                    } else if (p.last_ok === false) {
                      // Offline: Red / deep orange
                      latencyBadgeStyle = "bg-red-500/10 dark:bg-red-950/20 text-red-600 dark:text-red-400 border-red-500/20 dark:border-red-900/30";
                    }

                    return (
                      <Card
                        key={p.name}
                        className={`relative flex flex-col justify-between overflow-hidden bg-card border transition-all duration-200 hover:shadow-[0_4px_12px_rgba(0,0,0,0.03)] hover:-translate-y-[2px] ${
                          isActive
                            ? "border-primary/45 border-l-4 border-l-primary ring-1 ring-primary/10 shadow-sm"
                            : "border-border"
                        } ${isTesting ? "opacity-80 animate-pulse" : ""}`}
                      >
                        {/* Header of Card */}
                        <CardHeader className="pb-3 px-5 pt-5 gap-1">
                          <div className="flex items-start justify-between">
                            <div className="space-y-0.5">
                              <CardTitle className="text-[15px] font-bold tracking-tight text-foreground/90 truncate pr-2 flex items-center gap-1.5" title={p.name}>
                                {/* Blue decorative dot for active card */}
                                {isActive && (
                                  <span className="size-2 rounded-full bg-primary animate-pulse shrink-0" />
                                )}
                                {p.name}
                              </CardTitle>
                              {p.remark ? (
                                <CardDescription className="text-xs text-muted-foreground/75 font-medium line-clamp-1">
                                  {p.remark}
                                </CardDescription>
                              ) : (
                                <span className="block h-4" />
                              )}
                            </div>
                            {isActive && (
                              <span className="px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20 text-[9px] font-bold tracking-wide uppercase">
                                Active
                              </span>
                            )}
                          </div>
                        </CardHeader>

                        {/* Card Content: Details Block */}
                        <CardContent className="space-y-3.5 px-5 pb-4 text-xs">
                          {/* Monospace layout list, styled with paper canvas background */}
                          <div className="flex flex-col gap-1.5 p-3 rounded-lg bg-background border border-border">
                            <div className="flex justify-between items-center text-[11px] gap-2">
                              <span className="text-muted-foreground/80 font-medium">Endpoint:</span>
                              <span className="font-mono text-foreground/75 truncate max-w-[175px]" title={p.base_url}>
                                {p.base_url}
                              </span>
                            </div>
                            <div className="flex justify-between items-center text-[11px] gap-2">
                              <span className="text-muted-foreground/80 font-medium">Model:</span>
                              <span className="font-semibold text-foreground/80 truncate max-w-[175px]" title={p.model}>{p.model}</span>
                            </div>
                            <div className="flex justify-between items-center text-[11px] gap-2">
                              <span className="text-muted-foreground/80 font-medium">Key:</span>
                              <span className="font-mono text-muted-foreground/80">
                                {p.api_key.substring(0, 8)}••••••••
                              </span>
                            </div>
                          </div>

                          {/* Latency & Status Info */}
                          <div className="flex items-center justify-between min-h-[26px]">
                            <span className="text-muted-foreground/85 text-[11px] font-medium">Latency:</span>
                            {isTesting ? (
                              <div className="flex items-center gap-1 text-[11px] text-sticker-orange font-semibold">
                                <Loader2 className="size-3 animate-spin text-sticker-orange" />
                                testing...
                              </div>
                            ) : p.latency_ms !== undefined && p.last_ok !== undefined ? (
                              <div className="flex items-center gap-1.5">
                                <Badge className={`px-2 py-0.5 rounded-full text-[10px] font-bold border ${latencyBadgeStyle}`}>
                                  {p.last_ok ? `${p.latency_ms} ms` : "Offline"}
                                </Badge>
                                {p.last_test && (
                                  <span className="text-[10px] text-muted-foreground/60 font-medium">
                                    {formatDate(p.last_test)}
                                  </span>
                                )}
                              </div>
                            ) : (
                              <span className="text-[10px] text-muted-foreground/60 font-medium italic">
                                Never tested
                              </span>
                            )}
                          </div>
                        </CardContent>

                        {/* Actions Footer - Quiet utility strip */}
                        <div className="flex items-center gap-1 border-t border-border bg-background/50 p-3 justify-between">
                          {/* Left: action icons */}
                          <div className="flex items-center gap-1">
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleTestProvider(p.name)}
                              disabled={testingProvider !== null || testingAll || switching !== null}
                              className="size-7 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title="Test connection"
                            >
                              <Activity className="size-3.5" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => openEditForm(p)}
                              disabled={testingProvider !== null || testingAll || switching !== null}
                              className="size-7 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title="Edit provider"
                            >
                              <Edit2 className="size-3.5" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleDeleteProvider(p.name)}
                              disabled={isActive || switching !== null || testingAll || testingProvider !== null}
                              className="size-7 rounded-md text-red-500 hover:text-red-600 hover:bg-red-500/10 dark:hover:bg-red-950/20 disabled:opacity-40 transition-colors"
                              title={isActive ? "Active provider cannot be deleted" : "Delete provider"}
                            >
                              <Trash2 className="size-3.5" />
                            </Button>
                          </div>

                          {/* Right: Switch active button */}
                          <div className="w-24 flex justify-end">
                            {!isActive ? (
                              <Button
                                size="sm"
                                onClick={() => handleSwitch(p.name)}
                                disabled={switching !== null || testingAll || testingProvider !== null}
                                className="w-full text-[11px] h-7 bg-card text-foreground border border-border rounded-md hover:bg-muted font-medium shadow-sm transition-all duration-200 active:scale-95"
                              >
                                {isThisSwitching ? (
                                  <Loader2 className="size-3 animate-spin text-muted-foreground" />
                                ) : (
                                  "Switch"
                                )}
                              </Button>
                            ) : (
                              <div className="flex items-center justify-center gap-1 h-7 border border-primary/20 bg-primary/5 text-primary rounded-full text-[10px] font-bold px-3 py-0.5">
                                <Check className="size-3 shrink-0" />
                                Active
                              </div>
                            )}
                          </div>
                        </div>
                      </Card>
                    );
                  })}
                </div>
              )
            ) : (
              /* Empty State */
              <div className="py-20 flex flex-col items-center justify-center gap-3 border border-dashed border-border rounded-xl bg-card">
                <Compass className="size-9 text-muted-foreground/40" />
                <div className="text-center">
                  <p className="text-sm font-bold text-foreground/85">No providers found</p>
                  <p className="text-xs text-muted-foreground mt-1.5">
                    {searchQuery ? "Try altering your search filters" : "Create a new provider to get started."}
                  </p>
                </div>
                {!searchQuery && (
                  <Button onClick={openAddForm} className="mt-3 h-8 px-4 rounded-full bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/95 transition-transform active:scale-95 shadow-sm">
                    <Plus className="size-3.5 mr-1" /> Add Provider
                  </Button>
                )}
              </div>
            )}
          </section>
        )}
      </main>

      {/* Add / Edit Dialog Modal */}
      <Dialog open={showForm !== null} onOpenChange={(open) => !open && setShowForm(null)}>
        <DialogContent className="sm:max-w-md bg-card border border-border rounded-xl shadow-xl p-5 gap-4">
          <DialogHeader className="gap-1.5">
            <DialogTitle className="text-base font-bold tracking-tight">
              {showForm === "add" ? "Create Provider" : "Edit Provider"}
            </DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              Provide endpoint details for the cross-connect proxy relay.
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSubmitForm} className="space-y-4">
            <div className="space-y-1">
              <Label htmlFor="form-name" className="text-xs font-bold text-muted-foreground">
                Provider Name *
              </Label>
              <Input
                id="form-name"
                type="text"
                required
                value={formValues.name}
                onChange={(e) => setFormValues({ ...formValues, name: e.target.value })}
                placeholder="e.g. proxy-fast"
                className="bg-card border-border focus-visible:ring-primary focus-visible:border-primary h-9 rounded-[4px] text-sm shadow-sm transition-all"
              />
            </div>

            <div className="space-y-1">
              <Label htmlFor="form-url" className="text-xs font-bold text-muted-foreground">
                Base URL *
              </Label>
              <Input
                id="form-url"
                type="url"
                required
                value={formValues.base_url}
                onChange={(e) => setFormValues({ ...formValues, base_url: e.target.value })}
                placeholder="https://api.example.com/v1"
                className="bg-card border-border focus-visible:ring-primary focus-visible:border-primary h-9 rounded-[4px] text-sm shadow-sm transition-all"
              />
            </div>

            <div className="space-y-1">
              <Label htmlFor="form-key" className="text-xs font-bold text-muted-foreground">
                API Key *
              </Label>
              <Input
                id="form-key"
                type="password"
                required
                value={formValues.api_key}
                onChange={(e) => setFormValues({ ...formValues, api_key: e.target.value })}
                placeholder="sk-••••••••••••"
                className="bg-card border-border focus-visible:ring-primary focus-visible:border-primary h-9 rounded-[4px] text-sm shadow-sm transition-all"
              />
            </div>

            <div className="space-y-1">
              <div className="flex items-center justify-between pb-1">
                <Label htmlFor="form-model" className="text-xs font-bold text-muted-foreground">
                  Model *
                </Label>
                <Button
                  type="button"
                  variant="outline"
                  size="xs"
                  onClick={handleFetchModels}
                  disabled={fetchingModels || !formValues.base_url || !formValues.api_key}
                  className="h-6 text-[10px] border-border bg-card text-muted-foreground hover:bg-muted font-bold px-2 rounded-md shadow-sm transition-all"
                >
                  {fetchingModels ? (
                    <>
                      <Loader2 className="size-2.5 animate-spin mr-1" />
                      Loading...
                    </>
                  ) : (
                    <>
                      <Compass className="size-2.5 mr-1" />
                      Discover Models
                    </>
                  )}
                </Button>
              </div>

              {fetchedModels.length > 0 ? (
                <select
                  id="form-model-select"
                  value={formValues.model}
                  onChange={(e) => setFormValues({ ...formValues, model: e.target.value })}
                  className="flex h-9 w-full rounded-[4px] border border-border bg-card px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary text-foreground"
                >
                  <option value="">-- Select a model --</option>
                  {fetchedModels.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
              ) : (
                <Input
                  id="form-model"
                  type="text"
                  required
                  value={formValues.model}
                  onChange={(e) => setFormValues({ ...formValues, model: e.target.value })}
                  placeholder="e.g. gpt-4o"
                  className="bg-card border-border focus-visible:ring-primary focus-visible:border-primary h-9 rounded-[4px] text-sm shadow-sm transition-all"
                />
              )}
              {fetchError && <span className="text-[10px] text-red-500 font-medium block mt-1">{fetchError}</span>}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1">
                <Label htmlFor="form-wire" className="text-xs font-bold text-muted-foreground">
                  Wire API
                </Label>
                <Input
                  id="form-wire"
                  type="text"
                  value={formValues.wire_api}
                  onChange={(e) => setFormValues({ ...formValues, wire_api: e.target.value })}
                  placeholder="responses"
                  className="bg-card border-border focus-visible:ring-primary focus-visible:border-primary h-9 rounded-[4px] text-xs shadow-sm"
                />
              </div>

              <div className="space-y-1">
                <Label htmlFor="form-remark" className="text-xs font-bold text-muted-foreground">
                  Remark / Notes
                </Label>
                <Input
                  id="form-remark"
                  type="text"
                  value={formValues.remark}
                  onChange={(e) => setFormValues({ ...formValues, remark: e.target.value })}
                  placeholder="e.g. Backup relay"
                  className="bg-card border-border focus-visible:ring-primary focus-visible:border-primary h-9 rounded-[4px] text-xs shadow-sm"
                />
              </div>
            </div>

            <DialogFooter className="pt-3">
              <div className="flex w-full justify-end gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setShowForm(null)}
                  className="h-8 px-4 rounded-full border border-border bg-card text-xs font-semibold hover:bg-muted"
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  className="h-8 px-4 rounded-full bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/95 transition-transform active:scale-95 shadow-sm"
                >
                  {showForm === "add" ? "Create" : "Save Changes"}
                </Button>
              </div>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default App;
