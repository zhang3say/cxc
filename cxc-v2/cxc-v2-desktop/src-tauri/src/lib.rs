use cxc_v2_core::agent::{self, AgentConfigInput, AgentKind, CustomDirs};
use cxc_v2_core::config::{self, Config, Env};
use cxc_v2_core::connectivity::{self, ModelTestOutcome, ModelTestResult};
use cxc_v2_core::detect::{self, DetectedAgent};
use cxc_v2_core::migration::{self, ImportReport};
use cxc_v2_core::protocol::{self, ProbeReport, Protocol};
use cxc_v2_core::supplier::{self, SupplierInput};
use cxc_v2_core::sync::{self, merge::MergeReport, SyncConfig};
use cxc_v2_core::target;
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
struct UpdateInfo {
    current_version: String,
    latest_version: String,
    has_update: bool,
    download_url: Option<String>,
}

fn load_config() -> Result<Config, String> {
    let path = config::config_path().map_err(|error| error.to_string())?;
    if !path.exists() {
        if let Ok(legacy) = config::legacy_config_path() {
            if let Err(error) = migration::import_if_needed(&path, &legacy) {
                eprintln!("[CXC-v2] 旧配置导入失败: {error}");
            }
        }
    }
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
        .user_agent(concat!("CXC-v2/", env!("CARGO_PKG_VERSION")))
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
fn import_legacy_config(app: tauri::AppHandle) -> Result<ImportReport, String> {
    let mut cfg = load_config()?;
    let legacy = config::legacy_config_path().map_err(|error| error.to_string())?;
    let report =
        migration::import_into_config(&mut cfg, &legacy).map_err(|error| error.to_string())?;
    if report.imported {
        save_config(&cfg)?;
        let _ = update_tray_menu_with_config(&app, &cfg);
    }
    Ok(report)
}

#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
async fn check_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let client = reqwest::Client::builder()
        .user_agent("CXC-Desktop-App")
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get("https://api.github.com/repos/zhang3say/cxc/releases/latest")
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    #[derive(serde::Deserialize)]
    struct GithubRelease {
        tag_name: String,
        html_url: String,
    }
    let release: GithubRelease = response.json().await.map_err(|error| error.to_string())?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    Ok(UpdateInfo {
        has_update: version_gt(&latest, &current),
        current_version: current,
        latest_version: latest,
        download_url: Some(release.html_url),
    })
}

fn version_gt(left: &str, right: &str) -> bool {
    let parse = |value: &str| -> Vec<u64> {
        value
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let left = parse(left);
    let right = parse(right);
    for index in 0..left.len().max(right.len()) {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);
        if a != b {
            return a > b;
        }
    }
    false
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

    if let Some(tray) = app.tray_by_id("cxc_v2_tray") {
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

            let _tray = TrayIconBuilder::with_id("cxc_v2_tray")
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
            import_legacy_config,
            get_app_version,
            check_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CXC 2");
}
