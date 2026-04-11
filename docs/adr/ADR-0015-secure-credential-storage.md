# ADR-0015: Secure Credential Storage

**Date:** 2026-04-11
**Status:** Proposed
**Related:** ADR-0014 (XSS Hardening and Input Validation)

---

## Context

Server profiles are persisted to `~/.config/openehr-explorer/profiles.json` as plaintext JSON, including authentication credentials:

```json
{
  "profiles": [
    {
      "id": "abc-123",
      "name": "Production EHRBase",
      "base_url": "https://ehrbase.example.com",
      "auth_method": {
        "type": "basic",
        "username": "api-user",
        "password": "plaintext-password-here"
      }
    }
  ]
}
```

This was identified as the highest-severity finding in a pre-publication security audit. The risks:

1. **Local file access**: Any process running as the same user can read the credentials. On shared systems, incorrect file permissions (default `umask`) may expose them to other users.
2. **Backup exposure**: The file is included in home directory backups, cloud sync (Dropbox, iCloud), and disk images without any protection.
3. **Credential theft**: If combined with an XSS vulnerability (see ADR-0014), an attacker could invoke the `list_server_profiles` Tauri command to exfiltrate all stored credentials.
4. **No memory protection**: Credentials are held as regular Rust `String` values, which are not zeroed on drop and may persist in memory or swap.

The current implementation in `src-tauri/src/commands/server.rs` (lines 70-89) uses `serde_json::to_string_pretty()` followed by `fs::write()` with no encryption layer and no file permission hardening.

---

## Decision

### Primary approach: OS keychain integration via the `keyring` crate

Store credential secrets (passwords and bearer tokens) in the operating system's native credential manager:

| Platform | Backend |
|---|---|
| macOS | Keychain Services |
| Linux | Secret Service (GNOME Keyring / KDE Wallet) via D-Bus |
| Windows | Windows Credential Manager |

The Rust [`keyring`](https://crates.io/crates/keyring) crate provides a unified API across all three platforms.

### Storage architecture

Split the server profile into public metadata (stored in `profiles.json`) and secrets (stored in the OS keychain):

```
profiles.json (on disk)          OS Keychain
┌─────────────────────┐          ┌──────────────────────────┐
│ id: "abc-123"       │          │ service: "openehr-explorer" │
│ name: "Production"  │          │ account: "abc-123:password"│
│ base_url: "https://"│    ──►   │ secret: "the-password"    │
│ auth_method:        │          └──────────────────────────┘
│   type: "basic"     │
│   username: "user"  │
│   password: null     │  ← sentinel indicating "stored in keychain"
└─────────────────────┘
```

**Keychain entry naming convention:**
- Service name: `openehr-explorer`
- Account name: `{profile_id}:password` or `{profile_id}:token`

### Credential lifecycle

1. **Save profile**: Extract the secret from `AuthMethod`, store it in the keychain under `{profile_id}:{field}`, replace the field in the profile struct with a sentinel (empty string or `null`), then write the sanitized profile to `profiles.json`.

2. **Load profile**: Read `profiles.json`, then for each profile with a `basic` or `bearer` auth method, look up the secret from the keychain and populate the field in memory.

3. **Delete profile**: Remove the keychain entry, then remove from `profiles.json`.

4. **Test connection**: Resolve the secret from the keychain before building the HTTP request. Never pass unresolved profiles over Tauri IPC.

### Fallback for environments without a keychain

On headless Linux systems or environments where Secret Service is unavailable, fall back to an encrypted file:

1. On first run, generate a random 256-bit key and store it with restrictive file permissions (`0600`) at `~/.config/openehr-explorer/.key`.
2. Encrypt credentials using AES-256-GCM (via the `aes-gcm` crate) before writing to a `credentials.enc` file.
3. Display a one-time warning in the UI that credentials are stored with file-based encryption rather than the OS keychain.

This is less secure than a proper keychain (the key file can be read by the same user), but is still far better than plaintext and handles the common "Linux server with no desktop environment" case.

### File permission hardening

Regardless of the storage backend, set `profiles.json` permissions to `0600` (owner read/write only) on Unix systems:

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(&path, perms).ok();
}
```

### Memory protection

Add the `zeroize` crate and derive `Zeroize` + `ZeroizeOnDrop` on the `AuthMethod` enum's secret-bearing variants, so passwords are cleared from memory when profiles are dropped:

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
struct SensitiveString(String);
```

### IPC credential isolation

Currently, full `ServerProfile` objects (including plaintext credentials) are sent from the frontend to the backend and back via Tauri `invoke`. After this change:

- The backend resolves credentials internally from the keychain — the frontend never sees raw secrets.
- The frontend sends only the `profile_id` for operations that need authentication.
- The `save_server_profile` command still accepts credentials from the frontend (the user typed them), but immediately moves them to the keychain and never returns them.
- The `list_server_profiles` response omits secret fields (replaced with a boolean `has_password: true` flag for UI display).

---

## Consequences

### Positive
- Credentials are protected by OS-level access controls and encryption
- Eliminates the highest-severity finding from the security audit
- Prevents credential exfiltration via XSS (secrets never cross the IPC boundary after initial save)
- Memory zeroing prevents secrets lingering in process memory or swap
- File permission hardening protects against misconfigured systems

### Negative
- Adds platform-specific dependencies (`keyring` crate, Secret Service on Linux)
- Users on headless Linux need a running Secret Service daemon or accept file-based fallback
- Migration required for existing users: on first launch after update, the app must detect plaintext credentials in `profiles.json`, migrate them to the keychain, and rewrite the file without secrets
- More complex code path for credential resolution (load profile + keychain lookup vs. single file read)
- Testing becomes harder — tests need to mock the keychain or use a test keyring

### Neutral
- Admin credentials (`admin_auth_method`) follow the same pattern as primary credentials
- The docker-compose default credentials are unaffected (they're for local dev, not stored in profiles)

---

## Implementation Plan

### Phase 1: Core keychain integration
1. Add `keyring` and `zeroize` crates to `Cargo.toml` (pinned versions)
2. Create `src-tauri/src/credentials.rs` module with `store_secret()`, `load_secret()`, `delete_secret()` functions
3. Modify `save_profiles()` to extract and store secrets via keychain
4. Modify `load_profiles()` to resolve secrets from keychain
5. Harden file permissions on `profiles.json`

### Phase 2: IPC isolation
1. Create a `ServerProfilePublic` struct that omits secret fields
2. Change `list_server_profiles` to return `Vec<ServerProfilePublic>`
3. Change auth-requiring commands to accept `profile_id: String` instead of `profile: ServerProfile`
4. Add internal `resolve_profile()` that loads the full profile with secrets from keychain

### Phase 3: Migration and fallback
1. Implement first-launch migration: detect plaintext secrets, move to keychain, rewrite file
2. Implement encrypted-file fallback for environments without keychain
3. Add UI indicator showing which storage backend is active

### Phase 4: Frontend updates
1. Update `src/stores/server.ts` to work with `ServerProfilePublic` (no secret fields)
2. Update `ServerManager.vue` to only send credentials during save, not receive them back
3. Add password field UX: show "Password saved securely" instead of the actual value when editing existing profiles

### Files to modify

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | Add `keyring`, `zeroize`, `aes-gcm` dependencies |
| `src-tauri/src/credentials.rs` | New: keychain abstraction + encrypted-file fallback |
| `src-tauri/src/commands/server.rs` | Split save/load to separate secrets from metadata |
| `src-tauri/src/commands/ehr.rs` | Accept `profile_id` instead of full profile |
| `src-tauri/src/commands/query.rs` | Accept `profile_id` instead of full profile |
| `src-tauri/src/commands/composition.rs` | Accept `profile_id` instead of full profile |
| `src-tauri/src/commands/template.rs` | Accept `profile_id` instead of full profile |
| `src-tauri/src/lib.rs` | Register new commands, add credential state |
| `src/stores/server.ts` | Use `ServerProfilePublic`, send `profile_id` for operations |
| `src/views/ServerManager.vue` | Update form to handle opaque credential state |
