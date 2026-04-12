use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::commands::server::ServerProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub terminology_server_url: Option<String>,
    #[serde(default = "default_check_updates_on_startup")]
    pub check_updates_on_startup: bool,
}

fn default_version() -> u32 {
    1
}

fn default_check_updates_on_startup() -> bool {
    true
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            version: 1,
            terminology_server_url: Some("https://tx.fhir.ch/r4".to_string()),
            check_updates_on_startup: true,
        }
    }
}

fn get_settings_path() -> PathBuf {
    let config_dir = dirs_config_dir().join("openehr-explorer");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("settings.json")
}

fn dirs_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        PathBuf::from(".config")
    }
}

pub fn load_settings() -> GlobalSettings {
    let path = get_settings_path();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str::<GlobalSettings>(&data).unwrap_or_default()
    } else {
        let settings = GlobalSettings::default();
        // Write defaults on first launch
        if let Ok(data) = serde_json::to_string_pretty(&settings) {
            fs::write(&path, data).ok();
        }
        settings
    }
}

fn save_settings_to_disk(settings: &GlobalSettings) -> Result<(), String> {
    let path = get_settings_path();
    let data = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

/// Resolve the effective terminology server URL using the two-level hierarchy:
/// profile override > global default > None
pub fn effective_terminology_url(
    profile: &ServerProfile,
    settings: &GlobalSettings,
) -> Option<String> {
    profile
        .terminology_url
        .clone()
        .or_else(|| settings.terminology_server_url.clone())
}

#[tauri::command]
pub async fn get_config_dir() -> Result<String, String> {
    let config_dir = dirs_config_dir().join("openehr-explorer");
    config_dir
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Could not determine config directory".to_string())
}

#[tauri::command]
pub async fn get_settings() -> Result<GlobalSettings, String> {
    Ok(load_settings())
}

#[tauri::command]
pub async fn save_settings(settings: GlobalSettings) -> Result<GlobalSettings, String> {
    save_settings_to_disk(&settings)?;
    Ok(settings)
}
