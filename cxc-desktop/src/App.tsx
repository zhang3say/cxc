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
    <div className="min-h-screen bg-neutral-50 dark:bg-neutral-950 text-neutral-900 dark:text-neutral-50 font-sans transition-colors duration-200">
      {/* Header */}
      <header className="sticky top-0 z-40 w-full border-b border-neutral-200/80 dark:border-neutral-800/80 bg-white/80 dark:bg-neutral-900/80 backdrop-blur-md px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="relative flex items-center justify-center size-10 rounded-xl bg-neutral-950 dark:bg-neutral-50 text-neutral-50 dark:text-neutral-950 font-extrabold text-xl shadow-lg shadow-neutral-950/10 dark:shadow-neutral-50/5">
            C
            <span className="text-[10px] absolute bottom-1 right-1 font-bold text-neutral-400 dark:text-neutral-600">XC</span>
          </div>
          <div>
            <h1 className="text-lg font-bold tracking-tight">CXC Cross-Connect</h1>
            <p className="text-xs text-neutral-500 dark:text-neutral-400 font-medium">Relay Configuration Manager</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="icon"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            className="rounded-lg border-neutral-200 dark:border-neutral-800 hover:bg-neutral-100 dark:hover:bg-neutral-800"
            title="Toggle theme"
          >
            {theme === "dark" ? <Sun className="size-4 text-amber-500" /> : <Moon className="size-4 text-indigo-500" />}
          </Button>

          <Button
            variant="outline"
            size="icon"
            onClick={loadConfig}
            disabled={loading}
            className="rounded-lg border-neutral-200 dark:border-neutral-800 hover:bg-neutral-100 dark:hover:bg-neutral-800"
            title="Refresh configuration"
          >
            <RefreshCw className={`size-4 ${loading ? "animate-spin text-neutral-400" : ""}`} />
          </Button>

          <Button
            onClick={openAddForm}
            className="rounded-lg bg-neutral-900 dark:bg-neutral-50 text-neutral-50 dark:text-neutral-950 hover:bg-neutral-800 dark:hover:bg-neutral-200 shadow-sm"
          >
            <Plus className="size-4 mr-1.5" />
            Add Provider
          </Button>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-6xl mx-auto p-6 space-y-6">
        {/* Error Alert */}
        {error && (
          <div className="rounded-xl border border-red-200 bg-red-50/50 p-4 dark:border-red-900/50 dark:bg-red-950/20 text-red-700 dark:text-red-400 flex items-start gap-3 animate-in fade-in slide-in-from-top-2 duration-200">
            <AlertTriangle className="size-5 shrink-0 mt-0.5" />
            <div className="flex-1">
              <h5 className="font-semibold text-sm">System Alert</h5>
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
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-neutral-400 pointer-events-none" />
            <Input
              type="text"
              placeholder="Search by name, model, remark..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 h-9 w-full bg-white dark:bg-neutral-900 border-neutral-200 dark:border-neutral-800 rounded-lg placeholder-neutral-400 dark:placeholder-neutral-500 focus-visible:ring-neutral-950 dark:focus-visible:ring-neutral-50 focus-visible:ring-1"
            />
          </div>

          {config && config.providers.length > 0 && (
            <Button
              variant="outline"
              size="sm"
              onClick={handleTestAllProviders}
              disabled={testingAll || testingProvider !== null}
              className="w-full sm:w-auto h-9 border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 text-xs font-semibold rounded-lg hover:bg-neutral-100 dark:hover:bg-neutral-800"
            >
              {testingAll ? (
                <>
                  <Loader2 className="size-3.5 mr-1.5 animate-spin" />
                  Testing All...
                </>
              ) : (
                <>
                  <Activity className="size-3.5 mr-1.5 text-neutral-500" />
                  Test All Connections
                </>
              )}
            </Button>
          )}
        </div>

        {/* Loading State */}
        {loading && !config ? (
          <div className="flex flex-col items-center justify-center py-20 gap-4">
            <Loader2 className="size-8 animate-spin text-neutral-400 dark:text-neutral-600" />
            <p className="text-sm text-neutral-500 dark:text-neutral-400 font-medium">Fetching active configuration...</p>
          </div>
        ) : (
          <section className="space-y-4">
            {/* Grid of Providers */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
              {filteredProviders.map((p) => {
                const isActive = config?.active === p.name;
                const isThisSwitching = switching === p.name;
                const isTesting = testingProvider === p.name || (testingAll && !isActive);

                // Latency styling
                let latencyColor = "bg-neutral-100 text-neutral-700 dark:bg-neutral-800 dark:text-neutral-300";
                if (p.last_ok) {
                  if (p.latency_ms !== undefined) {
                    if (p.latency_ms < 150) {
                      latencyColor = "bg-emerald-50 dark:bg-emerald-950/30 text-emerald-600 dark:text-emerald-400 border border-emerald-200/55 dark:border-emerald-900/30";
                    } else if (p.latency_ms < 400) {
                      latencyColor = "bg-amber-50 dark:bg-amber-950/30 text-amber-600 dark:text-amber-400 border border-amber-200/55 dark:border-amber-900/30";
                    } else {
                      latencyColor = "bg-orange-50 dark:bg-orange-950/30 text-orange-600 dark:text-orange-400 border border-orange-200/55 dark:border-orange-900/30";
                    }
                  }
                } else if (p.last_ok === false) {
                  latencyColor = "bg-red-50 dark:bg-red-950/30 text-red-600 dark:text-red-400 border border-red-200/55 dark:border-red-900/30";
                }

                return (
                  <Card
                    key={p.name}
                    className={`relative flex flex-col justify-between overflow-hidden bg-white dark:bg-neutral-900 border transition-all duration-200 hover:shadow-md ${
                      isActive
                        ? "border-neutral-950 dark:border-neutral-50 ring-[1px] ring-neutral-950 dark:ring-neutral-50 shadow-sm"
                        : "border-neutral-200 dark:border-neutral-800"
                    } ${isTesting ? "opacity-75 animate-pulse" : ""}`}
                  >
                    <CardHeader className="pb-3 px-5 pt-5 gap-1.5">
                      <div className="flex items-start justify-between">
                        <div className="space-y-1">
                          <CardTitle className="text-base font-bold truncate pr-2" title={p.name}>
                            {p.name}
                          </CardTitle>
                          {p.remark ? (
                            <CardDescription className="text-xs font-medium text-neutral-400 dark:text-neutral-500 line-clamp-1">
                              {p.remark}
                            </CardDescription>
                          ) : (
                            <span className="block h-4" />
                          )}
                        </div>
                        {isActive && (
                          <Badge className="bg-neutral-950 dark:bg-neutral-50 text-neutral-50 dark:text-neutral-950 font-bold px-2 py-0.5 rounded-md hover:bg-neutral-950 dark:hover:bg-neutral-50 text-[10px]">
                            Active
                          </Badge>
                        )}
                      </div>
                    </CardHeader>

                    <CardContent className="space-y-3 px-5 pb-4 text-xs">
                      <div className="flex flex-col gap-1.5 p-3 rounded-lg bg-neutral-50 dark:bg-neutral-950/50 border border-neutral-100 dark:border-neutral-800/40">
                        <div className="flex justify-between items-center text-[11px]">
                          <span className="text-neutral-400 dark:text-neutral-500">Endpoint:</span>
                          <span className="font-mono text-neutral-600 dark:text-neutral-300 truncate max-w-[170px]" title={p.base_url}>
                            {p.base_url}
                          </span>
                        </div>
                        <div className="flex justify-between items-center text-[11px]">
                          <span className="text-neutral-400 dark:text-neutral-500">Model:</span>
                          <span className="font-semibold text-neutral-700 dark:text-neutral-200">{p.model}</span>
                        </div>
                        <div className="flex justify-between items-center text-[11px]">
                          <span className="text-neutral-400 dark:text-neutral-500">Key:</span>
                          <span className="font-mono text-neutral-500 dark:text-neutral-400">
                            {p.api_key.substring(0, 8)}••••••••
                          </span>
                        </div>
                      </div>

                      {/* Latency & Status info */}
                      <div className="flex items-center justify-between min-h-[26px]">
                        <span className="text-neutral-400 dark:text-neutral-500 text-[11px] font-medium">Latency:</span>
                        {isTesting ? (
                          <div className="flex items-center gap-1 text-[11px] text-amber-500 font-semibold">
                            <Loader2 className="size-3 animate-spin" />
                            testing...
                          </div>
                        ) : p.latency_ms !== undefined && p.last_ok !== undefined ? (
                          <div className="flex items-center gap-2">
                            <Badge className={`px-2 py-0.5 rounded text-[10px] font-bold ${latencyColor}`}>
                              {p.last_ok ? `${p.latency_ms} ms` : "Offline"}
                            </Badge>
                            {p.last_test && (
                              <span className="text-[10px] text-neutral-400 dark:text-neutral-500">
                                {formatDate(p.last_test)}
                              </span>
                            )}
                          </div>
                        ) : (
                          <span className="text-[10px] text-neutral-400 dark:text-neutral-500 font-medium italic">
                            Never tested
                          </span>
                        )}
                      </div>
                    </CardContent>

                    {/* Actions footer */}
                    <div className="flex items-center gap-1 border-t border-neutral-100 dark:border-neutral-800/40 bg-neutral-50/50 dark:bg-neutral-900/50 p-3 justify-between">
                      <div className="flex items-center gap-1.5">
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => handleTestProvider(p.name)}
                          disabled={testingProvider !== null || testingAll || switching !== null}
                          className="rounded-md hover:bg-neutral-200 dark:hover:bg-neutral-800 text-neutral-500 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-50"
                          title="Test connection"
                        >
                          <Activity className="size-3.5" />
                        </Button>

                        <Button
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => openEditForm(p)}
                          disabled={testingProvider !== null || testingAll || switching !== null}
                          className="rounded-md hover:bg-neutral-200 dark:hover:bg-neutral-800 text-neutral-500 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-50"
                          title="Edit provider"
                        >
                          <Edit2 className="size-3.5" />
                        </Button>

                        <Button
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => handleDeleteProvider(p.name)}
                          disabled={isActive || switching !== null || testingAll || testingProvider !== null}
                          className="rounded-md text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-950/20 disabled:opacity-40"
                          title={isActive ? "Active provider cannot be deleted" : "Delete provider"}
                        >
                          <Trash2 className="size-3.5" />
                        </Button>
                      </div>

                      <div className="w-24">
                        {!isActive ? (
                          <Button
                            size="sm"
                            onClick={() => handleSwitch(p.name)}
                            disabled={switching !== null || testingAll || testingProvider !== null}
                            className="w-full text-[11px] h-7 bg-neutral-900 dark:bg-neutral-50 text-neutral-50 dark:text-neutral-950 rounded-md hover:bg-neutral-800 dark:hover:bg-neutral-200 font-semibold"
                          >
                            {isThisSwitching ? (
                              <Loader2 className="size-3 animate-spin" />
                            ) : (
                              "Switch"
                            )}
                          </Button>
                        ) : (
                          <div className="flex items-center justify-center gap-1 h-7 border border-emerald-500/20 dark:border-emerald-400/20 bg-emerald-50/50 dark:bg-emerald-950/10 text-emerald-600 dark:text-emerald-400 rounded-md text-[10px] font-bold px-2 py-0.5">
                            <Check className="size-3 shrink-0" />
                            Active
                          </div>
                        )}
                      </div>
                    </div>
                  </Card>
                );
              })}

              {filteredProviders.length === 0 && (
                <div className="col-span-full py-16 flex flex-col items-center justify-center gap-3 border-2 border-dashed border-neutral-200 dark:border-neutral-800 rounded-xl bg-white dark:bg-neutral-900/50">
                  <Compass className="size-10 text-neutral-300 dark:text-neutral-700" />
                  <div className="text-center">
                    <p className="text-sm font-semibold text-neutral-500 dark:text-neutral-400">No providers found</p>
                    <p className="text-xs text-neutral-400 dark:text-neutral-500 mt-1">
                      {searchQuery ? "Try altering your search filters" : "Create a new provider to get started."}
                    </p>
                  </div>
                  {!searchQuery && (
                    <Button onClick={openAddForm} className="mt-2" size="sm">
                      <Plus className="size-3.5 mr-1" /> Add Provider
                    </Button>
                  )}
                </div>
              )}
            </div>
          </section>
        )}
      </main>

      {/* Add / Edit Dialog Modal */}
      <Dialog open={showForm !== null} onOpenChange={(open) => !open && setShowForm(null)}>
        <DialogContent className="sm:max-w-md bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-xl shadow-2xl p-5 gap-4">
          <DialogHeader className="gap-1">
            <DialogTitle className="text-lg font-bold">
              {showForm === "add" ? "Create Provider" : "Edit Provider"}
            </DialogTitle>
            <DialogDescription className="text-xs text-neutral-400 dark:text-neutral-500">
              Provide endpoint details for the cross-connect proxy relay.
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSubmitForm} className="space-y-4">
            <div className="space-y-1">
              <Label htmlFor="form-name" className="text-xs font-bold text-neutral-500 dark:text-neutral-400">
                Provider Name *
              </Label>
              <Input
                id="form-name"
                type="text"
                required
                value={formValues.name}
                onChange={(e) => setFormValues({ ...formValues, name: e.target.value })}
                placeholder="e.g. proxy-fast"
                className="bg-neutral-50 dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 focus-visible:ring-neutral-950 dark:focus-visible:ring-neutral-50 h-9 rounded-lg"
              />
            </div>

            <div className="space-y-1">
              <Label htmlFor="form-url" className="text-xs font-bold text-neutral-500 dark:text-neutral-400">
                Base URL *
              </Label>
              <Input
                id="form-url"
                type="url"
                required
                value={formValues.base_url}
                onChange={(e) => setFormValues({ ...formValues, base_url: e.target.value })}
                placeholder="https://api.example.com/v1"
                className="bg-neutral-50 dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 focus-visible:ring-neutral-950 dark:focus-visible:ring-neutral-50 h-9 rounded-lg"
              />
            </div>

            <div className="space-y-1">
              <Label htmlFor="form-key" className="text-xs font-bold text-neutral-500 dark:text-neutral-400">
                API Key *
              </Label>
              <Input
                id="form-key"
                type="password"
                required
                value={formValues.api_key}
                onChange={(e) => setFormValues({ ...formValues, api_key: e.target.value })}
                placeholder="sk-••••••••••••"
                className="bg-neutral-50 dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 focus-visible:ring-neutral-950 dark:focus-visible:ring-neutral-50 h-9 rounded-lg"
              />
            </div>

            <div className="space-y-1">
              <div className="flex items-center justify-between pb-1">
                <Label htmlFor="form-model" className="text-xs font-bold text-neutral-500 dark:text-neutral-400">
                  Model *
                </Label>
                <Button
                  type="button"
                  variant="outline"
                  size="xs"
                  onClick={handleFetchModels}
                  disabled={fetchingModels || !formValues.base_url || !formValues.api_key}
                  className="h-6 text-[10px] border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 text-neutral-600 dark:text-neutral-300 font-bold"
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
                  className="flex h-9 w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50 dark:border-neutral-800 dark:bg-neutral-950 text-neutral-900 dark:text-neutral-50 focus-visible:ring-neutral-950 dark:focus-visible:ring-neutral-50"
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
                  className="bg-neutral-50 dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 focus-visible:ring-neutral-950 dark:focus-visible:ring-neutral-50 h-9 rounded-lg"
                />
              )}
              {fetchError && <span className="text-[10px] text-red-500 font-medium block mt-1">{fetchError}</span>}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1">
                <Label htmlFor="form-wire" className="text-xs font-bold text-neutral-500 dark:text-neutral-400">
                  Wire API
                </Label>
                <Input
                  id="form-wire"
                  type="text"
                  value={formValues.wire_api}
                  onChange={(e) => setFormValues({ ...formValues, wire_api: e.target.value })}
                  placeholder="responses"
                  className="bg-neutral-50 dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 focus-visible:ring-neutral-950 dark:focus-visible:ring-neutral-50 h-9 rounded-lg text-xs"
                />
              </div>

              <div className="space-y-1">
                <Label htmlFor="form-remark" className="text-xs font-bold text-neutral-500 dark:text-neutral-400">
                  Remark / Notes
                </Label>
                <Input
                  id="form-remark"
                  type="text"
                  value={formValues.remark}
                  onChange={(e) => setFormValues({ ...formValues, remark: e.target.value })}
                  placeholder="e.g. Backup relay"
                  className="bg-neutral-50 dark:bg-neutral-950 border-neutral-200 dark:border-neutral-800 focus-visible:ring-neutral-950 dark:focus-visible:ring-neutral-50 h-9 rounded-lg text-xs"
                />
              </div>
            </div>

            <DialogFooter className="pt-2">
              <div className="flex w-full justify-end gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setShowForm(null)}
                  className="h-9 rounded-lg border-neutral-200 dark:border-neutral-800 text-xs font-bold"
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  className="h-9 rounded-lg bg-neutral-900 dark:bg-neutral-50 text-neutral-50 dark:text-neutral-950 hover:bg-neutral-800 dark:hover:bg-neutral-200 text-xs font-bold"
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
