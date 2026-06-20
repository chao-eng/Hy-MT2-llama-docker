mod config;

use config::AppConfig;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_dialog::DialogExt;

struct AppState {
    child_process: Mutex<Option<CommandChild>>,
    active_port: Mutex<Option<u16>>,
}

#[derive(Serialize)]
struct ServerStatus {
    running: bool,
    port: Option<u16>,
}

#[tauri::command]
fn get_config(app_handle: AppHandle) -> AppConfig {
    config::load_config(&app_handle)
}

#[tauri::command]
fn set_config(app_handle: AppHandle, config: AppConfig) -> Result<(), String> {
    config::save_config(&app_handle, &config)
}

#[tauri::command]
fn check_model_status(app_handle: AppHandle) -> bool {
    let config = config::load_config(&app_handle);
    if config.current_model.is_empty() {
        return false;
    }
    let model_path = Path::new(&config.model_dir).join(&config.current_model);
    model_path.exists()
}

#[tauri::command]
async fn select_directory(app_handle: AppHandle) -> Result<Option<String>, String> {
    let path_opt = app_handle.dialog().file().blocking_pick_folder();
    Ok(path_opt.map(|fp| fp.to_string()))
}

#[tauri::command]
async fn check_server_status(state: State<'_, AppState>) -> Result<ServerStatus, String> {
    let child_guard = state.child_process.lock().map_err(|e| e.to_string())?;
    let port_guard = state.active_port.lock().map_err(|e| e.to_string())?;
    Ok(ServerStatus {
        running: child_guard.is_some(),
        port: *port_guard,
    })
}

#[tauri::command]
async fn start_server(app_handle: AppHandle, state: State<'_, AppState>) -> Result<u16, String> {
    // 1. Stop any currently running server
    let mut child_guard = state.child_process.lock().map_err(|e| e.to_string())?;
    if let Some(child) = child_guard.take() {
        let _ = child.kill();
    }

    // 2. Load config
    let config = config::load_config(&app_handle);

    // 3. Determine port
    let port = if config.use_random_port {
        portpicker::pick_unused_port().ok_or_else(|| "No free port available".to_string())?
    } else {
        config.port
    };

    // 4. Ensure model file exists
    if config.current_model.is_empty() {
        return Err("No model selected. Please select a model in settings.".to_string());
    }
    let model_path = Path::new(&config.model_dir).join(&config.current_model);
    if !model_path.exists() {
        return Err(format!("Model file '{}' not found in models directory.", config.current_model));
    }

    // 5. Configure host
    let host = if config.allow_external { "0.0.0.0" } else { "127.0.0.1" };

    // 6. Spawn sidecar
    let sidecar = app_handle.shell().sidecar("llama-server")
        .map_err(|e| format!("Failed to find sidecar: {}", e))?;

    let (mut rx, child) = sidecar
        .args(vec![
            "-m".to_string(),
            model_path.to_string_lossy().to_string(),
            "-c".to_string(),
            config.context_size.to_string(),
            "-t".to_string(),
            config.threads.to_string(),
            "--host".to_string(),
            host.to_string(),
            "--port".to_string(),
            port.to_string(),
            "--parallel".to_string(),
            "2".to_string(),
            "--cont-batching".to_string(),
            "--kv-unified".to_string(),
        ])
        .spawn()
        .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

    // Forward stdout and stderr to standard console for logging
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line);
                    println!("llama-server-stdout: {}", text);
                }
                CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line);
                    eprintln!("llama-server-stderr: {}", text);
                }
                _ => {}
            }
        }
    });

    // 7. Store state
    *child_guard = Some(child);
    let mut port_guard = state.active_port.lock().map_err(|e| e.to_string())?;
    *port_guard = Some(port);

    Ok(port)
}

#[tauri::command]
async fn stop_server(state: State<'_, AppState>) -> Result<(), String> {
    let mut child_guard = state.child_process.lock().map_err(|e| e.to_string())?;
    if let Some(child) = child_guard.take() {
        let _ = child.kill();
    }
    let mut port_guard = state.active_port.lock().map_err(|e| e.to_string())?;
    *port_guard = None;
    Ok(())
}

#[tauri::command]
fn list_models(app_handle: AppHandle, custom_dir: Option<String>) -> Result<Vec<String>, String> {
    let dir_str = match custom_dir {
        Some(d) => d,
        None => {
            let config = config::load_config(&app_handle);
            config.model_dir
        }
    };
    let dir = Path::new(&dir_str);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut models = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.ends_with(".gguf") {
                        models.push(file_name);
                    }
                }
            }
        }
    }
    models.sort();
    Ok(models)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            child_process: Mutex::new(None),
            active_port: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            check_model_status,
            select_directory,
            check_server_status,
            start_server,
            stop_server,
            list_models
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app_handle.state::<AppState>();
                let mut child_guard = state.child_process.lock().unwrap();
                if let Some(child) = child_guard.take() {
                    let _ = child.kill();
                }
            }
        });
}
