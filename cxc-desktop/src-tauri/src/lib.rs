use cxc_core::config::{self, Config, Provider};
use cxc_core::target::{TargetAdapter, TargetConfig, codex::CodexAdapter, claude::ClaudeAdapter};
use tauri::{Manager, Emitter};
use tauri::menu::{Menu, MenuItem, CheckMenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};

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
fn switch_provider(app_handle: tauri::AppHandle, name: String, target_tool: String) -> Result<Config, String> {
    do_switch_provider(&app_handle, name, target_tool)
}

#[tauri::command]
fn add_provider(app_handle: tauri::AppHandle, provider: Provider, target_tool: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::add_provider(&mut cfg, &target_tool, provider).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
fn edit_provider(app_handle: tauri::AppHandle, old_name: String, updated: Provider, target_tool: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;

    // Check if the active provider for this target_tool is being edited
    let current_active = if target_tool == "codex" {
        cfg.codex_active.clone()
    } else {
        cfg.claude_active.clone()
    };

    if current_active == old_name || current_active == updated.name {
        // Write updated config to target tool's config file
        if target_tool == "codex" {
            let codex_adapter = CodexAdapter::new().map_err(|e| e.to_string())?;
            let tc = TargetConfig {
                base_url: updated.base_url.clone(),
                api_key: updated.api_key.clone(),
                model: updated.model.clone(),
                wire_api: if updated.wire_api.is_empty() { "responses".to_string() } else { updated.wire_api.clone() },
            };
            codex_adapter.write(&tc).map_err(|e| e.to_string())?;
        } else if target_tool == "claude" {
            let claude_adapter = ClaudeAdapter::new().map_err(|e| e.to_string())?;
            claude_adapter.write_provider(&updated).map_err(|e| e.to_string())?;
        }
    }

    config::edit_provider(&mut cfg, &target_tool, &old_name, updated).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
fn delete_provider(app_handle: tauri::AppHandle, name: String, target_tool: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::remove_provider(&mut cfg, &target_tool, &name).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
async fn test_provider(app_handle: tauri::AppHandle, name: String, target_tool: String) -> Result<Config, String> {
    let p = {
        let cfg = config::load().map_err(|e| e.to_string())?;
        config::get_provider(&cfg, &target_tool, &name)
            .ok_or_else(|| format!("provider \"{}\" not found", name))?
            .clone()
    };

    let tester = cxc_core::connectivity::Tester::new();
    let is_claude = target_tool == "claude";
    let res = tester.test(&p.base_url, &p.api_key, &p.model, is_claude).await;

    // Reload config AFTER the await to prevent overwriting other concurrent tests
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::update_test_result(&mut cfg, &target_tool, &name, res.latency_ms, res.ok).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);

    Ok(cfg)
}

#[tauri::command]
async fn test_all_providers(app_handle: tauri::AppHandle, target_tool: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let providers = if target_tool == "codex" {
        cfg.codex_providers.clone()
    } else {
        cfg.claude_providers.clone()
    };

    let mut tasks = vec![];
    let is_claude = target_tool == "claude";
    for p in providers {
        let is_claude = is_claude;
        tasks.push(tokio::spawn(async move {
            let tester = cxc_core::connectivity::Tester::new();
            let res = tester.test(&p.base_url, &p.api_key, &p.model, is_claude).await;
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
    cxc_core::connectivity::fetch_models(&base_url, &api_key).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings(app_handle: tauri::AppHandle, target_tool: String, source: String, custom_dir: String, claude_source: String, claude_custom_dir: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    cfg.codex_source = Some(source.clone());
    cfg.codex_custom_dir = custom_dir.clone();
    cfg.claude_source = Some(claude_source.clone());
    cfg.claude_custom_dir = claude_custom_dir.clone();
    config::save(&cfg).map_err(|e| e.to_string())?;

    // Re-apply active provider config to target tool after settings change
    if target_tool == "codex" {
        if let Some(p) = config::get_active(&cfg, "codex").cloned() {
            let codex_adapter = CodexAdapter::new().map_err(|e| e.to_string())?;
            let tc = TargetConfig {
                base_url: p.base_url.clone(),
                api_key: p.api_key.clone(),
                model: p.model.clone(),
                wire_api: if p.wire_api.is_empty() { "responses".to_string() } else { p.wire_api.clone() },
            };
            codex_adapter.write(&tc).map_err(|e| e.to_string())?;
        }
    } else if target_tool == "claude" {
        if let Some(p) = config::get_active(&cfg, "claude").cloned() {
            let claude_adapter = ClaudeAdapter::new().map_err(|e| e.to_string())?;
            claude_adapter.write_provider(&p).map_err(|e| e.to_string())?;
        }
    }

    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
fn get_app_version(app_handle: tauri::AppHandle) -> String {
    app_handle.package_info().version.to_string()
}


fn do_switch_provider(app_handle: &tauri::AppHandle, name: String, target_tool: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;

    let p = config::get_provider(&cfg, &target_tool, &name)
        .ok_or_else(|| format!("provider \"{}\" not found", name))?
        .clone();

    // Write to target tool config file
    if target_tool == "codex" {
        let codex_adapter = CodexAdapter::new().map_err(|e| e.to_string())?;

        let tc = TargetConfig {
            base_url: p.base_url.clone(),
            api_key: p.api_key.clone(),
            model: p.model.clone(),
            wire_api: if p.wire_api.is_empty() { "responses".to_string() } else { p.wire_api.clone() },
        };

        codex_adapter.write(&tc).map_err(|e| e.to_string())?;
    } else if target_tool == "claude" {
        let claude_adapter = ClaudeAdapter::new().map_err(|e| e.to_string())?;
        claude_adapter.write_provider(&p).map_err(|e| e.to_string())?;
    }

    config::set_active(&mut cfg, &target_tool, &name).map_err(|e| e.to_string())?;

    let _ = update_tray_menu(app_handle);

    let _ = notify_rust::Notification::new()
        .summary("CXC")
        .body(&format!("✓ Switched active provider to \"{}\"", name))
        .show();

    let _ = app_handle.emit("config-updated", &cfg);

    Ok(cfg)
}

fn update_tray_menu(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let cfg = config::load().map_err(|e| e.to_string())?;

    let menu = Menu::new(app_handle).map_err(|e| e.to_string())?;

    // System Tray 只显示 Codex providers（已知的设计限制，见 CONTEXT.md）
    // 托盘无法选择 Target Tool，固定使用 Codex 列表
    for p in &cfg.codex_providers {
        let is_active = cfg.codex_active == p.name;
        let item = CheckMenuItem::with_id(
            app_handle,
            format!("prov:{}", p.name),
            &p.name,
            true,
            is_active,
            None::<&str>,
        ).map_err(|e| e.to_string())?;

        menu.append(&item).map_err(|e| e.to_string())?;
    }

    let separator = PredefinedMenuItem::separator(app_handle).map_err(|e| e.to_string())?;
    menu.append(&separator).map_err(|e| e.to_string())?;

    let show_item = MenuItem::with_id(
        app_handle,
        "show_window",
        "Show Window",
        true,
        None::<&str>,
    ).map_err(|e| e.to_string())?;
    menu.append(&show_item).map_err(|e| e.to_string())?;

    let quit_item = MenuItem::with_id(
        app_handle,
        "quit",
        "Quit",
        true,
        None::<&str>,
    ).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    if let Some(tray) = app_handle.tray_by_id("cxc_tray") {
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(target_os = "macos")]
            const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray_iconTemplate.png");
            #[cfg(not(target_os = "macos"))]
            const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray_icon.png");

            let tray_icon = tauri::image::Image::from_bytes(TRAY_ICON_BYTES)
                .expect("Failed to load tray icon");

            let _tray = TrayIconBuilder::with_id("cxc_tray")
                .icon(tray_icon)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(move |tray, event| {
                    match event {
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
                    }
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
            get_app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
