//! Opt-in, anonymous usage analytics via the community `aptabase-rs` SDK (the official plugin is Tauri-only).
//!
//! Same policy as the Tauri app: **off by default**; only active when the user opts in (`--analytics`) AND an
//! App Key is present. Never send query text, EHR/composition ids, server URLs or credentials — only coarse
//! event names with a few non-identifying properties.

use aptabase_rs::{AptabaseClient, Builder, InitOptions};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct Analytics(Option<Arc<AptabaseClient>>);

impl Analytics {
    /// `host` is only needed for self-hosted keys (`A-SH-…`) and for pointing tests at a local endpoint.
    pub fn init(opted_in: bool, host: Option<String>) -> Self {
        if !opted_in {
            return Self(None);
        }
        // Compile-time key like the Tauri app; the runtime env var exists for local testing only.
        let key = option_env!("APTABASE_APP_KEY").map(str::to_string).or_else(|| std::env::var("APTABASE_APP_KEY").ok());
        let Some(key) = key.filter(|k| !k.is_empty()) else {
            return Self(None);
        };
        // The client spawns a polling task, so it must be built inside the Tokio runtime.
        let _guard = crate::net::runtime().enter();
        let client = Builder::new(&key, env!("CARGO_PKG_VERSION"))
            .with_options(InitOptions { host, flush_interval: None })
            .with_polling(true)
            .build();
        Self(Some(client))
    }

    pub fn enabled(&self) -> bool {
        self.0.is_some()
    }

    pub fn track(&self, name: &str, props: Option<Value>) {
        if let Some(c) = &self.0 {
            let _ = c.track_event(name, props);
        }
    }

    /// Flush pending events before exit (blocks briefly).
    pub fn flush_blocking(&self) {
        if let Some(c) = &self.0 {
            let c = c.clone();
            crate::net::runtime().block_on(async move {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(3), c.flush()).await;
            });
        }
    }
}
