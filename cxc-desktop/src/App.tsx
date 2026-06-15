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
  Settings,
  Globe,
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
  codex_source?: string;
  codex_custom_dir?: string;
}

const locales = {
  zh: {
    subtitle: "中转节点配置管理器",
    settings: "设置",
    toggleTheme: "切换主题",
    refreshConfig: "刷新配置",
    addProvider: "添加中转节点",
    searchPlaceholder: "搜索名称、模型、备注...",
    listView: "列表视图",
    cardView: "卡片视图",
    testAll: "一键测速",
    testingAll: "测试中...",
    activeCol: "已启用",
    providerNameCol: "服务商名称",
    endpointCol: "接口地址",
    modelCol: "模型名称",
    statusLatencyCol: "状态与延迟",
    actionsCol: "操作",
    neverTested: "未测试",
    testing: "测试中...",
    offline: "离线",
    activeLabel: "启用",
    activeTray: "Active",
    testConnTitle: "测试连接",
    editConnTitle: "修改节点",
    deleteConnTitle: "删除节点",
    activeDeleteTooltip: "启用的中转节点不可删除",
    switchBtn: "启用",
    systemAlert: "系统警报",
    noProviders: "未找到中转节点",
    tryAlteringSearch: "尝试修改您的搜索过滤条件",
    createFirst: "请配置您的第一个 API 中转节点以开始使用。",
    settingsTitle: "CXC 系统设置",
    settingsDesc: "配置 Codex 配置文件写入的相关设置。",
    codexSourceLabel: "Codex 来源配置",
    desktopAppOption: "Desktop 客户端",
    desktopAppDesc: "Windows 客户端 (~/.codex)",
    wslCliOption: "WSL 命令行",
    wslCliDesc: "WSL 子系统环境路径",
    customDirLabel: "Codex 自定义目录",
    wslRecommended: "WSL 环境推荐",
    wslPlaceholder: "例如: \\\\wsl.localhost\\Ubuntu\\home\\username\\.codex",
    appPlaceholder: "可选的自定义路径",
    wslNote: "WSL 注意事项: 请指定 WSL 中 .codex 文件夹的绝对 UNC network路径，以便 Windows 端的 CXC 能够成功写入配置文件。",
    appNote: "若留空，则默认使用您当前的用户家目录 (~/.codex)。",
    cancelBtn: "取消",
    saveSettingsBtn: "保存设置",
    savingSettingsBtn: "保存中...",
    createProviderTitle: "新建中转节点",
    editProviderTitle: "编辑中转节点",
    createProviderDesc: "配置一个新的 API 中转代理端点。",
    editProviderDesc: "更新中转代理节点的配置参数。",
    nameLabel: "中转站名称 *",
    namePlaceholder: "例如: DeepSeek Backup",
    baseUrlLabel: "Base URL (API 端点地址) *",
    apiKeyLabel: "API Key (机密密钥) *",
    modelLabel: "模型标识符 *",
    discoverModels: "发现模型",
    discovering: "拉取中...",
    wireApiLabel: "传输协议 Wire API",
    remarkLabel: "备注信息 / Notes",
    remarkPlaceholder: "例如: 高速备用节点",
    createBtn: "新建节点",
    saveChangesBtn: "保存修改",
    selectModelDefault: "-- 选择一个模型 --",
    confirmDelete: (name: string) => `确定要删除中转节点 "${name}" 吗？`,
    fillRequired: "请填写所有必填字段（名称、Base URL、API Key、模型）",
    fillRequiredDiscovery: "请先填写 Base URL 和 API Key",
    noModelsReturned: "接口未返回任何可用模型",
    fetchingConfig: "正在获取活动配置中..."
  },
  en: {
    subtitle: "Relay Configuration Manager",
    settings: "Settings",
    toggleTheme: "Toggle theme",
    refreshConfig: "Refresh configuration",
    addProvider: "Add Provider",
    searchPlaceholder: "Search by name, model, remark...",
    listView: "List View",
    cardView: "Card View",
    testAll: "Test All Connections",
    testingAll: "Testing...",
    activeCol: "Active",
    providerNameCol: "Provider Name",
    endpointCol: "Endpoint",
    modelCol: "Model",
    statusLatencyCol: "Status & Latency",
    actionsCol: "Actions",
    neverTested: "Never tested",
    testing: "testing...",
    offline: "Offline",
    activeLabel: "Active",
    activeTray: "Active",
    testConnTitle: "Test connection",
    editConnTitle: "Edit provider",
    deleteConnTitle: "Delete provider",
    activeDeleteTooltip: "Active provider cannot be deleted",
    switchBtn: "Switch",
    systemAlert: "System Alert",
    noProviders: "No providers found",
    tryAlteringSearch: "Try altering your search filters",
    createFirst: "Create a new provider to get started.",
    settingsTitle: "CXC System Settings",
    settingsDesc: "Configure global CXC settings and Target Tool sources.",
    codexSourceLabel: "Codex Source (Codex 来源)",
    desktopAppOption: "Desktop App",
    desktopAppDesc: "Desktop version (~/.codex)",
    wslCliOption: "WSL CLI",
    wslCliDesc: "WSL environment paths",
    customDirLabel: "Codex Custom Directory (自定义目录)",
    wslRecommended: "Recommended for WSL",
    wslPlaceholder: "e.g. \\\\wsl.localhost\\Ubuntu\\home\\username\\.codex",
    appPlaceholder: "Optional custom path",
    wslNote: "WSL Note: Please specify the absolute UNC network path to your WSL .codex folder so CXC on Windows can write config files successfully.",
    appNote: "Defaults to your home directory (~/.codex) if left blank.",
    cancelBtn: "Cancel",
    saveSettingsBtn: "Save Settings",
    savingSettingsBtn: "Saving...",
    createProviderTitle: "Create Provider",
    editProviderTitle: "Edit Provider",
    createProviderDesc: "Provide endpoint details for the cross-connect proxy relay.",
    editProviderDesc: "Update settings for relay provider.",
    nameLabel: "Provider Name *",
    namePlaceholder: "e.g. proxy-fast",
    baseUrlLabel: "Base URL *",
    apiKeyLabel: "API Key *",
    modelLabel: "Model *",
    discoverModels: "Discover Models",
    discovering: "Loading...",
    wireApiLabel: "Wire API",
    remarkLabel: "Remark / Notes",
    remarkPlaceholder: "e.g. Backup relay",
    createBtn: "Create",
    saveChangesBtn: "Save Changes",
    selectModelDefault: "-- Select a model --",
    confirmDelete: (name: string) => `Are you sure you want to remove provider "${name}"?`,
    fillRequired: "Please fill in all required fields (Name, Base URL, API Key, Model)",
    fillRequiredDiscovery: "Please fill in Base URL and API Key first",
    noModelsReturned: "No models returned from endpoint",
    fetchingConfig: "Fetching active configuration..."
  }
};

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

  const [lang, setLang] = useState<"zh" | "en">(( ) => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("cxc-lang");
      if (saved === "zh" || saved === "en") return saved;
      // Default to user system language if possible
      return window.navigator.language.startsWith("zh") ? "zh" : "en";
    }
    return "zh";
  });

  useEffect(() => {
    localStorage.setItem("cxc-lang", lang);
  }, [lang]);

  const t = locales[lang];

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

  // Settings State
  const [showSettings, setShowSettings] = useState<boolean>(false);
  const [settingsSource, setSettingsSource] = useState<string>("app");
  const [settingsCustomDir, setSettingsCustomDir] = useState<string>("");
  const [savingSettings, setSavingSettings] = useState<boolean>(false);
  const [appVersion, setAppVersion] = useState<string>("");

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

    invoke<string>("get_app_version")
      .then(setAppVersion)
      .catch((err) => console.error("Failed to get version:", err));

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
      // Initialize settings from loaded config
      setSettingsSource(cfg.codex_source || "app");
      setSettingsCustomDir(cfg.codex_custom_dir || "");
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
    if (!confirm(t.confirmDelete(name))) {
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

  async function handleSaveSettings(e: React.FormEvent) {
    e.preventDefault();
    try {
      setSavingSettings(true);
      setError(null);
      const updatedCfg = await invoke<Config>("save_settings", {
        source: settingsSource,
        customDir: settingsCustomDir,
      });
      setConfig(updatedCfg);
      setShowSettings(false);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setSavingSettings(false);
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

  // Latency & status render helper using Notion's decorative sticker palette + dot indicator
  const renderStatus = (p: Provider, isTesting: boolean) => {
    let dotClass = "bg-muted-foreground/30";
    let badgeStyle = "bg-muted text-muted-foreground/80 border-border dark:bg-muted/30 dark:text-muted-foreground/60 dark:border-border/50";
    let label = t.neverTested;

    if (isTesting) {
      dotClass = "bg-sticker-orange animate-ping";
      badgeStyle = "bg-sticker-orange/10 text-sticker-orange border-sticker-orange/20 dark:bg-sticker-orange/20 dark:text-orange-400 dark:border-sticker-orange/30";
      label = t.testing;
    } else if (p.last_ok) {
      if (p.latency_ms !== undefined) {
        if (p.latency_ms < 150) {
          dotClass = "bg-sticker-green";
          badgeStyle = "bg-sticker-green/10 text-sticker-green border-sticker-green/20 dark:bg-sticker-green/20 dark:text-emerald-400 dark:border-sticker-green/30";
        } else if (p.latency_ms < 400) {
          dotClass = "bg-yellow-500 dark:bg-yellow-400";
          badgeStyle = "bg-yellow-500/10 text-yellow-600 border-yellow-500/20 dark:bg-yellow-950/20 dark:text-yellow-400 dark:border-yellow-900/30";
        } else {
          dotClass = "bg-sticker-orange";
          badgeStyle = "bg-sticker-orange/15 text-sticker-orange border-sticker-orange/20 dark:bg-sticker-orange/25 dark:text-orange-400 dark:border-sticker-orange/30";
        }
        label = `${p.latency_ms} ms`;
      }
    } else if (p.last_ok === false) {
      dotClass = "bg-red-500";
      badgeStyle = "bg-red-500/10 text-red-600 border-red-500/20 dark:bg-red-950/20 dark:text-red-400 dark:border-red-900/30";
      label = t.offline;
    }

    return (
      <div className="flex flex-col gap-1 items-start">
        <Badge className={`px-2 py-0.5 rounded-full text-[10px] font-bold border flex items-center gap-1.5 shadow-none tracking-normal ${badgeStyle}`}>
          {isTesting ? (
            <Loader2 className="size-2.5 animate-spin text-sticker-orange shrink-0" />
          ) : (
            <span className={`size-1.5 rounded-full shrink-0 ${dotClass}`} />
          )}
          <span>{label}</span>
        </Badge>
        {!isTesting && p.last_test && (
          <span className="text-[10px] text-muted-foreground/60 font-medium pl-1">
            {formatDate(p.last_test)}
          </span>
        )}
      </div>
    );
  };

  return (
    <div className="min-h-screen bg-background text-foreground transition-colors duration-200">
      {/* Notion Document Navigation Chrome */}
      <header className="sticky top-0 z-40 w-full border-b border-border bg-card/90 backdrop-blur-md px-6 py-4 flex items-center justify-between shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        <div className="flex items-center gap-3.5">
          {/* Logo with official CXC image */}
          <div className="relative flex items-center justify-center size-10 rounded-xl bg-card border border-border shadow-sm overflow-hidden transition-transform hover:rotate-3 duration-200">
            <img src="/logo.png" alt="CXC Logo" className="size-full object-cover" />
          </div>
          <div>
            <h1 className="text-base font-bold tracking-tight text-foreground/95 flex items-center gap-1.5">
              CXC Cross-Connect
            </h1>
            <p className="text-xs text-muted-foreground font-medium">{t.subtitle}</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Language Toggle Button */}
          <Button
            variant="outline"
            size="icon"
            onClick={() => setLang(lang === "zh" ? "en" : "zh")}
            className="size-8 rounded-md border-border hover:bg-muted text-muted-foreground hover:text-foreground shadow-sm transition-all duration-200"
            title={lang === "zh" ? "Switch to English" : "切换为中文"}
          >
            <Globe className="size-4" />
          </Button>

          {/* Settings gear button */}
          <Button
            variant="outline"
            size="icon"
            onClick={() => setShowSettings(true)}
            className="size-8 rounded-md border-border hover:bg-muted text-muted-foreground hover:text-foreground shadow-sm transition-all duration-200"
            title={t.settings}
          >
            <Settings className="size-4" />
          </Button>

          {/* Theme switcher styled as utility button */}
          <Button
            variant="outline"
            size="icon"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            className="size-8 rounded-md border-border hover:bg-muted text-muted-foreground hover:text-foreground shadow-sm transition-all duration-200"
            title={t.toggleTheme}
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
            title={t.refreshConfig}
          >
            <RefreshCw className={`size-4 ${loading ? "animate-spin text-muted-foreground" : ""}`} />
          </Button>

          {/* Primary CTA: Pill-shaped in Notion Blue */}
          <Button
            onClick={openAddForm}
            className="h-8 px-4 rounded-full bg-primary text-primary-foreground hover:bg-primary/90 hover:scale-[1.02] active:scale-95 shadow-sm font-medium text-xs transition-all duration-200"
          >
            <Plus className="size-3.5 mr-1" />
            {t.addProvider}
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
              <h5 className="font-bold text-xs tracking-tight uppercase text-sticker-orange">{t.systemAlert}</h5>
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
              placeholder={t.searchPlaceholder}
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
                title={t.listView}
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
                title={t.cardView}
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
                    {t.testingAll}
                  </>
                ) : (
                  <>
                    <Activity className="size-3.5 mr-1.5 text-muted-foreground" />
                    {t.testAll}
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
            <p className="text-xs text-muted-foreground font-medium">{t.fetchingConfig}</p>
          </div>
        ) : (
          <section className="space-y-4">
            {/* Conditional list/card rendering */}
            {filteredProviders.length > 0 ? (
              viewMode === "list" ? (
                /* Table Database Layout */
                <div className="border border-border rounded-xl bg-card overflow-hidden divide-y divide-border shadow-[0_1px_3px_rgba(0,0,0,0.02)]">
                  {/* Table Header */}
                  <div className="hidden sm:flex items-center px-4 py-2 bg-muted/40 text-[10px] font-bold text-muted-foreground uppercase tracking-wider border-b border-border/60 gap-4">
                    <div className="w-6 shrink-0" />
                    <div className="w-40 shrink-0 pr-4">{t.providerNameCol}</div>
                    <div className="flex-1 min-w-0 pr-4">{t.endpointCol}</div>
                    <div className="w-32 shrink-0 pr-4">{t.modelCol}</div>
                    <div className="w-36 shrink-0 pr-4">{t.statusLatencyCol}</div>
                    <div className="w-24 shrink-0 text-center pr-4">{t.actionsCol}</div>
                    <div className="w-24 shrink-0 text-center">{t.activeCol}</div>
                  </div>

                  {filteredProviders.map((p) => {
                    const isActive = config?.active === p.name;
                    const isThisSwitching = switching === p.name;
                    const isTesting = testingProvider === p.name || (testingAll && !isActive);

                    return (
                      <div
                        key={p.name}
                        className={`flex flex-col sm:flex-row sm:items-center px-4 py-2.5 gap-3 sm:gap-4 transition-colors hover:bg-muted/30 group relative ${
                          isActive ? "bg-primary/[0.02] dark:bg-primary/[0.04]" : ""
                        } ${isTesting ? "opacity-80" : ""}`}
                      >
                        {isActive && <div className="absolute left-0 top-0 bottom-0 w-0.5 bg-primary" />}

                        {/* Active dot column */}
                        <div className="hidden sm:flex items-center justify-center w-6 size-5 shrink-0">
                          {isActive ? (
                            <span className="size-1.5 rounded-full bg-primary animate-pulse" />
                          ) : (
                            <span className="size-1.5 rounded-full bg-muted-foreground/30" />
                          )}
                        </div>

                        {/* Name & remark column */}
                        <div className="w-full sm:w-40 shrink-0 truncate pr-4">
                          <span className="text-sm font-bold text-foreground/90 tracking-tight block truncate" title={p.name}>
                            {p.name}
                          </span>
                          {p.remark && (
                            <p className="text-[11px] text-muted-foreground/75 truncate mt-0.5" title={p.remark}>
                              {p.remark}
                            </p>
                          )}
                        </div>

                        {/* Base URL column */}
                        <div className="flex-1 min-w-0 pr-4 text-xs font-mono text-foreground/70 truncate flex items-center gap-1.5">
                          <span className="text-muted-foreground/60 text-[10px] uppercase font-bold tracking-wider sm:hidden">Endpoint:</span>
                          <span className="truncate" title={p.base_url}>{p.base_url}</span>
                        </div>

                        {/* Model column */}
                        <div className="w-full sm:w-32 shrink-0 pr-4 text-xs flex items-center gap-1.5">
                          <span className="text-muted-foreground/60 text-[10px] uppercase font-bold tracking-wider sm:hidden">Model:</span>
                          <span className="font-semibold text-foreground/80 bg-background border border-border px-1.5 py-0.5 rounded text-[11px] truncate" title={p.model}>
                            {p.model}
                          </span>
                        </div>

                        {/* Latency & Status column using renderStatus */}
                        <div className="w-full sm:w-36 shrink-0 pr-4 text-xs flex items-center gap-1.5">
                          <span className="text-muted-foreground/60 text-[10px] uppercase font-bold tracking-wider sm:hidden">Status:</span>
                          {renderStatus(p, isTesting)}
                        </div>

                        {/* Quiet action buttons column */}
                        <div className="w-full sm:w-24 shrink-0 pr-4 flex items-center justify-end sm:justify-center gap-1">
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={() => handleTestProvider(p.name)}
                            disabled={testingProvider !== null || testingAll || switching !== null}
                            className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                            title={t.testConnTitle}
                          >
                            <Activity className="size-3" />
                          </Button>

                          <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={() => openEditForm(p)}
                            disabled={testingProvider !== null || testingAll || switching !== null}
                            className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                            title={t.editConnTitle}
                          >
                            <Edit2 className="size-3" />
                          </Button>

                          <Button
                            variant="ghost"
                            size="icon-sm"
                            onClick={() => handleDeleteProvider(p.name)}
                            disabled={isActive || switching !== null || testingAll || testingProvider !== null}
                            className="size-6 rounded-md text-red-500 hover:text-red-600 hover:bg-red-500/10 dark:hover:bg-red-950/20 disabled:opacity-40 transition-colors"
                            title={isActive ? t.activeDeleteTooltip : t.deleteConnTitle}
                          >
                            <Trash2 className="size-3" />
                          </Button>
                        </div>

                        {/* Switch Active Trigger column */}
                        <div className="w-full sm:w-24 shrink-0 flex justify-end">
                          {!isActive ? (
                            <Button
                              size="sm"
                              onClick={() => handleSwitch(p.name)}
                              disabled={switching !== null || testingAll || testingProvider !== null}
                              className="w-full text-[10px] h-6 bg-card text-foreground border border-border rounded hover:bg-muted font-medium shadow-none transition-all duration-200 active:scale-95"
                            >
                              {isThisSwitching ? (
                                <Loader2 className="size-2.5 animate-spin text-muted-foreground" />
                              ) : (
                                t.switchBtn
                              )}
                            </Button>
                          ) : (
                            <div className="flex items-center justify-center gap-1 h-6 border border-primary/20 bg-primary/5 text-primary rounded-full text-[10px] font-bold px-2.5 py-0.5 w-full">
                              <Check className="size-2.5 shrink-0" />
                              {t.activeLabel}
                            </div>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              ) : (
                /* Card Grid View Layout */
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {filteredProviders.map((p) => {
                    const isActive = config?.active === p.name;
                    const isThisSwitching = switching === p.name;
                    const isTesting = testingProvider === p.name || (testingAll && !isActive);

                    return (
                      <Card
                        key={p.name}
                        className={`relative flex flex-col justify-between overflow-hidden bg-card border transition-all duration-200 hover:shadow-[0_2px_8px_rgba(0,0,0,0.02)] hover:-translate-y-[1px] ${
                          isActive
                            ? "border-primary/45 border-l-4 border-l-primary ring-1 ring-primary/10 shadow-sm"
                            : "border-border"
                        } ${isTesting ? "opacity-80 animate-pulse" : ""}`}
                      >
                        {/* Header of Card */}
                        <CardHeader className="pb-2 px-4 pt-4 gap-0.5">
                          <div className="flex items-start justify-between gap-1.5">
                            <div className="space-y-0.5 min-w-0 flex-1">
                              <CardTitle className="text-sm font-bold tracking-tight text-foreground/90 truncate pr-1 flex items-center gap-1.5" title={p.name}>
                                {/* Blue decorative dot for active card */}
                                {isActive && (
                                  <span className="size-1.5 rounded-full bg-primary animate-pulse shrink-0" />
                                )}
                                <span className="truncate">{p.name}</span>
                              </CardTitle>
                              {p.remark && (
                                <CardDescription className="text-[11px] text-muted-foreground/75 font-medium line-clamp-1">
                                  {p.remark}
                                </CardDescription>
                              )}
                            </div>
                            {isActive && (
                              <span className="px-1.5 py-0.5 rounded bg-primary/10 text-primary border border-primary/20 text-[8px] font-bold tracking-wide uppercase shrink-0">
                                {t.activeLabel}
                              </span>
                            )}
                          </div>
                        </CardHeader>

                        {/* Card Content: Details Block */}
                        <CardContent className="space-y-2.5 px-4 pb-3 text-xs">
                          {/* Monospace layout list, styled with paper canvas background */}
                          <div className="flex flex-col gap-1 p-2 rounded-md bg-background border border-border">
                            <div className="flex justify-between items-center text-[10.5px] gap-2">
                              <span className="text-muted-foreground/80 font-medium">Endpoint:</span>
                              <span className="font-mono text-foreground/75 truncate max-w-[150px]" title={p.base_url}>
                                {p.base_url}
                              </span>
                            </div>
                            <div className="flex justify-between items-center text-[10.5px] gap-2">
                              <span className="text-muted-foreground/80 font-medium">Model:</span>
                              <span className="font-semibold text-foreground/80 truncate max-w-[150px]" title={p.model}>{p.model}</span>
                            </div>
                            <div className="flex justify-between items-center text-[10.5px] gap-2">
                              <span className="text-muted-foreground/80 font-medium">Key:</span>
                              <span className="font-mono text-muted-foreground/80">
                                {p.api_key.substring(0, 8)}••••••••
                              </span>
                            </div>
                          </div>

                          {/* Latency & Status Info */}
                          <div className="flex items-center justify-between min-h-[22px]">
                            <span className="text-muted-foreground/85 text-[10.5px] font-medium">Latency:</span>
                            {renderStatus(p, isTesting)}
                          </div>
                        </CardContent>

                        {/* Actions Footer - Quiet utility strip */}
                        <div className="flex items-center gap-1 border-t border-border bg-background/50 p-2 px-4 justify-between">
                          {/* Left: action icons */}
                          <div className="flex items-center gap-1">
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleTestProvider(p.name)}
                              disabled={testingProvider !== null || testingAll || switching !== null}
                              className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title={t.testConnTitle}
                            >
                              <Activity className="size-3" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => openEditForm(p)}
                              disabled={testingProvider !== null || testingAll || switching !== null}
                              className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title={t.editConnTitle}
                            >
                              <Edit2 className="size-3" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleDeleteProvider(p.name)}
                              disabled={isActive || switching !== null || testingAll || testingProvider !== null}
                              className="size-6 rounded-md text-red-500 hover:text-red-600 hover:bg-red-500/10 dark:hover:bg-red-950/20 disabled:opacity-40 transition-colors"
                              title={isActive ? t.activeDeleteTooltip : t.deleteConnTitle}
                            >
                              <Trash2 className="size-3" />
                            </Button>
                          </div>

                          {/* Right: Switch active button */}
                          <div className="w-20 flex justify-end">
                            {!isActive ? (
                              <Button
                                size="sm"
                                onClick={() => handleSwitch(p.name)}
                                disabled={switching !== null || testingAll || testingProvider !== null}
                                className="w-full text-[10px] h-6 bg-card text-foreground border border-border rounded hover:bg-muted font-medium shadow-none transition-all duration-200 active:scale-95"
                              >
                                {isThisSwitching ? (
                                  <Loader2 className="size-2.5 animate-spin text-muted-foreground" />
                                ) : (
                                  t.switchBtn
                                )}
                              </Button>
                            ) : (
                              <div className="flex items-center justify-center gap-1 h-6 border border-primary/20 bg-primary/5 text-primary rounded-full text-[10px] font-bold px-2.5 py-0.5">
                                <Check className="size-2.5 shrink-0" />
                                {t.activeLabel}
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
                  <p className="text-sm font-bold text-foreground/85">{t.noProviders}</p>
                  <p className="text-xs text-muted-foreground mt-1.5">
                    {searchQuery ? t.tryAlteringSearch : t.createFirst}
                  </p>
                </div>
                {!searchQuery && (
                  <Button onClick={openAddForm} className="mt-3 h-8 px-4 rounded-full bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/95 transition-transform active:scale-95 shadow-sm">
                    <Plus className="size-3.5 mr-1" /> {t.addProvider}
                  </Button>
                )}
              </div>
            )}
          </section>
        )}
      </main>

      {/* Settings Dialog Modal (Notion-style Settings) */}
      <Dialog open={showSettings} onOpenChange={(open) => !open && setShowSettings(false)}>
        <DialogContent className="sm:max-w-md bg-card border border-border rounded-xl shadow-xl p-5 gap-4">
          <DialogHeader className="gap-1.5">
            <DialogTitle className="text-base font-bold tracking-tight">
              {t.settingsTitle}
            </DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              {t.settingsDesc}
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSaveSettings} className="space-y-4">
            <div className="space-y-2">
              <Label className="text-xs font-bold text-muted-foreground">
                {t.codexSourceLabel}
              </Label>
              <div className="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onClick={() => setSettingsSource("app")}
                  className={`flex items-center justify-between p-3 rounded-lg border text-left transition-all ${
                    settingsSource === "app"
                      ? "border-primary bg-primary/[0.03] text-primary"
                      : "border-border bg-card text-foreground hover:bg-muted/30"
                  }`}
                >
                  <div>
                    <span className="text-xs font-bold block">{t.desktopAppOption}</span>
                    <span className="text-[10px] text-muted-foreground block mt-0.5">{t.desktopAppDesc}</span>
                  </div>
                  {settingsSource === "app" && <Check className="size-4 shrink-0" />}
                </button>

                <button
                  type="button"
                  onClick={() => setSettingsSource("wsl")}
                  className={`flex items-center justify-between p-3 rounded-lg border text-left transition-all ${
                    settingsSource === "wsl"
                      ? "border-primary bg-primary/[0.03] text-primary"
                      : "border-border bg-card text-foreground hover:bg-muted/30"
                  }`}
                >
                  <div>
                    <span className="text-xs font-bold block">{t.wslCliOption}</span>
                    <span className="text-[10px] text-muted-foreground block mt-0.5">{t.wslCliDesc}</span>
                  </div>
                  {settingsSource === "wsl" && <Check className="size-4 shrink-0" />}
                </button>
              </div>
            </div>

            {/* Custom directory input field */}
            <div className="space-y-1">
              <Label htmlFor="settings-custom-dir" className="text-xs font-bold text-muted-foreground flex items-center justify-between">
                <span>{t.customDirLabel}</span>
                {settingsSource === "wsl" && <span className="text-[10px] text-primary font-semibold">({t.wslRecommended})</span>}
              </Label>
              <Input
                id="settings-custom-dir"
                type="text"
                value={settingsCustomDir}
                onChange={(e) => setSettingsCustomDir(e.target.value)}
                placeholder={settingsSource === "wsl" ? t.wslPlaceholder : t.appPlaceholder}
                className="bg-card border-border focus-visible:ring-primary focus-visible:border-primary h-9 rounded-[4px] text-sm shadow-sm transition-all"
              />
              <p className="text-[10px] text-muted-foreground leading-normal mt-1">
                {settingsSource === "wsl" ? t.wslNote : t.appNote}
              </p>
            </div>

            <DialogFooter className="pt-3">
              <div className="flex w-full items-center justify-between gap-2">
                {appVersion && (
                  <span className="text-[10px] text-muted-foreground/40 font-medium">
                    v{appVersion}
                  </span>
                )}
                <div className="flex gap-2 justify-end ml-auto">
                  <Button
                  type="button"
                  variant="outline"
                  onClick={() => setShowSettings(false)}
                  className="h-8 px-4 rounded-full border border-border bg-card text-xs font-semibold hover:bg-muted"
                >
                  {t.cancelBtn}
                </Button>
                <Button
                  type="submit"
                  disabled={savingSettings}
                  className="h-8 px-4 rounded-full bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/95 transition-transform active:scale-95 shadow-sm"
                >
                  {savingSettings ? (
                    <>
                      <Loader2 className="size-3 animate-spin mr-1" />
                      {t.savingSettingsBtn}
                    </>
                  ) : (
                    t.saveSettingsBtn
                  )}
                </Button>
              </div>
            </div>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Add / Edit Dialog Modal */}
      <Dialog open={showForm !== null} onOpenChange={(open) => !open && setShowForm(null)}>
        <DialogContent className="sm:max-w-md bg-card border border-border rounded-xl shadow-xl p-5 gap-4">
          <DialogHeader className="gap-1.5">
            <DialogTitle className="text-base font-bold tracking-tight">
              {showForm === "add" ? t.createProviderTitle : t.editProviderTitle}
            </DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              {showForm === "add" ? t.createProviderDesc : t.editProviderDesc}
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSubmitForm} className="space-y-4">
            <div className="space-y-1">
              <Label htmlFor="form-name" className="text-xs font-bold text-muted-foreground">
                {t.nameLabel}
              </Label>
              <Input
                id="form-name"
                type="text"
                required
                value={formValues.name}
                onChange={(e) => setFormValues({ ...formValues, name: e.target.value })}
                placeholder={t.namePlaceholder}
                className="bg-card border-border focus-visible:ring-primary focus-visible:border-primary h-9 rounded-[4px] text-sm shadow-sm transition-all"
              />
            </div>

            <div className="space-y-1">
              <Label htmlFor="form-url" className="text-xs font-bold text-muted-foreground">
                {t.baseUrlLabel}
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
                {t.apiKeyLabel}
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
                  {t.modelLabel}
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
                      {t.discovering}
                    </>
                  ) : (
                    <>
                      <Compass className="size-2.5 mr-1" />
                      {t.discoverModels}
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
                  <option value="">{t.selectModelDefault}</option>
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
                  {t.wireApiLabel}
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
                  {t.remarkLabel}
                </Label>
                <Input
                  id="form-remark"
                  type="text"
                  value={formValues.remark}
                  onChange={(e) => setFormValues({ ...formValues, remark: e.target.value })}
                  placeholder={t.remarkPlaceholder}
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
                  {t.cancelBtn}
                </Button>
                <Button
                  type="submit"
                  className="h-8 px-4 rounded-full bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/95 transition-transform active:scale-95 shadow-sm"
                >
                  {showForm === "add" ? t.createBtn : t.saveChangesBtn}
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
