# ADR-0014: Encrypt Stored Server Credentials

**Date:** 2026-04-06

## Status

Proposed

## Context

Server profiles are persisted as JSON in `~/.config/openehr-explorer/profiles.json`. The `AuthMethod` enum stores credentials in plaintext:

```json
{
  "auth_method": {
    "type": "basic",
    "username": "ehrbase",
    "password": "SuperSecretPassword123"
  }
}
```

This applies to both `auth_method` and `admin_auth_method` fields on `ServerProfile`. Bearer tokens are equally exposed.

While the file sits in a user-owned config directory with standard file permissions, plaintext credentials are a risk:

1. **Accidental exposure** — users may share config files, back them up to cloud storage, or include them in bug reports without realizing credentials are present.
2. **Malware surface** — any process running as the user can trivially read the file and extract credentials.
3. **Compliance expectations** — healthcare environments (the primary audience for openEHR tooling) often require credentials at rest to be encrypted or stored in a platform keychain.
4. **Industry norm** — desktop applications like Docker Desktop, VS Code, and database GUIs use OS keychains or encrypted storage rather than plaintext files.

## Decision

We will use the **OS keychain** (via the [`keyring`](https://crates.io/crates/keyring) crate) as the primary credential store, with an **application-level encryption fallback** for environments where no keychain is available (headless Linux, CI, containers).

### Approach: OS Keychain with Encrypted Fallback

#### Primary — OS Keychain (`keyring` crate)

The `keyring` crate provides a unified Rust API across platform secret stores:

| Platform | Backend |
|----------|---------|
| macOS | Keychain Services |
| Windows | Credential Manager |
| Linux | Secret Service (GNOME Keyring / KDE Wallet) via `libsecret` |

Each credential is stored as a separate keychain entry keyed by `(service, profile_id)`:

- **Service name:** `openehr-explorer`
- **Account/key:** `{profile_id}:auth` or `{profile_id}:admin_auth`
- **Value:** JSON-serialized `AuthMethod` (only the sensitive variant, not the full profile)

The `profiles.json` file retains all non-secret fields. The `auth_method` field is replaced with a reference marker:

```json
{
  "id": "abc-123",
  "name": "My EHRBase",
  "base_url": "https://ehrbase.example.com",
  "server_type": "ehrbase",
  "auth_method": { "type": "keychain", "ref": "abc-123:auth" }
}
```

On load, the Rust backend resolves the reference by reading from the keychain. On save, credentials are written to the keychain and replaced with the marker in the JSON file.

#### Fallback — Application-Level Encryption

When the OS keychain is unavailable (detected at runtime when `keyring` returns a `NoBackend` or platform error), fall back to encrypting credentials in the JSON file using:

- **Algorithm:** AES-256-GCM (via the [`aes-gcm`](https://crates.io/crates/aes-gcm) crate from RustCrypto)
- **Key derivation:** A random 256-bit key generated on first run and stored in a separate file (`~/.config/openehr-explorer/.credentials-key`) with restrictive file permissions (`0600`)
- **Storage format:** Base64-encoded ciphertext + nonce in the JSON file:

```json
{
  "auth_method": {
    "type": "encrypted",
    "ciphertext": "base64...",
    "nonce": "base64..."
  }
}
```

This is not as secure as the OS keychain (the key file is on the same disk), but it prevents casual plaintext exposure — credentials are not readable by `cat` or a text editor, and they won't appear in accidental file shares or backups.

### Migration

On first load after the update:

1. Read `profiles.json` in the current plaintext format.
2. For each profile with a `basic` or `bearer` auth method:
   a. Attempt to store credentials in the OS keychain.
   b. If the keychain is unavailable, encrypt with AES-256-GCM.
   c. Replace the `auth_method` in the JSON with the appropriate marker (`keychain` or `encrypted`).
3. Write the updated `profiles.json`.

The migration is **one-way and automatic** — no user interaction required. A log message notes which storage backend was used.

### `AuthMethod` Enum Changes

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
    // New: reference to keychain-stored credential
    Keychain { ref_key: String },
    // New: encrypted credential in-file
    Encrypted { ciphertext: String, nonce: String },
}
```

A new internal function `resolve_auth(auth: &AuthMethod) -> Result<AuthMethod, String>` converts `Keychain` and `Encrypted` variants back into `Basic` or `Bearer` before use in HTTP requests. This keeps the rest of the codebase (HTTP client, request building) unchanged.

### New Dependencies

| Crate | Purpose | Version (approx.) |
|-------|---------|-------------------|
| `keyring` | OS keychain access | 3.x |
| `aes-gcm` | AES-256-GCM encryption (fallback) | 0.10.x |
| `rand` | Key generation | 0.8.x |
| `base64` | Encoding ciphertext for JSON storage | 0.22.x |

### Implementation Scope

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `keyring`, `aes-gcm`, `rand`, `base64` |
| `src-tauri/src/commands/server.rs` | Add `resolve_auth()`, update `load_profiles()` / `save_profiles()`, add migration logic |
| `src-tauri/src/credentials.rs` (new) | Keychain read/write, AES-256-GCM encrypt/decrypt, key file management |
| Frontend | No changes — the frontend sends `Basic`/`Bearer` auth to the backend; resolution is backend-only |

## Consequences

### Positive

- Credentials are no longer stored in plaintext on disk.
- OS keychain integration follows platform security best practices and user expectations.
- The encrypted fallback ensures the app still works on headless Linux / containers where no keychain daemon is running.
- Migration is automatic — existing users are protected on next launch with no manual steps.
- No frontend changes required — credential resolution is entirely in the Rust backend.

### Negative

- **Linux dependency:** The `keyring` crate requires `libsecret` (and a running Secret Service daemon) on Linux. Users without GNOME Keyring or KDE Wallet will fall back to the encrypted file approach. This should be documented.
- **Key file security (fallback):** The AES key stored in `.credentials-key` is protected only by file permissions. A determined attacker with user-level access can still decrypt credentials. This is a meaningful improvement over plaintext but not equivalent to OS keychain security.
- **Complexity increase:** The credential storage path now has three modes (plaintext legacy, keychain, encrypted) which increases the surface area for bugs. Thorough testing across platforms is needed.
- **Debugging friction:** Developers can no longer inspect `profiles.json` to quickly verify stored credentials. A Tauri command (debug-only) or CLI flag to dump decrypted profiles may be helpful during development.

### Alternatives Considered

**1. OS keychain only (no fallback):**
Simpler, but breaks on headless Linux, Docker containers, and CI environments where no keychain daemon runs. Not acceptable for a developer tool.

**2. Application-level encryption only (no keychain):**
Avoids the `libsecret` dependency on Linux, but misses the opportunity to use battle-tested platform security infrastructure. The key-on-disk model provides obfuscation more than true security.

**3. Tauri plugin `tauri-plugin-stronghold`:**
Tauri's official encrypted storage plugin (based on IOTA Stronghold). Provides an encrypted vault file. However, it requires a user-supplied password to unlock the vault on each app launch, which adds UX friction and doesn't exist in the current app flow. Could be reconsidered if we add a master password feature in the future.

**4. Do nothing:**
The current approach is common in developer tools and works. However, given the healthcare context and the low implementation cost, improving credential storage is worthwhile.
