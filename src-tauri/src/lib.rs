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
            commands::forum::forum_session,
            commands::forum::show_forum_window,
            commands::forum::hide_forum_window,
            commands::forum::fetch_forum,
            commands::forum::search_topics,
            commands::forum::fetch_topic,
            commands::game::game_first_page,
            commands::game::game_releases,
            commands::steam::steam_app,
            commands::steam::steam_match,
            commands::steam::remember_steam_match,
            commands::steam::steam_artwork,
            commands::library::topic_visits,
            commands::library::record_visit,
            commands::library::recent_visits,
            commands::library::follow_topic,
            commands::library::unfollow_topic,
            commands::library::followed_topics,
            commands::forum::save_snapshot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
