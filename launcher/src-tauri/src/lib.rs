//! Aracdia Launcher — Tauri 2 entry point.

mod download;
mod engine;
mod game;
mod launch;
mod paths;
mod profile;
mod settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(engine::EngineLock::default())
        .manage(launch::LaunchState::default())
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
            launch::launch_engine,
            launch::stop_engine,
            launch::is_engine_running,
            launch::current_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Aracdia launcher");
}
