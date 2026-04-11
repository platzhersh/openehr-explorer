mod commands;
pub mod credentials;
pub mod inspector;
pub mod settings;

use commands::{composition, ehr, query, server, template, terminology};

/// The official website URL for openEHR Explorer
pub const WEBSITE_URL: &str = "https://platzhersh.github.io/openehr-explorer/";

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
        .manage(terminology::TerminologyCache::default())
        .invoke_handler(tauri::generate_handler![
            // App
            get_app_version,
            // Server
            server::list_server_profiles,
            server::save_server_profile,
            server::delete_server_profile,
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
            // Settings
            settings::get_config_dir,
            settings::get_settings,
            settings::save_settings,
            // Terminology
            terminology::lookup_code,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
