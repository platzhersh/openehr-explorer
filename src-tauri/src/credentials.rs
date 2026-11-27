use aes_gcm::aead::{Aead, Generate, KeyInit, Nonce};
use aes_gcm::{Aes256Gcm, Key};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
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

/// Name of the marker file (in the config dir) that records which backend
/// holds the secrets, so it never flips between launches.
const BACKEND_MARKER: &str = "credential_backend";

#[derive(Debug)]
struct State {
    backend: StorageBackend,
    /// AES-256 key bytes, only loaded when using encrypted-file backend.
    file_key: Option<Vec<u8>>,
    /// True once the backend choice is persisted (marker written or legacy
    /// encrypted store found). An unconfirmed keychain backend is verified
    /// by the first successful `store_secret`, never by a startup probe.
    confirmed: bool,
}

/// Manages credential storage with OS keychain primary and encrypted-file fallback.
///
/// Startup performs **no keychain access**: on macOS every keychain operation
/// on an unsigned build can raise an ACL prompt, so the backend is chosen from
/// on-disk state only (see [`choose_initial_backend`]).
pub struct CredentialManager {
    state: Mutex<State>,
}

/// Pure backend choice from on-disk state: the persisted marker wins; without
/// one, an existing `credentials.enc` means the legacy fallback was in use;
/// otherwise the OS keychain is assumed (unconfirmed). Returns
/// `(backend, confirmed)`.
fn choose_initial_backend(
    marker: Option<StorageBackend>,
    encrypted_store_exists: bool,
) -> (StorageBackend, bool) {
    match marker {
        Some(b) => (b, true),
        None if encrypted_store_exists => (StorageBackend::EncryptedFile, true),
        None => (StorageBackend::OsKeychain, false),
    }
}

impl CredentialManager {
    /// Initialise the credential manager without touching the keychain.
    pub fn new(config_dir: &std::path::Path) -> Self {
        let marker = Self::read_marker(config_dir);
        let (backend, confirmed) =
            choose_initial_backend(marker, Self::encrypted_store_path(config_dir).exists());
        let file_key = (backend == StorageBackend::EncryptedFile)
            .then(|| Self::load_or_create_file_key(config_dir));
        Self {
            state: Mutex::new(State {
                backend,
                file_key,
                confirmed,
            }),
        }
    }

    pub fn backend(&self) -> StorageBackend {
        self.lock().backend.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn read_marker(config_dir: &std::path::Path) -> Option<StorageBackend> {
        match fs::read_to_string(config_dir.join(BACKEND_MARKER))
            .ok()?
            .trim()
        {
            "os_keychain" => Some(StorageBackend::OsKeychain),
            "encrypted_file" => Some(StorageBackend::EncryptedFile),
            _ => None,
        }
    }

    fn write_marker(config_dir: &std::path::Path, backend: &StorageBackend) {
        let value = match backend {
            StorageBackend::OsKeychain => "os_keychain",
            StorageBackend::EncryptedFile => "encrypted_file",
        };
        fs::create_dir_all(config_dir).ok();
        fs::write(config_dir.join(BACKEND_MARKER), value).ok();
    }

    /// Write `secret` to the keychain and verify it with a FRESH entry.
    fn keychain_store(account: &str, secret: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE_NAME, account)
            .map_err(|e| format!("Keychain entry error: {e}"))?;
        entry
            .set_password(secret)
            .map_err(|e| format!("Keychain store error: {e}"))?;
        // Read-back with a FRESH entry to catch phantom writes where
        // the Entry object caches the value in-memory but never
        // actually persists it to the system credential store.
        let verify = keyring::Entry::new(SERVICE_NAME, account)
            .map_err(|e| format!("Keychain verify entry error: {e}"))?;
        let readback = verify
            .get_password()
            .map_err(|e| format!("Keychain read-back failed: {e}"))?;
        if readback != secret {
            return Err("Keychain verification failed: stored value does not match".to_string());
        }
        Ok(())
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
        let result = self.store_inner(&account, &secret, config_dir);
        secret.zeroize();
        result
    }

    fn store_inner(
        &self,
        account: &str,
        secret: &str,
        config_dir: &std::path::Path,
    ) -> Result<(), String> {
        let (backend, confirmed) = {
            let st = self.lock();
            (st.backend.clone(), st.confirmed)
        };
        if backend == StorageBackend::OsKeychain {
            match Self::keychain_store(account, secret) {
                Ok(()) => {
                    if !confirmed {
                        Self::write_marker(config_dir, &StorageBackend::OsKeychain);
                        self.lock().confirmed = true;
                    }
                    return Ok(());
                }
                // Only a never-confirmed keychain may fall back, and never on
                // macOS: there a failure usually means a denied/cancelled ACL
                // prompt, and switching would strand existing keychain secrets.
                Err(e) if confirmed || cfg!(target_os = "macos") => return Err(e),
                Err(_) => {
                    let key = Self::load_or_create_file_key(config_dir);
                    Self::write_marker(config_dir, &StorageBackend::EncryptedFile);
                    let mut st = self.lock();
                    st.backend = StorageBackend::EncryptedFile;
                    st.file_key = Some(key);
                    st.confirmed = true;
                }
            }
        }
        let key = self
            .lock()
            .file_key
            .clone()
            .ok_or("Encryption key not loaded")?;
        let encrypted = Self::encrypt(&key, secret.as_bytes())?;
        let mut store = Self::load_encrypted_store(config_dir);
        store
            .entries
            .insert(account.to_string(), base64_encode(&encrypted));
        Self::save_encrypted_store(config_dir, &store)
    }

    /// Load a secret for a profile field. Returns None if not found.
    pub fn load_secret(
        &self,
        profile_id: &str,
        field: &str,
        config_dir: &std::path::Path,
    ) -> Result<Option<String>, String> {
        let account = format!("{profile_id}:{field}");
        let (backend, file_key) = {
            let st = self.lock();
            (st.backend.clone(), st.file_key.clone())
        };
        match backend {
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
                let key = file_key.as_ref().ok_or("Encryption key not loaded")?;
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
        let backend = self.backend();
        match backend {
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

    /// Load or generate the file-encryption key.
    fn load_or_create_file_key(config_dir: &std::path::Path) -> Vec<u8> {
        let key_path = config_dir.join(".key");
        if let Ok(data) = fs::read(&key_path) {
            if data.len() == 32 {
                return data;
            }
        }
        // Generate a new 256-bit key
        let key = Key::<Aes256Gcm>::generate().to_vec();
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
        let nonce = Nonce::<Aes256Gcm>::generate();
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| format!("Encryption error: {e}"))?;
        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    fn decrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < NONCE_SIZE {
            return Err("Encrypted data too short".to_string());
        }
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|e| format!("Cipher init error: {e}"))?;
        let nonce = Nonce::<Aes256Gcm>::try_from(&data[..NONCE_SIZE])
            .map_err(|_| "Invalid nonce length".to_string())?;
        cipher
            .decrypt(&nonce, &data[NONCE_SIZE..])
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

/// Harden file permissions on a path (Unix only; no-op elsewhere).
#[cfg(unix)]
pub fn harden_file_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).ok();
}

/// Harden file permissions on a path (Unix only; no-op elsewhere).
#[cfg(not(unix))]
pub fn harden_file_permissions(_path: &std::path::Path) {}

#[cfg(test)]
mod tests {
    use super::*;

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
            state: Mutex::new(State {
                backend: StorageBackend::EncryptedFile,
                file_key: Some(key),
                confirmed: true,
            }),
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

    #[test]
    fn marker_wins_over_everything() {
        assert_eq!(
            choose_initial_backend(Some(StorageBackend::OsKeychain), true),
            (StorageBackend::OsKeychain, true)
        );
        assert_eq!(
            choose_initial_backend(Some(StorageBackend::EncryptedFile), false),
            (StorageBackend::EncryptedFile, true)
        );
    }

    #[test]
    fn legacy_encrypted_store_keeps_file_backend() {
        assert_eq!(
            choose_initial_backend(None, true),
            (StorageBackend::EncryptedFile, true)
        );
    }

    #[test]
    fn fresh_install_assumes_unconfirmed_keychain() {
        assert_eq!(
            choose_initial_backend(None, false),
            (StorageBackend::OsKeychain, false)
        );
    }

    #[test]
    fn new_never_touches_keychain_and_reads_marker() {
        let dir = std::env::temp_dir().join("openehr-test-marker");
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).unwrap();
        let fresh = CredentialManager::new(&dir);
        assert_eq!(fresh.backend(), StorageBackend::OsKeychain);
        CredentialManager::write_marker(&dir, &StorageBackend::EncryptedFile);
        let again = CredentialManager::new(&dir);
        assert_eq!(again.backend(), StorageBackend::EncryptedFile);
        fs::remove_dir_all(&dir).ok();
    }
}
