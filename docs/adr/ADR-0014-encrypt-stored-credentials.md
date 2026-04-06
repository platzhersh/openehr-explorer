# ADR-0014: Encrypt Stored Server Credentials

**Date:** 2026-04-06

## Status

Accepted

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

We will encrypt credentials at rest using **AES-256-GCM** (via the [`aes-gcm`](https://crates.io/crates/aes-gcm) crate from RustCrypto) with a locally generated key.

### Why AES-256-GCM only (no OS keychain)

We considered using the OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service) as the primary store. However, OS keychains introduce significant cross-platform complexity:

- **macOS**: Keychain access is tied to the binary's code signing identity. In development, each recompile creates a new binary that cannot read entries created by the previous build, causing silent auth failures.
- **Linux**: Requires `libsecret` and a running Secret Service daemon (GNOME Keyring / KDE Wallet), which is unavailable on headless systems, containers, and minimal window managers.
- **Complexity**: Supporting both keychain and encrypted fallback means three storage modes (plaintext legacy, keychain, encrypted), increasing the surface area for bugs.

AES-256-GCM encryption works identically on all platforms with zero system dependencies beyond Rust itself.

### How It Works

- **Algorithm:** AES-256-GCM (authenticated encryption)
- **Key:** A random 256-bit key generated on first run, stored in `~/.config/openehr-explorer/.credentials-key` with restrictive file permissions (`0600` on Unix)
- **Storage format:** Base64-encoded ciphertext + nonce replace the plaintext credential in `profiles.json`:

```json
{
  "id": "abc-123",
  "name": "My EHRBase",
  "base_url": "https://ehrbase.example.com",
  "server_type": "ehrbase",
  "auth_method": {
    "type": "encrypted",
    "ciphertext": "k7Hx9mQ2v...base64...==",
    "nonce": "a1B2c3D4...base64...=="
  }
}
```

Non-secret fields (`name`, `base_url`, `server_type`, `terminology_url`) remain human-readable. Only `auth_method` and `admin_auth_method` are encrypted when they contain `Basic` or `Bearer` credentials.

### Runtime Flow

- **On save:** `save_profiles()` calls `secure_auth()` which encrypts `Basic`/`Bearer` → `Encrypted` before writing to disk.
- **On load:** `load_profiles()` calls `resolve_auth()` which decrypts `Encrypted` → `Basic`/`Bearer` for runtime use. The frontend always receives plaintext auth types.
- **On failure:** If decryption fails (e.g. key file deleted), auth falls back to `None` with a warning log. The user can re-enter credentials.

### Migration

On first load after the update:

1. Read `profiles.json` in the current plaintext format.
2. For each profile with a `basic` or `bearer` auth method, encrypt with AES-256-GCM.
3. Write the updated `profiles.json`.

The migration is **one-way and automatic** — no user interaction required.

### `AuthMethod` Enum Changes

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
    // New: encrypted credential at rest
    Encrypted { ciphertext: String, nonce: String },
}
```

A new internal function `resolve_auth(auth: &AuthMethod) -> Result<AuthMethod, String>` converts `Encrypted` back into `Basic` or `Bearer` before use in HTTP requests. This keeps the rest of the codebase unchanged.

### New Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `aes-gcm` | AES-256-GCM encryption | 0.10.3 |
| `rand` | Key generation | 0.8.5 |
| `log` | Warning/info logging | 0.4.22 |
| `base64` | Already a dependency | 0.22.1 |

### Implementation Scope

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `aes-gcm`, `rand`, `log` |
| `src-tauri/src/credentials.rs` (new) | AES-256-GCM encrypt/decrypt, key file management |
| `src-tauri/src/commands/server.rs` | Add `Encrypted` variant, `resolve_auth()`, `secure_auth()`, migration in `load_profiles()` |
| Frontend | No changes — credential resolution is entirely in the Rust backend |

## Consequences

### Positive

- Credentials are no longer stored in plaintext on disk.
- Works identically on macOS, Windows, and Linux with zero platform-specific dependencies.
- No system library requirements (`libsecret`, keychain daemons, etc.).
- No binary identity issues — encryption key is file-based, not tied to code signing.
- Migration is automatic — existing users are protected on next launch with no manual steps.
- No frontend changes required.

### Negative

- **Key file security:** The AES key in `.credentials-key` is protected only by file permissions (`0600`). A determined attacker with user-level filesystem access can read the key and decrypt credentials. This is a meaningful improvement over plaintext (prevents casual exposure, accidental sharing, grep-ability) but not equivalent to hardware-backed security.
- **Debugging friction:** Developers can no longer inspect `profiles.json` to see stored credentials. Re-entering credentials in the UI is the workaround.

### Alternatives Considered

**1. OS keychain (macOS Keychain / Windows Credential Manager / Linux Secret Service):**
Stronger security through platform-backed storage. Rejected due to cross-platform complexity: macOS ties keychain access to binary code signing identity (breaks on every recompile in dev), Linux requires `libsecret` + running daemon. The dual keychain+fallback approach tripled the storage modes and introduced subtle bugs.

**2. Tauri plugin `tauri-plugin-stronghold`:**
Tauri's official encrypted storage plugin (based on IOTA Stronghold). Provides an encrypted vault file. However, it requires a user-supplied password to unlock the vault on each app launch, which adds UX friction. Could be reconsidered if we add a master password feature in the future.

**3. Do nothing:**
The current approach is common in developer tools. However, given the healthcare context and the low implementation cost, improving credential storage is worthwhile.
