//! Aracdia Launcher — Tauri 2 entry point.

mod download;
mod engine;
mod paths;
mod profile;
mod settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(engine::EngineLock::default())
        .invoke_handler(tauri::generate_handler![
            profile::load_profile,
            profile::save_profile,
            profile::clear_profile,
            settings::load_settings,
            settings::save_settings,
            settings::reset_settings,
            engine::engine_status,
            engine::engine_current_target,
            engine::fetch_engine_release,
            engine::install_engine,
            engine::uninstall_engine,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Aracdia launcher");
}
