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

/// Write `data` to `path` atomically: a sibling temp file is written and flushed completely (with `harden` applied), and only then
/// renamed over the destination, so a failed write never leaves a truncated or half-written file behind.
pub fn write_atomic(path: &std::path::Path, data: &str, harden: bool) -> Result<(), String> {
    use std::io::Write;
    let dir = path.parent().ok_or_else(|| "invalid path".to_string())?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!(".{}.tmp", path.file_name().and_then(|n| n.to_str()).unwrap_or("file")));
    let written = (|| -> Result<(), String> {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        if harden {
            harden_file_permissions(&tmp);
        }
        f.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        e.to_string()
    })
}

/// An empty list only when `profiles.json` does not exist; an unreadable or unparseable file is an error, so callers
/// never mutate (and thereby overwrite) a store they could not read.
pub fn load_stored() -> Result<Vec<StoredProfile>, String> {
    let path = profiles_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    serde_json::from_str::<ProfileStore>(&data).map(|s| s.profiles).map_err(|e| format!("Could not parse {}: {e}. Fix or remove the file; nothing was changed.", path.display()))
}

fn save_stored(profiles: &[StoredProfile]) -> Result<(), String> {
    let data = serde_json::to_string_pretty(&ProfileStore { profiles: profiles.to_vec() }).map_err(|e| e.to_string())?;
    write_atomic(&profiles_path(), &data, true)
}

fn to_stored_auth(auth: &AuthMethod) -> StoredAuthMethod {
    match auth {
        AuthMethod::None => StoredAuthMethod::None,
        AuthMethod::Basic { username, .. } => StoredAuthMethod::Basic { username: username.clone() },
        AuthMethod::Bearer { .. } => StoredAuthMethod::Bearer,
    }
}

/// Write the secrets this auth method carries (an empty password/token means "keep the stored one").
fn put_auth_secrets(mgr: &CredentialManager, id: &str, prefix: &str, auth: &AuthMethod, dir: &std::path::Path) -> Result<(), String> {
    match auth {
        AuthMethod::None => {}
        AuthMethod::Basic { password, .. } => {
            if !password.is_empty() {
                mgr.store_secret(id, &format!("{prefix}password"), password.clone(), dir)?;
            }
        }
        AuthMethod::Bearer { token } => {
            // Normalise first: a whitespace-only token must keep the stored one, like an empty field.
            let token = normalize_bearer_token(token)?;
            if !token.is_empty() {
                mgr.store_secret(id, &format!("{prefix}token"), token, dir)?;
            }
        }
    }
    Ok(())
}

/// Secret fields this auth method makes obsolete; removed only after the profile itself was committed.
fn stale_secret_fields(prefix: &str, auth: &AuthMethod) -> Vec<String> {
    let drop: &[&str] = match auth {
        AuthMethod::None => &["password", "token"],
        AuthMethod::Basic { .. } => &["token"],
        AuthMethod::Bearer { .. } => &["password"],
    };
    drop.iter().map(|f| format!("{prefix}{f}")).collect()
}

const SECRET_FIELDS: [&str; 4] = ["password", "token", "admin_password", "admin_token"];

/// Current secrets of a profile, to restore them if committing the profile fails.
fn snapshot_secrets(mgr: &CredentialManager, id: &str, dir: &std::path::Path) -> Result<Vec<(&'static str, Option<String>)>, String> {
    SECRET_FIELDS.iter().map(|f| Ok((*f, mgr.load_secret(id, f, dir)?))).collect()
}

fn restore_secrets(mgr: &CredentialManager, id: &str, snapshot: &[(&'static str, Option<String>)], dir: &std::path::Path) {
    for (field, value) in snapshot {
        let _ = match value {
            Some(v) => mgr.store_secret(id, field, v.clone(), dir),
            None => mgr.delete_secret(id, field, dir),
        };
    }
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

/// Insert or update (matched by id). Returns the refreshed list. New secrets are staged first, the profile is committed,
/// and only then are obsolete secrets removed; if the commit fails the previous secrets are restored.
pub fn save(input: ProfileInput) -> Result<Vec<StoredProfile>, String> {
    let dir = config_dir();
    let mgr = cred_manager();
    let mut profiles = load_stored()?;
    let snapshot = snapshot_secrets(mgr, &input.id, &dir)?;
    let staged = put_auth_secrets(mgr, &input.id, "", &input.auth_method, &dir).and_then(|_| match &input.admin_auth_method {
        Some(admin) => put_auth_secrets(mgr, &input.id, "admin_", admin, &dir),
        None => Ok(()),
    });
    if let Err(e) = staged {
        restore_secrets(mgr, &input.id, &snapshot, &dir);
        return Err(e);
    }
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
    if let Err(e) = save_stored(&profiles) {
        restore_secrets(mgr, &input.id, &snapshot, &dir);
        return Err(e);
    }
    let mut stale = stale_secret_fields("", &input.auth_method);
    match &input.admin_auth_method {
        Some(admin) => stale.extend(stale_secret_fields("admin_", admin)),
        None => stale.extend(["admin_password".to_string(), "admin_token".to_string()]),
    }
    for field in stale {
        mgr.delete_secret(&input.id, &field, &dir)?;
    }
    Ok(profiles)
}

/// Remove the profile from the store first, then its secrets: a failed write keeps both the profile and its credentials.
pub fn delete(id: &str) -> Result<Vec<StoredProfile>, String> {
    let dir = config_dir();
    let mgr = cred_manager();
    let mut profiles = load_stored()?;
    profiles.retain(|p| p.id != id);
    save_stored(&profiles)?;
    mgr.delete_all_secrets(id, &dir)?;
    Ok(profiles)
}

/// Toggle: clicking the current default clears it; otherwise it becomes the sole default.
pub fn set_default(id: &str) -> Result<Vec<StoredProfile>, String> {
    let mut profiles = load_stored()?;
    toggle_default_flag(&mut profiles, id)?;
    save_stored(&profiles)?;
    Ok(profiles)
}

/// The in-memory part of `set_default`: clicking the current default clears it, otherwise `id` becomes the only default.
fn toggle_default_flag(profiles: &mut [StoredProfile], id: &str) -> Result<(), String> {
    let already = profiles.iter().find(|p| p.id == id).map(|p| p.is_default).ok_or_else(|| format!("Server profile '{id}' not found"))?;
    for p in profiles.iter_mut() {
        p.is_default = !already && p.id == id;
    }
    Ok(())
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
        let id = ps[0].id.clone();
        // Setting a default clears the flag on every other profile...
        let other = ps[1].id.clone();
        toggle_default_flag(&mut ps, &other).unwrap();
        assert!(ps.iter().filter(|p| p.is_default).count() == 1 && ps[1].is_default);
        // ...clicking the current default clears it; the first of the list is then the fallback.
        toggle_default_flag(&mut ps, &other).unwrap();
        assert!(ps.iter().all(|p| !p.is_default));
        assert_eq!(initial_active(&ps).unwrap().id, id);
        assert!(toggle_default_flag(&mut ps, "missing").is_err());
    }

    #[test]
    fn stale_fields_follow_the_auth_method() {
        assert_eq!(stale_secret_fields("", &AuthMethod::None), vec!["password", "token"]);
        assert_eq!(stale_secret_fields("", &AuthMethod::Basic { username: "u".into(), password: String::new() }), vec!["token"]);
        assert_eq!(stale_secret_fields("admin_", &AuthMethod::Bearer { token: String::new() }), vec!["admin_password"]);
    }

    #[test]
    fn atomic_write_replaces_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("oeh-atomic-{}", std::process::id()));
        let file = dir.join("x.json");
        write_atomic(&file, "one", true).unwrap();
        write_atomic(&file, "two", true).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "two");
        assert!(!dir.join(".x.json.tmp").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unreadable_store_is_an_error_not_an_empty_list() {
        let dir = std::env::temp_dir().join(format!("oeh-load-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bad = dir.join("profiles.json");
        fs::write(&bad, "{ not json").unwrap();
        assert!(serde_json::from_str::<ProfileStore>(&fs::read_to_string(&bad).unwrap()).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    /// End to end against a throw-away config dir (the only test that touches the process environment).
    #[test]
    fn save_switch_auth_and_delete_roundtrip() {
        let dir = std::env::temp_dir().join(format!("oeh-roundtrip-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        // Never write test secrets into a developer's real OS keychain.
        if matches!(cred_manager().backend(), StorageBackend::OsKeychain) {
            fs::remove_dir_all(&dir).ok();
            return;
        }
        let input = |auth: AuthMethod| ProfileInput { id: "p1".into(), name: "P".into(), base_url: "http://h".into(), server_type: ServerType::Ehrbase, auth_method: auth, admin_auth_method: None, terminology_url: None, api_path_prefix: Some("/".into()) };

        let list = save(input(AuthMethod::Basic { username: "u".into(), password: "pw".into() })).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].api_path_prefix.as_deref(), Some(""), "an explicit root survives as \"\"");
        assert!(matches!(resolve(&list[0]).unwrap().auth_method, AuthMethod::Basic { ref password, .. } if password == "pw"));

        // Empty password keeps the stored one; switching to bearer retires the password only after the commit.
        let list = save(input(AuthMethod::Basic { username: "u2".into(), password: String::new() })).unwrap();
        assert!(matches!(resolve(&list[0]).unwrap().auth_method, AuthMethod::Basic { ref password, .. } if password == "pw"));
        let list = save(input(AuthMethod::Bearer { token: "  Bearer tok123 ".into() })).unwrap();
        assert!(matches!(resolve(&list[0]).unwrap().auth_method, AuthMethod::Bearer { ref token } if token == "tok123"));
        // Whitespace-only token = keep.
        let list = save(input(AuthMethod::Bearer { token: "   ".into() })).unwrap();
        assert!(matches!(resolve(&list[0]).unwrap().auth_method, AuthMethod::Bearer { ref token } if token == "tok123"));

        // A store that cannot be read blocks every mutation and is left exactly as it was.
        let path = dir.join("openehr-explorer").join("profiles.json");
        fs::write(&path, "{ broken").unwrap();
        assert!(save(input(AuthMethod::None)).is_err());
        assert!(delete("p1").is_err());
        assert!(set_default("p1").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ broken");
        assert!(load_stored().is_err());
        fs::remove_file(&path).unwrap();
        assert!(load_stored().unwrap().is_empty(), "a missing file is just an empty store");

        // Delete removes profile and secrets.
        let list = save(input(AuthMethod::Bearer { token: "t".into() })).unwrap();
        assert_eq!(list.len(), 1);
        assert!(delete("p1").unwrap().is_empty());
        assert!(matches!(cred_manager().load_secret("p1", "token", &config_dir()), Ok(None)));
        fs::remove_dir_all(&dir).ok();
    }
}
