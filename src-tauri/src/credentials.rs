use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use zeroize::Zeroize;

const SERVICE_NAME: &str = "openehr-explorer";
const NONCE_SIZE: usize = 12;

/// Credential storage backend in use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackend {
    OsKeychain,
    EncryptedFile,
}

/// Encrypted credential store on disk (fallback when no keychain is available).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct EncryptedStore {
    /// Map of "profile_id:field" -> base64-encoded(nonce ++ ciphertext)
    entries: HashMap<String, String>,
}

/// Manages credential storage with OS keychain primary and encrypted-file fallback.
pub struct CredentialManager {
    backend: StorageBackend,
    /// AES-256 key bytes, only loaded when using encrypted-file backend.
    file_key: Option<Vec<u8>>,
}

impl CredentialManager {
    /// Initialise the credential manager, probing keychain availability.
    pub fn new(config_dir: &std::path::Path) -> Self {
        if Self::keychain_available() {
            return Self {
                backend: StorageBackend::OsKeychain,
                file_key: None,
            };
        }

        // Fall back to encrypted file
        let key = Self::load_or_create_file_key(config_dir);
        Self {
            backend: StorageBackend::EncryptedFile,
            file_key: Some(key),
        }
    }

    pub fn backend(&self) -> &StorageBackend {
        &self.backend
    }

    /// Store a secret for a profile field (e.g. "abc-123:password").
    /// For the OS keychain backend, performs a read-back verification
    /// using a FRESH Entry object (not the one that wrote) to ensure
    /// the secret was truly persisted to the system store.
    pub fn store_secret(
        &self,
        profile_id: &str,
        field: &str,
        mut secret: String,
        config_dir: &std::path::Path,
    ) -> Result<(), String> {
        let account = format!("{profile_id}:{field}");
        let result = match &self.backend {
            StorageBackend::OsKeychain => {
                let entry = keyring::Entry::new(SERVICE_NAME, &account)
                    .map_err(|e| format!("Keychain entry error: {e}"))?;
                entry
                    .set_password(&secret)
                    .map_err(|e| format!("Keychain store error: {e}"))?;
                // Read-back with a FRESH entry to catch phantom writes where
                // the Entry object caches the value in-memory but never
                // actually persists it to the system credential store.
                let verify = keyring::Entry::new(SERVICE_NAME, &account)
                    .map_err(|e| format!("Keychain verify entry error: {e}"))?;
                let readback = verify
                    .get_password()
                    .map_err(|e| format!("Keychain read-back failed: {e}"))?;
                if readback != secret {
                    return Err(
                        "Keychain verification failed: stored value does not match".to_string()
                    );
                }
                Ok(())
            }
            StorageBackend::EncryptedFile => {
                let key = self.file_key.as_ref().ok_or("Encryption key not loaded")?;
                let encrypted = Self::encrypt(key, secret.as_bytes())?;
                let mut store = Self::load_encrypted_store(config_dir);
                store.entries.insert(account, base64_encode(&encrypted));
                Self::save_encrypted_store(config_dir, &store)
            }
        };
        secret.zeroize();
        result
    }

    /// Load a secret for a profile field. Returns None if not found.
    pub fn load_secret(
        &self,
        profile_id: &str,
        field: &str,
        config_dir: &std::path::Path,
    ) -> Result<Option<String>, String> {
        let account = format!("{profile_id}:{field}");
        match &self.backend {
            StorageBackend::OsKeychain => {
                let entry = keyring::Entry::new(SERVICE_NAME, &account)
                    .map_err(|e| format!("Keychain entry error: {e}"))?;
                match entry.get_password() {
                    Ok(s) => Ok(Some(s)),
                    Err(keyring::Error::NoEntry) => Ok(None),
                    Err(e) => Err(format!("Keychain load error: {e}")),
                }
            }
            StorageBackend::EncryptedFile => {
                let key = self.file_key.as_ref().ok_or("Encryption key not loaded")?;
                let store = Self::load_encrypted_store(config_dir);
                match store.entries.get(&account) {
                    Some(encoded) => {
                        let data = base64_decode(encoded)?;
                        let plaintext = Self::decrypt(key, &data)?;
                        Ok(Some(
                            String::from_utf8(plaintext)
                                .map_err(|e| format!("Invalid UTF-8: {e}"))?,
                        ))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    /// Delete a secret for a profile field.
    pub fn delete_secret(
        &self,
        profile_id: &str,
        field: &str,
        config_dir: &std::path::Path,
    ) -> Result<(), String> {
        let account = format!("{profile_id}:{field}");
        match &self.backend {
            StorageBackend::OsKeychain => {
                let entry = keyring::Entry::new(SERVICE_NAME, &account)
                    .map_err(|e| format!("Keychain entry error: {e}"))?;
                match entry.delete_credential() {
                    Ok(()) => Ok(()),
                    Err(keyring::Error::NoEntry) => Ok(()), // already gone
                    Err(e) => Err(format!("Keychain delete error: {e}")),
                }
            }
            StorageBackend::EncryptedFile => {
                let mut store = Self::load_encrypted_store(config_dir);
                store.entries.remove(&account);
                Self::save_encrypted_store(config_dir, &store)
            }
        }
    }

    /// Delete all secrets for a profile (password, token, admin_password, admin_token).
    pub fn delete_all_secrets(
        &self,
        profile_id: &str,
        config_dir: &std::path::Path,
    ) -> Result<(), String> {
        for field in &["password", "token", "admin_password", "admin_token"] {
            self.delete_secret(profile_id, field, config_dir)?;
        }
        Ok(())
    }

    // --- Private helpers ---

    /// Probe whether the OS keychain is usable by performing a full
    /// write → read-back (fresh entry) → delete cycle. Uses a separate
    /// Entry object for the read-back to catch backends that cache
    /// values in-memory without actually persisting to the system store.
    fn keychain_available() -> bool {
        let probe_account = "__openehr_probe__";
        let probe_secret = "__probe_value__";

        // Write with one Entry object
        let write_entry = match keyring::Entry::new(SERVICE_NAME, probe_account) {
            Ok(e) => e,
            Err(_) => return false,
        };
        if write_entry.set_password(probe_secret).is_err() {
            return false;
        }

        // Read back with a FRESH Entry object to verify true persistence
        let read_entry = match keyring::Entry::new(SERVICE_NAME, probe_account) {
            Ok(e) => e,
            Err(_) => {
                let _ = write_entry.delete_credential();
                return false;
            }
        };
        let read_ok = match read_entry.get_password() {
            Ok(val) => val == probe_secret,
            Err(_) => false,
        };

        // Clean up
        let _ = write_entry.delete_credential();

        read_ok
    }

    /// Load or generate the file-encryption key.
    fn load_or_create_file_key(config_dir: &std::path::Path) -> Vec<u8> {
        let key_path = config_dir.join(".key");
        if let Ok(data) = fs::read(&key_path) {
            if data.len() == 32 {
                return data;
            }
        }
        // Generate a new 256-bit key
        let mut key = vec![0u8; 32];
        OsRng.fill_bytes(&mut key);
        fs::create_dir_all(config_dir).ok();
        fs::write(&key_path, &key).ok();
        // Set restrictive permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).ok();
        }
        key
    }

    fn encrypt(key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|e| format!("Cipher init error: {e}"))?;
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption error: {e}"))?;
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    fn decrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < NONCE_SIZE {
            return Err("Encrypted data too short".to_string());
        }
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|e| format!("Cipher init error: {e}"))?;
        let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
        cipher
            .decrypt(nonce, &data[NONCE_SIZE..])
            .map_err(|e| format!("Decryption error: {e}"))
    }

    fn encrypted_store_path(config_dir: &std::path::Path) -> PathBuf {
        config_dir.join("credentials.enc")
    }

    fn load_encrypted_store(config_dir: &std::path::Path) -> EncryptedStore {
        let path = Self::encrypted_store_path(config_dir);
        if path.exists() {
            let data = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            EncryptedStore::default()
        }
    }

    fn save_encrypted_store(
        config_dir: &std::path::Path,
        store: &EncryptedStore,
    ) -> Result<(), String> {
        let path = Self::encrypted_store_path(config_dir);
        let data = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
        fs::write(&path, &data).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).ok();
        }
        Ok(())
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| format!("Base64 decode error: {e}"))
}

/// Harden file permissions on a path (Unix only).
pub fn harden_file_permissions(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = vec![0x42u8; 32];
        let plaintext = b"my-secret-password";
        let encrypted = CredentialManager::encrypt(&key, plaintext).unwrap();
        let decrypted = CredentialManager::decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypted_store_roundtrip() {
        let dir = std::env::temp_dir().join("openehr-test-cred");
        fs::create_dir_all(&dir).ok();

        // Create manager with file backend by forcing it
        let key = CredentialManager::load_or_create_file_key(&dir);
        let mgr = CredentialManager {
            backend: StorageBackend::EncryptedFile,
            file_key: Some(key),
        };

        mgr.store_secret("prof-1", "password", "secret123".to_string(), &dir)
            .unwrap();
        let loaded = mgr.load_secret("prof-1", "password", &dir).unwrap();
        assert_eq!(loaded, Some("secret123".to_string()));

        mgr.delete_secret("prof-1", "password", &dir).unwrap();
        let deleted = mgr.load_secret("prof-1", "password", &dir).unwrap();
        assert_eq!(deleted, None);

        // Clean up
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"hello world";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_decrypt_short_data_fails() {
        let key = vec![0x42u8; 32];
        let result = CredentialManager::decrypt(&key, &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_file_permissions() {
        let dir = std::env::temp_dir().join("openehr-test-key");
        fs::create_dir_all(&dir).ok();
        let _key = CredentialManager::load_or_create_file_key(&dir);
        let key_path = dir.join(".key");
        assert!(key_path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::metadata(&key_path).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o600);
        }
        fs::remove_dir_all(&dir).ok();
    }
}
