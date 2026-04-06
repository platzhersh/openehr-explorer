use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const KEY_FILE_NAME: &str = ".credentials-key";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub ciphertext: String, // Base64
    pub nonce: String,      // Base64
}

fn get_key_file_path() -> PathBuf {
    let config_dir = dirs_config_dir().join("openehr-explorer");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join(KEY_FILE_NAME)
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

/// Load or generate the AES-256 key (32 bytes), stored in a dedicated file.
fn load_or_create_key() -> Result<[u8; 32], String> {
    let path = get_key_file_path();
    if path.exists() {
        let encoded = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let bytes = B64.decode(encoded.trim()).map_err(|e| e.to_string())?;
        if bytes.len() != 32 {
            return Err("Invalid key file length".into());
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(key)
    } else {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        let encoded = B64.encode(key);
        fs::write(&path, &encoded).map_err(|e| e.to_string())?;
        // Best-effort restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).ok();
        }
        Ok(key)
    }
}

/// Encrypt a plaintext string with AES-256-GCM.
pub fn encrypt(plaintext: &str) -> Result<EncryptedPayload, String> {
    let key_bytes = load_or_create_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| e.to_string())?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| e.to_string())?;

    Ok(EncryptedPayload {
        ciphertext: B64.encode(&ciphertext),
        nonce: B64.encode(nonce_bytes),
    })
}

/// Decrypt an AES-256-GCM encrypted payload.
pub fn decrypt(payload: &EncryptedPayload) -> Result<String, String> {
    let key_bytes = load_or_create_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| e.to_string())?;

    let ciphertext = B64.decode(&payload.ciphertext).map_err(|e| e.to_string())?;
    let nonce_bytes = B64.decode(&payload.nonce).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Failed to decrypt credential — key may have changed".to_string())?;

    String::from_utf8(plaintext).map_err(|e| e.to_string())
}
