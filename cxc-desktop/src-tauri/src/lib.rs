use cxc_core::config::{self, Config, Provider};
use cxc_core::target::{TargetAdapter, TargetConfig, codex::CodexAdapter};
use tauri::{Manager, Emitter};
use tauri::menu::{Menu, MenuItem, CheckMenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;

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
fn switch_provider(app_handle: tauri::AppHandle, name: String) -> Result<Config, String> {
    do_switch_provider(&app_handle, name)
}

#[tauri::command]
fn add_provider(app_handle: tauri::AppHandle, provider: Provider) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::add_provider(&mut cfg, provider).map_err(|e| e.to_string())?;
    config::save(&cfg).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
fn edit_provider(app_handle: tauri::AppHandle, old_name: String, updated: Provider) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    
    if cfg.active == old_name || cfg.active == updated.name {
        let adapter = CodexAdapter::new().map_err(|e| e.to_string())?;
        let tc = TargetConfig {
            base_url: updated.base_url.clone(),
            api_key: updated.api_key.clone(),
            model: updated.model.clone(),
            wire_api: if updated.wire_api.is_empty() { "responses".to_string() } else { updated.wire_api.clone() },
        };
        adapter.write(&tc).map_err(|e| e.to_string())?;
    }

    config::edit_provider(&mut cfg, &old_name, updated).map_err(|e| e.to_string())?;
    config::save(&cfg).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
fn delete_provider(app_handle: tauri::AppHandle, name: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::remove_provider(&mut cfg, &name).map_err(|e| e.to_string())?;
    config::save(&cfg).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
async fn test_provider(app_handle: tauri::AppHandle, name: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let p = config::get_provider(&cfg, &name)
        .ok_or_else(|| format!("provider \"{}\" not found", name))?
        .clone();

    let tester = cxc_core::connectivity::Tester::new();
    let res = tester.test(&p.base_url, &p.api_key, &p.model).await;

    config::update_test_result(&mut cfg, &name, res.latency_ms, res.ok).map_err(|e| e.to_string())?;
    config::save(&cfg).map_err(|e| e.to_string())?;
    let _ = update_tray_menu(&app_handle);

    Ok(cfg)
}

#[tauri::command]
async fn test_all_providers(app_handle: tauri::AppHandle) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    let providers = cfg.providers.clone();
    
    let mut tasks = vec![];
    for p in providers {
        tasks.push(tokio::spawn(async move {
            let tester = cxc_core::connectivity::Tester::new();
            let res = tester.test(&p.base_url, &p.api_key, &p.model).await;
            (p.name.clone(), res.ok, res.latency_ms)
        }));
    }
    
    for task in tasks {
        if let std::result::Result::Ok(res) = task.await {
            let (name, is_ok, latency) = res;
            let _ = config::update_test_result(&mut cfg, &name, latency, is_ok);
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
fn save_settings(app_handle: tauri::AppHandle, source: String, custom_dir: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    cfg.codex_source = Some(source);
    cfg.codex_custom_dir = custom_dir;
    config::save(&cfg).map_err(|e| e.to_string())?;

    if !cfg.active.is_empty() {
        if let Some(p) = config::get_provider(&cfg, &cfg.active) {
            let p = p.clone();
            let adapter = CodexAdapter::new().map_err(|e| e.to_string())?;
            let tc = TargetConfig {
                base_url: p.base_url.clone(),
                api_key: p.api_key.clone(),
                model: p.model.clone(),
                wire_api: if p.wire_api.is_empty() { "responses".to_string() } else { p.wire_api.clone() },
            };
            adapter.write(&tc).map_err(|e| e.to_string())?;
        }
    }

    let _ = update_tray_menu(&app_handle);
    Ok(cfg)
}

#[tauri::command]
fn get_app_version(app_handle: tauri::AppHandle) -> String {
    app_handle.package_info().version.to_string()
}


fn do_switch_provider(app_handle: &tauri::AppHandle, name: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    
    let p = config::get_provider(&cfg, &name)
        .ok_or_else(|| format!("provider \"{}\" not found", name))?
        .clone();
        
    let adapter = CodexAdapter::new().map_err(|e| e.to_string())?;
    
    let tc = TargetConfig {
        base_url: p.base_url.clone(),
        api_key: p.api_key.clone(),
        model: p.model.clone(),
        wire_api: if p.wire_api.is_empty() { "responses".to_string() } else { p.wire_api.clone() },
    };
    
    adapter.write(&tc).map_err(|e| e.to_string())?;
    
    config::set_active(&mut cfg, &name).map_err(|e| e.to_string())?;
    
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
    
    for p in &cfg.providers {
        let is_active = cfg.active == p.name;
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
                        let _ = do_switch_provider(app_handle, prov_name);
                    }
                })
                .build(app)?;
                
            let _ = update_tray_menu(&handle);
            
            Ok(())
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
