mod cache;
mod client;
mod commands;
mod download;
mod error;
mod extract;
mod forum_window;
mod game;
mod phpbb;
mod steam;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(client::ForumClient::new().expect("failed to build HTTP client"))
        .manage(steam::SteamClient::new().expect("failed to build Steam client"))
        .manage(commands::download::DownloadClient(
            download::client().expect("failed to build download client"),
        ))
        .manage(commands::download::Downloads::default())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            migrate_data(&data_dir);
            let cache_path = data_dir.join("cache.sqlite");
            app.manage(cache::Cache::open(&cache_path));
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let removed = handle
                    .state::<cache::Cache>()
                    .prune(&commands::expiring_kinds(), commands::PERMANENT_KINDS);
                log::info!("pruned {removed} cache entries");
            });
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
            commands::library::clear_recent_visits,
            commands::library::follow_topic,
            commands::library::unfollow_topic,
            commands::library::followed_topics,
            commands::bookmarks::set_forum_bookmark,
            commands::bookmarks::forum_bookmarks,
            commands::bookmarks::import_forum_bookmarks,
            commands::library::mark_owned,
            commands::library::clear_owned,
            commands::library::owned_releases,
            commands::library::unnotified,
            commands::library::mark_notified,
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::clear_cache,
            commands::settings::sign_out,
            commands::forum::save_snapshot,
            commands::extract::extract_archive,
            commands::download::download_file,
            commands::download::cancel_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

const OLD_IDENTIFIER: &str = "com.rinru.desktop";

fn migrate_data(data_dir: &std::path::Path) {
    let Some(old_dir) = data_dir.parent().map(|p| p.join(OLD_IDENTIFIER)) else {
        return;
    };
    if data_dir.join("cache.sqlite").exists() || !old_dir.join("cache.sqlite").exists() {
        return;
    }
    if std::fs::create_dir_all(data_dir).is_err() {
        return;
    }
    for name in ["cache.sqlite", "cache.sqlite-wal", "cache.sqlite-shm"] {
        let from = old_dir.join(name);
        if from.exists()
            && let Err(e) = std::fs::copy(&from, data_dir.join(name))
        {
            log::warn!("couldn't copy {name} from the old data folder: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("rimuru-migrate-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(OLD_IDENTIFIER)).unwrap();
        root
    }

    #[test]
    fn copies_old_data_into_an_empty_folder() {
        let root = temp_root("copy");
        std::fs::write(root.join(OLD_IDENTIFIER).join("cache.sqlite"), b"old").unwrap();
        let new_dir = root.join("net.tnrd.rimuru");
        migrate_data(&new_dir);
        assert_eq!(std::fs::read(new_dir.join("cache.sqlite")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn never_overwrites_existing_data() {
        let root = temp_root("keep");
        std::fs::write(root.join(OLD_IDENTIFIER).join("cache.sqlite"), b"old").unwrap();
        let new_dir = root.join("net.tnrd.rimuru");
        std::fs::create_dir_all(&new_dir).unwrap();
        std::fs::write(new_dir.join("cache.sqlite"), b"new").unwrap();
        migrate_data(&new_dir);
        assert_eq!(std::fs::read(new_dir.join("cache.sqlite")).unwrap(), b"new");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn does_nothing_without_old_data() {
        let root = temp_root("none");
        let new_dir = root.join("net.tnrd.rimuru");
        migrate_data(&new_dir);
        assert!(!new_dir.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
