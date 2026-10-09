use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

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
    /// Path appended to `base_url` to reach the openEHR REST API root.
    /// `None` means [`DEFAULT_API_PATH_PREFIX`] (`/rest/openehr/v1`).
    #[serde(default)]
    pub api_path_prefix: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

/// API path used when a profile doesn't set `api_path_prefix`. This is the
/// EHRBase / Better Platform convention, not something the openEHR REST spec
/// mandates — see OEH-104.
pub const DEFAULT_API_PATH_PREFIX: &str = "/rest/openehr/v1";

/// Normalizes a user-supplied prefix: trims whitespace, ensures a single
/// leading `/`, drops trailing `/`. `None` (field left blank in the UI) keeps
/// meaning "use the default"; an explicit empty or `/` value is preserved as an
/// empty string, meaning "no prefix — `base_url` is itself the API root".
pub fn normalize_api_path_prefix(prefix: Option<String>) -> Option<String> {
    let prefix = prefix?;
    let inner = prefix.trim().trim_matches('/');
    Some(if inner.is_empty() {
        String::new()
    } else {
        format!("/{}", inner)
    })
}

/// Joins `base_url` and an API path prefix into the openEHR REST API root.
pub fn join_api_root(base_url: &str, prefix: Option<&str>) -> String {
    let prefix = prefix.unwrap_or(DEFAULT_API_PATH_PREFIX);
    let prefix = prefix.trim_matches('/');
    let base = base_url.trim_end_matches('/');
    if prefix.is_empty() {
        base.to_string()
    } else {
        format!("{}/{}", base, prefix)
    }
}

impl ServerProfile {
    /// The openEHR REST API root (e.g. `https://host/rest/openehr/v1`), with no
    /// trailing slash. Spec-defined resource paths (`/ehr`, `/query/aql`, …) are
    /// appended to this.
    pub fn api_root(&self) -> String {
        join_api_root(&self.base_url, self.api_path_prefix.as_deref())
    }
}

impl From<ServerProfileInput> for ServerProfile {
    /// Converts an unsaved/inbound profile, normalizing the API path prefix the
    /// same way `save_server_profile` does so a connection test exercises exactly
    /// what would be stored.
    fn from(input: ServerProfileInput) -> Self {
        ServerProfile {
            id: input.id,
            name: input.name,
            base_url: input.base_url,
            server_type: input.server_type,
            auth_method: input.auth_method,
            admin_auth_method: input.admin_auth_method,
            terminology_url: input.terminology_url,
            api_path_prefix: normalize_api_path_prefix(input.api_path_prefix),
            is_default: false,
        }
    }
}

/// The URL "Test Connection" requests: the template list, a cheap
/// authenticated GET every openEHR CDR serves.
fn connection_test_url(profile: &ServerProfile) -> String {
    format!("{}/definition/template/adl1.4", profile.api_root())
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
    /// Path appended to `base_url` to reach the openEHR REST API root.
    /// `None` means [`DEFAULT_API_PATH_PREFIX`] (`/rest/openehr/v1`).
    #[serde(default)]
    pub api_path_prefix: Option<String>,
    pub credential_backend: String,
    #[serde(default)]
    pub is_default: bool,
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
    /// Path appended to `base_url` to reach the openEHR REST API root.
    /// `None` means [`DEFAULT_API_PATH_PREFIX`] (`/rest/openehr/v1`).
    #[serde(default)]
    pub api_path_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerType {
    Ehrbase,
    BetterPlatform,
    FerroEhr,
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
    /// Path appended to `base_url` to reach the openEHR REST API root.
    /// `None` means [`DEFAULT_API_PATH_PREFIX`] (`/rest/openehr/v1`).
    #[serde(default)]
    pub api_path_prefix: Option<String>,
    #[serde(default)]
    pub is_default: bool,
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

/// Singleton credential manager — the keychain probe runs exactly once
/// per application lifetime, preventing backend flipping between calls.
static CREDENTIAL_MANAGER: OnceLock<CredentialManager> = OnceLock::new();

fn cred_manager() -> &'static CredentialManager {
    CREDENTIAL_MANAGER.get_or_init(|| {
        let config_dir = get_config_dir();
        fs::create_dir_all(&config_dir).ok();
        CredentialManager::new(&config_dir)
    })
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
        api_path_prefix: stored.api_path_prefix.clone(),
        is_default: stored.is_default,
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
        api_path_prefix: profile.api_path_prefix.clone(),
        credential_backend: backend_str.to_string(),
        is_default: profile.is_default,
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
                if store_auth_secrets(mgr, &full.id, "", &full.auth_method, &config_dir).is_ok() {
                    if let Some(ref admin) = full.admin_auth_method {
                        store_auth_secrets(mgr, &full.id, "admin_", admin, &config_dir).ok();
                    }
                    new_profiles.push(StoredProfile {
                        id: full.id,
                        name: full.name,
                        base_url: full.base_url,
                        server_type: full.server_type,
                        auth_method: to_stored_auth(&full.auth_method),
                        admin_auth_method: full.admin_auth_method.as_ref().map(to_stored_auth),
                        terminology_url: full.terminology_url,
                        api_path_prefix: full.api_path_prefix,
                        is_default: full.is_default,
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

/// Resolve secrets for each stored profile and convert to the IPC-safe public
/// representation. Shared by every command that returns the profile list.
fn to_public_profiles(
    mgr: &CredentialManager,
    profiles: &[StoredProfile],
    config_dir: &std::path::Path,
) -> Result<Vec<ServerProfilePublic>, String> {
    let mut public = Vec::with_capacity(profiles.len());
    for sp in profiles {
        let resolved = load_resolved_profile(mgr, sp, config_dir)?;
        public.push(to_public_profile(&resolved, mgr.backend()));
    }
    Ok(public)
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_server_profiles() -> Result<Vec<ServerProfilePublic>, String> {
    let config_dir = get_config_dir();
    let mgr = cred_manager();
    let stored = load_stored_profiles();
    to_public_profiles(mgr, &stored, &config_dir)
}

#[tauri::command]
pub async fn save_server_profile(
    profile: ServerProfileInput,
) -> Result<Vec<ServerProfilePublic>, String> {
    let config_dir = get_config_dir();
    let mgr = cred_manager();

    // Store secrets in credential manager
    store_auth_secrets(mgr, &profile.id, "", &profile.auth_method, &config_dir)?;
    if let Some(ref admin) = profile.admin_auth_method {
        store_auth_secrets(mgr, &profile.id, "admin_", admin, &config_dir)?;
    } else {
        // Clean up admin secrets if admin auth is removed
        mgr.delete_secret(&profile.id, "admin_password", &config_dir)?;
        mgr.delete_secret(&profile.id, "admin_token", &config_dir)?;
    }

    let mut profiles = load_stored_profiles();

    // Preserve the existing default flag — the save form doesn't carry it.
    let is_default = profiles
        .iter()
        .find(|p| p.id == profile.id)
        .map(|p| p.is_default)
        .unwrap_or(false);

    // Build stored profile (without secrets)
    let stored_profile = StoredProfile {
        id: profile.id.clone(),
        name: profile.name,
        base_url: profile.base_url,
        server_type: profile.server_type,
        auth_method: to_stored_auth(&profile.auth_method),
        admin_auth_method: profile.admin_auth_method.as_ref().map(to_stored_auth),
        terminology_url: profile.terminology_url,
        api_path_prefix: normalize_api_path_prefix(profile.api_path_prefix),
        is_default,
    };

    if let Some(existing) = profiles.iter_mut().find(|p| p.id == stored_profile.id) {
        *existing = stored_profile;
    } else {
        profiles.push(stored_profile);
    }
    save_stored_profiles(&profiles)?;

    to_public_profiles(mgr, &profiles, &config_dir)
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

    to_public_profiles(mgr, &profiles, &config_dir)
}

#[tauri::command]
pub async fn set_default_server_profile(id: String) -> Result<Vec<ServerProfilePublic>, String> {
    let config_dir = get_config_dir();
    let mgr = cred_manager();

    let mut profiles = load_stored_profiles();
    let already_default = profiles
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.is_default)
        .ok_or_else(|| format!("Server profile '{}' not found", id))?;
    // Toggle: clicking the current default clears it, falling back to
    // first-of-list preselection; otherwise make it the sole default.
    for p in profiles.iter_mut() {
        p.is_default = !already_default && p.id == id;
    }
    save_stored_profiles(&profiles)?;

    to_public_profiles(mgr, &profiles, &config_dir)
}

#[tauri::command]
pub async fn test_server_connection(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&profile_id)?;
    let client = build_client(&profile);
    let url = connection_test_url(&profile);

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
    let full = ServerProfile::from(profile);
    let client = build_client(&full);
    let url = connection_test_url(&full);

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
        ServerType::FerroEhr => {
            // FerroEHR's status endpoint is unauthenticated (mounted outside
            // its auth layer) — sending credentials here would make version
            // detection fail for a reachable server whose stored credentials
            // happen to be invalid or expired.
            let url = format!("{}/rest/status", base);
            let resp = send_instrumented(
                &app,
                &client,
                build_request(&client, reqwest::Method::GET, &url, &AuthMethod::None),
            )
            .await?;

            if !resp.is_success {
                return Err(format!("Server returned HTTP {}", resp.status));
            }
            parse_ferroehr_status_json(&resp.body)
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
                let text = e.decode().unwrap_or_default();
                let text = unescape(&text).unwrap_or_default().into_owned();
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

/// Parses FerroEHR's `GET /rest/status` body: `{status, server_version,
/// openehr_rest_api_version, timestamp}`.
fn parse_ferroehr_status_json(body: &str) -> Result<ServerVersionInfo, String> {
    let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;

    let mut info = ServerVersionInfo::default();

    if let Some(obj) = json.as_object() {
        if let Some(v) = obj.get("server_version").and_then(|v| v.as_str()) {
            info.server_version = Some(v.to_string());
        }
    }

    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::{
        connection_test_url, join_api_root, normalize_api_path_prefix, to_public_profile,
        AuthMethod, ServerProfile, ServerProfileInput, ServerType, StoredProfile,
        DEFAULT_API_PATH_PREFIX,
    };
    use crate::credentials::StorageBackend;

    fn input(base_url: &str, prefix: Option<&str>) -> ServerProfileInput {
        ServerProfileInput {
            id: "1".into(),
            name: "n".into(),
            base_url: base_url.into(),
            server_type: ServerType::Generic,
            auth_method: AuthMethod::None,
            admin_auth_method: None,
            terminology_url: None,
            api_path_prefix: prefix.map(String::from),
        }
    }

    /// Regression guard for OEH-104: openEHR resource URLs must be built from
    /// `ServerProfile::api_root()`. A literal `/rest/openehr/v1` in non-test code
    /// of the command modules would silently ignore a profile's prefix.
    #[test]
    fn command_modules_do_not_hardcode_the_api_prefix() {
        let sources = [
            ("composition.rs", include_str!("composition.rs")),
            ("contribution.rs", include_str!("contribution.rs")),
            ("ehr.rs", include_str!("ehr.rs")),
            ("query.rs", include_str!("query.rs")),
            ("template.rs", include_str!("template.rs")),
        ];
        for (name, src) in sources {
            let production = src.split("#[cfg(test)]").next().unwrap_or(src);
            for (i, line) in production.lines().enumerate() {
                let is_comment = line.trim_start().starts_with("//");
                assert!(
                    is_comment || !line.contains("/rest/openehr/v1"),
                    "{name}:{} hardcodes /rest/openehr/v1; use profile.api_root()",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn connection_test_url_uses_default_prefix() {
        let p = ServerProfile::from(input("https://cdr.example.com/", None));
        assert_eq!(
            connection_test_url(&p),
            "https://cdr.example.com/rest/openehr/v1/definition/template/adl1.4"
        );
    }

    #[test]
    fn connection_test_url_uses_custom_prefix() {
        let p = ServerProfile::from(input("https://cdr.example.com", Some("openehr/v1/")));
        assert_eq!(
            connection_test_url(&p),
            "https://cdr.example.com/openehr/v1/definition/template/adl1.4"
        );
    }

    #[test]
    fn connection_test_url_with_slash_prefix_treats_base_url_as_api_root() {
        let p = ServerProfile::from(input("https://cdr.example.com/openehr/v1/", Some("/")));
        assert_eq!(p.api_path_prefix, Some(String::new()));
        assert_eq!(
            connection_test_url(&p),
            "https://cdr.example.com/openehr/v1/definition/template/adl1.4"
        );
    }

    #[test]
    fn explicit_empty_prefix_survives_storage_round_trip() {
        // `Some("")` ("base URL is the API root") must not collapse into `None`
        // ("use the default") when written to and read back from profiles.json.
        let mut stored: StoredProfile = serde_json::from_str(
            r#"{"id":"1","name":"n","base_url":"http://x","server_type":"generic","auth_method":{"type":"none"},"api_path_prefix":""}"#,
        )
        .unwrap();
        assert_eq!(stored.api_path_prefix, Some(String::new()));
        let json = serde_json::to_string(&stored).unwrap();
        let back: StoredProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.api_path_prefix, Some(String::new()));

        stored.api_path_prefix = None;
        let back: StoredProfile =
            serde_json::from_str(&serde_json::to_string(&stored).unwrap()).unwrap();
        assert_eq!(back.api_path_prefix, None);
    }

    #[test]
    fn saving_an_edited_empty_prefix_keeps_it_empty() {
        // The edit form shows a stored "" as "/", which normalizes back to "".
        assert_eq!(
            normalize_api_path_prefix(Some(String::new())),
            Some(String::new())
        );
        assert_eq!(
            normalize_api_path_prefix(Some("/".into())),
            Some(String::new())
        );
    }

    #[test]
    fn public_profile_exposes_the_prefix_unchanged() {
        for prefix in [None, Some(String::new()), Some("/openehr/v1".to_string())] {
            let mut p = ServerProfile::from(input("http://x", None));
            p.api_path_prefix = prefix.clone();
            let public = to_public_profile(&p, &StorageBackend::EncryptedFile);
            assert_eq!(public.api_path_prefix, prefix);
        }
    }

    #[test]
    fn api_root_defaults_to_rest_openehr_v1() {
        assert_eq!(DEFAULT_API_PATH_PREFIX, "/rest/openehr/v1");
        assert_eq!(
            join_api_root("https://cdr.example.com", None),
            "https://cdr.example.com/rest/openehr/v1"
        );
        assert_eq!(
            join_api_root("https://cdr.example.com/", None),
            "https://cdr.example.com/rest/openehr/v1"
        );
    }

    #[test]
    fn api_root_uses_custom_prefix() {
        assert_eq!(
            join_api_root("https://cdr.example.com/", Some("/openehr/v1")),
            "https://cdr.example.com/openehr/v1"
        );
        assert_eq!(
            join_api_root("https://cdr.example.com", Some("openehr/v1/")),
            "https://cdr.example.com/openehr/v1"
        );
    }

    #[test]
    fn api_root_empty_prefix_means_base_url_is_the_root() {
        assert_eq!(
            join_api_root("https://cdr.example.com/openehr/v1/", Some("")),
            "https://cdr.example.com/openehr/v1"
        );
    }

    #[test]
    fn normalize_prefix_cleans_user_input() {
        assert_eq!(normalize_api_path_prefix(None), None);
        // An explicit empty/"/" prefix means "base URL is the API root" and must
        // survive a save → edit → save round trip (it is not the same as unset).
        assert_eq!(
            normalize_api_path_prefix(Some("   ".into())),
            Some(String::new())
        );
        assert_eq!(
            normalize_api_path_prefix(Some(String::new())),
            Some(String::new())
        );
        assert_eq!(
            normalize_api_path_prefix(Some(" openehr/v1/ ".into())),
            Some("/openehr/v1".to_string())
        );
        assert_eq!(
            normalize_api_path_prefix(Some("/".into())),
            Some(String::new())
        );
    }

    #[test]
    fn legacy_profile_without_prefix_still_deserializes() {
        let json = r#"{"id":"1","name":"n","base_url":"http://x","server_type":"ehrbase","auth_method":{"type":"none"}}"#;
        let p: super::ServerProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.api_path_prefix, None);
        assert_eq!(p.api_root(), "http://x/rest/openehr/v1");
    }

    use super::*;

    #[test]
    fn parses_ferroehr_status_json() {
        let body = r#"{"status":"UP","server_version":"0.4.0","openehr_rest_api_version":"1.1.0","timestamp":"2026-08-24T12:00:00Z"}"#;
        let info = parse_ferroehr_status_json(body).unwrap();
        assert_eq!(info.server_version.as_deref(), Some("0.4.0"));
        assert_eq!(info.ehrbase_version, None);
    }

    #[test]
    fn parse_ferroehr_status_json_rejects_invalid_json() {
        assert!(parse_ferroehr_status_json("not json").is_err());
    }
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
    load_resolved_profile(mgr, &sp, &config_dir)
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
