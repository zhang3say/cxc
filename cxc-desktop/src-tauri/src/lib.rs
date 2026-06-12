use cxc_core::config::{self, Config, Provider};
use cxc_core::target::{TargetAdapter, TargetConfig, codex::CodexAdapter};

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
fn switch_provider(name: String) -> Result<Config, String> {
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
    
    Ok(cfg)
}

#[tauri::command]
fn add_provider(provider: Provider) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::add_provider(&mut cfg, provider).map_err(|e| e.to_string())?;
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
fn edit_provider(old_name: String, updated: Provider) -> Result<Config, String> {
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
    Ok(cfg)
}

#[tauri::command]
fn delete_provider(name: String) -> Result<Config, String> {
    let mut cfg = config::load().map_err(|e| e.to_string())?;
    config::remove_provider(&mut cfg, &name).map_err(|e| e.to_string())?;
    config::save(&cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[tauri::command]
async fn fetch_models(base_url: String, api_key: String) -> Result<Vec<String>, String> {
    cxc_core::connectivity::fetch_models(&base_url, &api_key).await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet, 
            get_config, 
            switch_provider,
            add_provider,
            edit_provider,
            delete_provider,
            fetch_models
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
