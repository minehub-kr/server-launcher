mod backup;
mod config;
mod java;
mod models;
mod plugins;
mod runtime;
mod settings;
mod system;
mod versions;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(models::AppState::default())
        .invoke_handler(tauri::generate_handler![
            versions::list_server_versions,
            settings::list_profiles,
            settings::create_profile,
            settings::update_profile,
            settings::delete_profile,
            settings::choose_server_directory,
            java::scan_java_versions,
            versions::resolve_server_plan,
            config::read_server_config,
            config::save_server_config,
            config::lookup_minecraft_profile,
            config::read_access_lists,
            config::save_access_lists,
            runtime::start_server,
            runtime::stop_server,
            runtime::send_server_command,
            runtime::server_status,
            plugins::search_modrinth,
            plugins::install_modrinth_plugin,
            plugins::list_plugins,
            plugins::set_plugin_enabled,
            backup::create_backup,
            system::open_server_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
