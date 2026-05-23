//! Aracdia Launcher — Tauri 2 entry point.

mod content;
mod download;
mod engine;
mod game;
mod launch;
mod paths;
mod profile;
mod server;
mod settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(engine::EngineLock::default())
        .manage(content::ContentLock::default())
        .manage(launch::LaunchState::default())
        .manage(server::ServerState::default())
        .setup(|app| {
            // Try to auto-start the local server in the background as soon as
            // the launcher boots. We don't block startup on it: if the engine
            // isn't installed yet, this will fail and the user will trigger
            // the install pipeline by clicking JOUER, which will retry.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match server::ensure_started(&handle).await {
                    Ok(info) => {
                        eprintln!(
                            "[launcher] local server up: pid={} bind={} port={}",
                            info.pid, info.bind, info.port
                        );
                    }
                    Err(err) => {
                        eprintln!(
                            "[launcher] could not auto-start local server: {err}. \
                             Will retry on JOUER once the engine is installed."
                        );
                    }
                }
            });
            Ok(())
        })
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
            content::content_status,
            content::fetch_content_release,
            content::install_content,
            launch::launch_engine,
            launch::stop_engine,
            launch::is_engine_running,
            launch::current_session,
            server::start_server,
            server::stop_server,
            server::server_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Aracdia launcher");
}
