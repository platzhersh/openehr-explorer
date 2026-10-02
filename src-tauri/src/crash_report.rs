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

/// `true` exactly once per session while consent is given.
pub fn should_report() -> bool {
    ANALYTICS_ENABLED.load(Ordering::Relaxed) && !REPORTED.swap(true, Ordering::Relaxed)
}

/// Reduce a panic source location to `last/three/components.rs:line`.
///
/// Absolute paths can contain the local username (`/home/<user>/.cargo/...`),
/// so only the trailing components are kept.
pub fn sanitize_location(file: &str, line: u32) -> String {
    let normalized = file.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|p| !p.is_empty()).collect();
    let start = parts.len().saturating_sub(KEPT_PATH_COMPONENTS);
    let mut out = format!("{}:{}", parts[start..].join("/"), line);
    out.truncate(MAX_LOCATION_LEN);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_trailing_path_components() {
        assert_eq!(
            sanitize_location(
                "/home/alice/.cargo/registry/src/x/reqwest-0.13/src/lib.rs",
                42
            ),
            "reqwest-0.13/src/lib.rs:42"
        );
    }

    #[test]
    fn handles_windows_paths_and_short_paths() {
        assert_eq!(
            sanitize_location(r"C:\Users\bob\proj\src-tauri\src\commands\ehr.rs", 7),
            "src/commands/ehr.rs:7"
        );
        assert_eq!(sanitize_location("main.rs", 1), "main.rs:1");
        assert_eq!(sanitize_location("", 0), ":0");
    }

    #[test]
    fn location_is_length_capped() {
        let long = format!("{}/x.rs", "a".repeat(500));
        assert!(sanitize_location(&long, 1).len() <= MAX_LOCATION_LEN);
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
