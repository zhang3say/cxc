use cxc_core::config::{self, Config, Provider};
use cxc_core::target::{
    claude::ClaudeAdapter, codex::CodexAdapter, grok::GrokAdapter, TargetAdapter, TargetConfig,
};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_config() -> Result<Config, String> {
    config::load().map_err(|e| e.to_string())
}

#[tauri::command]
fn switch_provider(
    app_handle: tauri::AppHandle,
    name: String,
    target_tool: String,
) -> Result<Config, String> {
    do_switch_provider(&app_handle, name, target_tool)
}

#[tauri::command]
fn add_provider(
    app_handle: tauri::AppHandle,
    provider: Provider,
    target_tool: String,
) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::add_provider(&mut cfg, &target_tool, provider).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
fn edit_provider(
    app_handle: tauri::AppHandle,
    old_name: String,
    updated: Provider,
    target_tool: String,
) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;

    // Check if the active provider for this target_tool is being edited
    let current_source = config::effective_source(&cfg).to_string();
    let current_active = config::get_active(&cfg, &target_tool, &current_source)
        .map(|p| p.name.clone())
        .unwrap_or_default();

    if current_active == old_name || current_active == updated.name {
        write_provider_to_target(&cfg, &target_tool, &updated)?;
    }

    config::edit_provider(&mut cfg, &target_tool, &old_name, updated).map_err(|e| e.to_string())?;
    let _ = update_tray_menu_with_config(&app_handle, &cfg);
    Ok(cfg)
}

#[tauri::command]
fn delete_provider(
    app_handle: tauri::AppHandle,
    name: String,
    target_tool: String,
) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::remove_provider(&mut cfg, &target_tool, &name).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
async fn test_provider(
    app_handle: tauri::AppHandle,
    name: String,
    target_tool: String,
) -> Result<Config, String> {
    let p = {
        let cfg = config::load().map_err(|e| e.to_string())?;
        config::get_provider(&cfg, &target_tool, &name)
            .ok_or_else(|| format!("provider \"{}\" not found", name))?
            .clone()
    };

    let tester = cxc_core::connectivity::Tester::new();
    // Claude uses Anthropic Messages API; Codex and Grok use OpenAI-compatible chat completions
    let is_claude = target_tool == "claude";
    let res = tester
        .test(&p.base_url, &p.api_key, &p.model, is_claude)
        .await;

    // Reload config AFTER the await to prevent overwriting other concurrent tests
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::update_test_result(&mut cfg, &target_tool, &name, res.latency_ms, res.ok)
        .map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);

    Ok(cfg)
}

#[tauri::command]
async fn test_all_providers(
    app_handle: tauri::AppHandle,
    target_tool: String,
) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let providers = match target_tool.as_str() {
        "codex" => cfg.codex_providers.clone(),
        "claude" => cfg.claude_providers.clone(),
        "grok" => cfg.grok_providers.clone(),
        other => return Err(format!("Unknown target tool: {}", other)),
    };

    let mut tasks = vec![];
    let is_claude = target_tool == "claude";
    for p in providers {
        let is_claude = is_claude;
        tasks.push(tokio::spawn(async move {
            let tester = cxc_core::connectivity::Tester::new();
            let res = tester
                .test(&p.base_url, &p.api_key, &p.model, is_claude)
                .await;
            (p.name.clone(), res.ok, res.latency_ms)
        }));
    }

    for task in tasks {
        if let std::result::Result::Ok(res) = task.await {
            let (name, is_ok, latency) = res;
            let _ = config::update_test_result(&mut cfg, &target_tool, &name, latency, is_ok);
        }
    }

    config::save(&cfg).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
async fn fetch_models(base_url: String, api_key: String) -> Result<Vec<String>, String> {
    cxc_core::connectivity::fetch_models(&base_url, &api_key)
        .await
        .map_err(|e| e.to_string())
}

fn apply_source_settings(
    cfg: &mut Config,
    source: &str,
    custom_dir: &str,
    claude_custom_dir: &str,
    grok_custom_dir: &str,
) -> Result<(), String> {
    config::set_global_source(cfg, source);
    cfg.codex_custom_dir = custom_dir.to_string();
    cfg.claude_custom_dir = claude_custom_dir.to_string();
    cfg.grok_custom_dir = grok_custom_dir.to_string();
    Ok(())
}

fn default_wire_api_for_tool(target_tool: &str) -> &'static str {
    if target_tool == "grok" {
        "chat_completions"
    } else {
        "responses"
    }
}

fn provider_to_target_config(p: &Provider, target_tool: &str) -> TargetConfig {
    TargetConfig {
        base_url: p.base_url.clone(),
        api_key: p.api_key.clone(),
        model: p.model.clone(),
        wire_api: if p.wire_api.is_empty() {
            default_wire_api_for_tool(target_tool).to_string()
        } else {
            p.wire_api.clone()
        },
    }
}

fn write_provider_to_target(
    cfg: &Config,
    target_tool: &str,
    provider: &Provider,
) -> Result<(), String> {
    match target_tool {
        "codex" => {
            let adapter = CodexAdapter::new_from_config(cfg).map_err(|e| e.to_string())?;
            adapter
                .write(&provider_to_target_config(provider, target_tool))
                .map_err(|e| e.to_string())
        }
        "claude" => {
            let adapter = ClaudeAdapter::new_from_config(cfg).map_err(|e| e.to_string())?;
            adapter.write_provider(provider).map_err(|e| e.to_string())
        }
        "grok" => {
            let adapter = GrokAdapter::new_from_config(cfg).map_err(|e| e.to_string())?;
            adapter
                .write(&provider_to_target_config(provider, target_tool))
                .map_err(|e| e.to_string())
        }
        other => Err(format!("Unknown target tool: {}", other)),
    }
}

#[tauri::command]
fn save_settings(
    app_handle: tauri::AppHandle,
    target_tool: String,
    source: String,
    custom_dir: String,
    claude_custom_dir: String,
    grok_custom_dir: String,
) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    apply_source_settings(
        &mut cfg,
        &source,
        &custom_dir,
        &claude_custom_dir,
        &grok_custom_dir,
    )?;
    config::save(&cfg).map_err(|e| e.to_string())?;

    // Re-apply the active provider only for the target tool that initiated this save.
    let src = config::effective_source(&cfg).to_string();
    if let Some(p) = config::get_active(&cfg, &target_tool, &src).cloned() {
        write_provider_to_target(&cfg, &target_tool, &p)?;
    }

    let _ = update_tray_menu_with_config(&app_handle, &cfg);
    Ok(cfg)
}

#[tauri::command]
fn get_app_version(app_handle: tauri::AppHandle) -> String {
    app_handle.package_info().version.to_string()
}

#[derive(serde::Serialize)]
struct UpdateInfo {
    version: String,
    url: String,
    changelog: String,
}

#[tauri::command]
async fn check_update() -> Result<UpdateInfo, String> {
    println!("[CXC-Update] Starting update check...");
    let client = reqwest::Client::builder()
        .user_agent("CXC-Desktop-App")
        .build()
        .map_err(|e| {
            let err_msg = format!("Failed to build HTTP client: {}", e);
            println!("[CXC-Update] Error: {}", err_msg);
            err_msg
        })?;

    let res = client
        .get("https://api.github.com/repos/zhang3say/cxc/releases/latest")
        .send()
        .await
        .map_err(|e| {
            let err_msg = format!("HTTP request failed: {}", e);
            println!("[CXC-Update] Error: {}", err_msg);
            err_msg
        })?;

    println!("[CXC-Update] Response received with status: {}", res.status());
    if !res.status().is_success() {
        let err_msg = format!("Request failed with status: {}", res.status());
        println!("[CXC-Update] Error: {}", err_msg);
        return Err(err_msg);
    }

    #[derive(serde::Deserialize)]
    struct GithubRelease {
        tag_name: String,
        html_url: String,
        body: Option<String>,
    }

    let release: GithubRelease = res.json().await.map_err(|e| {
        let err_msg = format!("Failed to parse JSON: {}", e);
        println!("[CXC-Update] Error: {}", err_msg);
        err_msg
    })?;

    println!("[CXC-Update] Found remote version: {}", release.tag_name);

    Ok(UpdateInfo {
        version: release.tag_name,
        url: release.html_url,
        changelog: release.body.unwrap_or_default(),
    })
}

fn do_switch_provider(
    app_handle: &tauri::AppHandle,
    name: String,
    target_tool: String,
) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;

    let p = config::get_provider(&cfg, &target_tool, &name)
        .ok_or_else(|| format!("provider \"{}\" not found", name))?
        .clone();

    write_provider_to_target(&cfg, &target_tool, &p)?;

    let source = config::effective_source(&cfg).to_string();

    config::set_active(&mut cfg, &target_tool, &source, &name).map_err(|e| e.to_string())?;

    let _ = update_tray_menu_with_config(app_handle, &cfg);

    notify_provider_switched(name);

    let _ = app_handle.emit("config-updated", &cfg);

    Ok(cfg)
}

fn update_tray_menu(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    update_tray_menu_with_config(app_handle, &cfg)
}

fn update_tray_menu_with_config(app_handle: &tauri::AppHandle, cfg: &Config) -> Result<(), String> {
    let menu = Menu::new(app_handle).map_err(|e| e.to_string())?;

    // System Tray 只显示 Codex providers（已知的设计限制，见 CONTEXT.md）
    // 托盘无法选择 Target Tool，固定使用 Codex 列表
    let codex_source = cfg
        .codex_source
        .clone()
        .unwrap_or_else(|| config::effective_source(cfg).to_string());
    for p in &cfg.codex_providers {
        let is_active = match config::get_active(&cfg, "codex", &codex_source) {
            Some(active_p) => active_p.name == p.name,
            None => false,
        };
        let item = CheckMenuItem::with_id(
            app_handle,
            format!("prov:{}", p.name),
            &p.name,
            true,
            is_active,
            None::<&str>,
        )
        .map_err(|e| e.to_string())?;

        menu.append(&item).map_err(|e| e.to_string())?;
    }

    let separator = PredefinedMenuItem::separator(app_handle).map_err(|e| e.to_string())?;
    menu.append(&separator).map_err(|e| e.to_string())?;

    let show_item = MenuItem::with_id(app_handle, "show_window", "Show Window", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    menu.append(&show_item).map_err(|e| e.to_string())?;

    let quit_item = MenuItem::with_id(app_handle, "quit", "Quit", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    if let Some(tray) = app_handle.tray_by_id("cxc_tray") {
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn notify_provider_switched(name: String) {
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary("CXC")
            .body(&format!("✓ Switched active provider to \"{}\"", name))
            .show();
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
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
                .on_tray_icon_event(move |tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app_handle = tray.app_handle();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_menu_event(move |app_handle, event| {
                    let id = event.id.as_ref();
                    if id == "quit" {
                        app_handle.exit(0);
                    } else if id == "show_window" {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if id.starts_with("prov:") {
                        let prov_name = id.trim_start_matches("prov:").to_string();
                        // System Tray 固定使用 Codex（已知设计限制）
                        let _ = do_switch_provider(app_handle, prov_name, "codex".to_string());
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
            greet,
            get_config,
            switch_provider,
            add_provider,
            edit_provider,
            delete_provider,
            test_provider,
            test_all_providers,
            fetch_models,
            save_settings,
            get_app_version,
            check_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_sources() -> Config {
        Config {
            codex_source: Some("app".to_string()),
            codex_custom_dir: "C:\\Users\\lee\\.codex".to_string(),
            claude_source: Some("wsl".to_string()),
            claude_custom_dir: r"\\wsl.localhost\Ubuntu\home\lee\.claude".to_string(),
            grok_source: Some("app".to_string()),
            grok_custom_dir: r"\\wsl.localhost\Ubuntu\home\lee\.grok".to_string(),
            ..Config::default()
        }
    }

    #[test]
    fn test_apply_source_settings_updates_global_source_to_wsl() {
        let mut cfg = cfg_with_sources();

        apply_source_settings(
            &mut cfg,
            "wsl",
            r"\\wsl.localhost\Ubuntu-24.04\home\leezi\.codex",
            "C:\\Users\\lee\\.claude",
            r"\\wsl.localhost\Ubuntu-24.04\home\leezi\.grok",
        )
        .unwrap();

        assert_eq!(cfg.codex_source.as_deref(), Some("wsl"));
        assert_eq!(cfg.claude_source.as_deref(), Some("wsl"));
        assert_eq!(cfg.grok_source.as_deref(), Some("wsl"));
        assert_eq!(
            cfg.codex_custom_dir,
            r"\\wsl.localhost\Ubuntu-24.04\home\leezi\.codex"
        );
        assert_eq!(cfg.claude_custom_dir, "C:\\Users\\lee\\.claude");
        assert_eq!(
            cfg.grok_custom_dir,
            r"\\wsl.localhost\Ubuntu-24.04\home\leezi\.grok"
        );
    }

    #[test]
    fn test_apply_source_settings_updates_global_source_to_app() {
        let mut cfg = cfg_with_sources();

        apply_source_settings(
            &mut cfg,
            "app",
            "C:\\Users\\lee\\.codex",
            "C:\\Users\\lee\\.claude",
            "C:\\Users\\lee\\.grok",
        )
        .unwrap();

        assert_eq!(cfg.codex_source.as_deref(), Some("app"));
        assert_eq!(cfg.codex_custom_dir, "C:\\Users\\lee\\.codex");
        assert_eq!(cfg.claude_source.as_deref(), Some("app"));
        assert_eq!(cfg.claude_custom_dir, "C:\\Users\\lee\\.claude");
        assert_eq!(cfg.grok_source.as_deref(), Some("app"));
        assert_eq!(cfg.grok_custom_dir, "C:\\Users\\lee\\.grok");
    }
}
