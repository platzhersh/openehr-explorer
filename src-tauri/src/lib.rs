mod commands;
pub mod credentials;
pub mod inspector;
pub mod settings;

use commands::{composition, ehr, query, server, template, terminology};
use tauri::{
    menu::{Menu, MenuItem, MenuItemKind, HELP_SUBMENU_ID},
    Emitter,
};
use tauri_plugin_aptabase::EventTracker;

/// Menu id for the manually-triggered "Check for Updates…" item added to the
/// native app menu (see `install_update_check_menu_item`). The frontend
/// listens for the `check-for-updates-requested` event emitted when this
/// item is clicked and runs the same update check as the Settings page
/// button / startup check.
const CHECK_FOR_UPDATES_MENU_ID: &str = "check_for_updates";

/// Adds a "Check for Updates…" item to the native application menu, since
/// Tauri's default menu (built by `Menu::default`) has no such item and the
/// updater plugin only exposes a JS API — there is no built-in menu entry to
/// trigger it. Clicking the item emits `check-for-updates-requested`, which
/// the frontend's update store listens for and handles the same way as the
/// Settings page button.
///
/// Platform placement:
/// - **macOS:** inserted into the app submenu (the one titled after the app
///   name), directly below "About", since that's where "Check for Updates…"
///   conventionally lives in native macOS apps.
/// - **Windows/Linux:** inserted into the "Help" submenu, above "About".
fn install_update_check_menu_item(app: &tauri::App) -> tauri::Result<()> {
    let handle = app.handle();
    let menu = Menu::default(handle)?;

    let check_for_updates = MenuItem::with_id(
        handle,
        CHECK_FOR_UPDATES_MENU_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;

    #[cfg(target_os = "macos")]
    {
        // The app submenu is always the first item in Tauri's default macOS
        // menu (see `Menu::default`); it holds About/Services/Hide/Quit.
        if let Some(MenuItemKind::Submenu(app_submenu)) = menu.items()?.into_iter().next() {
            app_submenu.insert(&check_for_updates, 1)?;
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(MenuItemKind::Submenu(help_submenu)) = menu.get(HELP_SUBMENU_ID) {
            help_submenu.insert(&check_for_updates, 0)?;
        }
    }

    app.set_menu(menu)?;
    Ok(())
}

/// The official website URL for openEHR Explorer
pub const WEBSITE_URL: &str = "https://platzhersh.github.io/openehr-explorer/";

/// Aptabase App Key injected at compile time via the `APTABASE_APP_KEY` env var.
///
/// When empty (e.g. local dev builds without the var set, or forks that have not
/// provisioned their own key) the Aptabase plugin gracefully no-ops: no events
/// are dispatched and nothing is sent over the network. The key is expected to
/// be provided in CI only — see `.github/workflows/ci.yml` — so malicious forks
/// cannot accidentally bake it into their binaries.
const APTABASE_APP_KEY: &str = match option_env!("APTABASE_APP_KEY") {
    Some(k) => k,
    None => "",
};

/// Get the application version from Cargo.toml
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Migrate any plaintext credentials from profiles.json to secure storage
    server::migrate_plaintext_credentials();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_aptabase::Builder::new(APTABASE_APP_KEY).build())
        .manage(terminology::TerminologyCache::default())
        .setup(|app| {
            install_update_check_menu_item(app)?;

            app.on_menu_event(|app_handle, event| {
                if event.id() == CHECK_FOR_UPDATES_MENU_ID {
                    let _ = app_handle.emit("check-for-updates-requested", ());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // App
            get_app_version,
            // Server
            server::list_server_profiles,
            server::save_server_profile,
            server::delete_server_profile,
            server::set_default_server_profile,
            server::test_server_connection,
            server::test_unsaved_connection,
            server::get_server_version,
            server::get_credential_backend,
            // EHR
            ehr::list_ehrs,
            ehr::get_ehr_detail,
            ehr::create_ehr,
            ehr::update_ehr_status,
            ehr::delete_ehr,
            ehr::search_ehrs,
            // Composition
            composition::get_composition,
            composition::get_composition_flat,
            composition::get_composition_versions,
            composition::create_composition,
            composition::update_composition,
            composition::delete_composition,
            // Template
            template::list_templates,
            template::get_web_template,
            template::get_template_opt,
            template::upload_template,
            template::get_template_example,
            template::get_term_bindings,
            // AQL Query
            query::execute_aql,
            query::list_saved_queries,
            query::save_query,
            query::delete_saved_query,
            // STORED_QUERY
            query::list_stored_queries,
            query::get_stored_query_definition,
            query::execute_stored_query,
            // Settings
            settings::get_config_dir,
            settings::get_settings,
            settings::save_settings,
            // Terminology
            terminology::lookup_code,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Flush any pending Aptabase events before shutdown so the
                // final buffered payload is not lost. Safe no-op when the
                // plugin was initialised without a key.
                app_handle.flush_events_blocking();
            }
        });
}
