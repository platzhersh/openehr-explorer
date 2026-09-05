mod commands;
pub mod credentials;
pub mod inspector;
pub mod settings;

use commands::{composition, contribution, dashboard, ehr, query, server, template, terminology};
#[cfg(not(target_os = "macos"))]
use tauri::menu::HELP_SUBMENU_ID;
use tauri::{
    image::Image,
    menu::{AboutMetadata, AboutMetadataBuilder, Menu, MenuItem, MenuItemKind, PredefinedMenuItem},
    Emitter,
};
use tauri_plugin_aptabase::EventTracker;

/// Display name shown in the native "About" dialog/panel, matching
/// `productName` in `tauri.conf.json`.
const APP_NAME: &str = "openEHR Explorer";

/// Menu id for the manually-triggered "Check for Updates…" item added to the
/// native app menu (see `install_update_check_menu_item`). The frontend
/// listens for the `check-for-updates-requested` event emitted when this
/// item is clicked and runs the same update check as the Settings page
/// button / startup check.
const CHECK_FOR_UPDATES_MENU_ID: &str = "check_for_updates";

/// Builds the metadata shown in the native "About" dialog/panel.
///
/// `Menu::default` wires up a "Check for Updates…" item and a predefined
/// About item, but that default About item has no icon of its own — the OS
/// falls back to a generic placeholder (e.g. a blank folder on macOS for a
/// dev build that isn't a signed `.app` bundle). Setting `icon` explicitly
/// here makes the app's own logo show up in the About dialog on every
/// platform, dev and release builds alike.
fn about_metadata() -> AboutMetadata<'static> {
    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))
        .expect("bundled icons/icon.png must be a valid PNG");

    AboutMetadataBuilder::new()
        .name(Some(APP_NAME))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .website(Some(WEBSITE_URL))
        .website_label(Some(WEBSITE_URL))
        .icon(Some(icon))
        .build()
}

/// Replaces the native menu's default "About" item with one carrying our own
/// metadata (see `about_metadata`), and adds a "Check for Updates…" item,
/// since the updater plugin only exposes a JS API — there is no built-in
/// menu entry to trigger it. Clicking the item emits
/// `check-for-updates-requested`, which the frontend's update store listens
/// for and handles the same way as the Settings page button.
///
/// Platform placement:
/// - **macOS:** inserted into the app submenu (the one titled after the app
///   name), directly below "About", since that's where "Check for Updates…"
///   conventionally lives in native macOS apps.
/// - **Windows/Linux:** inserted into the "Help" submenu, above "About".
fn install_update_check_menu_item(app: &tauri::App) -> tauri::Result<()> {
    let handle = app.handle();
    let menu = Menu::default(handle)?;
    // `PredefinedMenuItem::about`'s `text` becomes the menu item's label
    // verbatim — it does not prepend "About" the way `Menu::default`'s own
    // item does, so that has to be spelled out here.
    let about = PredefinedMenuItem::about(
        handle,
        Some(&format!("About {APP_NAME}")),
        Some(about_metadata()),
    )?;

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
        // menu (see `Menu::default`); it holds About/Services/Hide/Quit, with
        // the default About item first.
        if let Some(MenuItemKind::Submenu(app_submenu)) = menu.items()?.into_iter().next() {
            if let Some(MenuItemKind::Predefined(default_about)) =
                app_submenu.items()?.into_iter().next()
            {
                app_submenu.remove(&default_about)?;
            }
            app_submenu.insert(&about, 0)?;
            app_submenu.insert(&check_for_updates, 1)?;
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(MenuItemKind::Submenu(help_submenu)) = menu.get(HELP_SUBMENU_ID) {
            if let Some(MenuItemKind::Predefined(default_about)) =
                help_submenu.items()?.into_iter().next()
            {
                help_submenu.remove(&default_about)?;
            }
            help_submenu.insert(&check_for_updates, 0)?;
            help_submenu.insert(&about, 1)?;
        }
    }

    app.set_menu(menu)?;
    Ok(())
}

/// The official website URL for openEHR Explorer
pub const WEBSITE_URL: &str = "https://openehr-explorer.dev/";

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
            ehr::get_directory,
            ehr::get_directory_version,
            ehr::create_directory,
            ehr::update_directory,
            ehr::delete_directory,
            ehr::get_directory_versions,
            ehr::get_directory_version_contribution,
            ehr::get_ehr_status,
            ehr::get_ehr_status_version,
            ehr::get_ehr_status_versions,
            ehr::get_ehr_status_version_contribution,
            // Composition
            composition::get_composition,
            composition::get_composition_flat,
            composition::get_composition_versions,
            composition::get_composition_version_contribution,
            composition::create_composition,
            composition::update_composition,
            composition::delete_composition,
            // Contribution
            contribution::get_contribution,
            // Dashboard
            dashboard::get_dashboard_counts,
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
            terminology::describe_code,
            terminology::expand_valueset,
            terminology::validate_code,
            terminology::test_subsumption,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `about_metadata` decodes the bundled PNG into an `Image` at runtime
    /// (via the `image-png` feature); this guards against the icon file
    /// going missing or becoming unreadable without surfacing as a panic
    /// only when a user opens the About dialog.
    #[test]
    fn about_metadata_decodes_icon() {
        let metadata = about_metadata();
        assert!(metadata.icon.is_some());
        assert_eq!(metadata.name.as_deref(), Some(APP_NAME));
    }
}
