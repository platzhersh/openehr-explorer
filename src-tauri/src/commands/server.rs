use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::credentials;
use crate::inspector::send_instrumented;

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
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
    /// Credential stored in the OS keychain (macOS Keychain, Windows Credential Manager, etc.)
    Keychain {
        ref_key: String,
    },
    /// Credential encrypted with AES-256-GCM (fallback when no keychain is available)
    Encrypted {
        ciphertext: String,
        nonce: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVersionInfo {
    pub ehrbase_version: Option<String>,
    pub sdk_version: Option<String>,
    pub archie_version: Option<String>,
    pub jvm_version: Option<String>,
    pub os_version: Option<String>,
    pub postgres_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileStore {
    profiles: Vec<ServerProfile>,
}

fn get_profiles_path() -> PathBuf {
    let config_dir = dirs_config_dir().join("openehr-explorer");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("profiles.json")
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

/// Resolve a stored `AuthMethod` (Keychain/Encrypted) back to its plaintext form (Basic/Bearer).
/// `None`, `Basic`, and `Bearer` pass through unchanged.
fn resolve_auth(auth: &AuthMethod) -> Result<AuthMethod, String> {
    match auth {
        AuthMethod::None | AuthMethod::Basic { .. } | AuthMethod::Bearer { .. } => Ok(auth.clone()),
        AuthMethod::Keychain { ref_key } => {
            let json = credentials::keychain_retrieve(ref_key)?;
            serde_json::from_str::<AuthMethod>(&json).map_err(|e| e.to_string())
        }
        AuthMethod::Encrypted { ciphertext, nonce } => {
            let payload = credentials::EncryptedPayload {
                ciphertext: ciphertext.clone(),
                nonce: nonce.clone(),
            };
            let json = credentials::decrypt(&payload)?;
            serde_json::from_str::<AuthMethod>(&json).map_err(|e| e.to_string())
        }
    }
}

/// Convert a plaintext `AuthMethod` (Basic/Bearer) into a secure stored form.
/// Uses the OS keychain when available, otherwise falls back to AES-256-GCM encryption.
/// `None`, `Keychain`, and `Encrypted` pass through unchanged.
fn secure_auth(
    profile_id: &str,
    key_suffix: &str,
    auth: &AuthMethod,
) -> Result<AuthMethod, String> {
    match auth {
        AuthMethod::None | AuthMethod::Keychain { .. } | AuthMethod::Encrypted { .. } => {
            Ok(auth.clone())
        }
        AuthMethod::Basic { .. } | AuthMethod::Bearer { .. } => {
            let json = serde_json::to_string(auth).map_err(|e| e.to_string())?;
            let ref_key = format!("{}:{}", profile_id, key_suffix);

            if credentials::keychain_available() {
                credentials::keychain_store(&ref_key, &json)?;
                Ok(AuthMethod::Keychain { ref_key })
            } else {
                let payload = credentials::encrypt(&json)?;
                Ok(AuthMethod::Encrypted {
                    ciphertext: payload.ciphertext,
                    nonce: payload.nonce,
                })
            }
        }
    }
}

/// Delete stored credentials for a profile from keychain.
/// Uses well-known ref_key format since loaded profiles have resolved auth.
fn delete_profile_credentials(profile_id: &str) {
    credentials::keychain_delete(&format!("{}:auth", profile_id)).ok();
    credentials::keychain_delete(&format!("{}:admin_auth", profile_id)).ok();
}

/// Load profiles from disk and resolve all credentials to plaintext for runtime use.
fn load_profiles() -> Vec<ServerProfile> {
    let path = get_profiles_path();
    if !path.exists() {
        return Vec::new();
    }
    let data = fs::read_to_string(&path).unwrap_or_default();
    let mut profiles = serde_json::from_str::<ProfileStore>(&data)
        .map(|s| s.profiles)
        .unwrap_or_default();

    // Check if any profiles have plaintext credentials that need migration
    let needs_migration = profiles.iter().any(|p| {
        matches!(
            &p.auth_method,
            AuthMethod::Basic { .. } | AuthMethod::Bearer { .. }
        ) || matches!(
            &p.admin_auth_method,
            Some(AuthMethod::Basic { .. }) | Some(AuthMethod::Bearer { .. })
        )
    });

    if needs_migration {
        // Migrate plaintext credentials to secure storage
        let mut stored_profiles = profiles.clone();
        for profile in &mut stored_profiles {
            if matches!(
                &profile.auth_method,
                AuthMethod::Basic { .. } | AuthMethod::Bearer { .. }
            ) {
                if let Ok(secured) = secure_auth(&profile.id, "auth", &profile.auth_method) {
                    profile.auth_method = secured;
                }
            }
            if let Some(ref admin_auth) = profile.admin_auth_method {
                if matches!(
                    admin_auth,
                    AuthMethod::Basic { .. } | AuthMethod::Bearer { .. }
                ) {
                    if let Ok(secured) = secure_auth(&profile.id, "admin_auth", admin_auth) {
                        profile.admin_auth_method = Some(secured);
                    }
                }
            }
        }
        // Write migrated profiles back to disk (best-effort, don't fail the load)
        let store = ProfileStore {
            profiles: stored_profiles,
        };
        if let Ok(json) = serde_json::to_string_pretty(&store) {
            fs::write(&path, json).ok();
        }
        log::info!(
            "Migrated {} profile(s) to secure credential storage",
            profiles.len()
        );
    }

    // Resolve all credentials to plaintext for runtime use.
    // If resolution fails (e.g. keychain access denied after recompile on macOS),
    // fall back to AuthMethod::None so the frontend shows a valid state and the
    // user can re-enter credentials.
    for profile in &mut profiles {
        match resolve_auth(&profile.auth_method) {
            Ok(resolved) => profile.auth_method = resolved,
            Err(e) => {
                log::warn!(
                    "Failed to resolve credentials for profile '{}': {}. Auth reset to None.",
                    profile.name,
                    e
                );
                profile.auth_method = AuthMethod::None;
            }
        }
        if let Some(ref admin_auth) = profile.admin_auth_method {
            match resolve_auth(admin_auth) {
                Ok(resolved) => profile.admin_auth_method = Some(resolved),
                Err(e) => {
                    log::warn!(
                        "Failed to resolve admin credentials for profile '{}': {}. Admin auth reset to None.",
                        profile.name,
                        e
                    );
                    profile.admin_auth_method = Some(AuthMethod::None);
                }
            }
        }
    }

    profiles
}

/// Save profiles to disk with credentials secured (keychain or encrypted).
fn save_profiles(profiles: &[ServerProfile]) -> Result<(), String> {
    let path = get_profiles_path();

    // Secure credentials before writing to disk
    let stored_profiles: Result<Vec<ServerProfile>, String> = profiles
        .iter()
        .map(|p| {
            let secured_auth = secure_auth(&p.id, "auth", &p.auth_method)?;
            let secured_admin = match &p.admin_auth_method {
                Some(auth) => Some(secure_auth(&p.id, "admin_auth", auth)?),
                None => None,
            };
            Ok(ServerProfile {
                auth_method: secured_auth,
                admin_auth_method: secured_admin,
                ..p.clone()
            })
        })
        .collect();

    let store = ProfileStore {
        profiles: stored_profiles?,
    };
    let data = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

fn build_client(_profile: &ServerProfile) -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(false)
        .timeout(std::time::Duration::from_secs(30)) // 30 second timeout
        .connect_timeout(std::time::Duration::from_secs(10)) // 10 second connection timeout
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
        // Keychain/Encrypted should already be resolved before reaching here
        AuthMethod::Keychain { .. } | AuthMethod::Encrypted { .. } => {
            log::warn!("Unresolved credential reference passed to build_request — skipping auth");
            req
        }
    }
}

#[tauri::command]
pub async fn list_server_profiles() -> Result<Vec<ServerProfile>, String> {
    Ok(load_profiles())
}

#[tauri::command]
pub async fn save_server_profile(profile: ServerProfile) -> Result<Vec<ServerProfile>, String> {
    let mut profiles = load_profiles();
    if let Some(existing) = profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
    save_profiles(&profiles)?;
    Ok(profiles)
}

#[tauri::command]
pub async fn delete_server_profile(id: String) -> Result<Vec<ServerProfile>, String> {
    let mut profiles = load_profiles();
    // Clean up keychain entries for the deleted profile
    delete_profile_credentials(&id);
    profiles.retain(|p| p.id != id);
    save_profiles(&profiles)?;
    Ok(profiles)
}

#[tauri::command]
pub async fn test_server_connection(
    app: tauri::AppHandle,
    profile: ServerProfile,
) -> Result<String, String> {
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
pub async fn get_server_version(
    app: tauri::AppHandle,
    profile: ServerProfile,
) -> Result<ServerVersionInfo, String> {
    let client = build_client(&profile);
    let url = format!("{}/rest/status", profile.base_url.trim_end_matches('/'));

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

fn parse_version_xml(xml_body: &str) -> Result<ServerVersionInfo, String> {
    let mut reader = Reader::from_str(xml_body);
    reader.config_mut().trim_text(true);

    let mut version_info = ServerVersionInfo {
        ehrbase_version: None,
        sdk_version: None,
        archie_version: None,
        jvm_version: None,
        os_version: None,
        postgres_version: None,
    };

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
                    "ehrbase_version" => version_info.ehrbase_version = Some(text),
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

// Re-export helpers for other command modules
pub fn get_profile_by_id(id: &str) -> Result<ServerProfile, String> {
    load_profiles()
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("Server profile '{}' not found", id))
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
