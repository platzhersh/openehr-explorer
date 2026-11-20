//! Opt-in, anonymous usage analytics via the community `aptabase-rs` SDK (the official plugin is Tauri-only).
//!
//! **Off by default**; active only when the user opts in (Settings, or `--analytics`) AND an App Key is present. Can be
//! switched on and off at runtime. Never send query text, EHR/composition ids, server URLs or credentials: only
//! coarse event names with a few non-identifying properties. Unlike the Tauri app this sends no unconditional
//! `session_started` ping: nothing at all is sent until the user opts in.

use aptabase_rs::{AptabaseClient, Builder, InitOptions};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct Analytics {
    inner: Arc<Mutex<Option<Arc<AptabaseClient>>>>,
    host: Option<String>,
}

/// Compile-time key like the Tauri app; the runtime env var exists for local testing only.
fn app_key() -> Option<String> {
    option_env!("APTABASE_APP_KEY").map(str::to_string).or_else(|| std::env::var("APTABASE_APP_KEY").ok()).filter(|k| !k.is_empty())
}

impl Analytics {
    /// `host` is only needed for self-hosted keys (`A-SH-…`) and for pointing tests at a local endpoint.
    pub fn init(opted_in: bool, host: Option<String>) -> Self {
        let a = Self { inner: Arc::default(), host };
        a.set_enabled(opted_in);
        a
    }

    /// Whether this build has an App Key at all (without one, opting in sends nothing).
    pub fn has_key() -> bool {
        app_key().is_some()
    }

    pub fn enabled(&self) -> bool {
        self.inner.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    /// Switch collection on or off immediately. Turning it off flushes what is pending, then drops the client.
    pub fn set_enabled(&self, on: bool) {
        let Ok(mut guard) = self.inner.lock() else { return };
        if on {
            if guard.is_none() {
                if let Some(key) = app_key() {
                    // The client spawns a polling task, so it must be built inside the Tokio runtime.
                    let _rt = crate::net::runtime().enter();
                    let client = Builder::new(&key, env!("CARGO_PKG_VERSION"))
                        .with_options(InitOptions { host: self.host.clone(), flush_interval: None })
                        .with_polling(true)
                        .build();
                    *guard = Some(client);
                }
            }
        } else if let Some(client) = guard.take() {
            flush(client);
        }
    }

    pub fn track(&self, name: &str, props: Option<Value>) {
        if let Ok(guard) = self.inner.lock() {
            if let Some(c) = guard.as_ref() {
                let _ = c.track_event(name, props);
            }
        }
    }

    /// Flush pending events before exit (blocks briefly).
    pub fn flush_blocking(&self) {
        let client = self.inner.lock().ok().and_then(|g| g.clone());
        if let Some(c) = client {
            flush(c);
        }
    }
}

fn flush(c: Arc<AptabaseClient>) {
    crate::net::runtime().block_on(async move {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(3), c.flush()).await;
    });
}
