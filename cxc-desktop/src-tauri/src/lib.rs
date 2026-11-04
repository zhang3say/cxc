use cxc_core::agent::{self, AgentConfigInput, AgentKind, CustomDirs};
use cxc_core::config::{self, Config, Env};
use cxc_core::connectivity::{self, ModelTestOutcome, ModelTestResult};
use cxc_core::detect::{self, DetectedAgent};
use cxc_core::protocol::{self, ProbeReport, Protocol};
use cxc_core::supplier::{self, SupplierInput};
use cxc_core::sync::{self, merge::MergeReport, SyncConfig};
use cxc_core::target;
use serde::Serialize;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

#[derive(Serialize)]
struct ProbeResponse {
    config: Config,
    report: ProbeReport,
}

#[derive(Serialize)]
struct ModelsResponse {
    config: Config,
    models: Vec<String>,
}

#[derive(Serialize)]
struct ModelTestResponse {
    config: Config,
    result: ModelTestResult,
}

#[derive(Serialize)]
struct BulkTestResponse {
    config: Config,
    outcomes: Vec<ModelTestOutcome>,
}

#[derive(Serialize)]
struct SyncResponse {
    config: Config,
    report: MergeReport,
}

#[derive(Serialize)]
struct UpdateAsset {
    name: String,
    url: String,
    size: u64,
}

#[derive(Serialize)]
struct UpdateInfo {
    current_version: String,
    latest_version: String,
    has_update: bool,
    release_name: String,
    release_url: String,
    release_notes: String,
    published_at: String,
    /// 当前平台最合适的安装包；没有匹配时为 None。
    download_url: Option<String>,
    assets: Vec<UpdateAsset>,
}

const RELEASES_API: &str = "https://api.github.com/repos/zhang3say/cxc/releases/latest";

/// 从 GitHub 标签里取出版本号：`cxc-desktop-v1.0.1` / `v1.0.1` / `1.0.1` → `1.0.1`。
/// 取第一个数字开始、连续的数字与点；遇到其它字符停止，所以 `v1.0.1-beta.1` → `1.0.1`。
fn parse_tag_version(tag: &str) -> String {
    let Some(start) = tag.find(|c: char| c.is_ascii_digit()) else {
        return String::new();
    };
    tag[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .trim_end_matches('.')
        .to_string()
}

/// 按平台挑安装包：Windows 优先 `-setup.exe`，macOS 按架构选 dmg，Linux 不发布安装包。
fn pick_asset_for(assets: &[UpdateAsset], os: &str, arch: &str) -> Option<String> {
    let find = |suffix: &str| {
        assets
            .iter()
            .find(|asset| asset.name.to_lowercase().ends_with(suffix))
            .map(|asset| asset.url.clone())
    };
    match os {
        "windows" => find("-setup.exe").or_else(|| find(".msi")),
        "macos" => match arch {
            "aarch64" => find("aarch64.dmg").or_else(|| find(".dmg")),
            _ => find("x64.dmg").or_else(|| find(".dmg")),
        },
        _ => None,
    }
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

fn version_gt(left: &str, right: &str) -> bool {
    let left = version_parts(left);
    let right = version_parts(right);
    for index in 0..left.len().max(right.len()) {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);
        if a != b {
            return a > b;
        }
    }
    false
}

#[derive(serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

async fn fetch_update_info(current: &str) -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("CXC/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|error| format!("无法创建网络请求：{error}"))?;
    let response = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("连接 GitHub 失败：{error}"))?;
    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return Err("GitHub API 访问受限（未登录每小时 60 次），请稍后再试".to_string());
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err("还没有可用的发布版本".to_string());
    }
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }

    let release: GithubRelease = response
        .json()
        .await
        .map_err(|error| format!("解析发布信息失败：{error}"))?;
    let assets: Vec<UpdateAsset> = release
        .assets
        .into_iter()
        .map(|asset| UpdateAsset {
            name: asset.name,
            url: asset.browser_download_url,
            size: asset.size,
        })
        .collect();
    let latest = parse_tag_version(&release.tag_name);
    let download_url = pick_asset_for(&assets, std::env::consts::OS, std::env::consts::ARCH);
    Ok(UpdateInfo {
        has_update: !latest.is_empty() && version_gt(&latest, current),
        current_version: current.to_string(),
        latest_version: if latest.is_empty() {
            release.tag_name.clone()
        } else {
            latest
        },
        release_name: release.name.unwrap_or_else(|| release.tag_name.clone()),
        release_url: release.html_url,
        release_notes: release.body.unwrap_or_default(),
        published_at: release.published_at.unwrap_or_default(),
        download_url,
        assets,
    })
}

fn load_config() -> Result<Config, String> {
    config::load().map_err(|error| error.to_string())
}

fn save_config(cfg: &Config) -> Result<(), String> {
    config::save(cfg).map_err(|error| error.to_string())
}

fn parse_agent(value: &str) -> Result<AgentKind, String> {
    AgentKind::parse(value).ok_or_else(|| format!("未知 Agent：{value}"))
}

fn parse_protocol(value: &str) -> Result<Protocol, String> {
    Protocol::parse(value).ok_or_else(|| format!("未知协议：{value}"))
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("CXC/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_config() -> Result<Config, String> {
    load_config()
}

#[tauri::command]
fn get_envs() -> Vec<Env> {
    detect::reachable_envs()
}

#[tauri::command]
fn get_env_infos() -> Vec<detect::EnvInfo> {
    detect::env_infos()
}

#[tauri::command]
fn detect_agents() -> Result<Vec<DetectedAgent>, String> {
    let cfg = load_config()?;
    Ok(detect::detect_all(&cfg.custom_dirs))
}

#[tauri::command]
fn get_path_env() -> String {
    detect::current_path()
}

#[tauri::command]
fn add_supplier(app: tauri::AppHandle, input: SupplierInput) -> Result<Config, String> {
    let mut cfg = load_config()?;
    supplier::add_supplier(&mut cfg, input).map_err(|error| error.to_string())?;
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
fn edit_supplier(
    app: tauri::AppHandle,
    id: String,
    input: SupplierInput,
) -> Result<Config, String> {
    let mut cfg = load_config()?;
    supplier::edit_supplier(&mut cfg, &id, input).map_err(|error| error.to_string())?;
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
fn delete_supplier(app: tauri::AppHandle, id: String) -> Result<Config, String> {
    let mut cfg = load_config()?;
    supplier::remove_supplier(&mut cfg, &id).map_err(|error| error.to_string())?;
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
async fn probe_supplier(app: tauri::AppHandle, id: String) -> Result<ProbeResponse, String> {
    let mut cfg = load_config()?;
    let supplier = cfg
        .supplier(&id)
        .cloned()
        .ok_or_else(|| format!("供应商不存在：{id}"))?;
    let key = supplier
        .default_key()
        .ok_or_else(|| format!("中转站没有可用账号：{id}"))?;
    let client = http_client()?;
    let report = protocol::probe_supplier(&supplier, &key.api_key, &client).await;

    if let Some(target) = cfg.supplier_mut(&id) {
        target.protocols = report.protocols.clone();
        target.last_probe = Some(chrono::Local::now());
    }
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(ProbeResponse {
        config: cfg,
        report,
    })
}

#[tauri::command]
async fn fetch_supplier_models(id: String) -> Result<ModelsResponse, String> {
    let mut cfg = load_config()?;
    let supplier = cfg
        .supplier(&id)
        .cloned()
        .ok_or_else(|| format!("供应商不存在：{id}"))?;
    let key = supplier
        .default_key()
        .ok_or_else(|| format!("中转站没有可用账号：{id}"))?;
    let client = http_client()?;
    let models = connectivity::fetch_models(&supplier.base_url, &key.api_key, &client)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(target) = cfg.supplier_mut(&id) {
        supplier::replace_discovered_models(target, models.clone());
    }
    save_config(&cfg)?;
    Ok(ModelsResponse {
        config: cfg,
        models,
    })
}

#[tauri::command]
async fn test_supplier_model(
    id: String,
    model: String,
    protocol: String,
    key_id: Option<String>,
) -> Result<ModelTestResponse, String> {
    let mut cfg = load_config()?;
    let supplier = cfg
        .supplier(&id)
        .cloned()
        .ok_or_else(|| format!("供应商不存在：{id}"))?;
    let protocol = parse_protocol(&protocol)?;
    let key = supplier::effective_key(&supplier, key_id.as_deref())
        .ok_or_else(|| format!("中转站没有可用账号：{id}"))?;
    let client = http_client()?;
    let result = connectivity::test_model(&supplier, &key.api_key, &model, protocol, &client).await;
    supplier::record_model_test(
        &mut cfg,
        &id,
        &model,
        protocol,
        Some(&key.id),
        result.ok,
        result.latency_ms,
        result.error.clone(),
    )
    .map_err(|error| error.to_string())?;
    save_config(&cfg)?;
    Ok(ModelTestResponse {
        config: cfg,
        result,
    })
}

#[tauri::command]
async fn test_supplier_models(
    id: String,
    key_id: Option<String>,
) -> Result<BulkTestResponse, String> {
    let mut cfg = load_config()?;
    let supplier = cfg
        .supplier(&id)
        .cloned()
        .ok_or_else(|| format!("供应商不存在：{id}"))?;
    let key = supplier::effective_key(&supplier, key_id.as_deref())
        .ok_or_else(|| format!("中转站没有可用账号：{id}"))?;
    let client = http_client()?;
    let outcomes = connectivity::test_supplier_models(&supplier, &key.api_key, &client).await;
    for outcome in &outcomes {
        let _ = supplier::record_model_test(
            &mut cfg,
            &id,
            &outcome.model,
            outcome.protocol,
            Some(&key.id),
            outcome.result.ok,
            outcome.result.latency_ms,
            outcome.result.error.clone(),
        );
    }
    save_config(&cfg)?;
    Ok(BulkTestResponse {
        config: cfg,
        outcomes,
    })
}

#[tauri::command]
fn add_binding(
    app: tauri::AppHandle,
    agent: String,
    env: String,
    input: AgentConfigInput,
    activate: bool,
) -> Result<Config, String> {
    let mut cfg = load_config()?;
    let kind = parse_agent(&agent)?;
    let env = Env::parse(&env);
    let adapter = target::adapter_for(kind);
    agent::add_binding(&mut cfg, kind, env, input, activate, adapter.as_ref())
        .map_err(|error| error.to_string())?;
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    if activate {
        notify_written(kind, env, &cfg);
    }
    let _ = app.emit("config-updated", &cfg);
    Ok(cfg)
}

#[tauri::command]
fn edit_binding(
    app: tauri::AppHandle,
    agent: String,
    env: String,
    id: String,
    input: AgentConfigInput,
) -> Result<Config, String> {
    let mut cfg = load_config()?;
    let kind = parse_agent(&agent)?;
    let env = Env::parse(&env);
    let adapter = target::adapter_for(kind);
    agent::edit_binding(&mut cfg, kind, env, &id, input, adapter.as_ref())
        .map_err(|error| error.to_string())?;
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    notify_written(kind, env, &cfg);
    let _ = app.emit("config-updated", &cfg);
    Ok(cfg)
}

#[tauri::command]
fn delete_binding(
    app: tauri::AppHandle,
    agent: String,
    env: String,
    id: String,
) -> Result<Config, String> {
    let mut cfg = load_config()?;
    let kind = parse_agent(&agent)?;
    let env = Env::parse(&env);
    agent::delete_binding(&mut cfg, kind, env, &id).map_err(|error| error.to_string())?;
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
fn activate_binding(
    app: tauri::AppHandle,
    agent: String,
    env: String,
    id: String,
) -> Result<Config, String> {
    let mut cfg = load_config()?;
    let kind = parse_agent(&agent)?;
    let env = Env::parse(&env);
    let adapter = target::adapter_for(kind);
    agent::activate_binding(&mut cfg, kind, env, &id, adapter.as_ref())
        .map_err(|error| error.to_string())?;
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    notify_written(kind, env, &cfg);
    let _ = app.emit("config-updated", &cfg);
    Ok(cfg)
}

#[tauri::command]
fn rewrite_agent(agent: String, env: String) -> Result<Config, String> {
    let cfg = load_config()?;
    let kind = parse_agent(&agent)?;
    let env = Env::parse(&env);
    let adapter = target::adapter_for(kind);
    agent::rewrite(&cfg, kind, env, adapter.as_ref()).map_err(|error| error.to_string())?;
    notify_written(kind, env, &cfg);
    Ok(cfg)
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, custom_dirs: CustomDirs) -> Result<Config, String> {
    let mut cfg = load_config()?;
    cfg.custom_dirs = custom_dirs;
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
fn get_sync_config() -> Result<SyncConfig, String> {
    Ok(config::get_sync_config(&load_config()?))
}

#[tauri::command]
fn save_sync_config(sync: SyncConfig) -> Result<Config, String> {
    let mut cfg = load_config()?;
    config::update_sync_config(&mut cfg, sync);
    save_config(&cfg)?;
    Ok(cfg)
}

#[tauri::command]
async fn test_gist_connection(token: String, gist_id: Option<String>) -> Result<String, String> {
    sync::test_gist_connection(&token, gist_id.as_deref())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn sync_push(app: tauri::AppHandle) -> Result<SyncResponse, String> {
    let mut cfg = load_config()?;
    let mut sync_config = config::get_sync_config(&cfg);
    sync::push_to_gist(&cfg, &mut sync_config)
        .await
        .map_err(|error| error.to_string())?;
    config::update_sync_config(&mut cfg, sync_config);
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(SyncResponse {
        config: cfg,
        report: MergeReport::default(),
    })
}

#[tauri::command]
async fn sync_pull(app: tauri::AppHandle) -> Result<SyncResponse, String> {
    let mut cfg = load_config()?;
    let mut sync_config = config::get_sync_config(&cfg);
    let report = sync::pull_from_gist(&mut cfg, &mut sync_config, sync::merge::SyncMode::Overwrite)
        .await
        .map_err(|error| error.to_string())?;
    config::update_sync_config(&mut cfg, sync_config);
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(SyncResponse {
        config: cfg,
        report,
    })
}

#[tauri::command]
async fn sync_merge(app: tauri::AppHandle) -> Result<SyncResponse, String> {
    let mut cfg = load_config()?;
    let mut sync_config = config::get_sync_config(&cfg);
    let report = sync::pull_from_gist(&mut cfg, &mut sync_config, sync::merge::SyncMode::Merge)
        .await
        .map_err(|error| error.to_string())?;
    config::update_sync_config(&mut cfg, sync_config);
    save_config(&cfg)?;
    let _ = update_tray_menu_with_config(&app, &cfg);
    Ok(SyncResponse {
        config: cfg,
        report,
    })
}

#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
async fn check_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    fetch_update_info(&current).await
}

fn agent_label(kind: AgentKind, env: Env) -> String {
    format!("{} ({})", kind.display_name(), env.label())
}

fn update_tray_menu_with_config(app: &tauri::AppHandle, cfg: &Config) -> Result<(), String> {
    let menu = Menu::new(app).map_err(|error| error.to_string())?;

    for env in detect::reachable_envs() {
        for kind in AgentKind::ALL {
            let Some(config) = cfg.config_of(kind, env) else {
                continue;
            };
            let label = format!("{} · {}", agent_label(kind, env), config.model);
            let item = CheckMenuItem::with_id(
                app,
                format!("rewrite:{}:{}", kind.as_str(), env.as_str()),
                label,
                true,
                false,
                None::<&str>,
            )
            .map_err(|error| error.to_string())?;
            menu.append(&item).map_err(|error| error.to_string())?;
        }
    }

    let separator = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    menu.append(&separator).map_err(|error| error.to_string())?;
    let show = MenuItem::with_id(
        app,
        "show_window",
        "打开 CXC / Open CXC",
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    menu.append(&show).map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "退出 / Quit", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    menu.append(&quit).map_err(|error| error.to_string())?;

    if let Some(tray) = app.tray_by_id("cxc_tray") {
        tray.set_menu(Some(menu))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn update_tray_menu(app: &tauri::AppHandle) -> Result<(), String> {
    let cfg = load_config()?;
    update_tray_menu_with_config(app, &cfg)
}

fn notify_written(kind: AgentKind, env: Env, cfg: &Config) {
    let Some(config) = cfg.config_of(kind, env) else {
        return;
    };
    let title = agent_label(kind, env);
    let body = format!("已写入 {}", config.model);
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary(&title)
            .body(&body)
            .show();
    });
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

// 与 tauri.conf.json 的 identifier 保持一致，供任务栏分组与系统通知使用。
// 快捷方式不再写入 AppUserModel.ID（避免 MSI 警告），改由进程显式声明身份。
#[cfg(target_os = "windows")]
fn set_app_user_model_id() {
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SetCurrentProcessExplicitAppUserModelID(app_id: *const u16) -> i32;
    }

    let app_id: Vec<u16> = "com.cxc.desktop\0".encode_utf16().collect();
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());
    }
}

// Tauri 只设置小图标，任务栏按钮读取的是大图标。这里从 exe 资源里再补一次。
#[cfg(target_os = "windows")]
fn apply_taskbar_icon(window: &tauri::WebviewWindow) {
    const WM_SETICON: u32 = 0x0080;
    const ICON_BIG: usize = 1;
    const IMAGE_ICON: u32 = 1;
    const LR_DEFAULTSIZE: u32 = 0x0040;
    const LR_SHARED: u32 = 0x8000;
    // tauri-build 以 IDI_APPLICATION(32512) 的编号把 icon.ico 写入 exe 资源。
    const ICON_RESOURCE_ID: usize = 32512;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn LoadImageW(
            instance: isize,
            name: *const u16,
            kind: u32,
            cx: i32,
            cy: i32,
            load: u32,
        ) -> isize;
        fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
    }

    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        if instance == 0 {
            return;
        }
        let icon = LoadImageW(
            instance,
            ICON_RESOURCE_ID as *const u16,
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE | LR_SHARED,
        );
        if icon != 0 {
            SendMessageW(hwnd.0 as isize, WM_SETICON, ICON_BIG, icon);
        }
    }
}

fn rewrite_from_tray(app: &tauri::AppHandle, agent: &str, env: &str) {
    let Ok(kind) = parse_agent(agent) else {
        return;
    };
    let env = Env::parse(env);
    let Ok(cfg) = load_config() else {
        return;
    };
    let adapter = target::adapter_for(kind);
    if agent::rewrite(&cfg, kind, env, adapter.as_ref()).is_ok() {
        notify_written(kind, env, &cfg);
        let _ = app.emit("config-updated", &cfg);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    set_app_user_model_id();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(target_os = "macos")]
            const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray_iconTemplate.png");
            #[cfg(not(target_os = "macos"))]
            const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray_icon.png");

            let tray_icon =
                tauri::image::Image::from_bytes(TRAY_ICON_BYTES).expect("Failed to load tray icon");

            let _tray = TrayIconBuilder::with_id("cxc_tray")
                .icon(tray_icon)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    if id == "quit" {
                        app.exit(0);
                    } else if id == "show_window" {
                        show_main_window(app);
                    } else if let Some(rest) = id.strip_prefix("rewrite:") {
                        let mut parts = rest.splitn(2, ':');
                        if let (Some(agent), Some(env)) = (parts.next(), parts.next()) {
                            rewrite_from_tray(app, agent, env);
                        }
                    }
                })
                .build(app)?;

            let _ = update_tray_menu(&handle);

            // 无边框窗口在 Windows 上偶尔拿不到任务栏图标，显式再设一次。
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
                #[cfg(target_os = "windows")]
                apply_taskbar_icon(&window);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_envs,
            get_env_infos,
            detect_agents,
            get_path_env,
            add_supplier,
            edit_supplier,
            delete_supplier,
            probe_supplier,
            fetch_supplier_models,
            test_supplier_model,
            test_supplier_models,
            add_binding,
            edit_binding,
            delete_binding,
            activate_binding,
            rewrite_agent,
            save_settings,
            get_sync_config,
            save_sync_config,
            test_gist_connection,
            sync_push,
            sync_pull,
            sync_merge,
            get_app_version,
            check_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CXC");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> UpdateAsset {
        UpdateAsset {
            name: name.to_string(),
            url: format!("https://example.com/{name}"),
            size: 1,
        }
    }

    #[test]
    fn parses_release_tag_versions() {
        assert_eq!(parse_tag_version("cxc-desktop-v1.0.1"), "1.0.1");
        assert_eq!(parse_tag_version("v1.0.1"), "1.0.1");
        assert_eq!(parse_tag_version("1.0.1"), "1.0.1");
        assert_eq!(parse_tag_version("v1.0.1-beta.1"), "1.0.1");
        assert_eq!(parse_tag_version("cxc-desktop-v2.0.0."), "2.0.0");
        assert_eq!(parse_tag_version("no-version"), "");
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(version_gt("1.0.10", "1.0.9"));
        assert!(version_gt("1.1.0", "1.0.9"));
        assert!(!version_gt("1.0.1", "1.0.1"));
        assert!(!version_gt("1.0", "1.0.0"));
        assert!(version_gt("1.0.1", "1.0"));
        assert!(!version_gt("0.9.9", "1.0.0"));
    }

    #[test]
    fn picks_windows_installer() {
        let assets = vec![
            asset("CXC_1.0.1_x64_en-US.msi"),
            asset("CXC_1.0.1_x64-setup.exe"),
            asset("CXC_1.0.1_aarch64.dmg"),
        ];
        assert_eq!(
            pick_asset_for(&assets, "windows", "x86_64").as_deref(),
            Some("https://example.com/CXC_1.0.1_x64-setup.exe")
        );
    }

    #[test]
    fn falls_back_to_msi_on_windows() {
        let assets = vec![asset("CXC_1.0.1_x64_en-US.msi")];
        assert_eq!(
            pick_asset_for(&assets, "windows", "x86_64").as_deref(),
            Some("https://example.com/CXC_1.0.1_x64_en-US.msi")
        );
    }

    #[test]
    fn picks_macos_dmg_by_arch() {
        let assets = vec![asset("CXC_1.0.1_x64.dmg"), asset("CXC_1.0.1_aarch64.dmg")];
        assert_eq!(
            pick_asset_for(&assets, "macos", "aarch64").as_deref(),
            Some("https://example.com/CXC_1.0.1_aarch64.dmg")
        );
        assert_eq!(
            pick_asset_for(&assets, "macos", "x86_64").as_deref(),
            Some("https://example.com/CXC_1.0.1_x64.dmg")
        );
    }

    #[test]
    fn matches_asset_names_case_insensitively() {
        let assets = vec![asset("CXC_1.0.1_X64-SETUP.EXE")];
        assert_eq!(
            pick_asset_for(&assets, "windows", "x86_64").as_deref(),
            Some("https://example.com/CXC_1.0.1_X64-SETUP.EXE")
        );
    }

    #[test]
    fn returns_none_without_matching_asset() {
        let assets = vec![asset("CXC_1.0.1_aarch64.dmg")];
        assert_eq!(pick_asset_for(&assets, "linux", "x86_64"), None);
        assert_eq!(pick_asset_for(&[], "windows", "x86_64"), None);
    }

    #[tokio::test]
    #[ignore = "需要网络，手动执行 cargo test -p cxc-desktop live_check_update -- --ignored"]
    async fn live_check_update() {
        let info = fetch_update_info("0.1.1").await.expect("请求 GitHub 失败");
        // 标签可能是 v1.0.0 或 cxc-desktop-v1.0.0，解析后都应是非空版本号。
        assert!(!info.latest_version.is_empty());
        assert!(info.latest_version.chars().next().unwrap().is_ascii_digit());
        assert!(info.has_update);
        assert!(!info.release_notes.is_empty());
    }
}
