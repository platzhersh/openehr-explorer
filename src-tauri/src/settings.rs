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
    /// Opt-in flag for anonymous usage analytics (Aptabase). Defaults to `false`
    /// per ADR-0018 — no events are sent unless the user explicitly enables it
    /// from the Settings page.
    #[serde(default)]
    pub analytics_enabled: bool,
    /// Tracks whether we've already prompted the user for an analytics
    /// consent decision. Defaults to `false`, which means "never asked" —
    /// the frontend shows a one-time first-run dialog when it sees this
    /// flag, then flips it to `true` regardless of which choice the user
    /// made. This is how we distinguish "user explicitly opted out" from
    /// "we haven't asked yet" without having to make `analytics_enabled`
    /// an `Option<bool>`.
    #[serde(default)]
    pub analytics_consent_asked: bool,
    /// Master toggle for the route-aware feature tour and "What's New"
    /// system (see PRD-0018). When `false`, tours never auto-start and the
    /// What's New modal never appears on version upgrades — the user can
    /// still trigger either manually. Defaults to `true`.
    #[serde(default = "default_tours_enabled")]
    pub tours_enabled: bool,
    /// IDs of feature tours the user has completed or explicitly skipped.
    /// A tour whose ID appears here never auto-starts again, though it can
    /// still be replayed manually from Settings.
    #[serde(default)]
    pub completed_tours: Vec<String>,
    /// App version the user last saw the "What's New" summary for. `None`
    /// means either a fresh install or an upgrade from a version predating
    /// this field — both are treated as "nothing to announce yet" so we
    /// don't dump the full changelog on a brand-new user; the frontend just
    /// records the current version as the baseline.
    #[serde(default)]
    pub last_seen_version: Option<String>,
}

fn default_tours_enabled() -> bool {
    true
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
            analytics_enabled: false,
            analytics_consent_asked: false,
            tours_enabled: true,
            completed_tours: Vec::new(),
            last_seen_version: None,
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
