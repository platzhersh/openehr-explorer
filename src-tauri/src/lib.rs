mod commands;

use commands::{composition, ehr, query, server, template};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            // Server
            server::list_server_profiles,
            server::save_server_profile,
            server::delete_server_profile,
            server::test_server_connection,
            // EHR
            ehr::list_ehrs,
            ehr::get_ehr_detail,
            ehr::create_ehr,
            ehr::update_ehr_status,
            ehr::delete_ehr,
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
            // AQL Query
            query::execute_aql,
            query::list_saved_queries,
            query::save_query,
            query::delete_saved_query,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
