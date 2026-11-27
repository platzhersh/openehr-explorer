use aes_gcm::aead::{Aead, Generate, KeyInit, Nonce};
use aes_gcm::{Aes256Gcm, Key};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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

/// Keychain item holding ALL secrets as one JSON map. macOS asks for access
/// per keychain item, so one item per secret meant one prompt per secret (per
/// server) on an unsigned build; a single item means a single prompt.
const VAULT_ACCOUNT: &str = "__secrets_v2__";

/// Minimal named-item store, so the vault logic is testable without a keychain.
trait ItemStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, value: &str) -> Result<(), String>;
    fn delete(&self, account: &str) -> Result<(), String>;
}

/// [`ItemStore`] backed by the OS keychain. Every call builds a fresh `Entry`.
struct KeyringItems;

impl KeyringItems {
    fn entry(account: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE_NAME, account).map_err(|e| format!("Keychain entry error: {e}"))
    }
}

impl ItemStore for KeyringItems {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        match Self::entry(account)?.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("Keychain load error: {e}")),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        Self::entry(account)?
            .set_password(value)
            .map_err(|e| format!("Keychain store error: {e}"))
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Keychain delete error: {e}")),
        }
    }
}

#[derive(Default)]
struct VaultCache {
    /// Decoded vault; `None` until first needed (one keychain read per run).
    map: Option<HashMap<String, String>>,
    /// Legacy per-secret items already looked up, so a missing secret is
    /// not probed again.
    legacy_checked: HashSet<String>,
}

/// All secrets in a single keychain item, cached in memory after the first
/// read. Secrets stored by older versions as one item each are migrated into
/// the vault the first time they are loaded.
struct KeychainVault {
    items: Box<dyn ItemStore>,
    cache: Mutex<VaultCache>,
}

impl KeychainVault {
    fn new(items: Box<dyn ItemStore>) -> Self {
        Self {
            items,
            cache: Mutex::new(VaultCache::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, VaultCache> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn ensure_loaded(&self, cache: &mut VaultCache) -> Result<(), String> {
        if cache.map.is_none() {
            let map = match self.items.get(VAULT_ACCOUNT)? {
                Some(json) => serde_json::from_str(&json)
                    .map_err(|e| format!("Keychain vault is corrupt: {e}"))?,
                None => HashMap::new(),
            };
            cache.map = Some(map);
        }
        Ok(())
    }

    /// Write the vault and verify it by reading it back.
    fn persist(&self, map: &HashMap<String, String>) -> Result<(), String> {
        let json = serde_json::to_string(map).map_err(|e| e.to_string())?;
        self.items.set(VAULT_ACCOUNT, &json)?;
        // Read back to catch backends that accept a write but never persist it.
        match self.items.get(VAULT_ACCOUNT)? {
            Some(back) if back == json => Ok(()),
            _ => Err("Keychain verification failed: stored value does not match".to_string()),
        }
    }

    fn load(&self, account: &str) -> Result<Option<String>, String> {
        let mut guard = self.lock();
        self.ensure_loaded(&mut guard)?;
        let VaultCache {
            map,
            legacy_checked,
        } = &mut *guard;
        let map = map.as_mut().ok_or("Keychain vault not loaded")?;
        if let Some(v) = map.get(account) {
            return Ok(Some(v.clone()));
        }
        if !legacy_checked.insert(account.to_string()) {
            return Ok(None);
        }
        // Migrate a secret stored by an older version as its own item.
        match self.items.get(account)? {
            Some(legacy) => {
                map.insert(account.to_string(), legacy.clone());
                if let Err(e) = self.persist(map) {
                    map.remove(account);
                    return Err(e);
                }
                self.items.delete(account).ok();
                Ok(Some(legacy))
            }
            None => Ok(None),
        }
    }

    fn store(&self, account: &str, secret: &str) -> Result<(), String> {
        let mut guard = self.lock();
        self.ensure_loaded(&mut guard)?;
        let map = guard.map.as_mut().ok_or("Keychain vault not loaded")?;
        let previous = map.insert(account.to_string(), secret.to_string());
        if let Err(e) = self.persist(map) {
            match previous {
                Some(p) => map.insert(account.to_string(), p),
                None => map.remove(account),
            };
            return Err(e);
        }
        // Drop a stale per-secret item from an older version (no-op if absent).
        self.items.delete(account).ok();
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        let mut guard = self.lock();
        self.ensure_loaded(&mut guard)?;
        let map = guard.map.as_mut().ok_or("Keychain vault not loaded")?;
        if let Some(previous) = map.remove(account) {
            if let Err(e) = self.persist(map) {
                map.insert(account.to_string(), previous);
                return Err(e);
            }
        }
        self.items.delete(account).ok();
        Ok(())
    }
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
    vault: KeychainVault,
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
            vault: KeychainVault::new(Box::new(KeyringItems)),
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

    /// Store a secret for a profile field (e.g. "abc-123:password").
    /// The OS keychain backend writes the single secrets item and verifies it
    /// by reading it back.
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
            match self.vault.store(account, secret) {
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
            StorageBackend::OsKeychain => self.vault.load(&account),
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
            StorageBackend::OsKeychain => self.vault.delete(&account),
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
            vault: KeychainVault::new(Box::new(KeyringItems)),
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

    /// In-memory item store that counts reads of each account.
    #[derive(Clone, Default)]
    struct MemItems {
        items: std::sync::Arc<Mutex<HashMap<String, String>>>,
        gets: std::sync::Arc<Mutex<Vec<String>>>,
    }

    impl ItemStore for MemItems {
        fn get(&self, account: &str) -> Result<Option<String>, String> {
            self.gets.lock().unwrap().push(account.to_string());
            Ok(self.items.lock().unwrap().get(account).cloned())
        }
        fn set(&self, account: &str, value: &str) -> Result<(), String> {
            self.items
                .lock()
                .unwrap()
                .insert(account.to_string(), value.to_string());
            Ok(())
        }
        fn delete(&self, account: &str) -> Result<(), String> {
            self.items.lock().unwrap().remove(account);
            Ok(())
        }
    }

    #[test]
    fn vault_roundtrip_uses_one_keychain_item() {
        let mem = MemItems::default();
        let vault = KeychainVault::new(Box::new(mem.clone()));
        vault.store("p1:password", "s1").unwrap();
        vault.store("p2:token", "s2").unwrap();
        assert_eq!(vault.load("p1:password").unwrap(), Some("s1".into()));
        assert_eq!(vault.load("p2:token").unwrap(), Some("s2".into()));
        assert_eq!(mem.items.lock().unwrap().len(), 1);
        vault.delete("p1:password").unwrap();
        assert_eq!(vault.load("p1:password").unwrap(), None);
    }

    #[test]
    fn vault_reads_the_keychain_once_for_many_secrets() {
        let mem = MemItems::default();
        let json = serde_json::to_string(&HashMap::from([
            ("a:password".to_string(), "1".to_string()),
            ("b:password".to_string(), "2".to_string()),
            ("c:token".to_string(), "3".to_string()),
        ]))
        .unwrap();
        mem.items.lock().unwrap().insert(VAULT_ACCOUNT.into(), json);
        // Fresh process: new vault over the same keychain.
        let vault = KeychainVault::new(Box::new(mem.clone()));
        for acct in ["a:password", "b:password", "c:token"] {
            assert!(vault.load(acct).unwrap().is_some());
        }
        let gets = mem.gets.lock().unwrap();
        assert_eq!(gets.iter().filter(|g| *g == VAULT_ACCOUNT).count(), 1);
        assert_eq!(gets.len(), 1);
    }

    #[test]
    fn vault_migrates_legacy_items_and_removes_them() {
        let mem = MemItems::default();
        mem.items
            .lock()
            .unwrap()
            .insert("old:password".into(), "legacy".into());
        let vault = KeychainVault::new(Box::new(mem.clone()));
        assert_eq!(vault.load("old:password").unwrap(), Some("legacy".into()));
        assert!(!mem.items.lock().unwrap().contains_key("old:password"));
        // Survives a restart via the vault item alone.
        let again = KeychainVault::new(Box::new(mem));
        assert_eq!(again.load("old:password").unwrap(), Some("legacy".into()));
    }

    #[test]
    fn vault_does_not_reprobe_missing_secrets() {
        let mem = MemItems::default();
        let vault = KeychainVault::new(Box::new(mem.clone()));
        assert_eq!(vault.load("x:password").unwrap(), None);
        assert_eq!(vault.load("x:password").unwrap(), None);
        let gets = mem.gets.lock().unwrap();
        assert_eq!(gets.iter().filter(|g| *g == "x:password").count(), 1);
    }
}
