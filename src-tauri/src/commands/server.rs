use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::inspector::send_instrumented;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub server_type: ServerType,
    pub auth_method: AuthMethod,
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

fn load_profiles() -> Vec<ServerProfile> {
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

fn save_profiles(profiles: &[ServerProfile]) -> Result<(), String> {
    let path = get_profiles_path();
    let store = ProfileStore {
        profiles: profiles.to_vec(),
    };
    let data = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

fn build_client(_profile: &ServerProfile) -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(false)
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
