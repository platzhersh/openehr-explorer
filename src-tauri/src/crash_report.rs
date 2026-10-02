//! Opt-in panic reporting via Aptabase (see ADR-0018, "Crash and error reports").
//!
//! The Aptabase plugin's panic hook runs below the frontend consent gate, so
//! the Rust side keeps its own mirror of `analytics_enabled`. It is seeded from
//! the persisted settings at startup and refreshed whenever settings are saved.
//!
//! Panic *messages* are never sent: they routinely embed server URLs, EHR IDs,
//! AQL or JSON fragments (e.g. `unwrap()` on a `reqwest`/serde error). Only a
//! scrubbed `file:line` location is reported.

use std::sync::atomic::{AtomicBool, Ordering};

static ANALYTICS_ENABLED: AtomicBool = AtomicBool::new(false);
/// At most one panic report per session, so a crash loop can't exhaust the
/// Aptabase free-tier quota.
static REPORTED: AtomicBool = AtomicBool::new(false);

const MAX_LOCATION_LEN: usize = 120;
const KEPT_PATH_COMPONENTS: usize = 3;

/// Mirror the user's analytics consent for use inside the panic hook.
pub fn set_enabled(enabled: bool) {
    ANALYTICS_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Claim the process's single panic-report slot when analytics consent is enabled.
/// Returns `false` without consuming the slot when disabled, or if already claimed.
/// Toggling consent does not reset the slot, even if sending the report fails.
pub fn should_report() -> bool {
    ANALYTICS_ENABLED.load(Ordering::Relaxed) && !REPORTED.swap(true, Ordering::Relaxed)
}

/// Reduce a panic source location to trailing path components and a line number.
///
/// - Relative paths (our own crate, e.g. `src/commands/ehr.rs`) keep their
///   last three components.
/// - Dependency paths under a cargo registry (`.../registry/src/<index>/<crate>/...`)
///   are reduced to `<crate>/...`, again the last three components.
/// - Any other absolute path (Unix, Windows drive or UNC) becomes `unknown`,
///   since `/home/<user>/main.rs` would leak the username.
///
/// Accepted paths have `:line` appended, then are truncated to 120 Unicode scalar
/// values, which can truncate the line suffix. Retained components are not redacted.
pub fn sanitize_location(file: &str, line: u32) -> String {
    let normalized = file.replace('\\', "/");
    let is_absolute = normalized.starts_with('/') || normalized.as_bytes().get(1) == Some(&b':');
    let relevant = if is_absolute {
        match normalized.split_once("/registry/src/") {
            // Skip the registry index directory (`index.crates.io-<hash>`).
            Some((_, rest)) => rest.split_once('/').map(|(_, crate_path)| crate_path),
            None => None,
        }
    } else {
        Some(normalized.as_str())
    };
    let Some(relevant) = relevant else {
        return "unknown".to_string();
    };
    let parts: Vec<&str> = relevant.split('/').filter(|p| !p.is_empty()).collect();
    let start = parts.len().saturating_sub(KEPT_PATH_COMPONENTS);
    let out = format!("{}:{}", parts[start..].join("/"), line);
    // Truncate on a char boundary: slicing mid-codepoint would panic inside
    // the panic hook.
    out.chars().take(MAX_LOCATION_LEN).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_trailing_components_of_relative_paths() {
        assert_eq!(
            sanitize_location("src-tauri/src/commands/ehr.rs", 42),
            "src/commands/ehr.rs:42"
        );
        assert_eq!(sanitize_location("main.rs", 1), "main.rs:1");
    }

    #[test]
    fn dependency_paths_drop_everything_before_the_crate() {
        assert_eq!(
            sanitize_location(
                "/home/alice/.cargo/registry/src/index.crates.io-abc/reqwest-0.13/src/lib.rs",
                42
            ),
            "reqwest-0.13/src/lib.rs:42"
        );
    }

    #[test]
    fn other_absolute_paths_are_unknown() {
        assert_eq!(sanitize_location("/home/alice/main.rs", 1), "unknown");
        assert_eq!(
            sanitize_location(r"C:\Users\bob\proj\main.rs", 7),
            "unknown"
        );
        assert_eq!(sanitize_location(r"\\server\share\x.rs", 7), "unknown");
    }

    #[test]
    fn location_is_length_capped_without_splitting_chars() {
        let long = format!("{}/x.rs", "a".repeat(500));
        assert!(sanitize_location(&long, 1).len() <= MAX_LOCATION_LEN);
        // Multibyte char straddling the byte limit must not panic.
        let multibyte = format!("{}é/x.rs", "a".repeat(MAX_LOCATION_LEN - 2));
        let out = sanitize_location(&multibyte, 1);
        assert!(out.chars().count() <= MAX_LOCATION_LEN);
    }

    #[test]
    fn reports_only_when_enabled_and_only_once() {
        // Single test for the shared statics to avoid cross-test races.
        set_enabled(false);
        assert!(!should_report());
        set_enabled(true);
        assert!(should_report());
        assert!(!should_report());
        set_enabled(false);
    }
}
