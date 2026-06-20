mod config;

use config::AppConfig;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_dialog::DialogExt;
use futures_util::StreamExt;

struct AppState {
    child_process: Mutex<Option<CommandChild>>,
    active_port: Mutex<Option<u16>>,
    downloading: Mutex<bool>,
}

#[derive(Serialize, Clone)]
struct DownloadProgress {
    percentage: f64,
    speed: f64, // MB/s
    downloaded_bytes: u64,
    total_bytes: u64,
    finished: bool,
    error: Option<String>,
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
    let model_path = Path::new(&config.model_dir).join("Hy-MT2-1.8B-1.25Bit.gguf");
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
    let model_path = Path::new(&config.model_dir).join("Hy-MT2-1.8B-1.25Bit.gguf");
    if !model_path.exists() {
        return Err("Model file not found. Please download the model first.".to_string());
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
async fn download_model(app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut downloading_guard = state.downloading.lock().map_err(|e| e.to_string())?;
    if *downloading_guard {
        return Err("Download already in progress".to_string());
    }
    *downloading_guard = true;

    let config = config::load_config(&app_handle);
    let model_dir = PathBuf::from(&config.model_dir);
    let _ = fs::create_dir_all(&model_dir);
    let target_path = model_dir.join("Hy-MT2-1.8B-1.25Bit.gguf");
    let tmp_path = model_dir.join("Hy-MT2-1.8B-1.25Bit.gguf.tmp");

    let app_clone = app_handle.clone();

    tokio::spawn(async move {
        let result = do_download(&app_clone, &tmp_path, &target_path).await;
        
        let state_clone = app_clone.state::<AppState>();
        let mut downloading_guard = state_clone.downloading.lock().unwrap();
        *downloading_guard = false;

        let payload = match result {
            Ok(_) => DownloadProgress {
                percentage: 100.0,
                speed: 0.0,
                downloaded_bytes: 0,
                total_bytes: 0,
                finished: true,
                error: None,
            },
            Err(err) => DownloadProgress {
                percentage: 0.0,
                speed: 0.0,
                downloaded_bytes: 0,
                total_bytes: 0,
                finished: false,
                error: Some(err),
            },
        };
        let _ = app_clone.emit("download-progress", payload);
    });

    Ok(())
}

async fn do_download(
    app_handle: &AppHandle,
    tmp_path: &PathBuf,
    target_path: &PathBuf,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = "https://modelscope.cn/api/v1/models/Tencent-Hunyuan/Hy-MT2-1.8B-1.25Bit-GGUF/repo?op=view&path=Hy-MT2-1.8B-1.25Bit.gguf";
    
    let res = client.get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Download failed with status code: {}", res.status()));
    }

    let total_size = res.content_length().unwrap_or(0);
    let mut file = fs::File::create(tmp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut stream = res.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();
    let start_time = Instant::now();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Error while downloading chunk: {}", e))?;
        use std::io::Write;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;
        
        downloaded += chunk.len() as u64;

        if last_emit.elapsed().as_millis() >= 400 {
            let percentage = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
            
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            let speed = if elapsed_secs > 0.0 {
                (downloaded as f64 / 1024.0 / 1024.0) / elapsed_secs
            } else {
                0.0
            };

            let payload = DownloadProgress {
                percentage,
                speed,
                downloaded_bytes: downloaded,
                total_bytes: total_size,
                finished: false,
                error: None,
            };
            
            let _ = app_handle.emit("download-progress", payload);
            last_emit = Instant::now();
        }
    }

    fs::rename(tmp_path, target_path)
        .map_err(|e| format!("Failed to finalize model file: {}", e))?;

    Ok(())
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
            downloading: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            check_model_status,
            select_directory,
            check_server_status,
            start_server,
            stop_server,
            download_model
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
