//! Server profiles, compatible with the Tauri app (`src-tauri/src/commands/server.rs`).
//!
//! Same files, same serde shapes, same credential handling:
//! * `<config>/openehr-explorer/profiles.json` holds profiles **without secrets** (`{"profiles":[...]}`),
//! * secrets live in the OS keychain (service `openehr-explorer`, account `<profile id>:<field>`) or, when no
//!   keychain is available, in the encrypted fallback store. Both are handled by the *same* `credentials.rs` file
//!   the Tauri app compiles, included via `#[path]` in `main.rs`, so the formats cannot drift.
//!
//! The semantics mirror the Tauri commands: an empty password/token on save means "keep the stored secret",
//! saving preserves the profile's default flag, deleting removes all four secret fields, and the default toggle
//! clears the flag when clicked on the current default.

use crate::credentials::{harden_file_permissions, CredentialManager, StorageBackend};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerType {
    Ehrbase,
    BetterPlatform,
    FerroEhr,
    Generic,
}

impl ServerType {
    pub const ALL: [ServerType; 4] = [ServerType::Ehrbase, ServerType::BetterPlatform, ServerType::FerroEhr, ServerType::Generic];

    /// The serialized name, as shown by the Tauri UI badge.
    pub fn as_str(self) -> &'static str {
        match self {
            ServerType::Ehrbase => "ehrbase",
            ServerType::BetterPlatform => "better_platform",
            ServerType::FerroEhr => "ferro_ehr",
            ServerType::Generic => "generic",
        }
    }
}

/// Auth with resolved secrets. Never written to disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
}

/// On-disk auth: secrets stripped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoredAuthMethod {
    None,
    Basic { username: String },
    Bearer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub server_type: ServerType,
    pub auth_method: StoredAuthMethod,
    #[serde(default)]
    pub admin_auth_method: Option<StoredAuthMethod>,
    #[serde(default)]
    pub terminology_url: Option<String>,
    #[serde(default)]
    pub api_path_prefix: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProfileStore {
    profiles: Vec<StoredProfile>,
}

/// A profile with its secrets resolved, ready to make requests with.
#[derive(Debug, Clone)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub server_type: ServerType,
    pub auth_method: AuthMethod,
    pub api_path_prefix: Option<String>,
}

/// What the form submits. Empty secret = keep the stored one (same as the Tauri `ServerProfileInput`).
#[derive(Debug, Clone)]
pub struct ProfileInput {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub server_type: ServerType,
    pub auth_method: AuthMethod,
    pub admin_auth_method: Option<AuthMethod>,
    pub terminology_url: Option<String>,
    pub api_path_prefix: Option<String>,
}

pub const DEFAULT_API_PATH_PREFIX: &str = "/rest/openehr/v1";

// ---------------------------------------------------------------------------
// Paths + credential manager (same rules as the Tauri app, including its XDG/HOME logic)
// ---------------------------------------------------------------------------

pub fn config_dir() -> PathBuf {
    let base = if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        PathBuf::from(".config")
    };
    base.join("openehr-explorer")
}

fn profiles_path() -> PathBuf {
    let dir = config_dir();
    fs::create_dir_all(&dir).ok();
    dir.join("profiles.json")
}

/// Singleton so the keychain probe runs once per process (backend never flips between calls).
fn cred_manager() -> &'static CredentialManager {
    static MGR: OnceLock<CredentialManager> = OnceLock::new();
    MGR.get_or_init(|| {
        let dir = config_dir();
        fs::create_dir_all(&dir).ok();
        CredentialManager::new(&dir)
    })
}

pub fn credential_backend() -> &'static str {
    match cred_manager().backend() {
        StorageBackend::OsKeychain => "os_keychain",
        StorageBackend::EncryptedFile => "encrypted_file",
    }
}

// ---------------------------------------------------------------------------
// Normalisation helpers (verbatim semantics from server.rs)
// ---------------------------------------------------------------------------

/// `None` (blank field) = use the default; an explicit empty/`/` value is preserved as "" (base URL is the root).
pub fn normalize_api_path_prefix(prefix: Option<String>) -> Option<String> {
    let prefix = prefix?;
    let inner = prefix.trim().trim_matches('/');
    Some(if inner.is_empty() { String::new() } else { format!("/{inner}") })
}

pub fn normalize_bearer_token(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let token = match trimmed.get(..7) {
        Some(prefix) if prefix.eq_ignore_ascii_case("bearer ") => trimmed[7..].trim(),
        _ => trimmed,
    };
    if token.contains('\u{2026}') || token.contains("...") {
        return Err("The bearer token contains an ellipsis (… or ...): your browser's devtools truncated it. Copy the full value.".to_string());
    }
    if token.chars().any(|c| !c.is_ascii_graphic()) {
        return Err("The bearer token contains whitespace or non-ASCII characters and is not a valid token. Paste only the token itself.".to_string());
    }
    Ok(token.to_string())
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

pub fn load_stored() -> Vec<StoredProfile> {
    let path = profiles_path();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str::<ProfileStore>(&data).map(|s| s.profiles).unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_stored(profiles: &[StoredProfile]) -> Result<(), String> {
    let path = profiles_path();
    let data = serde_json::to_string_pretty(&ProfileStore { profiles: profiles.to_vec() }).map_err(|e| e.to_string())?;
    fs::write(&path, &data).map_err(|e| e.to_string())?;
    harden_file_permissions(&path);
    Ok(())
}

fn to_stored_auth(auth: &AuthMethod) -> StoredAuthMethod {
    match auth {
        AuthMethod::None => StoredAuthMethod::None,
        AuthMethod::Basic { username, .. } => StoredAuthMethod::Basic { username: username.clone() },
        AuthMethod::Bearer { .. } => StoredAuthMethod::Bearer,
    }
}

fn store_auth_secrets(mgr: &CredentialManager, id: &str, prefix: &str, auth: &AuthMethod, dir: &std::path::Path) -> Result<(), String> {
    match auth {
        AuthMethod::None => {
            mgr.delete_secret(id, &format!("{prefix}password"), dir)?;
            mgr.delete_secret(id, &format!("{prefix}token"), dir)?;
        }
        AuthMethod::Basic { password, .. } => {
            if !password.is_empty() {
                mgr.store_secret(id, &format!("{prefix}password"), password.clone(), dir)?;
            }
            mgr.delete_secret(id, &format!("{prefix}token"), dir)?;
        }
        AuthMethod::Bearer { token } => {
            if !token.is_empty() {
                let token = normalize_bearer_token(token)?;
                mgr.store_secret(id, &format!("{prefix}token"), token, dir)?;
            }
            mgr.delete_secret(id, &format!("{prefix}password"), dir)?;
        }
    }
    Ok(())
}

fn resolve_auth(mgr: &CredentialManager, id: &str, prefix: &str, stored: &StoredAuthMethod, dir: &std::path::Path) -> Result<AuthMethod, String> {
    Ok(match stored {
        StoredAuthMethod::None => AuthMethod::None,
        StoredAuthMethod::Basic { username } => AuthMethod::Basic {
            username: username.clone(),
            password: mgr.load_secret(id, &format!("{prefix}password"), dir)?.unwrap_or_default(),
        },
        StoredAuthMethod::Bearer => AuthMethod::Bearer { token: mgr.load_secret(id, &format!("{prefix}token"), dir)?.unwrap_or_default() },
    })
}

/// Resolve a stored profile's secrets (keychain / encrypted store) into a usable profile.
pub fn resolve(stored: &StoredProfile) -> Result<ServerProfile, String> {
    let dir = config_dir();
    Ok(ServerProfile {
        id: stored.id.clone(),
        name: stored.name.clone(),
        base_url: stored.base_url.clone(),
        server_type: stored.server_type,
        auth_method: resolve_auth(cred_manager(), &stored.id, "", &stored.auth_method, &dir)?,
        api_path_prefix: stored.api_path_prefix.clone(),
    })
}

/// Whether a secret is stored for this profile's primary auth (drives the "leave empty to keep" hint).
pub fn has_stored_secret(stored: &StoredProfile) -> bool {
    let dir = config_dir();
    let field = match stored.auth_method {
        StoredAuthMethod::Basic { .. } => "password",
        StoredAuthMethod::Bearer => "token",
        StoredAuthMethod::None => return false,
    };
    matches!(cred_manager().load_secret(&stored.id, field, &dir), Ok(Some(s)) if !s.is_empty())
}

/// Insert or update (matched by id). Returns the refreshed list.
pub fn save(input: ProfileInput) -> Result<Vec<StoredProfile>, String> {
    let dir = config_dir();
    let mgr = cred_manager();
    store_auth_secrets(mgr, &input.id, "", &input.auth_method, &dir)?;
    if let Some(admin) = &input.admin_auth_method {
        store_auth_secrets(mgr, &input.id, "admin_", admin, &dir)?;
    } else {
        mgr.delete_secret(&input.id, "admin_password", &dir)?;
        mgr.delete_secret(&input.id, "admin_token", &dir)?;
    }
    let mut profiles = load_stored();
    // Preserve the existing default flag; the form doesn't carry it.
    let is_default = profiles.iter().find(|p| p.id == input.id).map(|p| p.is_default).unwrap_or(false);
    let stored = StoredProfile {
        id: input.id.clone(),
        name: input.name,
        base_url: input.base_url,
        server_type: input.server_type,
        auth_method: to_stored_auth(&input.auth_method),
        admin_auth_method: input.admin_auth_method.as_ref().map(to_stored_auth),
        terminology_url: input.terminology_url,
        api_path_prefix: normalize_api_path_prefix(input.api_path_prefix),
        is_default,
    };
    if let Some(existing) = profiles.iter_mut().find(|p| p.id == stored.id) {
        *existing = stored;
    } else {
        profiles.push(stored);
    }
    save_stored(&profiles)?;
    Ok(profiles)
}

pub fn delete(id: &str) -> Result<Vec<StoredProfile>, String> {
    let dir = config_dir();
    let mgr = cred_manager();
    mgr.delete_all_secrets(id, &dir)?;
    let mut profiles = load_stored();
    profiles.retain(|p| p.id != id);
    save_stored(&profiles)?;
    Ok(profiles)
}

/// Toggle: clicking the current default clears it; otherwise it becomes the sole default.
pub fn set_default(id: &str) -> Result<Vec<StoredProfile>, String> {
    let mut profiles = load_stored();
    let already = profiles.iter().find(|p| p.id == id).map(|p| p.is_default).ok_or_else(|| format!("Server profile '{id}' not found"))?;
    for p in profiles.iter_mut() {
        p.is_default = !already && p.id == id;
    }
    save_stored(&profiles)?;
    Ok(profiles)
}

/// The profile the app starts on: the default one, else the first (same rule as the Tauri store).
pub fn initial_active(profiles: &[StoredProfile]) -> Option<&StoredProfile> {
    profiles.iter().find(|p| p.is_default).or_else(|| profiles.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// profiles.json exactly as the Tauri app writes it (serde_json::to_string_pretty of ProfileStore).
    const TAURI_FIXTURE: &str = r#"{
  "profiles": [
    {
      "id": "11111111-1111-4111-8111-111111111111",
      "name": "Local EHRBase",
      "base_url": "http://localhost:8080/ehrbase",
      "server_type": "ehrbase",
      "auth_method": { "type": "basic", "username": "ehrbase-user" },
      "admin_auth_method": { "type": "basic", "username": "ehrbase-admin" },
      "terminology_url": "https://tx.example.org/fhir",
      "api_path_prefix": null,
      "is_default": true
    },
    {
      "id": "22222222-2222-4222-8222-222222222222",
      "name": "Better",
      "base_url": "https://better.example.org",
      "server_type": "better_platform",
      "auth_method": { "type": "bearer" },
      "api_path_prefix": "/rest/v1",
      "is_default": false
    },
    {
      "id": "33333333-3333-4333-8333-333333333333",
      "name": "Legacy without optional fields",
      "base_url": "https://x.example.org",
      "server_type": "ferro_ehr",
      "auth_method": { "type": "none" }
    }
  ]
}"#;

    #[test]
    fn reads_what_tauri_writes() {
        let store: ProfileStore = serde_json::from_str(TAURI_FIXTURE).unwrap();
        assert_eq!(store.profiles.len(), 3);
        assert_eq!(store.profiles[0].auth_method, StoredAuthMethod::Basic { username: "ehrbase-user".into() });
        assert_eq!(store.profiles[0].admin_auth_method, Some(StoredAuthMethod::Basic { username: "ehrbase-admin".into() }));
        assert!(store.profiles[0].is_default);
        assert_eq!(store.profiles[1].auth_method, StoredAuthMethod::Bearer);
        assert_eq!(store.profiles[1].api_path_prefix.as_deref(), Some("/rest/v1"));
        assert_eq!(store.profiles[2].server_type, ServerType::FerroEhr);
        assert!(!store.profiles[2].is_default);
        assert_eq!(initial_active(&store.profiles).unwrap().name, "Local EHRBase");
    }

    #[test]
    fn writes_the_same_shape_and_never_a_secret() {
        let p = StoredProfile {
            id: "i".into(),
            name: "n".into(),
            base_url: "http://h".into(),
            server_type: ServerType::BetterPlatform,
            auth_method: to_stored_auth(&AuthMethod::Basic { username: "u".into(), password: "SECRET".into() }),
            admin_auth_method: None,
            terminology_url: None,
            api_path_prefix: Some("/x".into()),
            is_default: false,
        };
        let json = serde_json::to_string(&ProfileStore { profiles: vec![p] }).unwrap();
        assert!(!json.contains("SECRET") && !json.contains("password"));
        assert!(json.contains(r#""server_type":"better_platform""#));
        assert!(json.contains(r#""auth_method":{"type":"basic","username":"u"}"#));
        assert!(json.contains(r#""admin_auth_method":null"#));
    }

    #[test]
    fn api_prefix_normalisation_matches_tauri() {
        assert_eq!(normalize_api_path_prefix(None), None);
        assert_eq!(normalize_api_path_prefix(Some("  rest/v2/ ".into())).as_deref(), Some("/rest/v2"));
        assert_eq!(normalize_api_path_prefix(Some("/".into())).as_deref(), Some(""));
        assert_eq!(normalize_api_path_prefix(Some("".into())).as_deref(), Some(""));
    }

    #[test]
    fn bearer_token_normalisation_matches_tauri() {
        assert_eq!(normalize_bearer_token("  Bearer abc.def.ghi \n").unwrap(), "abc.def.ghi");
        assert!(normalize_bearer_token("abc…").is_err());
        assert!(normalize_bearer_token("a b").is_err());
    }

    #[test]
    fn default_toggle_semantics() {
        let mut ps: Vec<StoredProfile> = serde_json::from_str::<ProfileStore>(TAURI_FIXTURE).unwrap().profiles;
        // clicking the current default clears it; first-of-list is then the fallback
        let (id, already) = (ps[0].id.clone(), ps[0].is_default);
        for p in ps.iter_mut() {
            p.is_default = !already && p.id == id;
        }
        assert!(ps.iter().all(|p| !p.is_default));
        assert_eq!(initial_active(&ps).unwrap().name, "Local EHRBase");
    }
}
