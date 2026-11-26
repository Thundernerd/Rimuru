mod client;
mod commands;
mod error;
mod forum_window;
mod phpbb;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(client::ForumClient::new().expect("failed to build HTTP client"))
        .setup(|app| {
            forum_window::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // Closing the forum window only hides it; the session lives there.
            (forum_window::LABEL, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            // The hidden forum window would otherwise keep the app running.
            ("main", WindowEvent::Destroyed) => window.app_handle().exit(0),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::forum_session,
            commands::show_forum_window,
            commands::hide_forum_window,
            commands::fetch_forum,
            commands::fetch_topic,
            commands::save_snapshot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
