//! Global settings, compatible with the Tauri app (`src-tauri/src/settings.rs`): the same `settings.json` in the same
//! config directory, the same serde shape and defaults, so toggling a setting in either app is seen by the other.
//!
//! Fields this PoC does not use (updates, tours, `completed_tours`, `last_seen_version`, `analytics_consent_asked`) are
//! preserved untouched on save, so saving here never discards the Tauri app's state.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalSettings {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub terminology_server_url: Option<String>,
    #[serde(default = "default_true")]
    pub check_updates_on_startup: bool,
    /// Opt-in analytics; **off by default**.
    #[serde(default)]
    pub analytics_enabled: bool,
    #[serde(default)]
    pub analytics_consent_asked: bool,
    #[serde(default = "default_true")]
    pub tours_enabled: bool,
    #[serde(default)]
    pub completed_tours: Vec<String>,
    #[serde(default)]
    pub last_seen_version: Option<String>,
}

fn default_version() -> u32 {
    1
}

fn default_true() -> bool {
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

pub fn path() -> PathBuf {
    crate::profiles::config_dir().join("settings.json")
}

/// Load, falling back to defaults when the file is missing (same as the Tauri app). Unlike the Tauri app this does not write
/// the defaults back on first launch; nothing is written until the user saves. A file that exists but cannot be read or
/// parsed also yields defaults in memory, but `save` will not silently overwrite it (see `save`).
pub fn load() -> GlobalSettings {
    match fs::read_to_string(path()) {
        Ok(data) => serde_json::from_str::<GlobalSettings>(&data).unwrap_or_default(),
        Err(_) => GlobalSettings::default(),
    }
}

/// Atomic write. If the existing file cannot be parsed it is first backed up to `settings.json.invalid`, so saving the
/// defaults over a corrupted (or newer-format) file never destroys the only copy of its contents.
pub fn save(settings: &GlobalSettings) -> Result<(), String> {
    let path = path();
    if let Ok(existing) = fs::read_to_string(&path) {
        if serde_json::from_str::<GlobalSettings>(&existing).is_err() {
            fs::write(path.with_extension("json.invalid"), existing).map_err(|e| format!("Could not back up the unreadable settings file: {e}"))?;
        }
    }
    let data = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    crate::profiles::write_atomic(&path, &data, false)
}

/// The files the app keeps in its config directory, for the "Settings Files" panel (name, what it holds, present?).
pub fn config_files() -> Vec<(&'static str, &'static str, bool)> {
    let dir = crate::profiles::config_dir();
    [
        ("profiles.json", "Server profiles (no secrets)"),
        ("settings.json", "Global settings"),
        ("credentials.enc", "Encrypted credentials (only without an OS keychain)"),
    ]
    .into_iter()
    .map(|(f, what)| (f, what, dir.join(f).exists()))
    .collect()
}

/// Open a directory in the platform file manager.
pub fn open_dir(dir: &std::path::Path) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener).arg(dir).spawn().map(|_| ()).map_err(|e| format!("Could not run {opener}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// settings.json as the Tauri app writes it.
    const TAURI_FIXTURE: &str = r#"{
  "version": 1,
  "terminology_server_url": "https://tx.example.org/fhir",
  "check_updates_on_startup": false,
  "analytics_enabled": true,
  "analytics_consent_asked": true,
  "tours_enabled": false,
  "completed_tours": ["app", "ehrs"],
  "last_seen_version": "0.10.3"
}"#;

    #[test]
    fn reads_what_tauri_writes() {
        let s: GlobalSettings = serde_json::from_str(TAURI_FIXTURE).unwrap();
        assert_eq!(s.terminology_server_url.as_deref(), Some("https://tx.example.org/fhir"));
        assert!(!s.check_updates_on_startup && s.analytics_enabled && s.analytics_consent_asked && !s.tours_enabled);
        assert_eq!(s.completed_tours, vec!["app", "ehrs"]);
        assert_eq!(s.last_seen_version.as_deref(), Some("0.10.3"));
    }

    #[test]
    fn round_trip_preserves_fields_this_app_does_not_edit() {
        let mut s: GlobalSettings = serde_json::from_str(TAURI_FIXTURE).unwrap();
        s.terminology_server_url = None; // an edit made here
        let back: GlobalSettings = serde_json::from_str(&serde_json::to_string_pretty(&s).unwrap()).unwrap();
        assert_eq!(back.completed_tours, vec!["app", "ehrs"]);
        assert_eq!(back.last_seen_version.as_deref(), Some("0.10.3"));
        assert!(back.analytics_consent_asked && !back.tours_enabled);
    }

    #[test]
    fn missing_fields_use_the_tauri_serde_defaults() {
        let s: GlobalSettings = serde_json::from_str("{}").unwrap();
        // serde defaults (not `Default`): terminology None, updates and tours on, analytics off
        assert_eq!(s.terminology_server_url, None);
        assert!(s.check_updates_on_startup && s.tours_enabled && !s.analytics_enabled);
        assert_eq!(s.version, 1);
    }

    #[test]
    fn analytics_is_off_by_default() {
        assert!(!GlobalSettings::default().analytics_enabled);
        assert_eq!(GlobalSettings::default().terminology_server_url.as_deref(), Some("https://tx.fhir.ch/r4"));
    }
}
