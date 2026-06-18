import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
  Eye,
  EyeOff,
  ChevronDown,
} from "lucide-react";

interface ClaudeModels {
  opus?: string;
  sonnet?: string;
  haiku?: string;
  fable?: string;
  subagent?: string;
}

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
  claude_models?: ClaudeModels;
}

interface Config {
  // 旧字段（向后兼容，已废弃）
  active: string;
  providers: Provider[];
  // 新字段：按 Target Tool 分开存储
  codex_active_app?: string;
  codex_active_wsl?: string;
  codex_providers: Provider[];
  claude_active_app?: string;
  claude_active_wsl?: string;
  claude_providers: Provider[];
  // 设置字段
  codex_source?: string;
  codex_custom_dir?: string;
  claude_source?: string;
  claude_custom_dir?: string;
}

const locales = {
  zh: {
    subtitle: "中转节点配置管理器",
    settings: "设置",
    toggleTheme: "切换主题",
    refreshConfig: "刷新配置",
    addProvider: "添加中转节点",
    searchPlaceholder: "搜索...",
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
    settingsDesc: "配置 Codex 和 Claude CLI 配置文件写入的相关设置。",
    codexSourceLabel: "Codex 来源配置",
    claudeSourceLabel: "Claude CLI 来源配置",
    desktopAppOption: "Desktop 客户端",
    desktopAppDesc: "Windows 客户端 (~/.codex)",
    wslCliOption: "WSL 命令行",
    wslCliDesc: "WSL 子系统环境路径",
    customDirLabel: "Codex 自定义目录",
    claudeCustomDirLabel: "Claude CLI 自定义目录",
    wslRecommended: "WSL 环境推荐",
    wslPlaceholder: "例如: \\\\wsl.localhost\\Ubuntu\\home\\username\\.codex",
    claudeWslPlaceholder: "例如: \\\\wsl.localhost\\Ubuntu\\home\\username\\.claude",
    appPlaceholder: "可选的自定义路径",
    wslNote: "WSL 注意事项: 请指定 WSL 中 .codex 文件夹的绝对 UNC network路径，以便 Windows 端的 CXC 能够成功写入配置文件。",
    claudeWslNote: "WSL 注意事项: 请指定 WSL 中 .claude 文件夹的绝对 UNC network路径，以便 Windows 端的 CXC 能够成功写入配置文件。",
    appNote: "若留空，则默认使用您当前的用户家目录 (~/.codex)。",
    claudeAppNote: "若留空，则默认使用您当前的用户家目录 (~/.claude)。",
    cancelBtn: "取消",
    saveSettingsBtn: "保存设置",
    savingSettingsBtn: "保存中...",
    createProviderTitle: "新建中转节点",
    editProviderTitle: "编辑中转节点",
    createProviderDesc: "配置一个新的 API 中转代理端点。",
    editProviderDesc: "更新中转代理节点的配置参数。",
    nameLabel: "中转站名称 *",
    namePlaceholder: "DeepSeek Backup",
    baseUrlLabel: "Base URL (API 端点地址) *",
    apiKeyLabel: "API Key (机密密钥) *",
    modelLabel: "模型标识符 *",
    discoverModels: "发现模型",
    discovering: "拉取中...",
    wireApiLabel: "传输协议 Wire API",
    remarkLabel: "备注信息 / Notes",
    remarkPlaceholder: "高速备用节点",
    createBtn: "新建节点",
    saveChangesBtn: "保存修改",
    selectModelDefault: "-- 选择一个模型 --",
    confirmDelete: (name: string) => `确定要删除中转节点 "${name}" 吗？`,
    fillRequired: "请填写所有必填字段（名称、Base URL、API Key、模型）",
    fillRequiredDiscovery: "请先填写 Base URL 和 API Key",
    noModelsReturned: "接口未返回任何可用模型",
    fetchingConfig: "正在获取活动配置中...",
    quickSwitchTitle: "快速切换模型",
    quickSwitchDesc: "正在为节点 \"{name}\" 切换模型...",
    fetchingModels: "正在拉取可用模型列表...",
    searchModelPlaceholder: "搜索模型...",
    noModelsFound: "未找到匹配的模型",
    retryBtn: "重试",
    quickSwitchTooltip: "点击快速切换模型",
    endpointTooltip: "Ctrl + 左键：在浏览器中打开\n普通左键：复制到剪贴板",
    copiedToClipboard: "已复制到剪贴板",
    advancedSettings: "高级配置",
    defaultModelPlaceholder: "-- 默认（使用主模型）--",
    generalSettingsLabel: "常规设置",
    themeLabel: "界面主题",
    languageLabel: "语言设置",
    reloadConfigBtn: "重新加载配置文件",
    reloadSuccess: "配置文件已成功重新加载！",
    githubLabel: "开源社区",
    githubDesc: "访问 GitHub 仓库获取最新版本和反馈",
    githubBtn: "前往仓库",
    offlineTooltipTitle: "连接测试超时",
    offlineTooltipDesc: "检测到中转未给出有效响应。不过，部分中转地址出于安全或防护考虑会禁用测试端点，这并不影响其作为 API 节点的实际使用。您依然可以尝试启用它并在具体客户端中发起调用测试。"
  },
  en: {
    subtitle: "Relay Configuration Manager",
    settings: "Settings",
    toggleTheme: "Toggle theme",
    refreshConfig: "Refresh configuration",
    addProvider: "Add Provider",
    searchPlaceholder: "Search...",
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
    claudeSourceLabel: "Claude CLI Source (Claude CLI 来源)",
    desktopAppOption: "Desktop App",
    desktopAppDesc: "Desktop version (~/.codex)",
    wslCliOption: "WSL CLI",
    wslCliDesc: "WSL environment paths",
    customDirLabel: "Codex Custom Directory (自定义目录)",
    claudeCustomDirLabel: "Claude CLI Custom Directory",
    wslRecommended: "Recommended for WSL",
    wslPlaceholder: "\\\\wsl.localhost\\Ubuntu\\home\\username\\.codex",
    claudeWslPlaceholder: "\\\\wsl.localhost\\Ubuntu\\home\\username\\.claude",
    appPlaceholder: "Optional custom path",
    wslNote: "WSL Note: Please specify the absolute UNC network path to your WSL .codex folder so CXC on Windows can write config files successfully.",
    claudeWslNote: "WSL Note: Please specify the absolute UNC network path to your WSL .claude folder so CXC on Windows can write config files successfully.",
    appNote: "Defaults to your home directory (~/.codex) if left blank.",
    claudeAppNote: "Defaults to your home directory (~/.claude) if left blank.",
    cancelBtn: "Cancel",
    saveSettingsBtn: "Save Settings",
    savingSettingsBtn: "Saving...",
    createProviderTitle: "Create Provider",
    editProviderTitle: "Edit Provider",
    createProviderDesc: "Provide endpoint details for the cross-connect proxy relay.",
    editProviderDesc: "Update settings for relay provider.",
    nameLabel: "Provider Name *",
    namePlaceholder: "proxy-fast",
    baseUrlLabel: "Base URL *",
    apiKeyLabel: "API Key *",
    modelLabel: "Model *",
    discoverModels: "Discover Models",
    discovering: "Loading...",
    wireApiLabel: "Wire API",
    remarkLabel: "Remark / Notes",
    remarkPlaceholder: "Backup relay",
    createBtn: "Create",
    saveChangesBtn: "Save Changes",
    selectModelDefault: "-- Select a model --",
    confirmDelete: (name: string) => `Are you sure you want to remove provider "${name}"?`,
    fillRequired: "Please fill in all required fields (Name, Base URL, API Key, Model)",
    fillRequiredDiscovery: "Please fill in Base URL and API Key first",
    noModelsReturned: "No models returned from endpoint",
    fetchingConfig: "Fetching active configuration...",
    quickSwitchTitle: "Quick Switch Model",
    quickSwitchDesc: "Switching model for provider \"{name}\"...",
    fetchingModels: "Fetching available models...",
    searchModelPlaceholder: "Search models...",
    noModelsFound: "No matching models found",
    retryBtn: "Retry",
    quickSwitchTooltip: "Click to quick switch model",
    endpointTooltip: "Ctrl + Click: open in browser\nClick: copy to clipboard",
    copiedToClipboard: "Copied to clipboard",
    advancedSettings: "Advanced Settings",
    defaultModelPlaceholder: "-- Default (Use Main Model) --",
    generalSettingsLabel: "General Settings",
    themeLabel: "Theme Settings",
    languageLabel: "Language",
    reloadConfigBtn: "Reload Configuration",
    reloadSuccess: "Configuration reloaded successfully!",
    githubLabel: "Community",
    githubDesc: "Visit GitHub repository for updates & feedback",
    githubBtn: "Visit Repo",
    offlineTooltipTitle: "Connection Timeout",
    offlineTooltipDesc: "No valid response was received from the server. However, some relays disable speed test endpoints for security reasons. This does not affect actual API usage, and you may still enable and try it."
  }
};

interface ToggleSwitchProps {
  checked: boolean;
  onChange: () => void;
  loading?: boolean;
  disabled?: boolean;
  title?: string;
}

const ToggleSwitch = ({
  checked,
  onChange,
  loading,
  disabled,
  title
}: ToggleSwitchProps) => {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled || loading}
      onClick={onChange}
      title={title}
      className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-300 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60 cxc-no-drag ${
        checked
          ? "bg-primary shadow-[inset_0_1px_2px_rgba(0,0,0,0.1)]"
          : "bg-muted-foreground/25 dark:bg-white/10"
      }`}
    >
      <span
        className={`pointer-events-none flex items-center justify-center size-4 rounded-full bg-white shadow-sm ring-0 transition-transform duration-300 ${
          checked ? "translate-x-[18px]" : "translate-x-[2px]"
        }`}
      >
        {loading && (
          <Loader2 className="size-2.5 animate-spin text-primary shrink-0" />
        )}
      </span>
    </button>
  );
};

const initialFormValues: Omit<Provider, "last_test" | "latency_ms" | "last_ok"> = {
  name: "",
  base_url: "",
  api_key: "",
  model: "",
  wire_api: "responses",
  remark: "",
  claude_models: {
    opus: "",
    sonnet: "",
    haiku: "",
    fable: "",
    subagent: "",
  },
};

const mockConfig: Config = {
  active: "Official",
  providers: [],
  codex_active_app: "PatewayAI",
codex_active_wsl: "",
  codex_providers: [
    {
      name: "Official",
      base_url: "https://claude.ai/download",
      model: "claude-3-5-sonnet",
      api_key: "sk-official-key-placeholder",
      wire_api: "responses",
      remark: "Official client link",
      latency_ms: 12,
      last_ok: true,
      last_test: new Date().toISOString()
    },
    {
      name: "PatewayAI",
      base_url: "https://pateway.ai",
      model: "claude-3-5-sonnet",
      api_key: "sk-pateway-key-placeholder",
      wire_api: "responses",
      remark: "Fast API proxy",
      latency_ms: 45,
      last_ok: true,
      last_test: new Date().toISOString()
    },
    {
      name: "ClaudeAPI",
      base_url: "https://claudeapi.com",
      model: "claude-3-5-sonnet-v2",
      api_key: "sk-claudeapi-key-placeholder",
      wire_api: "responses",
      remark: "Claude native API endpoint",
      latency_ms: 210,
      last_ok: true,
      last_test: new Date().toISOString()
    },
    {
      name: "Codex OAuth",
      base_url: "http://localhost:3000",
      model: "gpt-4o",
      api_key: "sk-local-key-placeholder",
      wire_api: "responses",
      remark: "Account provider via Local Routing",
      last_ok: undefined,
      last_test: undefined
    }
  ],
  claude_active_app: "ClaudeAPI",
claude_active_wsl: "ClaudeAPI",
  claude_providers: [
    {
      name: "Official",
      base_url: "https://claude.ai/download",
      model: "claude-3-5-sonnet",
      api_key: "sk-official-key-placeholder",
      wire_api: "responses",
      remark: "Official client link",
      latency_ms: 12,
      last_ok: true,
      last_test: new Date().toISOString()
    },
    {
      name: "PatewayAI",
      base_url: "https://pateway.ai",
      model: "claude-3-5-sonnet",
      api_key: "sk-pateway-key-placeholder",
      wire_api: "responses",
      remark: "Fast API proxy",
      latency_ms: 45,
      last_ok: true,
      last_test: new Date().toISOString()
    },
    {
      name: "ClaudeAPI",
      base_url: "https://claudeapi.com",
      model: "claude-3-5-sonnet-v2",
      api_key: "sk-claudeapi-key-placeholder",
      wire_api: "responses",
      remark: "Claude native API endpoint",
      latency_ms: 210,
      last_ok: true,
      last_test: new Date().toISOString()
    }
  ],
  codex_source: "app",
  codex_custom_dir: "",
  claude_source: "wsl",
  claude_custom_dir: "",
};

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
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
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [targetTool, setTargetTool] = useState<"codex" | "claude">(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("cxc-target-tool");
      if (saved === "codex" || saved === "claude") return saved;
    }
    return "codex";
  });
  const [settingsSource, setSettingsSource] = useState<string>("app");
  const [settingsCustomDir, setSettingsCustomDir] = useState<string>("");
  const [claudeSource, setClaudeSource] = useState<string>("wsl");
  const [claudeCustomDir, setClaudeCustomDir] = useState<string>("");
  const [savingSettings, setSavingSettings] = useState<boolean>(false);
  const [appVersion, setAppVersion] = useState<string>("");

  useEffect(() => {
    localStorage.setItem("cxc-target-tool", targetTool);
  }, [targetTool]);

  // Form State
  const [showForm, setShowForm] = useState<"add" | "edit" | null>(null);
  const [editingName, setEditingName] = useState<string | null>(null);
  const [formValues, setFormValues] = useState(initialFormValues);
  const [showApiKey, setShowApiKey] = useState<boolean>(false);
  const [showAdvanced, setShowAdvanced] = useState<boolean>(false);

  // Model Discovery State
  const [fetchingModels, setFetchingModels] = useState<boolean>(false);
  const [fetchedModels, setFetchedModels] = useState<string[]>([]);
  const [fetchError, setFetchError] = useState<string | null>(null);

  // Connectivity Test State
  const [testingProviders, setTestingProviders] = useState<Record<string, boolean>>({});
  const [testingAll, setTestingAll] = useState<"codex" | "claude" | null>(null);

  // Quick Model Switch State
  const [quickSwitchProvider, setQuickSwitchProvider] = useState<Provider | null>(null);
  const [quickSwitchFetching, setQuickSwitchFetching] = useState<boolean>(false);
  const [quickSwitchModels, setQuickSwitchModels] = useState<string[]>([]);
  const [quickSwitchError, setQuickSwitchError] = useState<string | null>(null);
  const [quickSwitchSearch, setQuickSwitchSearch] = useState<string>("");

  // Toast notification state
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  // 顶栏滚动收缩及半透明模糊控制状态
  const [isScrolled, setIsScrolled] = useState<boolean>(false);

  // 一体化背景板原型方案状态
  const [vibeMode] = useState<"standard" | "acrylic" | "mica" | "aurora">(() => {
    return (localStorage.getItem("cxc-vibe-mode") as any) || "mica";
  });
  
  // 正常运行状态

  // 窗口所属平台风格
  const [platform, setPlatform] = useState<"macos" | "windows" | "linux">("macos");

  // 侦测真实平台以及拦截浏览器特性
  useEffect(() => {
    // 侦测实际运行的操作系统平台
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes("windows") || ua.includes("win32") || ua.includes("win64")) {
      setPlatform("windows");
    } else if (ua.includes("linux")) {
      setPlatform("linux");
    } else {
      setPlatform("macos");
    }
  }, []);

  // 浏览器沙箱特性屏蔽 (Browser Shield)
  useEffect(() => {
    if (vibeMode !== "standard") {
      const handleContextMenu = (e: MouseEvent) => {
        // 在真实的 Tauri 客户端中完全禁用右键菜单，但在浏览器预览调试时允许右键
        const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI_INTERNALS__?.invoke?.toString().includes("Mock");
        if (isTauri) {
          e.preventDefault();
        }
      };

      const handleKeyDown = (e: KeyboardEvent) => {
        const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI_INTERNALS__?.invoke?.toString().includes("Mock");
        if (isTauri) {
          // 拦截刷新 (F5, Ctrl+R, Cmd+R) 和网页打印 (Ctrl+P, Cmd+P)
          if (
            e.key === "F5" || 
            ((e.ctrlKey || e.metaKey) && e.key === "r") || 
            ((e.ctrlKey || e.metaKey) && e.key === "p")
          ) {
            e.preventDefault();
          }
        }
      };

      window.addEventListener("contextmenu", handleContextMenu);
      window.addEventListener("keydown", handleKeyDown);
      return () => {
        window.removeEventListener("contextmenu", handleContextMenu);
        window.removeEventListener("keydown", handleKeyDown);
      };
    }
  }, [vibeMode]);

  // 保存 vibeMode 并在根元素上应用相应的 CSS 类
  useEffect(() => {
    localStorage.setItem("cxc-vibe-mode", vibeMode);
  }, [vibeMode]);

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

  const handleWindowAction = async (action: "minimize" | "maximize" | "close") => {
    if (typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI_INTERNALS__?.isMock) {
      try {
        const appWindow = getCurrentWindow();
        if (action === "minimize") {
          await appWindow.minimize();
        } else if (action === "maximize") {
          await appWindow.toggleMaximize();
        } else if (action === "close") {
          await appWindow.close();
        }
      } catch (err) {
        console.error("Window action failed:", err);
      }
    } else {
      const zh = lang === "zh";
      const actionText = action === "minimize" ? (zh ? "最小化" : "Minimize") : action === "maximize" ? (zh ? "最大化" : "Maximize") : (zh ? "关闭" : "Close");
      setToastMessage(`${zh ? "模拟窗口操作" : "Mock window action"}: ${actionText}`);
      setTimeout(() => setToastMessage(null), 2000);
    }
  };

  const handleOpenGithub = async () => {
    try {
      const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI_INTERNALS__?.invoke?.toString().includes("Mock");
      if (isTauri) {
        await openUrl("https://github.com/zhang3say/cxc");
      } else {
        window.open("https://github.com/zhang3say/cxc", "_blank");
      }
    } catch (err) {
      console.error("Failed to open URL:", err);
      window.open("https://github.com/zhang3say/cxc", "_blank");
    }
  };

  // 全局 Ctrl+F/Cmd+F 聚焦搜索框快捷键
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        searchInputRef.current?.focus();
      }
    };
    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => {
      window.removeEventListener("keydown", handleGlobalKeyDown);
    };
  }, []);

  const getWindowClasses = () => {
    const base = "flex flex-col text-foreground transition-all duration-300 relative select-none ";
    
    const layout = "w-full h-screen overflow-hidden ";
      
    if (vibeMode === "standard") {
      return base + layout + " bg-background";
    }
    
    const vibeClass = "vibe-integrated ";
    if (vibeMode === "acrylic") {
      return base + layout + vibeClass + " bg-background/55 backdrop-blur-[24px] saturate-[1.25]";
    }
    
    if (vibeMode === "mica") {
      return base + layout + vibeClass + (theme === "dark" 
        ? " bg-[#151515]/85 backdrop-blur-[20px] saturate-[1.15]"
        : " bg-[#f6f5f4]/80 backdrop-blur-[20px] saturate-[1.1]");
    }
    
    if (vibeMode === "aurora") {
      return base + layout + vibeClass + " bg-background/30 backdrop-blur-[35px]";
    }
    
    return base + layout;
  };

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
      setClaudeSource(cfg.claude_source || "wsl");
      setClaudeCustomDir(cfg.claude_custom_dir || "");
    } catch (e: any) {
      const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI_INTERNALS__?.invoke?.toString().includes("Mock");
      if (!isTauri) {
        // Fallback to mock config in browser
        setConfig(mockConfig);
        setSettingsSource(mockConfig.codex_source || "app");
        setSettingsCustomDir(mockConfig.codex_custom_dir || "");
        setClaudeSource(mockConfig.claude_source || "wsl");
        setClaudeCustomDir(mockConfig.claude_custom_dir || "");
      } else {
        setError(e.toString());
      }
    } finally {
      setLoading(false);
    }
  }

  async function handleSwitch(name: string) {
    try {
      setSwitching(name);
      setError(null);
      const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI_INTERNALS__?.invoke?.toString().includes("Mock");
      if (!isTauri && config) {
        // Simulate local switch
        await new Promise((resolve) => setTimeout(resolve, 600));
        const updated = {
          ...config,
        };
        if (targetTool === "codex") {
          const codexSource = config.codex_source || "app";
          if (codexSource === "wsl") updated.codex_active_wsl = name;
          else updated.codex_active_app = name;
        } else {
          const claudeSource = config.claude_source || "wsl";
          if (claudeSource === "app") updated.claude_active_app = name;
          else updated.claude_active_wsl = name;
        }
        setConfig(updated);
      } else {
        const updatedCfg = await invoke<Config>("switch_provider", {
          name,
          targetTool
        });
        setConfig(updatedCfg);
      }
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setSwitching(null);
    }
  }

  async function handleReload(name: string) {
    try {
      setSwitching(name);
      setError(null);
      const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI_INTERNALS__?.invoke?.toString().includes("Mock");
      if (!isTauri && config) {
        await new Promise((resolve) => setTimeout(resolve, 600));
      } else {
        const updatedCfg = await invoke<Config>("switch_provider", {
          name,
          targetTool
        });
        setConfig(updatedCfg);
      }
      showToast(lang === "zh" ? `已重载 ${name} 并重新应用配置` : `Reloaded ${name} and reapplied config`);
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
    setShowApiKey(false);
    setShowAdvanced(false);
  }

  function openEditForm(p: Provider) {
    setFormValues({
      name: p.name,
      base_url: p.base_url,
      api_key: p.api_key,
      model: p.model,
      wire_api: p.wire_api,
      remark: p.remark || "",
      claude_models: p.claude_models || {
        opus: "",
        sonnet: "",
        haiku: "",
        fable: "",
        subagent: "",
      },
    });
    setFetchedModels([]);
    setFetchError(null);
    setShowForm("edit");
    setEditingName(p.name);
    setShowApiKey(false);
    setShowAdvanced(false);
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

  async function handleQuickFetchModels(p: Provider) {
    setQuickSwitchProvider(p);
    setQuickSwitchFetching(true);
    setQuickSwitchError(null);
    setQuickSwitchModels([]);
    setQuickSwitchSearch("");
    try {
      const models = await invoke<string[]>("fetch_models", {
        baseUrl: p.base_url,
        apiKey: p.api_key,
      });
      if (models.length === 0) {
        setQuickSwitchError(t.noModelsReturned || "No models returned from endpoint");
      } else {
        setQuickSwitchModels(models);
      }
    } catch (e: any) {
      setQuickSwitchError(e.toString());
    } finally {
      setQuickSwitchFetching(false);
    }
  }

  async function handleRetryQuickFetch() {
    if (!quickSwitchProvider) return;
    setQuickSwitchFetching(true);
    setQuickSwitchError(null);
    try {
      const models = await invoke<string[]>("fetch_models", {
        baseUrl: quickSwitchProvider.base_url,
        apiKey: quickSwitchProvider.api_key,
      });
      if (models.length === 0) {
        setQuickSwitchError(t.noModelsReturned || "No models returned from endpoint");
      } else {
        setQuickSwitchModels(models);
      }
    } catch (e: any) {
      setQuickSwitchError(e.toString());
    } finally {
      setQuickSwitchFetching(false);
    }
  }

  function handleCloseQuickSwitch() {
    setQuickSwitchProvider(null);
    setQuickSwitchFetching(false);
    setQuickSwitchModels([]);
    setQuickSwitchError(null);
    setQuickSwitchSearch("");
  }

  async function handleSelectQuickModel(model: string) {
    if (!quickSwitchProvider) return;
    try {
      const updatedProvider = { ...quickSwitchProvider, model };
      const newConfig = await invoke<Config>("edit_provider", {
        oldName: quickSwitchProvider.name,
        updated: updatedProvider,
        targetTool,
      });
      setConfig(newConfig);
      handleCloseQuickSwitch();
    } catch (e: any) {
      setQuickSwitchError(e.toString());
    }
  }

  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => {
      setToastMessage(null);
    }, 2000);
  };

  async function handleToggleSource() {
    if (!config) return;
    try {
      setError(null);
      if (targetTool === "codex") {
        const currentSrc = config.codex_source || "app";
        const newSource = currentSrc === "wsl" ? "app" : "wsl";
        
        const updatedCfg = await invoke<Config>("save_settings", {
          targetTool,
          source: newSource,
          customDir: config.codex_custom_dir || "",
          claudeSource: newSource,
          claudeCustomDir: config.claude_custom_dir || "",
        });
        setConfig(updatedCfg);
        setSettingsSource(newSource);
        setClaudeSource(newSource);
        showToast(lang === "zh" ? `已切换环境为 ${newSource === "wsl" ? "WSL" : "Desktop"}` : `Environment switched to ${newSource === "wsl" ? "WSL" : "Desktop"}`);
      } else {
        const currentSrc = config.claude_source || "wsl";
        const newSource = currentSrc === "wsl" ? "app" : "wsl";
        
        const updatedCfg = await invoke<Config>("save_settings", {
          targetTool,
          source: newSource,
          customDir: config.codex_custom_dir || "",
          claudeSource: newSource,
          claudeCustomDir: config.claude_custom_dir || "",
        });
        setConfig(updatedCfg);
        setSettingsSource(newSource);
        setClaudeSource(newSource);
        showToast(lang === "zh" ? `已切换环境为 ${newSource === "wsl" ? "WSL" : "Desktop"}` : `Environment switched to ${newSource === "wsl" ? "WSL" : "Desktop"}`);
      }
    } catch (e: any) {
      setError(e.toString());
    }
  }

  const handleEndpointClick = async (e: React.MouseEvent, url: string) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      try {
        await openUrl(url);
      } catch (err) {
        console.error("Failed to open URL:", err);
      }
    } else {
      try {
        await navigator.clipboard.writeText(url);
        showToast(t.copiedToClipboard);
      } catch (err) {
        console.error("Failed to copy URL:", err);
      }
    }
  };

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
        updatedCfg = await invoke<Config>("add_provider", { provider: formValues, targetTool });
      } else {
        updatedCfg = await invoke<Config>("edit_provider", {
          oldName: editingName,
          updated: formValues,
          targetTool
        });
      }
      setConfig(updatedCfg);
      setShowForm(null);
    } catch (e: any) {
      setError(e.toString());
    }
  }

  async function handleDeleteProvider(name: string) {
    setDeleteTarget(name);
  }

  async function confirmDeleteProvider() {
    if (!deleteTarget) return;
    try {
      setError(null);
      const updatedCfg = await invoke<Config>("delete_provider", { name: deleteTarget, targetTool });
      setConfig(updatedCfg);
      setDeleteTarget(null);
    } catch (e: any) {
      setError(e.toString());
      setDeleteTarget(null);
    }
  }

  async function handleTestProvider(name: string) {
    try {
      setTestingProviders(prev => ({ ...prev, [name]: true }));
      setError(null);
      const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI_INTERNALS__?.invoke?.toString().includes("Mock");
      if (!isTauri && config) {
        await new Promise((resolve) => setTimeout(resolve, 800));
        const targetListKey = targetTool === "codex" ? "codex_providers" : "claude_providers";
        const providers = [...config[targetListKey]];
        const index = providers.findIndex((p) => p.name === name);
        if (index !== -1) {
          providers[index] = {
            ...providers[index],
            latency_ms: Math.floor(Math.random() * 200) + 20,
            last_ok: true,
            last_test: new Date().toISOString(),
          };
        }
        setConfig({
          ...config,
          [targetListKey]: providers,
        });
      } else {
        const updatedCfg = await invoke<Config>("test_provider", { name, targetTool });
        setConfig(updatedCfg);
      }
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setTestingProviders(prev => {
        const next = { ...prev };
        delete next[name];
        return next;
      });
    }
  }

  async function handleTestAllProviders() {
    try {
      setTestingAll(targetTool);
      setError(null);
      const updatedCfg = await invoke<Config>("test_all_providers", { targetTool });
      setConfig(updatedCfg);
    } catch (e: any) {
      setError(e.toString());
    } finally {
      setTestingAll(null);
    }
  }

  async function handleSaveSettings(e: React.FormEvent) {
    e.preventDefault();
    try {
      setSavingSettings(true);
      setError(null);
      const finalSource = targetTool === "codex" ? settingsSource : claudeSource;
      const updatedCfg = await invoke<Config>("save_settings", {
        targetTool,
        source: finalSource,
        customDir: settingsCustomDir,
        claudeSource: finalSource,
        claudeCustomDir: claudeCustomDir,
      });
      setConfig(updatedCfg);
      setSettingsSource(finalSource);
      setClaudeSource(finalSource);
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

  // 根据当前选中的 Target Tool 获取对应的 providers 列表和 active 名称
  const currentProviders = config
    ? (targetTool === "codex" ? config.codex_providers : config.claude_providers) ?? []
    : [];
  const currentActive = config
    ? (targetTool === "codex" 
        ? ((config.codex_source || "app") === "wsl" ? config.codex_active_wsl : config.codex_active_app) 
        : ((config.claude_source || "wsl") === "app" ? config.claude_active_app : config.claude_active_wsl)) ?? ""
    : "";

  const filteredProviders = currentProviders.filter(
    (p) =>
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.model.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (p.remark && p.remark.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  const filteredQuickSwitchModels = quickSwitchModels.filter((model) =>
    model.toLowerCase().includes(quickSwitchSearch.toLowerCase())
  );

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

    const isOffline = !isTesting && p.last_ok === false;

    return (
      <div className="flex flex-col gap-1 items-start">
        <div className="relative group/tooltip">
          <Badge className={`px-2 py-0.5 rounded-full text-[10px] font-bold border flex items-center gap-1.5 shadow-none tracking-normal ${badgeStyle}`}>
            {isTesting ? (
              <Loader2 className="size-2.5 animate-spin text-sticker-orange shrink-0" />
            ) : (
              <span className={`size-1.5 rounded-full shrink-0 ${dotClass}`} />
            )}
            <span>{label}</span>
            {isOffline && (
              <svg className="size-2.5 opacity-60 ml-0.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10"></circle>
                <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path>
                <line x1="12" y1="17" x2="12.01" y2="17"></line>
              </svg>
            )}
          </Badge>

          {isOffline && (
            /* Custom Frosted-Glass Tooltip for Offline Warning */
            <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 w-56 p-2.5 rounded-lg bg-popover/95 backdrop-blur-[6px] border border-border/80 shadow-lg text-[10px] text-popover-foreground pointer-events-none opacity-0 scale-95 origin-bottom group-hover/tooltip:opacity-100 group-hover/tooltip:scale-100 transition-all duration-150 z-50">
              <div className="font-extrabold flex items-center gap-1.5 text-red-500">
                <AlertTriangle className="size-3 shrink-0" />
                <span>{t.offlineTooltipTitle}</span>
              </div>
              <p className="mt-1 text-muted-foreground font-medium leading-normal">
                {t.offlineTooltipDesc}
              </p>
            </div>
          )}
        </div>
        {!isTesting && p.last_test && (
          <span className="text-[10px] text-muted-foreground/60 font-medium pl-1">
            {formatDate(p.last_test)}
          </span>
        )}
      </div>
    );
  };

  const renderAppContent = () => {
    // 渲染 macOS 控制按钮（红绿灯）的子组件
    const WindowControls = () => {
      const [isHovered, setIsHovered] = useState(false);
      // 如果不是 macos，Windows 平台会在 Header 的最右端渲染其控制按钮
      if (platform !== "macos") {
        return null;
      }
      return (
        <div 
          className="flex items-center gap-1.5 mr-2 shrink-0 cursor-default select-none cxc-no-drag"
          onMouseEnter={() => setIsHovered(true)}
          onMouseLeave={() => setIsHovered(false)}
        >
          <button 
            onClick={() => handleWindowAction("close")}
            className="relative flex items-center justify-center size-3 rounded-full bg-[#ff5f56] hover:bg-[#e04b40] transition-colors focus:outline-none cursor-pointer"
            title={lang === "zh" ? "关闭" : "Close"}
          >
            {isHovered && <span className="text-[8px] text-[#4c0002] font-bold absolute leading-none">×</span>}
          </button>
          <button 
            onClick={() => handleWindowAction("minimize")}
            className="relative flex items-center justify-center size-3 rounded-full bg-[#ffbd2e] hover:bg-[#e0a324] transition-colors focus:outline-none cursor-pointer"
            title={lang === "zh" ? "最小化" : "Minimize"}
          >
            {isHovered && <span className="text-[9px] text-[#5c3e00] font-bold absolute leading-none" style={{ top: '-1.5px' }}>-</span>}
          </button>
          <button 
            onClick={() => handleWindowAction("maximize")}
            className="relative flex items-center justify-center size-3 rounded-full bg-[#27c93f] hover:bg-[#1fa330] transition-colors focus:outline-none cursor-pointer"
            title={lang === "zh" ? "最大化" : "Maximize"}
          >
            {isHovered && <span className="text-[7px] text-[#004d05] font-bold absolute leading-none" style={{ top: '0.5px' }}>+</span>}
          </button>
        </div>
      );
    };

    // 渲染 Windows/Linux 扁平化控制三键
    const WindowsWindowControls = () => {
      if (platform === "macos") {
        return null;
      }
      return (
        <div className="flex items-center ml-2.5 shrink-0 cxc-no-drag border-l border-border/40 pl-1 h-8 gap-0.5">
          <button
            onClick={() => handleWindowAction("minimize")}
            className="size-7 hover:bg-muted/80 flex items-center justify-center transition-colors focus:outline-none text-foreground/70 hover:text-foreground cursor-pointer rounded-md"
            title={lang === "zh" ? "最小化" : "Minimize"}
          >
            <svg className="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="5" y1="12" x2="19" y2="12"></line>
            </svg>
          </button>
          <button
            onClick={() => handleWindowAction("maximize")}
            className="size-7 hover:bg-muted/80 flex items-center justify-center transition-colors focus:outline-none text-foreground/70 hover:text-foreground cursor-pointer rounded-md"
            title={lang === "zh" ? "最大化" : "Maximize"}
          >
            <svg className="size-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2"></rect>
            </svg>
          </button>
          <button
            onClick={() => handleWindowAction("close")}
            className="size-7 hover:bg-red-500 hover:text-white flex items-center justify-center transition-colors focus:outline-none text-foreground/70 cursor-pointer rounded-md"
            title={lang === "zh" ? "关闭" : "Close"}
          >
            <svg className="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
      );
    };

    const currentSource = config
      ? (targetTool === "codex" ? config.codex_source : config.claude_source)
      : (targetTool === "codex" ? "app" : "wsl");

    return (
      <>
        {/* Unified Single-Bar Header */}
        <header className={`sticky top-0 z-40 w-full transition-all duration-300 cxc-drag flex items-center justify-between px-6 ${
          vibeMode === "standard"
            ? `py-2 bg-card/75 backdrop-blur-md border-b shadow-[0_1px_2px_rgba(0,0,0,0.02)] ${isScrolled ? "border-border shadow-md" : "border-border/80"}`
            : `py-3.5 border-b transition-all duration-300 ${
                isScrolled
                  ? "bg-background/40 dark:bg-background/25 backdrop-blur-lg border-border/30 shadow-[0_4px_12px_rgba(0,0,0,0.03)]"
                  : "bg-transparent border-transparent shadow-none"
              }`
        }`}>
          <div className="flex items-center gap-3.5 cxc-no-drag">
            {/* 仅在无标题栏一体化模式下显示 macOS 红绿灯窗口控制 */}
            {vibeMode !== "standard" && <WindowControls />}
            
            {/* Brand Identity Block (Logo + Title) */}
            <div className="flex items-center gap-1.5 shrink-0 select-none">
              {/* Logo with official CXC image */}
              <div className="relative flex items-center justify-center size-8 rounded-lg bg-card border border-border shadow-sm overflow-hidden transition-transform hover:rotate-3 duration-200 shrink-0">
                <img src="/logo.png" alt="CXC Logo" className="size-full object-cover" />
              </div>
              <h1 
                className="text-sm font-black tracking-tighter shrink-0 bg-gradient-to-r from-[#0075de] to-[#00a3ff] dark:from-[#2eaadc] dark:to-[#0075de] bg-clip-text text-transparent leading-none"
                style={{ fontFamily: "'Geist Mono Variable', 'Geist Mono', monospace" }}
              >
                CXC
              </h1>
            </div>

            <div className="h-4 w-px bg-border/60 mx-1 shrink-0" />

            {/* Target Tool Switcher with absolute active slide animation */}
          <div className="relative flex items-center p-0.5 rounded-lg bg-muted/40 border border-border/60 shrink-0 h-8 w-16 overflow-hidden animate-in fade-in duration-300">
            {/* Sliding Active Indicator */}
            <div
              className="absolute top-0.5 bottom-0.5 rounded-[6px] bg-card shadow-sm border border-border/10 transition-all duration-300 ease-out"
              style={{
                left: targetTool === "codex" ? "2px" : "calc(50% + 1px)",
                width: "calc(50% - 3px)",
              }}
            />
            <button
              onClick={() => setTargetTool("codex")}
              className="relative z-10 flex items-center justify-center w-1/2 h-full rounded-[6px] transition-colors duration-200 cursor-pointer"
              title="Codex"
            >
              <svg className={`size-3.5 transition-all duration-300 ${targetTool === "codex" ? "scale-110 rotate-3 text-foreground" : "opacity-60 text-muted-foreground hover:text-foreground"}`} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M19.503 0H4.496A4.496 4.496 0 000 4.496v15.007A4.496 4.496 0 004.496 24h15.007A4.496 4.496 0 0024 19.503V4.496A4.496 4.496 0 0019.503 0z" fill="currentColor" opacity="0.1"></path>
                <path d="M9.064 3.344a4.578 4.578 0 012.285-.312c1 .115 1.891.54 2.673 1.275.01.01.024.017.037.021a.09.09 0 00.043 0 4.55 4.55 0 013.046.275l.047.022.116.057a4.581 4.581 0 012.188 2.399c.209.51.313 1.041.315 1.595a4.24 4.24 0 01-.134 1.223.123.123 0 00.03.115c.594.607.988 1.33 1.183 2.17.289 1.425-.007 2.71-.887 3.854l-.136.166a4.548 4.548 0 01-2.201 1.388.123.123 0 00-.081.076c-.191.551-.383 1.023-.74 1.494-.9 1.187-2.222 1.846-3.711 1.838-1.187-.006-2.239-.44-3.157-1.302a.107.107 0 00-.105-.024c-.388.125-.78.143-1.204.138a4.441 4.441 0 01-1.945-.466 4.544 4.544 0 01-1.61-1.335c-.152-.202-.303-.392-.414-.617a5.81 5.81 0 01-.37-.961 4.582 4.582 0 01-.014-2.298.124.124 0 00.006-.056.085.085 0 00-.027-.048 4.467 4.467 0 01-1.034-1.651 3.896 3.896 0 01-.251-1.192 5.189 5.189 0 01.141-1.6c.337-1.112.982-1.985 1.933-2.618.212-.141.413-.251.601-.33.215-.089.43-.164.646-.227a.098.098 0 00.065-.066 4.51 4.51 0 01.829-1.615 4.535 4.535 0 011.837-1.388zm3.482 10.565a.637.637 0 000 1.272h3.636a.637.637 0 100-1.272h-3.636zM8.462 9.23a.637.637 0 00-1.106.631l1.272 2.224-1.266 2.136a.636.636 0 101.095.649l1.454-2.455a.636.636 0 00.005-.64L8.462 9.23z" fill="url(#codex-gradient-unified)"></path>
                <defs>
                  <linearGradient id="codex-gradient-unified" x1="12" x2="12" y1="3" y2="21" gradientUnits="userSpaceOnUse">
                    <stop stopColor="#B1A7FF"></stop>
                    <stop offset=".5" stopColor="#7A9DFF"></stop>
                    <stop offset="1" stopColor="#3941FF"></stop>
                  </linearGradient>
                </defs>
              </svg>
            </button>
            <button
              onClick={() => setTargetTool("claude")}
              className="relative z-10 flex items-center justify-center w-1/2 h-full rounded-[6px] transition-colors duration-200 cursor-pointer"
              title="Claude"
            >
              <svg className={`size-3.5 transition-all duration-300 ${targetTool === "claude" ? "scale-110 -rotate-3 text-foreground" : "opacity-60 text-muted-foreground hover:text-foreground"}`} viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M4.709 15.955l4.72-2.647.08-.23-.08-.128H9.2l-.79-.048-2.698-.073-2.339-.097-2.266-.122-.571-.121L0 11.784l.055-.352.48-.321.686.06 1.52.103 2.278.158 1.652.097 2.449.255h.389l.055-.157-.134-.098-.103-.097-2.358-1.596-2.552-1.688-1.336-.972-.724-.491-.364-.462-.158-1.008.656-.722.881.06.225.061.893.686 1.908 1.476 2.491 1.833.365.304.145-.103.019-.073-.164-.274-1.355-2.446-1.446-2.49-.644-1.032-.17-.619a2.97 2.97 0 01-.104-.729L6.283.134 6.696 0l.996.134.42.364.62 1.414 1.002 2.229 1.555 3.03.456.898.243.832.091.255h.158V9.01l.128-1.706.237-2.095.23-2.695.08-.76.376-.91.747-.492.584.28.48.685-.067.444-.286 1.851-.559 2.903-.364 1.942h.212l.243-.242.985-1.306 1.652-2.064.73-.82.85-.904.547-.431h1.033l.76 1.129-.34 1.166-1.064 1.347-.881 1.142-1.264 1.7-.79 1.36.073.11.188-.02 2.856-.606 1.543-.28 1.841-.315.833.388.091.395-.328.807-1.969.486-2.309.462-3.439.813-.042.03.049.061 1.549.146.662.036h1.622l3.02.225.79.522.474.638-.079.485-1.215.62-1.64-.389-3.829-.91-1.312-.329h-.182v.11l1.093 1.068 2.006 1.81 2.509 2.33.127.578-.322.455-.34-.049-2.205-1.657-.851-.747-1.926-1.62h-.128v.17l.444.649 2.345 3.521.122 1.08-.17.353-.608.213-.668-.122-1.374-1.925-1.415-2.167-1.143-1.943-.14.08-.674 7.254-.316.37-.729.28-.607-.461-.322-.747.322-1.476.389-1.924.315-1.53.286-1.9.17-.632-.012-.042-.14.018-1.434 1.967-2.18 2.945-1.726 1.845-.414.164-.717-.37.067-.662.401-.589 2.388-3.036 1.44-1.882.93-1.086-.006-.158h-.055L4.132 18.56l-1.13.146-.487-.456.061-.746.231-.243 1.908-1.312-.006.006z" fill="#D97757" fillRule="nonzero"></path>
              </svg>
            </button>
          </div>

          {/* Environment Source Status Badge (with Custom Tooltip) */}
          <div className="relative group/tooltip">
            <button
              onClick={handleToggleSource}
              className="flex items-center gap-1 px-1.5 py-0.5 rounded border border-border/40 bg-muted/30 hover:bg-muted/60 transition-colors text-[9px] font-extrabold tracking-wider text-muted-foreground hover:text-foreground cursor-pointer shrink-0 select-none uppercase shadow-[inset_0_-1px_0_rgba(0,0,0,0.02)] animate-in fade-in slide-in-from-left-2 duration-300"
            >
              <span className={`size-1.5 rounded-full shrink-0 ${currentSource === "wsl" ? "bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.4)]" : "bg-blue-500 shadow-[0_0_6px_rgba(59,130,246,0.4)]"}`} />
              <span>{currentSource === "wsl" ? "WSL" : "Desktop"}</span>
            </button>
            
            {/* Custom High-Quality Hover Tooltip */}
            <div className="absolute top-full left-1/2 -translate-x-1/2 mt-1.5 w-48 p-2 rounded-lg bg-popover/95 backdrop-blur-[6px] border border-border/80 shadow-lg text-[10px] text-popover-foreground pointer-events-none opacity-0 scale-95 origin-top group-hover/tooltip:opacity-100 group-hover/tooltip:scale-100 transition-all duration-150 z-50">
              <div className="font-extrabold flex items-center gap-1.5">
                <span className={`size-1.5 rounded-full ${currentSource === "wsl" ? "bg-emerald-500" : "bg-blue-500"}`} />
                <span>{lang === "zh" ? "当前配置写入环境" : "Active Write Env"}</span>
              </div>
              <p className="mt-1 text-muted-foreground font-medium leading-normal">
                {currentSource === "wsl"
                  ? (lang === "zh" ? "当前已将配置写入本地 WSL 配置文件。点击可一键静默切换为 Desktop 环境，未配置自定义目录时将自动使用默认路径。" : "Currently writing to the WSL configuration file. Click to silently toggle to Desktop environment (defaults to standard paths if not custom-configured).")
                  : (lang === "zh" ? "当前已将配置写入本地 Desktop 配置文件。点击可一键静默切换为 WSL 环境，未配置自定义目录时将自动使用默认路径。" : "Currently writing to the Desktop configuration file. Click to silently toggle to WSL environment (defaults to standard paths if not custom-configured).")
                }
              </p>
            </div>
          </div>
        </div>

        {/* Center search box (Dynamic wake-up layout with shortcut helper kbd) */}
        <div className="relative w-40 sm:w-48 focus-within:w-60 transition-all duration-300 mx-4 group">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground/35 group-focus-within:text-primary transition-colors pointer-events-none" />
          <Input
            ref={searchInputRef}
            type="text"
            placeholder={t.searchPlaceholder}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8 pr-12 h-8 w-full bg-muted/20 border border-transparent hover:border-border/40 focus:border-primary focus:bg-background rounded-md placeholder-muted-foreground/30 focus:placeholder-muted-foreground/50 text-xs shadow-none transition-all focus:ring-0 focus-visible:ring-0 focus-visible:outline-none"
          />
          <div className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center pointer-events-none transition-all duration-200 opacity-100 scale-100 group-focus-within:opacity-0 group-focus-within:scale-95 group-focus-within:translate-x-1">
            <kbd className="hidden sm:inline-flex h-5 select-none items-center gap-0.5 rounded border border-border/40 bg-muted/30 px-1.5 font-mono text-[9px] font-medium text-muted-foreground/45 shadow-[inset_0_-1px_0_rgba(0,0,0,0.02)]">
              {platform === "macos" ? "⌘" : "Ctrl"} F
            </kbd>
          </div>
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          {/* Quick Speed Test */}
          {currentProviders.length > 0 && (
            <Button
              variant="outline"
              size="icon"
              onClick={handleTestAllProviders}
              disabled={testingAll !== null || Object.keys(testingProviders).length > 0}
              className="size-8 border-border bg-card text-foreground shadow-sm transition-all"
              title={testingAll !== null ? t.testingAll : t.testAll}
            >
              {testingAll !== null ? (
                <Loader2 className="size-4 animate-spin text-muted-foreground" />
              ) : (
                <Activity className="size-4 text-muted-foreground" />
              )}
            </Button>
          )}

          {/* Add Provider Button */}
          <Button
            onClick={openAddForm}
            className="size-8 rounded-md bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm transition-all flex items-center justify-center cursor-pointer shrink-0"
            title={t.addProvider}
          >
            <Plus className="size-4" />
          </Button>

          <div className="h-4 w-px bg-border/60 mx-0.5" />

          {/* View mode toggle */}
          <Button
            variant="outline"
            size="icon"
            onClick={() => setViewMode(viewMode === "list" ? "card" : "list")}
            className="size-8 rounded-md border-border hover:bg-muted text-muted-foreground hover:text-foreground shadow-sm"
            title={viewMode === "list" ? t.cardView : t.listView}
          >
            {viewMode === "list" ? <LayoutGrid className="size-4" /> : <List className="size-4" />}
          </Button>

          {/* System Settings */}
          <Button
            variant="outline"
            size="icon"
            onClick={() => setShowSettings(true)}
            className="size-8 rounded-md border-border hover:bg-muted text-muted-foreground hover:text-foreground shadow-sm"
            title={t.settings}
          >
            <Settings className="size-4" />
          </Button>
          {/* 仅在非标准模式且为 Windows/Linux 下，在最右侧渲染扁平化控制三键 */}
          {vibeMode !== "standard" && <WindowsWindowControls />}
        </div>
      </header>

      {/* Scroll Wrapper */}
      <div 
        className="flex-1 w-full overflow-y-auto custom-scrollbar"
        onScroll={(e) => setIsScrolled(e.currentTarget.scrollTop > 0)}
      >
        {/* Main Container */}
        <main className="max-w-[1800px] w-full mx-auto px-6 py-4 space-y-6">
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



        {/* Loading State */}
        {loading && !config ? (
          <div className="flex flex-col items-center justify-center py-24 gap-4">
            <Loader2 className="size-7 animate-spin text-muted-foreground/50" />
            <p className="text-xs text-muted-foreground font-medium">{t.fetchingConfig}</p>
          </div>
        ) : (
          <section key={targetTool} className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-300">
            {/* Conditional list/card rendering */}
            {filteredProviders.length > 0 ? (
              viewMode === "list" ? (
                /* Table Database Layout */
                <div className="overflow-x-auto">
                  <div className="w-full min-w-[800px] flex flex-col gap-2.5 pb-2.5 px-3.5">
                    {/* Table Header */}
                    <div className="hidden sm:flex items-center px-4 py-1.5 text-[10px] font-bold text-muted-foreground uppercase tracking-wider gap-4 opacity-70">
                      <div className="w-6 shrink-0" />
                      <div className="w-32 sm:w-36 shrink-0 pr-4">{t.providerNameCol}</div>
                      <div className="flex-1 min-w-0 pr-4">{t.endpointCol}</div>
                      <div className="w-24 sm:w-28 shrink-0 pr-4">{t.modelCol}</div>
                      <div className="w-28 sm:w-32 shrink-0 pr-4">{t.statusLatencyCol}</div>
                      <div className="w-24 shrink-0 text-center pr-4">{t.actionsCol}</div>
                      <div className="w-24 shrink-0 text-left">{t.activeCol}</div>
                    </div>

                    {filteredProviders.map((p) => {
                      const isActive = currentActive === p.name;
                      const isThisSwitching = switching === p.name;
                      const isTesting = !!testingProviders[p.name] || testingAll === targetTool;

                      return (
                        <div
                          key={p.name}
                          className={`flex flex-col sm:flex-row sm:items-center px-4 py-3 gap-3 sm:gap-4 rounded-xl border transition-all duration-200 group relative ${
                            vibeMode === "standard"
                              ? isActive
                                ? "bg-primary/[0.04] border-primary/55 ring-1 ring-primary/5 shadow-[0_2px_10px_rgba(59,130,246,0.05)]"
                                : "bg-card border-border hover:border-border/80 hover:shadow-[0_2px_8px_rgba(0,0,0,0.02)]"
                              : isActive
                                ? "bg-primary/[0.06] dark:bg-primary/[0.09] border-primary/45 ring-1 ring-primary/5 shadow-[0_0_12px_rgba(59,130,246,0.1)]"
                                : "bg-card/25 backdrop-blur-md border-border/40 hover:border-border/70 hover:bg-card/35 dark:border-white/5 dark:hover:border-white/10"
                          } ${isTesting ? "opacity-80" : ""}`}
                        >
                          {/* Active dot column */}
                          <div className="hidden sm:flex items-center justify-center w-6 size-5 shrink-0">
                            {isActive ? (
                              <span className="size-1.5 rounded-full bg-primary animate-pulse" />
                            ) : (
                              <span className="size-1.5 rounded-full bg-muted-foreground/30" />
                            )}
                          </div>

                          {/* Name & remark column */}
                          <div className="w-full sm:w-36 shrink-0 truncate pr-4">
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
                          <div className="flex flex-1 min-w-0 pr-4 text-xs font-mono text-foreground/70 items-center gap-1.5">
                            <span className="text-muted-foreground/60 text-[10px] uppercase font-bold tracking-wider sm:hidden">Endpoint:</span>
                            <span
                              onClick={(e) => handleEndpointClick(e, p.base_url)}
                              className="flex-1 min-w-0 truncate cursor-pointer hover:text-primary hover:underline transition-colors"
                              title={t.endpointTooltip}
                            >
                              {p.base_url}
                            </span>
                          </div>

                          {/* Model column */}
                          <div className="w-full sm:w-28 shrink-0 pr-4 text-xs flex items-center gap-1.5">
                            <span className="text-muted-foreground/60 text-[10px] uppercase font-bold tracking-wider sm:hidden">Model:</span>
                            <span
                              onClick={() => handleQuickFetchModels(p)}
                              className={`font-semibold text-foreground/80 border px-1.5 py-0.5 rounded text-[11px] cursor-pointer transition-all truncate ${
                                vibeMode === "standard"
                                  ? "bg-background border-border hover:bg-muted/85 hover:border-primary/30"
                                  : "bg-white/30 dark:bg-white/5 border-black/5 dark:border-white/5 hover:bg-white/50 dark:hover:bg-white/10 hover:border-primary/20 dark:hover:border-primary/30"
                              }`}
                              title={t.quickSwitchTooltip}
                            >
                              {p.model}
                            </span>
                          </div>

                          {/* Latency & Status column using renderStatus */}
                          <div className="w-full sm:w-32 shrink-0 pr-4 text-xs flex items-center gap-1.5">
                            <span className="text-muted-foreground/60 text-[10px] uppercase font-bold tracking-wider sm:hidden">Status:</span>
                            {renderStatus(p, isTesting)}
                          </div>

                          {/* Quiet action buttons column */}
                          <div className="w-full sm:w-24 shrink-0 pr-4 flex items-center justify-end sm:justify-center gap-1">
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleTestProvider(p.name)}
                              disabled={testingProviders[p.name] || testingAll !== null || switching !== null}
                              className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title={t.testConnTitle}
                            >
                              <Activity className="size-3" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => openEditForm(p)}
                              disabled={testingProviders[p.name] || testingAll !== null || switching !== null}
                              className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title={t.editConnTitle}
                            >
                              <Edit2 className="size-3" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleDeleteProvider(p.name)}
                              disabled={isActive || switching !== null || testingAll !== null || testingProviders[p.name]}
                              className="size-6 rounded-md text-red-500 hover:text-red-600 hover:bg-red-500/10 dark:hover:bg-red-950/20 disabled:opacity-40 transition-colors"
                              title={isActive ? t.activeDeleteTooltip : t.deleteConnTitle}
                            >
                              <Trash2 className="size-3" />
                            </Button>
                          </div>

                          {/* Switch Active Trigger column */}
                          <div className="w-full sm:w-24 shrink-0 flex items-center justify-start gap-1.5 pl-1.5 relative">
                            {isActive && (
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                onClick={() => handleReload(p.name)}
                                disabled={switching !== null || testingAll !== null || testingProviders[p.name]}
                                className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground opacity-0 group-hover:opacity-100 transition-all duration-200"
                                title={lang === "zh" ? "重载配置并写入" : "Reload and rewrite config"}
                              >
                                <RefreshCw className={`size-3 ${isThisSwitching ? "animate-spin" : ""}`} />
                              </Button>
                            )}
                            <ToggleSwitch
                              checked={isActive}
                              loading={isThisSwitching}
                              disabled={switching !== null || testingAll !== null || testingProviders[p.name]}
                              onChange={() => {
                                if (!isActive) {
                                  handleSwitch(p.name);
                                }
                              }}
                              title={isActive ? t.activeLabel : t.switchBtn}
                            />
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ) : (
                /* Card Grid View Layout */
                <div className="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-4">
                  {filteredProviders.map((p) => {
                    const isActive = currentActive === p.name;
                    const isThisSwitching = switching === p.name;
                    const isTesting = !!testingProviders[p.name] || testingAll === targetTool;

                    return (
                      <Card
                        key={p.name}
                        className={`relative group flex flex-col justify-between transition-all duration-200 hover:-translate-y-[1px] ${
                          vibeMode === "standard"
                            ? "bg-card border border-border hover:shadow-[0_2px_8px_rgba(0,0,0,0.02)]"
                            : "bg-card/30 backdrop-blur-md border border-white/10 dark:border-white/5"
                        } ${
                          isActive
                            ? vibeMode === "standard"
                              ? "bg-primary/[0.04] border-primary/55 ring-1 ring-primary/10 shadow-[0_2px_10px_rgba(59,130,246,0.05)]"
                              : "bg-primary/[0.06] dark:bg-primary/[0.10] border-primary/45 ring-1 ring-primary/5 shadow-sm"
                            : "overflow-hidden"
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
                          <div className={`flex flex-col gap-1 p-2 rounded-md border ${
                            vibeMode === "standard"
                              ? "bg-background border-border"
                              : "bg-white/20 dark:bg-black/20 border-black/5 dark:border-white/5"
                          }`}>
                            <div className="flex justify-between items-center text-[10.5px] gap-2">
                              <span className="text-muted-foreground/80 font-medium">Endpoint:</span>
                              <span
                                onClick={(e) => handleEndpointClick(e, p.base_url)}
                                className="font-mono text-foreground/75 truncate max-w-[150px] cursor-pointer hover:text-primary hover:underline transition-colors"
                                title={t.endpointTooltip}
                              >
                                {p.base_url}
                              </span>
                            </div>
                            <div className="flex justify-between items-center text-[10.5px] gap-2">
                              <span className="text-muted-foreground/80 font-medium">Model:</span>
                              <span
                                onClick={() => handleQuickFetchModels(p)}
                                className={`font-semibold text-foreground/80 border px-1.5 py-0.5 rounded text-[10px] cursor-pointer transition-all truncate max-w-[150px] ${
                                  vibeMode === "standard"
                                    ? "bg-background border-border hover:bg-muted/80 hover:border-primary/30"
                                    : "bg-white/30 dark:bg-white/5 border-black/5 dark:border-white/5 hover:bg-white/50 dark:hover:bg-white/10 hover:border-primary/20 dark:hover:border-primary/30"
                                }`}
                                title={t.quickSwitchTooltip}
                              >
                                {p.model}
                              </span>
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
                        <div className={`flex items-center gap-1 border-t p-2 px-4 justify-between ${
                          vibeMode === "standard"
                            ? "border-border bg-background/50"
                            : "border-black/5 dark:border-white/5 bg-transparent"
                        }`}>
                          {/* Left: action icons */}
                          <div className="flex items-center gap-1">
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleTestProvider(p.name)}
                              disabled={testingProviders[p.name] || testingAll !== null || switching !== null}
                              className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title={t.testConnTitle}
                            >
                              <Activity className="size-3" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => openEditForm(p)}
                              disabled={testingProviders[p.name] || testingAll !== null || switching !== null}
                              className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                              title={t.editConnTitle}
                            >
                              <Edit2 className="size-3" />
                            </Button>

                            <Button
                              variant="ghost"
                              size="icon-sm"
                              onClick={() => handleDeleteProvider(p.name)}
                              disabled={isActive || switching !== null || testingAll !== null || testingProviders[p.name]}
                              className="size-6 rounded-md text-red-500 hover:text-red-600 hover:bg-red-500/10 dark:hover:bg-red-950/20 disabled:opacity-40 transition-colors"
                              title={isActive ? t.activeDeleteTooltip : t.deleteConnTitle}
                            >
                              <Trash2 className="size-3" />
                            </Button>
                          </div>

                          {/* Right: Switch active button */}
                          <div className="w-24 flex justify-end items-center pr-1 gap-1.5">
                            {isActive && (
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                onClick={() => handleReload(p.name)}
                                disabled={switching !== null || testingAll !== null || testingProviders[p.name]}
                                className="size-6 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground opacity-0 group-hover:opacity-100 transition-all duration-200"
                                title={lang === "zh" ? "重载配置并写入" : "Reload and rewrite config"}
                              >
                                <RefreshCw className={`size-3 ${isThisSwitching ? "animate-spin" : ""}`} />
                              </Button>
                            )}
                            <ToggleSwitch
                              checked={isActive}
                              loading={isThisSwitching}
                              disabled={switching !== null || testingAll !== null || testingProviders[p.name]}
                              onChange={() => {
                                if (!isActive) {
                                  handleSwitch(p.name);
                                }
                              }}
                              title={isActive ? t.activeLabel : t.switchBtn}
                            />
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
                  <Button onClick={openAddForm} className="mt-3 h-8 px-4 rounded-full bg-primary text-primary-foreground text-xs font-semibold hover:bg-primary/95 transition-transform active:scale-95 shadow-sm flex items-center gap-1">
                    <Plus className="size-3.5" />{t.addProvider}
                  </Button>
                )}
              </div>
            )}
          </section>
        )}
      </main>
      </div>

      {/* Settings Dialog Modal (Notion-style Settings) */}
      <Dialog open={showSettings} onOpenChange={(open) => !open && setShowSettings(false)}>
        <DialogContent className="sm:max-w-md bg-card/95 dark:bg-card/90 backdrop-blur-xl border border-border/50 rounded-2xl shadow-2xl p-6 gap-0 overflow-hidden">
          <DialogHeader className="gap-1.5 pb-4 border-b border-border/30 mb-4">
            <DialogTitle className="text-base font-bold tracking-tight">
              {t.settingsTitle}
            </DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              {targetTool === "codex" ? "配置 Codex 配置文件写入的相关设置。" : "配置 Claude CLI 配置文件写入的相关设置。"}
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSaveSettings} className="space-y-4">
            {targetTool === "codex" ? (
              <>
                {/* Codex Configuration */}
                <div className="space-y-2">
                  <Label className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 block">
                    {t.codexSourceLabel}
                  </Label>
                  <div className="grid grid-cols-2 gap-2.5">
                    <button
                      type="button"
                      onClick={() => setSettingsSource("app")}
                      className={`flex items-center justify-between p-3.5 rounded-xl border text-left transition-all duration-200 active:scale-[0.98] ${
                        settingsSource === "app"
                          ? "border-primary/60 bg-primary/[0.04] text-primary shadow-sm shadow-primary/5"
                          : "border-border/30 bg-muted/10 text-foreground hover:bg-muted/20"
                      }`}
                    >
                      <div>
                        <span className="text-xs font-bold block">{t.desktopAppOption}</span>
                        <span className="text-[10px] text-muted-foreground block mt-0.5">{t.desktopAppDesc}</span>
                      </div>
                      {settingsSource === "app" && <Check className="size-4 shrink-0 text-primary" />}
                    </button>

                    <button
                      type="button"
                      onClick={() => setSettingsSource("wsl")}
                      className={`flex items-center justify-between p-3.5 rounded-xl border text-left transition-all duration-200 active:scale-[0.98] ${
                        settingsSource === "wsl"
                          ? "border-primary/60 bg-primary/[0.04] text-primary shadow-sm shadow-primary/5"
                          : "border-border/30 bg-muted/10 text-foreground hover:bg-muted/20"
                      }`}
                    >
                      <div>
                        <span className="text-xs font-bold block">{t.wslCliOption}</span>
                        <span className="text-[10px] text-muted-foreground block mt-0.5">{t.wslCliDesc}</span>
                      </div>
                      {settingsSource === "wsl" && <Check className="size-4 shrink-0 text-primary" />}
                    </button>
                  </div>
                </div>

                {/* Codex custom directory */}
                <div className="space-y-1.5">
                  <Label htmlFor="settings-custom-dir" className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 flex items-center justify-between">
                    <span>{t.customDirLabel}</span>
                    {settingsSource === "wsl" && <span className="text-[10px] text-primary font-semibold">({t.wslRecommended})</span>}
                  </Label>
                  <Input
                    id="settings-custom-dir"
                    type="text"
                    value={settingsCustomDir}
                    onChange={(e) => setSettingsCustomDir(e.target.value)}
                    placeholder={settingsSource === "wsl" ? t.wslPlaceholder : t.appPlaceholder}
                    className="bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 h-9 rounded-md text-xs shadow-inner transition-all duration-200 focus:bg-background/80"
                  />
                  <p className="text-[10px] text-muted-foreground/80 leading-normal mt-1">
                    {settingsSource === "wsl" ? t.wslNote : t.appNote}
                  </p>
                </div>
              </>
            ) : (
              <>
                {/* Claude CLI Configuration */}
                <div className="space-y-2">
                  <Label className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 block">
                    {t.claudeSourceLabel}
                  </Label>
                  <div className="grid grid-cols-2 gap-2.5">
                    <button
                      type="button"
                      onClick={() => setClaudeSource("wsl")}
                      className={`flex items-center justify-between p-3.5 rounded-xl border text-left transition-all duration-200 active:scale-[0.98] ${
                        claudeSource === "wsl"
                          ? "border-primary/60 bg-primary/[0.04] text-primary shadow-sm shadow-primary/5"
                          : "border-border/30 bg-muted/10 text-foreground hover:bg-muted/20"
                      }`}
                    >
                      <div>
                        <span className="text-xs font-bold block">{t.wslCliOption}</span>
                        <span className="text-[10px] text-muted-foreground block mt-0.5">WSL 环境 (~/.claude)</span>
                      </div>
                      {claudeSource === "wsl" && <Check className="size-4 shrink-0 text-primary" />}
                    </button>

                    <button
                      type="button"
                      onClick={() => setClaudeSource("app")}
                      className={`flex items-center justify-between p-3.5 rounded-xl border text-left transition-all duration-200 active:scale-[0.98] ${
                        claudeSource === "app"
                          ? "border-primary/60 bg-primary/[0.04] text-primary shadow-sm shadow-primary/5"
                          : "border-border/30 bg-muted/10 text-foreground hover:bg-muted/20"
                      }`}
                    >
                      <div>
                        <span className="text-xs font-bold block">{t.desktopAppOption}</span>
                        <span className="text-[10px] text-muted-foreground block mt-0.5">Windows 客户端 (~/.claude)</span>
                      </div>
                      {claudeSource === "app" && <Check className="size-4 shrink-0 text-primary" />}
                    </button>
                  </div>
                </div>

                {/* Claude custom directory */}
                <div className="space-y-1.5">
                  <Label htmlFor="claude-custom-dir" className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 flex items-center justify-between">
                    <span>{t.claudeCustomDirLabel}</span>
                    {claudeSource === "app" && <span className="text-[10px] text-primary font-semibold">({t.wslRecommended})</span>}
                  </Label>
                  <Input
                    id="claude-custom-dir"
                    type="text"
                    value={claudeCustomDir}
                    onChange={(e) => setClaudeCustomDir(e.target.value)}
                    placeholder={claudeSource === "app" ? t.claudeWslPlaceholder : t.appPlaceholder}
                    className="bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 h-9 rounded-md text-xs shadow-inner transition-all duration-200 focus:bg-background/80"
                  />
                  <p className="text-[10px] text-muted-foreground/80 leading-normal mt-1">
                    {claudeSource === "app" ? t.claudeWslNote : t.claudeAppNote}
                  </p>
                </div>
              </>
            )}

            {/* General Settings (Theme, Language, Reload Config) */}
            <div className="border-t border-border/30 pt-4 mt-5 space-y-4">
              <Label className="text-[11px] font-extrabold text-foreground/90 flex items-center gap-1.5 uppercase tracking-wider">
                <Settings className="size-3.5 text-muted-foreground/70" />
                <span>{t.generalSettingsLabel}</span>
              </Label>
              
              <div className="grid grid-cols-2 gap-4 pt-1">
                {/* Theme toggle */}
                <div className="space-y-1.5">
                  <Label className="text-[11px] font-bold text-muted-foreground">
                    {t.themeLabel}
                  </Label>
                  <div className="flex items-center gap-1 bg-muted/20 p-0.5 rounded-lg border border-border/20">
                    <button
                      type="button"
                      onClick={() => setTheme("light")}
                      className={`flex-1 flex items-center justify-center gap-1 py-1 rounded-md text-[10px] font-bold transition-all cursor-pointer active:scale-[0.96] ${
                        theme === "light"
                          ? "bg-background text-foreground shadow-sm border border-border/10"
                          : "text-muted-foreground hover:text-foreground hover:bg-muted/10"
                      }`}
                    >
                      <Sun className="size-3 text-amber-500" />
                      <span>{lang === "zh" ? "浅色" : "Light"}</span>
                    </button>
                    <button
                      type="button"
                      onClick={() => setTheme("dark")}
                      className={`flex-1 flex items-center justify-center gap-1 py-1 rounded-md text-[10px] font-bold transition-all cursor-pointer active:scale-[0.96] ${
                        theme === "dark"
                          ? "bg-background text-foreground shadow-sm border border-border/10"
                          : "text-muted-foreground hover:text-foreground hover:bg-muted/10"
                      }`}
                    >
                      <Moon className="size-3 text-primary" />
                      <span>{lang === "zh" ? "深色" : "Dark"}</span>
                    </button>
                  </div>
                </div>

                {/* Language switcher */}
                <div className="space-y-1.5">
                  <Label className="text-[11px] font-bold text-muted-foreground">
                    {t.languageLabel}
                  </Label>
                  <div className="flex items-center gap-1 bg-muted/20 p-0.5 rounded-lg border border-border/20">
                    <button
                      type="button"
                      onClick={() => setLang("zh")}
                      className={`flex-1 py-1 rounded-md text-[10px] font-bold transition-all cursor-pointer active:scale-[0.96] ${
                        lang === "zh"
                          ? "bg-background text-foreground shadow-sm border border-border/10"
                          : "text-muted-foreground hover:text-foreground hover:bg-muted/10"
                      }`}
                    >
                      中文
                    </button>
                    <button
                      type="button"
                      onClick={() => setLang("en")}
                      className={`flex-1 py-1 rounded-md text-[10px] font-bold transition-all cursor-pointer active:scale-[0.96] ${
                        lang === "en"
                          ? "bg-background text-foreground shadow-sm border border-border/10"
                          : "text-muted-foreground hover:text-foreground hover:bg-muted/10"
                      }`}
                    >
                      English
                    </button>
                  </div>
                </div>
              </div>

              {/* Reload configuration & GitHub Link */}
              <div className="space-y-2.5 pt-1">
                <Button
                  type="button"
                  variant="outline"
                  onClick={async () => {
                    await loadConfig();
                    setToastMessage(t.reloadSuccess);
                    setTimeout(() => setToastMessage(null), 2500);
                  }}
                  disabled={loading}
                  className="w-full h-8 border border-border/40 bg-muted/5 hover:bg-muted/15 text-[11px] font-extrabold flex items-center justify-center gap-1.5 rounded-lg shadow-sm active:scale-[0.98] transition-transform cursor-pointer"
                >
                  <RefreshCw className={`size-3 text-muted-foreground ${loading ? "animate-spin" : ""}`} />
                  <span>{t.reloadConfigBtn}</span>
                </Button>

                <div className="flex items-center justify-between border border-border/30 bg-muted/5 px-3 py-2 rounded-xl">
                  <div className="flex flex-col">
                    <span className="text-[11px] font-extrabold text-foreground">{t.githubLabel}</span>
                    <span className="text-[9px] text-muted-foreground mt-0.5 leading-normal max-w-[210px]">{t.githubDesc}</span>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleOpenGithub}
                    className="h-7 px-2.5 border border-border/40 bg-muted/5 hover:bg-muted/15 text-[10px] font-extrabold flex items-center gap-1 rounded-lg shadow-sm active:scale-[0.98] transition-transform cursor-pointer shrink-0"
                  >
                    <svg className="size-3 text-muted-foreground" viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                      <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.865 8.167 6.839 9.49.5.092.682-.217.682-.482 0-.237-.008-.866-.013-1.7-2.782.603-3.369-1.34-3.369-1.34-.454-1.156-1.11-1.464-1.11-1.464-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.831.092-.646.35-1.086.636-1.336-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.203 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.579.688.481C19.137 20.164 22 16.418 22 12c0-5.523-4.477-10-10-10z" />
                    </svg>
                    <span>{t.githubBtn}</span>
                  </Button>
                </div>
              </div>
            </div>

            <DialogFooter className="pt-4 border-t border-border/30 mt-6">
              <div className="flex w-full items-center justify-between gap-2">
                {appVersion && (
                  <span className="text-[10px] text-muted-foreground/40 font-medium select-none">
                    v{appVersion}
                  </span>
                )}
                <div className="flex gap-2 justify-end ml-auto">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setShowSettings(false)}
                    className="h-8 px-4 rounded-lg border border-border/40 bg-muted/5 text-xs font-bold hover:bg-muted/15 active:scale-[0.98] transition-transform"
                  >
                    {t.cancelBtn}
                  </Button>
                  <Button
                    type="submit"
                    disabled={savingSettings}
                    className="h-8 px-4 rounded-lg bg-primary text-primary-foreground text-xs font-bold hover:bg-primary/95 transition-all active:scale-[0.98] shadow-sm shadow-primary/10 flex items-center justify-center"
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
        <DialogContent className="sm:max-w-md bg-card/95 dark:bg-card/90 backdrop-blur-xl border border-border/50 rounded-2xl shadow-2xl p-6 gap-0 overflow-hidden">
          <DialogHeader className="gap-1.5 pb-4 border-b border-border/30 mb-4">
            <DialogTitle className="text-base font-bold tracking-tight">
              {showForm === "add" ? t.createProviderTitle : t.editProviderTitle}
            </DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              {showForm === "add" ? t.createProviderDesc : t.editProviderDesc}
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSubmitForm} className="space-y-4">
            <div className="max-h-[50vh] sm:max-h-[55vh] overflow-y-auto pr-2 -mr-2 space-y-4">
              <div className="space-y-1.5">
                <Label htmlFor="form-name" className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 block">
                  {t.nameLabel}
                </Label>
                <Input
                  id="form-name"
                  type="text"
                  required
                  value={formValues.name}
                  onChange={(e) => setFormValues({ ...formValues, name: e.target.value })}
                  placeholder={t.namePlaceholder}
                  className="bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 h-9 rounded-md text-xs shadow-inner transition-all duration-200 focus:bg-background/80"
                />
              </div>

              <div className="space-y-1.5">
                <Label htmlFor="form-url" className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 block">
                  {t.baseUrlLabel}
                </Label>
                <Input
                  id="form-url"
                  type="url"
                  required
                  value={formValues.base_url}
                  onChange={(e) => setFormValues({ ...formValues, base_url: e.target.value })}
                  placeholder="https://api.example.com/v1"
                  className="bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 h-9 rounded-md text-xs shadow-inner transition-all duration-200 focus:bg-background/80"
                />
              </div>

              <div className="space-y-1.5">
                <Label htmlFor="form-key" className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 block">
                  {t.apiKeyLabel}
                </Label>
                <div className="relative">
                  <Input
                    id="form-key"
                    type={showApiKey ? "text" : "password"}
                    required
                    value={formValues.api_key}
                    onChange={(e) => setFormValues({ ...formValues, api_key: e.target.value })}
                    placeholder="sk-••••••••••••"
                    className="bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 h-9 rounded-md text-xs shadow-inner pr-9 transition-all duration-200 focus:bg-background/80"
                  />
                  <button
                    type="button"
                    onClick={() => setShowApiKey(!showApiKey)}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors p-1"
                  >
                    {showApiKey ? (
                      <EyeOff className="size-4" />
                    ) : (
                      <Eye className="size-4" />
                    )}
                  </button>
                </div>
              </div>

              <div className="space-y-1.5">
                <div className="flex items-center justify-between pb-1.5">
                  <Label htmlFor="form-model" className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 block">
                    {t.modelLabel}
                  </Label>
                  <Button
                    type="button"
                    variant="outline"
                    size="xs"
                    onClick={handleFetchModels}
                    disabled={fetchingModels || !formValues.base_url || !formValues.api_key}
                    className="h-6 text-[10px] border border-border/40 bg-muted/5 text-muted-foreground hover:bg-muted/15 font-extrabold px-2.5 rounded-md shadow-sm transition-all active:scale-[0.96]"
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
                    className="flex h-9 w-full rounded-md border border-border/30 bg-muted/10 px-3 py-2 text-xs focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary text-foreground transition-all duration-200 focus:bg-background/80"
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
                    required={targetTool === "codex"}
                    value={formValues.model}
                    onChange={(e) => setFormValues({ ...formValues, model: e.target.value })}
                    placeholder="gpt-4o"
                    className="bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 h-9 rounded-md text-xs shadow-inner transition-all duration-200 focus:bg-background/80"
                  />
                )}
                {fetchError && <span className="text-[10px] text-red-500 font-medium block mt-1">{fetchError}</span>}
              </div>

              <div className="space-y-1.5">
                <Label htmlFor="form-remark" className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 block">
                  {t.remarkLabel}
                </Label>
                <Input
                  id="form-remark"
                  type="text"
                  value={formValues.remark}
                  onChange={(e) => setFormValues({ ...formValues, remark: e.target.value })}
                  placeholder={t.remarkPlaceholder}
                  className="bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 h-9 rounded-md text-xs shadow-inner transition-all duration-200 focus:bg-background/80"
                />
              </div>

              {/* Advanced Settings Toggle Button */}
              <div className="pt-1">
                <button
                  type="button"
                  onClick={() => setShowAdvanced(!showAdvanced)}
                  className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground font-semibold transition-colors focus:outline-none py-1.5 cursor-pointer"
                >
                  <ChevronDown className={`size-3.5 transition-transform duration-200 ${showAdvanced ? "rotate-180 text-primary" : ""}`} />
                  <span>{t.advancedSettings}</span>
                </button>
              </div>

              {/* Collapsible Advanced Form Fields */}
              {showAdvanced && (
                <div className="space-y-4 pt-3 border-t border-border/30 animate-in fade-in slide-in-from-top-2 duration-200">
                  {targetTool === "claude" && (
                    <div className="space-y-3 border border-border/30 rounded-xl p-4 bg-muted/5">
                      <Label className="text-[11px] font-extrabold text-muted-foreground/80 flex items-center gap-1.5 uppercase tracking-wider">
                        <span className="bg-primary/10 text-primary px-1.5 py-0.5 rounded-md text-[9px] font-bold">Optional</span>
                        Claude Models Override
                      </Label>
                      <div className="grid grid-cols-2 gap-3">
                        <div className="space-y-1">
                          <Label className="text-[10px] font-extrabold text-muted-foreground/75">Opus</Label>
                          {fetchedModels.length > 0 ? (
                            <select
                              value={formValues.claude_models?.opus || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, opus: e.target.value } })}
                              className="flex h-7.5 w-full rounded-md border border-border/30 bg-muted/10 px-2 text-xs focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary text-foreground transition-all duration-200 focus:bg-background/80"
                            >
                              <option value="">{t.defaultModelPlaceholder}</option>
                              {fetchedModels.map((m) => (
                                <option key={m} value={m}>
                                  {m}
                                </option>
                              ))}
                            </select>
                          ) : (
                            <Input
                              type="text"
                              value={formValues.claude_models?.opus || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, opus: e.target.value } })}
                              placeholder="claude-3-opus-20240229"
                              className="h-7.5 text-xs rounded-md bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 transition-all duration-200 focus:bg-background/80"
                            />
                          )}
                        </div>
                        <div className="space-y-1">
                          <Label className="text-[10px] font-extrabold text-muted-foreground/75">Sonnet</Label>
                          {fetchedModels.length > 0 ? (
                            <select
                              value={formValues.claude_models?.sonnet || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, sonnet: e.target.value } })}
                              className="flex h-7.5 w-full rounded-md border border-border/30 bg-muted/10 px-2 text-xs focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary text-foreground transition-all duration-200 focus:bg-background/80"
                            >
                              <option value="">{t.defaultModelPlaceholder}</option>
                              {fetchedModels.map((m) => (
                                <option key={m} value={m}>
                                  {m}
                                </option>
                              ))}
                            </select>
                          ) : (
                            <Input
                              type="text"
                              value={formValues.claude_models?.sonnet || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, sonnet: e.target.value } })}
                              placeholder="claude-3-5-sonnet-20240620"
                              className="h-7.5 text-xs rounded-md bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 transition-all duration-200 focus:bg-background/80"
                            />
                          )}
                        </div>
                        <div className="space-y-1">
                          <Label className="text-[10px] font-extrabold text-muted-foreground/75">Haiku</Label>
                          {fetchedModels.length > 0 ? (
                            <select
                              value={formValues.claude_models?.haiku || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, haiku: e.target.value } })}
                              className="flex h-7.5 w-full rounded-md border border-border/30 bg-muted/10 px-2 text-xs focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary text-foreground transition-all duration-200 focus:bg-background/80"
                            >
                              <option value="">{t.defaultModelPlaceholder}</option>
                              {fetchedModels.map((m) => (
                                <option key={m} value={m}>
                                  {m}
                                </option>
                              ))}
                            </select>
                          ) : (
                            <Input
                              type="text"
                              value={formValues.claude_models?.haiku || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, haiku: e.target.value } })}
                              placeholder="claude-3-haiku-20240307"
                              className="h-7.5 text-xs rounded-md bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 transition-all duration-200 focus:bg-background/80"
                            />
                          )}
                        </div>
                        <div className="space-y-1">
                          <Label className="text-[10px] font-extrabold text-muted-foreground/75">Fable</Label>
                          {fetchedModels.length > 0 ? (
                            <select
                              value={formValues.claude_models?.fable || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, fable: e.target.value } })}
                              className="flex h-7.5 w-full rounded-md border border-border/30 bg-muted/10 px-2 text-xs focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary text-foreground transition-all duration-200 focus:bg-background/80"
                            >
                              <option value="">{t.defaultModelPlaceholder}</option>
                              {fetchedModels.map((m) => (
                                <option key={m} value={m}>
                                  {m}
                                </option>
                              ))}
                            </select>
                          ) : (
                            <Input
                              type="text"
                              value={formValues.claude_models?.fable || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, fable: e.target.value } })}
                              placeholder="Custom mapping..."
                              className="h-7.5 text-xs rounded-md bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 transition-all duration-200 focus:bg-background/80"
                            />
                          )}
                        </div>
                        <div className="space-y-1 col-span-2 pt-1 border-t border-border/30 mt-1">
                          <Label className="text-[10px] font-extrabold text-muted-foreground/75 flex justify-between items-center">
                            <span>Subagent</span>
                            <span className="font-normal opacity-70">{lang === "zh" ? "(不填则继承主模型)" : "(Leave empty to inherit)"}</span>
                          </Label>
                          {fetchedModels.length > 0 ? (
                            <select
                              value={formValues.claude_models?.subagent || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, subagent: e.target.value } })}
                              className="flex h-7.5 w-full rounded-md border border-border/30 bg-muted/10 px-2 text-xs focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary text-foreground transition-all duration-200 focus:bg-background/80"
                            >
                              <option value="">{t.defaultModelPlaceholder}</option>
                              {fetchedModels.map((m) => (
                                <option key={m} value={m}>
                                  {m}
                                </option>
                              ))}
                            </select>
                          ) : (
                            <Input
                              type="text"
                              value={formValues.claude_models?.subagent || ""}
                              onChange={(e) => setFormValues({ ...formValues, claude_models: { ...formValues.claude_models, subagent: e.target.value } })}
                              placeholder="claude-3-haiku-20240307"
                              className="h-7.5 text-xs rounded-md bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 transition-all duration-200 focus:bg-background/80"
                            />
                          )}
                        </div>
                      </div>
                    </div>
                  )}

                  <div className="space-y-1.5">
                    <Label htmlFor="form-wire" className="text-[11px] font-extrabold tracking-wider uppercase text-muted-foreground/80 block">
                      {t.wireApiLabel}
                    </Label>
                    <Input
                      id="form-wire"
                      type="text"
                      value={formValues.wire_api}
                      onChange={(e) => setFormValues({ ...formValues, wire_api: e.target.value })}
                      placeholder="responses"
                      className="bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 h-9 rounded-md text-xs shadow-inner transition-all duration-200 focus:bg-background/80"
                    />
                  </div>
                </div>
              )}
            </div>

            <DialogFooter className="pt-4 border-t border-border/30 mt-6">
              <div className="flex w-full justify-end gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setShowForm(null)}
                  className="h-8 px-4 rounded-lg border border-border/40 bg-muted/5 text-xs font-bold hover:bg-muted/15 active:scale-[0.98] transition-transform"
                >
                  {t.cancelBtn}
                </Button>
                <Button
                  type="submit"
                  className="h-8 px-4 rounded-lg bg-primary text-primary-foreground text-xs font-bold hover:bg-primary/95 transition-all active:scale-[0.98] shadow-sm shadow-primary/10"
                >
                  {showForm === "add" ? t.createBtn : t.saveChangesBtn}
                </Button>
              </div>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Quick Model Switch Dialog */}
      <Dialog open={quickSwitchProvider !== null} onOpenChange={(open) => !open && handleCloseQuickSwitch()}>
        <DialogContent className="sm:max-w-md bg-card/95 dark:bg-card/90 backdrop-blur-xl border border-border/50 rounded-2xl shadow-2xl p-6 gap-0 overflow-hidden">
          <DialogHeader className="gap-1.5 pb-4 border-b border-border/30 mb-4">
            <DialogTitle className="text-base font-bold tracking-tight">
              {t.quickSwitchTitle}
            </DialogTitle>
            <DialogDescription className="text-xs text-muted-foreground">
              {t.quickSwitchDesc.replace("{name}", quickSwitchProvider?.name || "")}
            </DialogDescription>
          </DialogHeader>

          {quickSwitchFetching ? (
            <div className="flex flex-col items-center justify-center py-10 gap-3">
              <Loader2 className="size-6 animate-spin text-muted-foreground/60" />
              <p className="text-xs text-muted-foreground font-medium">{t.fetchingModels}</p>
            </div>
          ) : quickSwitchError ? (
            <div className="space-y-4 py-2">
              <div className="rounded-lg border border-red-200/60 bg-red-500/5 p-3 dark:border-red-900/40 dark:bg-red-950/10 text-red-600 dark:text-red-400 text-xs leading-normal">
                {quickSwitchError}
              </div>
              <div className="flex justify-end gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  onClick={handleRetryQuickFetch}
                  className="h-8 text-xs font-bold rounded-lg border border-border/40 bg-muted/5 text-muted-foreground hover:bg-muted/15 active:scale-[0.98] transition-transform"
                >
                  {t.retryBtn}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={handleCloseQuickSwitch}
                  className="h-8 text-xs font-bold rounded-lg border border-border/40 bg-muted/5 text-muted-foreground hover:bg-muted/15 active:scale-[0.98] transition-transform"
                >
                  {t.cancelBtn}
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              {/* Search filter for models */}
              <div className="relative mb-3">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground/50 pointer-events-none" />
                <Input
                  type="text"
                  placeholder={t.searchModelPlaceholder}
                  value={quickSwitchSearch}
                  onChange={(e) => setQuickSwitchSearch(e.target.value)}
                  className="pl-8 h-8.5 w-full bg-muted/10 border-border/30 focus-visible:ring-primary/40 focus-visible:border-primary/60 rounded-md placeholder-muted-foreground/50 text-xs transition-all focus:bg-background/80"
                />
              </div>

              {/* Scrollable list of models */}
              <div className="border border-border/30 rounded-xl max-h-60 overflow-y-auto divide-y divide-border/20 bg-muted/5 mt-3">
                {filteredQuickSwitchModels.length > 0 ? (
                  filteredQuickSwitchModels.map((model) => {
                    const isCurrent = quickSwitchProvider?.model === model;
                    return (
                      <button
                        key={model}
                        onClick={() => handleSelectQuickModel(model)}
                        className={`w-full text-left px-3 py-2.5 text-xs transition-all flex items-center justify-between hover:bg-primary/[0.03] active:scale-[0.99] ${
                          isCurrent ? "font-bold text-primary bg-primary/[0.04]" : "text-foreground/80 hover:text-foreground"
                        }`}
                      >
                        <span className="truncate pr-4">{model}</span>
                        {isCurrent && <Check className="size-3.5 text-primary shrink-0" />}
                      </button>
                    );
                  })
                ) : (
                  <div className="text-center py-6 text-xs text-muted-foreground">
                    {t.noModelsFound}
                  </div>
                )}
              </div>

              <div className="flex justify-end pt-4 border-t border-border/30 mt-4">
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={handleCloseQuickSwitch}
                  className="h-8 text-xs font-bold rounded-lg border border-border/40 bg-muted/5 text-muted-foreground hover:bg-muted/15 active:scale-[0.98] transition-transform"
                >
                  {t.cancelBtn}
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>

      {/* Confirm Delete Dialog */}
      <Dialog open={deleteTarget !== null} onOpenChange={(open) => !open && setDeleteTarget(null)}>
        <DialogContent className="sm:max-w-md bg-card/95 dark:bg-card/90 backdrop-blur-xl border border-border/50 rounded-2xl shadow-2xl p-6 gap-0 overflow-hidden duration-300 ease-[cubic-bezier(0.34,1.56,0.64,1)]">
          <div className="flex items-start gap-4">
            <div className="flex-shrink-0 w-10 h-10 rounded-full bg-red-500/10 dark:bg-red-500/20 text-red-500 flex items-center justify-center border border-red-500/20 shadow-[0_0_15px_rgba(239,68,68,0.12)]">
              <AlertTriangle className="size-5 animate-pulse" />
            </div>
            <div className="flex-1 min-w-0">
              <DialogHeader className="gap-1.5 pb-0">
                <DialogTitle className="text-base font-bold tracking-tight text-foreground">
                  {lang === "zh" ? "确认删除" : "Confirm Delete"}
                </DialogTitle>
                <DialogDescription className="text-xs text-muted-foreground pt-1 leading-relaxed">
                  {deleteTarget ? t.confirmDelete(deleteTarget) : ""}
                </DialogDescription>
              </DialogHeader>
            </div>
          </div>

          <DialogFooter className="pt-4 border-t border-border/30 mt-6">
            <div className="flex w-full justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => setDeleteTarget(null)}
                className="h-8 px-4 rounded-lg border border-border/40 bg-muted/5 text-xs font-bold hover:bg-muted/15 text-muted-foreground hover:text-foreground active:scale-[0.98] transition-all duration-200"
              >
                {t.cancelBtn}
              </Button>
              <Button
                type="button"
                variant="destructive"
                onClick={confirmDeleteProvider}
                className="h-8 px-4 rounded-lg text-xs font-bold bg-gradient-to-r from-red-500 to-rose-600 hover:from-red-600 hover:to-rose-700 text-white border border-red-600/30 hover:shadow-lg hover:shadow-red-500/25 active:scale-[0.98] transition-all duration-200"
              >
                {lang === "zh" ? "确认删除" : "Confirm"}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Floating Toast Notification */}
      {toastMessage && (
        <div className="fixed bottom-4 right-4 z-50 bg-card border border-border shadow-lg px-4 py-2.5 rounded-lg text-xs font-semibold text-foreground flex items-center gap-2 animate-in fade-in slide-in-from-bottom-3 duration-200">
          <Check className="size-3.5 text-primary shrink-0" />
          <span>{toastMessage}</span>
        </div>
      )}
      </>
    );
  };

  return (
    <>
      <style>{`
        /* 全局毛玻璃以及一体化背景的自定义样式覆盖 */
        @keyframes float-blob-1 {
          0% { transform: translate(0px, 0px) scale(1); }
          33% { transform: translate(60px, -45px) scale(1.1); }
          66% { transform: translate(-30px, 30px) scale(0.95); }
          100% { transform: translate(0px, 0px) scale(1); }
        }
        @keyframes float-blob-2 {
          0% { transform: translate(0px, 0px) scale(1.15); }
          50% { transform: translate(-45px, 60px) scale(0.9); }
          100% { transform: translate(0px, 0px) scale(1.15); }
        }
        @keyframes float-blob-3 {
          0% { transform: translate(0px, 0px) scale(0.95); }
          33% { transform: translate(-30px, -60px) scale(1.05); }
          66% { transform: translate(60px, 45px) scale(0.9); }
          100% { transform: translate(0px, 0px) scale(0.95); }
        }
        .animate-float-blob-1 { animation: float-blob-1 18s infinite alternate ease-in-out; }
        .animate-float-blob-2 { animation: float-blob-2 22s infinite alternate ease-in-out; }
        .animate-float-blob-3 { animation: float-blob-3 15s infinite alternate ease-in-out; }
        
        .cxc-drag { -webkit-app-region: drag; }
        .cxc-no-drag { -webkit-app-region: no-drag; }
        
        /* 1. 顶部状态栏 Header 彻底融入背景 (无背景，无底部边框，无阴影，仅用上下间距保护) */
        .vibe-integrated header {
          background-color: transparent !important;
          backdrop-filter: none !important;
          border-bottom: none !important;
          box-shadow: none !important;
          padding-top: 16px !important;
          padding-bottom: 16px !important;
        }
        
        /* 2. 表格主容器一体化 (彻底去掉外边框、阴影和背景，让表格行与界面外框直接融为一体) */
        .vibe-integrated .border.border-border.rounded-xl.bg-card {
          background-color: transparent !important;
          backdrop-filter: none !important;
          border: none !important;
          box-shadow: none !important;
          border-radius: 0 !important;
        }
        
        /* 3. 表格头部一体化 (无背景，底线极其淡雅，文字稍作淡化) */
        .vibe-integrated .hidden.sm\\:flex.items-center.bg-muted\\/40 {
          background-color: transparent !important;
          border-bottom-color: rgba(128, 128, 128, 0.08) !important;
          opacity: 0.8;
        }
        
        /* 4. 表格行样式重构 (去除硬线，交互 Hover 浮起极光圆角磨砂层) */
        .vibe-integrated .divide-y > * {
          border-bottom-color: rgba(128, 128, 128, 0.07) !important;
        }
        .vibe-integrated .divide-y > *:last-child {
          border-bottom: none !important;
        }
        
        .vibe-integrated .transition-colors.hover\\:bg-muted\\/30 {
          background-color: transparent !important;
        }
        .vibe-integrated .transition-colors.hover\\:bg-muted\\/30:hover {
          background-color: rgba(255, 255, 255, 0.25) !important;
          box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.1) !important;
        }
        .dark .vibe-integrated .transition-colors.hover\\:bg-muted\\/30:hover {
          background-color: rgba(255, 255, 255, 0.04) !important;
          box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.03) !important;
        }
        
        .vibe-integrated .bg-primary\\/\\[0\\.02\\] {
          background-color: rgba(0, 117, 222, 0.03) !important;
        }
        .dark .vibe-integrated .bg-primary\\/\\[0\\.04\\] {
          background-color: rgba(46, 170, 220, 0.05) !important;
        }
        
        /* 5. 行内模型标签 (p.model) 改为磨砂微光胶囊，消除硬性白背景 */
        .vibe-integrated .font-semibold.text-foreground\\/80.bg-background {
          background-color: rgba(255, 255, 255, 0.5) !important;
          border-color: rgba(0, 0, 0, 0.06) !important;
          box-shadow: 0 1px 2px rgba(0, 0, 0, 0.01) !important;
        }
        .dark .vibe-integrated .font-semibold.text-foreground\\/80.bg-background {
          background-color: rgba(255, 255, 255, 0.05) !important;
          border-color: rgba(255, 255, 255, 0.04) !important;
          color: rgba(255, 255, 255, 0.8) !important;
          box-shadow: none !important;
        }
        .vibe-integrated .font-semibold.text-foreground\\/80.bg-background:hover {
          background-color: rgba(255, 255, 255, 0.75) !important;
          border-color: rgba(0, 117, 222, 0.25) !important;
        }
        .dark .vibe-integrated .font-semibold.text-foreground\\/80.bg-background:hover {
          background-color: rgba(255, 255, 255, 0.1) !important;
          border-color: rgba(46, 170, 220, 0.3) !important;
        }
        
        /* 6. 卡片视图下的一体化 (Card Grid) */
        .vibe-integrated .grid .bg-card {
          background-color: rgba(255, 255, 255, 0.28) !important;
          backdrop-filter: blur(14px) !important;
          border-color: rgba(255, 255, 255, 0.2) !important;
          box-shadow: 0 4px 20px -5px rgba(0, 0, 0, 0.02) !important;
        }
        .dark .vibe-integrated .grid .bg-card {
          background-color: rgba(26, 26, 26, 0.3) !important;
          border-color: rgba(255, 255, 255, 0.04) !important;
          box-shadow: 0 4px 20px -5px rgba(0, 0, 0, 0.2) !important;
        }
        
        /* 7. 输入框等控件的玻璃化 */
        .vibe-integrated input {
          background-color: rgba(255, 255, 255, 0.45) !important;
          border-color: rgba(0, 0, 0, 0.08) !important;
          backdrop-filter: blur(4px) !important;
        }
        .dark .vibe-integrated input {
          background-color: rgba(255, 255, 255, 0.03) !important;
          border-color: rgba(255, 255, 255, 0.05) !important;
        }
        .vibe-integrated input:focus {
          border-color: rgba(0, 117, 222, 0.4) !important;
          background-color: rgba(255, 255, 255, 0.6) !important;
        }
        .dark .vibe-integrated input:focus {
          border-color: rgba(46, 170, 220, 0.4) !important;
          background-color: rgba(255, 255, 255, 0.06) !important;
        }
        
        /* 8. 顶部切换器 (Switcher)、一键测速及其他配置胶囊按钮 */
        .vibe-integrated .bg-muted\\/40 {
          background-color: rgba(128, 128, 128, 0.06) !important;
          border-color: rgba(128, 128, 128, 0.1) !important;
        }
        .vibe-integrated .border-border.bg-card {
          background-color: rgba(255, 255, 255, 0.35) !important;
          border-color: rgba(128, 128, 128, 0.08) !important;
        }
        .dark .vibe-integrated .border-border.bg-card {
          background-color: rgba(255, 255, 255, 0.04) !important;
          border-color: rgba(255, 255, 255, 0.04) !important;
        }
        
        /* 9. 基础边框微调 */
        .vibe-integrated .border-border {
          border-color: rgba(128, 128, 128, 0.08) !important;
        }
        .dark .vibe-integrated .border-border {
          border-color: rgba(255, 255, 255, 0.04) !important;
        }
        
        /* 10. 主区域在窗口固定高度下的自适应弹性滚动 */
        .vibe-integrated main {
          flex: 1 !important;
          overflow-y: auto !important;
          min-height: 0 !important;
        }
        
        /* 滚动条美化 */
        .custom-scrollbar::-webkit-scrollbar {
          width: 5px;
          height: 5px;
        }
        .custom-scrollbar::-webkit-scrollbar-track {
          background: transparent;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb {
          background: rgba(128, 128, 128, 0.2);
          border-radius: 99px;
        }
        .vibe-integrated .custom-scrollbar::-webkit-scrollbar-thumb {
          background: rgba(128, 128, 128, 0.15);
        }
        .custom-scrollbar::-webkit-scrollbar-thumb:hover {
          background: rgba(128, 128, 128, 0.35);
        }
      `}</style>

      <div className={getWindowClasses()}>
        {vibeMode === "aurora" && (
          <div className="absolute inset-0 z-0 overflow-hidden pointer-events-none opacity-30 mix-blend-screen dark:opacity-25">
            <div className="absolute -top-1/4 -left-1/4 size-[400px] rounded-full bg-purple-600/35 blur-[70px] animate-float-blob-1" />
            <div className="absolute -bottom-1/4 -right-1/4 size-[450px] rounded-full bg-orange-600/25 blur-[80px] animate-float-blob-2" />
            <div className="absolute top-1/3 left-1/3 size-[350px] rounded-full bg-cyan-600/30 blur-[60px] animate-float-blob-3" />
          </div>
        )}
        
        {vibeMode === "mica" && (
          <div className="absolute inset-0 z-0 overflow-hidden pointer-events-none bg-gradient-to-tr from-primary/[0.04] via-transparent to-orange-500/[0.04]" />
        )}

        {renderAppContent()}
      </div>
    </>
  );
}

export default App;
