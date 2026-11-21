//! Opt-in, anonymous usage analytics via the community `aptabase-rs` SDK (the official plugin is Tauri-only).
//!
//! **Off by default**; active only when the user opts in (Settings, or `--analytics`) AND an App Key is present. Can be
//! switched on and off at runtime. Never send query text, EHR/composition ids, server URLs or credentials: only
//! coarse event names with a few non-identifying properties. Unlike the Tauri app this sends no unconditional
//! `session_started` ping: nothing at all is sent until the user opts in.

use aptabase_rs::{AptabaseClient, Builder, InitOptions};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How often queued events are sent while analytics is on.
const FLUSH_EVERY: Duration = Duration::from_secs(60);

/// A running client together with the task that flushes it, so opting out can stop both at once.
struct Active {
    client: Arc<AptabaseClient>,
    flusher: tokio::task::JoinHandle<()>,
}

#[derive(Clone, Default)]
pub struct Analytics {
    inner: Arc<Mutex<Option<Active>>>,
    host: Option<String>,
}

/// Compile-time key like the Tauri app; the runtime env var exists for local testing only.
fn app_key() -> Option<String> {
    option_env!("APTABASE_APP_KEY").map(str::to_string).or_else(|| std::env::var("APTABASE_APP_KEY").ok()).filter(|k| !k.is_empty())
}

/// The checks the SDK does silently (and answers by disabling itself): done up front so a client that would never send is
/// not stored, `enabled()` stays false, and the reason can be shown. Valid keys: `A-US-…`, `A-EU-…`, `A-DEV-…`, `A-SH-…`
/// (the latter needs an http(s) host).
fn validate(key: &str, host: Option<&str>) -> Result<(), String> {
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() != 3 || !["US", "EU", "DEV", "SH"].contains(&parts[1]) {
        return Err("the Aptabase App Key is not valid".to_string());
    }
    let host_ok = |h: &str| h.strip_prefix("https://").or_else(|| h.strip_prefix("http://")).is_some_and(|rest| !rest.trim_matches('/').is_empty() && !rest.contains(char::is_whitespace));
    match (parts[1], host) {
        ("SH", None) => Err("a self-hosted App Key needs --analytics-host".to_string()),
        (_, Some(h)) if !host_ok(h) => Err(format!("the analytics host '{h}' is not an http(s) URL")),
        _ => Ok(()),
    }
}

impl Analytics {
    /// `host` is only needed for self-hosted keys (`A-SH-…`) and for pointing tests at a local endpoint.
    pub fn init(opted_in: bool, host: Option<String>) -> Self {
        let a = Self { inner: Arc::default(), host };
        if let Err(e) = a.set_enabled(opted_in) {
            eprintln!("analytics disabled: {e}");
        }
        a
    }

    /// Whether this build has an App Key at all (without one, opting in sends nothing).
    pub fn has_key() -> bool {
        app_key().is_some()
    }

    pub fn enabled(&self) -> bool {
        self.inner.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    /// Switch collection on or off immediately. Turning it off stops the flusher and drops the client together with any
    /// events still queued: nothing is sent after the user opted out. Never blocks. Opting in without a usable key/host
    /// is a no-op that reports why.
    pub fn set_enabled(&self, on: bool) -> Result<(), String> {
        let Ok(mut guard) = self.inner.lock() else { return Err("analytics state is unavailable".to_string()) };
        if !on {
            if let Some(active) = guard.take() {
                active.flusher.abort();
            }
            return Ok(());
        }
        if guard.is_some() {
            return Ok(());
        }
        let Some(key) = app_key() else { return Ok(()) }; // no key in this build: nothing to start (shown in Settings)
        validate(&key, self.host.as_deref())?;
        let _rt = crate::net::runtime().enter();
        let client = Builder::new(&key, env!("CARGO_PKG_VERSION")).with_options(InitOptions { host: self.host.clone(), flush_interval: None }).build();
        let flusher = {
            let c = client.clone();
            crate::net::runtime().spawn(async move {
                loop {
                    tokio::time::sleep(FLUSH_EVERY).await;
                    c.flush().await;
                }
            })
        };
        *guard = Some(Active { client, flusher });
        Ok(())
    }

    pub fn track(&self, name: &str, props: Option<Value>) {
        if let Ok(guard) = self.inner.lock() {
            if let Some(a) = guard.as_ref() {
                let _ = a.client.track_event(name, props);
            }
        }
    }

    /// Flush pending events before exit (blocks briefly).
    pub fn flush_blocking(&self) {
        let client = self.inner.lock().ok().and_then(|g| g.as_ref().map(|a| a.client.clone()));
        if let Some(c) = client {
            crate::net::runtime().block_on(async move {
                let _ = tokio::time::timeout(Duration::from_secs(3), c.flush()).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_and_host_validation() {
        assert!(validate("A-EU-1234567890", None).is_ok());
        assert!(validate("A-DEV-0000000000", Some("http://127.0.0.1:9999")).is_ok());
        assert!(validate("A-SH-1234567890", None).is_err(), "self-hosted needs a host");
        assert!(validate("A-SH-1234567890", Some("https://aptabase.example.org")).is_ok());
        assert!(validate("A-XX-1234567890", None).is_err());
        assert!(validate("nonsense", None).is_err());
        assert!(validate("A-EU-1234567890", Some("not a url")).is_err());
        assert!(validate("A-EU-1234567890", Some("https://")).is_err());
    }

    #[test]
    fn disabling_without_a_client_is_fine() {
        let a = Analytics::default();
        assert!(a.set_enabled(false).is_ok());
        assert!(!a.enabled());
    }
}
