use quick_xml::events::Event;
use quick_xml::reader::Reader;
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
    #[serde(default)]
    pub admin_auth_method: Option<AuthMethod>,
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
