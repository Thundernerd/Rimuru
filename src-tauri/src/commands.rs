//! Commands for the main window. All are `async`: reading the forum window's
//! cookies from a synchronous command deadlocks on Windows.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::client::ForumClient;
use crate::error::Result;
use crate::forum_window;
use crate::phpbb::{self, ForumPage, Session, TopicPage};

#[tauri::command]
pub async fn forum_session(app: AppHandle, client: State<'_, ForumClient>) -> Result<Session> {
    let html = client.get(&app, "index.php").await?;
    Ok(phpbb::parse_session(&html))
}

/// Shows the forum window, on the login page by default.
#[tauri::command]
pub async fn show_forum_window(app: AppHandle, path: Option<String>) -> Result<()> {
    forum_window::show(&app, path.as_deref().unwrap_or("ucp.php?mode=login"))
}

#[tauri::command]
pub async fn hide_forum_window(app: AppHandle) -> Result<()> {
    forum_window::hide(&app)
}

#[tauri::command]
pub async fn fetch_forum(
    app: AppHandle,
    client: State<'_, ForumClient>,
    forum_id: u64,
    start: Option<u32>,
) -> Result<ForumPage> {
    let start = start.unwrap_or(0);
    let path = format!("viewforum.php?f={forum_id}&start={start}");
    let html = client.get(&app, &path).await?;
    Ok(phpbb::parse_forum(
        &html,
        &forum_window::forum_url(&path)?,
        forum_id,
        start,
    ))
}

#[tauri::command]
pub async fn fetch_topic(
    app: AppHandle,
    client: State<'_, ForumClient>,
    topic_id: u64,
    start: Option<u32>,
) -> Result<TopicPage> {
    let start = start.unwrap_or(0);
    let path = format!("viewtopic.php?t={topic_id}&start={start}");
    let html = client.get(&app, &path).await?;
    Ok(phpbb::parse_topic(
        &html,
        &forum_window::forum_url(&path)?,
        topic_id,
        start,
    ))
}

/// Saves the raw HTML of a forum page (e.g. `viewforum.php?f=10`) to the
/// app's data directory, for turning into parser test fixtures. Returns the
/// file path. The page contains the signed-in username, so check it before
/// committing.
#[tauri::command]
pub async fn save_snapshot(
    app: AppHandle,
    client: State<'_, ForumClient>,
    path: String,
) -> Result<PathBuf> {
    let html = client.get(&app, &path).await?;
    let dir = app.path().app_data_dir()?.join("snapshots");
    std::fs::create_dir_all(&dir)?;
    let name: String = path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let file = dir.join(format!("{name}.html"));
    std::fs::write(&file, html)?;
    Ok(file)
}
