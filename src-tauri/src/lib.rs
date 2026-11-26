mod cache;
mod client;
mod commands;
mod error;
mod forum_window;
mod game;
mod phpbb;
mod steam;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(client::ForumClient::new().expect("failed to build HTTP client"))
        .manage(steam::SteamClient::new().expect("failed to build Steam client"))
        .setup(|app| {
            let cache_path = app.path().app_data_dir()?.join("cache.sqlite");
            app.manage(cache::Cache::open(&cache_path));
            forum_window::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            (forum_window::LABEL, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            ("main", WindowEvent::Destroyed) => window.app_handle().exit(0),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::forum_session,
            commands::show_forum_window,
            commands::hide_forum_window,
            commands::fetch_forum,
            commands::search_topics,
            commands::fetch_topic,
            commands::game_first_page,
            commands::game_releases,
            commands::steam_app,
            commands::steam_match,
            commands::remember_steam_match,
            commands::steam_artwork,
            commands::topic_visits,
            commands::record_visit,
            commands::recent_visits,
            commands::follow_topic,
            commands::unfollow_topic,
            commands::followed_topics,
            commands::save_snapshot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
