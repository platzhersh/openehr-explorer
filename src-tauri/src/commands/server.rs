use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use zeroize::Zeroize;

use crate::credentials::{harden_file_permissions, CredentialManager, StorageBackend};
use crate::inspector::send_instrumented;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Internal server profile with resolved secrets. Never sent over IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub server_type: ServerType,
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub admin_auth_method: Option<AuthMethod>,
    #[serde(default)]
    pub terminology_url: Option<String>,
}

/// Public profile returned over IPC — secrets are replaced with flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfilePublic {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub server_type: ServerType,
    pub auth_method: AuthMethodPublic,
    #[serde(default)]
    pub admin_auth_method: Option<AuthMethodPublic>,
    #[serde(default)]
    pub terminology_url: Option<String>,
    pub credential_backend: String,
}

/// Inbound profile from the frontend when saving (credentials included).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfileInput {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub server_type: ServerType,
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub admin_auth_method: Option<AuthMethod>,
    #[serde(default)]
    pub terminology_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerType {
    Ehrbase,
    BetterPlatform,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
}

/// Public auth representation — secrets replaced with boolean flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethodPublic {
    None,
    Basic {
        username: String,
        has_password: bool,
    },
    Bearer {
        has_token: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerVersionInfo {
    pub server_version: Option<String>,
    pub ehrbase_version: Option<String>,
    pub sdk_version: Option<String>,
    pub archie_version: Option<String>,
    pub jvm_version: Option<String>,
    pub os_version: Option<String>,
    pub postgres_version: Option<String>,
}

// ---------------------------------------------------------------------------
// On-disk storage (no secrets)
// ---------------------------------------------------------------------------

/// What gets written to profiles.json — secrets stripped out.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// On-disk auth method — secrets replaced with sentinel null/empty values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoredAuthMethod {
    None,
    Basic { username: String },
    Bearer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileStore {
    profiles: Vec<StoredProfile>,
}

/// Legacy format for migration detection
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyProfileStore {
    profiles: Vec<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn get_profiles_path() -> PathBuf {
    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("profiles.json")
}

pub fn get_config_dir() -> PathBuf {
    dirs_config_dir().join("openehr-explorer")
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

// ---------------------------------------------------------------------------
// Credential helpers
// ---------------------------------------------------------------------------

fn cred_manager() -> CredentialManager {
    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir).ok();
    CredentialManager::new(&config_dir)
}

/// Extract secrets from an AuthMethod and store them in the credential manager.
/// If a secret value is empty, the existing secret (if any) is preserved.
fn store_auth_secrets(
    mgr: &CredentialManager,
    profile_id: &str,
    prefix: &str,
    auth: &AuthMethod,
    config_dir: &std::path::Path,
) -> Result<(), String> {
    match auth {
        AuthMethod::None => {
            // Clean up any previously stored secrets for this prefix
            mgr.delete_secret(profile_id, &format!("{prefix}password"), config_dir)?;
            mgr.delete_secret(profile_id, &format!("{prefix}token"), config_dir)?;
        }
        AuthMethod::Basic { password, .. } => {
            // Only update if a non-empty password was provided (empty = keep existing)
            if !password.is_empty() {
                mgr.store_secret(
                    profile_id,
                    &format!("{prefix}password"),
                    password.clone(),
                    config_dir,
                )?;
            }
            mgr.delete_secret(profile_id, &format!("{prefix}token"), config_dir)?;
        }
        AuthMethod::Bearer { token } => {
            // Only update if a non-empty token was provided (empty = keep existing)
            if !token.is_empty() {
                mgr.store_secret(
                    profile_id,
                    &format!("{prefix}token"),
                    token.clone(),
                    config_dir,
                )?;
            }
            mgr.delete_secret(profile_id, &format!("{prefix}password"), config_dir)?;
        }
    }
    Ok(())
}

/// Resolve secrets from the credential manager into an AuthMethod.
fn resolve_auth_secrets(
    mgr: &CredentialManager,
    profile_id: &str,
    prefix: &str,
    stored: &StoredAuthMethod,
    config_dir: &std::path::Path,
) -> Result<AuthMethod, String> {
    match stored {
        StoredAuthMethod::None => Ok(AuthMethod::None),
        StoredAuthMethod::Basic { username } => {
            let password = mgr
                .load_secret(profile_id, &format!("{prefix}password"), config_dir)?
                .unwrap_or_default();
            Ok(AuthMethod::Basic {
                username: username.clone(),
                password,
            })
        }
        StoredAuthMethod::Bearer => {
            let token = mgr
                .load_secret(profile_id, &format!("{prefix}token"), config_dir)?
                .unwrap_or_default();
            Ok(AuthMethod::Bearer { token })
        }
    }
}

/// Convert an AuthMethod to its stored (secret-free) representation.
fn to_stored_auth(auth: &AuthMethod) -> StoredAuthMethod {
    match auth {
        AuthMethod::None => StoredAuthMethod::None,
        AuthMethod::Basic { username, .. } => StoredAuthMethod::Basic {
            username: username.clone(),
        },
        AuthMethod::Bearer { .. } => StoredAuthMethod::Bearer,
    }
}

/// Convert an AuthMethod to its public (IPC-safe) representation.
fn to_public_auth(auth: &AuthMethod) -> AuthMethodPublic {
    match auth {
        AuthMethod::None => AuthMethodPublic::None,
        AuthMethod::Basic {
            username, password, ..
        } => AuthMethodPublic::Basic {
            username: username.clone(),
            has_password: !password.is_empty(),
        },
        AuthMethod::Bearer { token } => AuthMethodPublic::Bearer {
            has_token: !token.is_empty(),
        },
    }
}

// ---------------------------------------------------------------------------
// Profile persistence
// ---------------------------------------------------------------------------

fn load_stored_profiles() -> Vec<StoredProfile> {
    let path = get_profiles_path();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str::<ProfileStore>(&data)
            .map(|s| s.profiles)
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_stored_profiles(profiles: &[StoredProfile]) -> Result<(), String> {
    let path = get_profiles_path();
    let store = ProfileStore {
        profiles: profiles.to_vec(),
    };
    let data = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    fs::write(&path, &data).map_err(|e| e.to_string())?;
    harden_file_permissions(&path);
    Ok(())
}

/// Load a full ServerProfile (with resolved secrets) by ID.
fn load_resolved_profile(
    mgr: &CredentialManager,
    stored: &StoredProfile,
    config_dir: &std::path::Path,
) -> Result<ServerProfile, String> {
    let auth_method = resolve_auth_secrets(mgr, &stored.id, "", &stored.auth_method, config_dir)?;
    let admin_auth_method = match &stored.admin_auth_method {
        Some(admin) => Some(resolve_auth_secrets(
            mgr, &stored.id, "admin_", admin, config_dir,
        )?),
        None => None,
    };
    Ok(ServerProfile {
        id: stored.id.clone(),
        name: stored.name.clone(),
        base_url: stored.base_url.clone(),
        server_type: stored.server_type.clone(),
        auth_method,
        admin_auth_method,
        terminology_url: stored.terminology_url.clone(),
    })
}

/// Convert a resolved ServerProfile to a ServerProfilePublic for IPC.
fn to_public_profile(profile: &ServerProfile, backend: &StorageBackend) -> ServerProfilePublic {
    let backend_str = match backend {
        StorageBackend::OsKeychain => "os_keychain",
        StorageBackend::EncryptedFile => "encrypted_file",
    };
    ServerProfilePublic {
        id: profile.id.clone(),
        name: profile.name.clone(),
        base_url: profile.base_url.clone(),
        server_type: profile.server_type.clone(),
        auth_method: to_public_auth(&profile.auth_method),
        admin_auth_method: profile.admin_auth_method.as_ref().map(to_public_auth),
        terminology_url: profile.terminology_url.clone(),
        credential_backend: backend_str.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Migration: detect plaintext secrets in profiles.json and move to keychain
// ---------------------------------------------------------------------------

pub fn migrate_plaintext_credentials() {
    let path = get_profiles_path();
    if !path.exists() {
        return;
    }
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return,
    };
    let legacy: LegacyProfileStore = match serde_json::from_str(&data) {
        Ok(l) => l,
        Err(_) => return,
    };

    let config_dir = get_config_dir();
    let mgr = cred_manager();
    let mut needs_rewrite = false;
    let mut new_profiles: Vec<StoredProfile> = Vec::new();

    for profile_val in &legacy.profiles {
        // Check if this profile has plaintext secrets by looking for password/token in auth_method
        let has_plaintext = has_plaintext_secret(profile_val, "auth_method")
            || has_plaintext_secret(profile_val, "admin_auth_method");

        if has_plaintext {
            // Parse as full ServerProfile (legacy format with secrets)
            if let Ok(full) = serde_json::from_value::<ServerProfile>(profile_val.clone()) {
                // Store secrets in credential manager
                if store_auth_secrets(&mgr, &full.id, "", &full.auth_method, &config_dir).is_ok() {
                    if let Some(ref admin) = full.admin_auth_method {
                        store_auth_secrets(&mgr, &full.id, "admin_", admin, &config_dir).ok();
                    }
                    new_profiles.push(StoredProfile {
                        id: full.id,
                        name: full.name,
                        base_url: full.base_url,
                        server_type: full.server_type,
                        auth_method: to_stored_auth(&full.auth_method),
                        admin_auth_method: full.admin_auth_method.as_ref().map(to_stored_auth),
                        terminology_url: full.terminology_url,
                    });
                    needs_rewrite = true;
                    continue;
                }
            }
        }

        // Already clean or could not parse — try to keep as stored profile
        if let Ok(stored) = serde_json::from_value::<StoredProfile>(profile_val.clone()) {
            new_profiles.push(stored);
        }
    }

    if needs_rewrite {
        save_stored_profiles(&new_profiles).ok();
    } else {
        // Just harden permissions even if no migration needed
        harden_file_permissions(&path);
    }
}

fn has_plaintext_secret(val: &serde_json::Value, field: &str) -> bool {
    if let Some(auth) = val.get(field) {
        if let Some(obj) = auth.as_object() {
            return obj.contains_key("password") || obj.contains_key("token");
        }
    }
    false
}

// ---------------------------------------------------------------------------
// HTTP client helpers
// ---------------------------------------------------------------------------

fn build_client(_profile: &ServerProfile) -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(false)
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
}

fn build_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    auth: &AuthMethod,
) -> reqwest::RequestBuilder {
    let req = client.request(method, url);
    match auth {
        AuthMethod::None => req,
        AuthMethod::Basic { username, password } => req.basic_auth(username, Some(password)),
        AuthMethod::Bearer { token } => req.bearer_auth(token),
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_server_profiles() -> Result<Vec<ServerProfilePublic>, String> {
    let config_dir = get_config_dir();
    let mgr = cred_manager();
    let stored = load_stored_profiles();
    let mut public = Vec::with_capacity(stored.len());
    for sp in &stored {
        let resolved = load_resolved_profile(&mgr, sp, &config_dir)?;
        public.push(to_public_profile(&resolved, mgr.backend()));
    }
    Ok(public)
}

#[tauri::command]
pub async fn save_server_profile(
    profile: ServerProfileInput,
) -> Result<Vec<ServerProfilePublic>, String> {
    let config_dir = get_config_dir();
    let mgr = cred_manager();

    // Store secrets in credential manager
    store_auth_secrets(&mgr, &profile.id, "", &profile.auth_method, &config_dir)?;
    if let Some(ref admin) = profile.admin_auth_method {
        store_auth_secrets(&mgr, &profile.id, "admin_", admin, &config_dir)?;
    } else {
        // Clean up admin secrets if admin auth is removed
        mgr.delete_secret(&profile.id, "admin_password", &config_dir)?;
        mgr.delete_secret(&profile.id, "admin_token", &config_dir)?;
    }

    // Build stored profile (without secrets)
    let stored_profile = StoredProfile {
        id: profile.id.clone(),
        name: profile.name,
        base_url: profile.base_url,
        server_type: profile.server_type,
        auth_method: to_stored_auth(&profile.auth_method),
        admin_auth_method: profile.admin_auth_method.as_ref().map(to_stored_auth),
        terminology_url: profile.terminology_url,
    };

    let mut profiles = load_stored_profiles();
    if let Some(existing) = profiles.iter_mut().find(|p| p.id == stored_profile.id) {
        *existing = stored_profile;
    } else {
        profiles.push(stored_profile);
    }
    save_stored_profiles(&profiles)?;

    // Return public list
    let mut public = Vec::with_capacity(profiles.len());
    for sp in &profiles {
        let resolved = load_resolved_profile(&mgr, sp, &config_dir)?;
        public.push(to_public_profile(&resolved, mgr.backend()));
    }
    Ok(public)
}

#[tauri::command]
pub async fn delete_server_profile(id: String) -> Result<Vec<ServerProfilePublic>, String> {
    let config_dir = get_config_dir();
    let mgr = cred_manager();

    // Delete all secrets for this profile
    mgr.delete_all_secrets(&id, &config_dir)?;

    let mut profiles = load_stored_profiles();
    profiles.retain(|p| p.id != id);
    save_stored_profiles(&profiles)?;

    // Return public list
    let mut public = Vec::with_capacity(profiles.len());
    for sp in &profiles {
        let resolved = load_resolved_profile(&mgr, sp, &config_dir)?;
        public.push(to_public_profile(&resolved, mgr.backend()));
    }
    Ok(public)
}

#[tauri::command]
pub async fn test_server_connection(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&profile_id)?;
    let client = build_client(&profile);
    let url = format!(
        "{}/rest/openehr/v1/definition/template/adl1.4",
        profile.base_url.trim_end_matches('/')
    );

    let resp = send_instrumented(
        &app,
        &client,
        build_request(&client, reqwest::Method::GET, &url, &profile.auth_method),
    )
    .await?;

    if resp.is_success {
        Ok(format!("Connected successfully (HTTP {})", resp.status))
    } else {
        Err(format!("Server returned HTTP {}", resp.status))
    }
}

#[tauri::command]
pub async fn test_unsaved_connection(
    app: tauri::AppHandle,
    profile: ServerProfileInput,
) -> Result<String, String> {
    let full = ServerProfile {
        id: profile.id,
        name: profile.name,
        base_url: profile.base_url,
        server_type: profile.server_type,
        auth_method: profile.auth_method,
        admin_auth_method: profile.admin_auth_method,
        terminology_url: profile.terminology_url,
    };
    let client = build_client(&full);
    let url = format!(
        "{}/rest/openehr/v1/definition/template/adl1.4",
        full.base_url.trim_end_matches('/')
    );

    let resp = send_instrumented(
        &app,
        &client,
        build_request(&client, reqwest::Method::GET, &url, &full.auth_method),
    )
    .await?;

    if resp.is_success {
        Ok(format!("Connected successfully (HTTP {})", resp.status))
    } else {
        Err(format!("Server returned HTTP {}", resp.status))
    }
}

#[tauri::command]
pub async fn get_server_version(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<ServerVersionInfo, String> {
    let profile = get_profile_by_id(&profile_id)?;
    let client = build_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    match profile.server_type {
        ServerType::Ehrbase => {
            let url = format!("{}/rest/status", base);
            let resp = send_instrumented(
                &app,
                &client,
                build_request(&client, reqwest::Method::GET, &url, &profile.auth_method),
            )
            .await?;

            if !resp.is_success {
                return Err(format!("Server returned HTTP {}", resp.status));
            }
            parse_version_xml(&resp.body)
        }
        ServerType::BetterPlatform => {
            let url = format!("{}/rest/v1", base);
            let resp = send_instrumented(
                &app,
                &client,
                build_request(
                    &client,
                    reqwest::Method::OPTIONS,
                    &url,
                    &profile.auth_method,
                ),
            )
            .await?;

            if !resp.is_success {
                return Err(format!("Server returned HTTP {}", resp.status));
            }
            parse_version_json(&resp.body)
        }
        ServerType::Generic => {
            Err("Version detection is not available for generic openEHR servers".to_string())
        }
    }
}

#[tauri::command]
pub async fn get_credential_backend() -> Result<String, String> {
    let mgr = cred_manager();
    Ok(match mgr.backend() {
        StorageBackend::OsKeychain => "os_keychain".to_string(),
        StorageBackend::EncryptedFile => "encrypted_file".to_string(),
    })
}

// ---------------------------------------------------------------------------
// XML / JSON version parsers
// ---------------------------------------------------------------------------

fn parse_version_xml(xml_body: &str) -> Result<ServerVersionInfo, String> {
    let mut reader = Reader::from_str(xml_body);
    reader.config_mut().trim_text(true);

    let mut version_info = ServerVersionInfo::default();

    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                match current_tag.as_str() {
                    "ehrbase_version" => {
                        version_info.server_version = Some(text.clone());
                        version_info.ehrbase_version = Some(text);
                    }
                    "openehr_sdk_version" => version_info.sdk_version = Some(text),
                    "archie_version" => version_info.archie_version = Some(text),
                    "jvm_version" => version_info.jvm_version = Some(text),
                    "os_version" => version_info.os_version = Some(text),
                    "postgres_version" => version_info.postgres_version = Some(text),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Error parsing XML: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(version_info)
}

fn parse_version_json(body: &str) -> Result<ServerVersionInfo, String> {
    let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;

    let mut info = ServerVersionInfo::default();

    if let Some(obj) = json.as_object() {
        if let Some(v) = obj.get("solutionVersion").and_then(|v| v.as_str()) {
            info.server_version = Some(v.to_string());
        }
    }

    Ok(info)
}

// ---------------------------------------------------------------------------
// Re-export helpers for other command modules
// ---------------------------------------------------------------------------

pub fn get_profile_by_id(id: &str) -> Result<ServerProfile, String> {
    let config_dir = get_config_dir();
    let mgr = cred_manager();
    let stored = load_stored_profiles();
    let sp = stored
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("Server profile '{}' not found", id))?;
    load_resolved_profile(&mgr, &sp, &config_dir)
}

pub fn make_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    auth: &AuthMethod,
) -> reqwest::RequestBuilder {
    build_request(client, method, url, auth)
}

pub fn create_client(profile: &ServerProfile) -> reqwest::Client {
    build_client(profile)
}
