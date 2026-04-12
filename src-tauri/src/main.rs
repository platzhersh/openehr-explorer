// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `tauri-plugin-aptabase` calls raw `tokio::spawn` from its `.setup()`
    // hook to start a background flush loop. Raw `tokio::spawn` panics with
    // "there is no reactor running, must be called from the context of a
    // Tokio 1.x runtime" unless a Tokio runtime is currently entered on
    // this thread.
    //
    // We avoid `#[tokio::main]` on purpose here: it would `block_on` inside
    // the main thread, turning the Tauri UI event loop into a blocking
    // Tokio worker. Instead we build a multi-thread runtime on the side and
    // grab an `EnterGuard`, which sets the thread-local runtime context so
    // `tokio::spawn` resolves without hijacking the main thread. The guard
    // (and the runtime) live for the entire Tauri app lifetime — dropped
    // implicitly when `run()` returns on shutdown.
    let rt = tokio::runtime::Runtime::new().expect("failed to start Tokio runtime");
    let _guard = rt.enter();

    openehr_explorer_lib::run()
}
