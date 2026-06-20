use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub use_random_port: bool,
    pub model_dir: String,
    pub current_model: String,
    pub threads: usize,
    pub context_size: usize,
    pub allow_external: bool,
    pub prompt_template: String,
}

impl AppConfig {
    pub fn default_with_app(app_handle: &tauri::AppHandle) -> Self {
        let app_data = app_handle.path().app_data_dir().unwrap_or_default();
        let default_model_dir = app_data.join("models").to_string_lossy().to_string();
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            port: 8080,
            use_random_port: true,
            model_dir: default_model_dir,
            current_model: "".to_string(),
            threads: cpu_cores,
            context_size: 2048,
            allow_external: false,
            prompt_template: "Translate the following text into {target_lang}. Note that you should only output the translated result without any additional explanation:\n\n{source_text}".to_string(),
        }
    }
}

fn get_config_path(app_handle: &tauri::AppHandle) -> PathBuf {
    let config_dir = app_handle.path().app_config_dir().unwrap_or_default();
    // Ensure config directory exists
    let _ = fs::create_dir_all(&config_dir);
    config_dir.join("config.json")
}

pub fn load_config(app_handle: &tauri::AppHandle) -> AppConfig {
    let path = get_config_path(app_handle);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    
    // Save default config if not existing or invalid
    let config = AppConfig::default_with_app(app_handle);
    let _ = save_config(app_handle, &config);
    config
}

pub fn save_config(app_handle: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = get_config_path(app_handle);
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    Ok(())
}
