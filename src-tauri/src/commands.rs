//! Commands for the main window. All are `async`: reading the forum window's
//! cookies from a synchronous command deadlocks on Windows.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::client::ForumClient;
use crate::error::Result;
use serde::Serialize;

use crate::forum_window;
use crate::game::{self, GameInfo, Release};
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

/// Lists a page of a forum. Announcements and stickies repeat on every page,
/// so they are left out unless `include_pinned` is true.
#[tauri::command]
pub async fn fetch_forum(
    app: AppHandle,
    client: State<'_, ForumClient>,
    forum_id: u64,
    start: Option<u32>,
    include_pinned: Option<bool>,
) -> Result<ForumPage> {
    let start = start.unwrap_or(0);
    let path = format!("viewforum.php?f={forum_id}&start={start}");
    let html = client.get(&app, &path).await?;
    let mut page = phpbb::parse_forum(&html, &forum_window::forum_url(&path)?, forum_id, start);
    if !include_pinned.unwrap_or(false) {
        page.topics.retain(|t| !t.kind.is_pinned());
    }
    Ok(page)
}

#[tauri::command]
pub async fn fetch_topic(
    app: AppHandle,
    client: State<'_, ForumClient>,
    topic_id: u64,
    start: Option<u32>,
) -> Result<TopicPage> {
    topic_page(&app, &client, topic_id, start.unwrap_or(0)).await
}

async fn topic_page(
    app: &AppHandle,
    client: &ForumClient,
    topic_id: u64,
    start: u32,
) -> Result<TopicPage> {
    let path = format!("viewtopic.php?t={topic_id}&start={start}");
    let html = client.get(app, &path).await?;
    let url = forum_window::forum_url(&path)?;
    Ok(phpbb::parse_topic(&html, &url, topic_id, start))
}

#[derive(Debug, Serialize)]
pub struct GameDetail {
    pub topic_id: u64,
    pub title: Option<String>,
    /// From the topic's first post, when it holds the game's details.
    pub info: Option<GameInfo>,
    /// Newest first.
    pub releases: Vec<Release>,
    /// `start=` offsets of the topic pages that were read.
    pub pages_read: Vec<u32>,
    pub total_pages: Option<u32>,
}

/// A game topic's details and releases. Reads page 1 for the game info,
/// then up to `max_pages - 1` pages from the end (default 5 pages in all),
/// since new releases are posted at the end of long topics. Pages are
/// fetched one after another to keep the load on the forum low.
#[tauri::command]
pub async fn fetch_game(
    app: AppHandle,
    client: State<'_, ForumClient>,
    topic_id: u64,
    max_pages: Option<u32>,
) -> Result<GameDetail> {
    let max_pages = max_pages.unwrap_or(5).max(1) as usize;
    let first = topic_page(&app, &client, topic_id, 0).await?;
    let info = first.posts.first().and_then(game::parse_game_info);
    let title = first.title.clone();
    let total_pages = first.pagination.total_pages;

    let mut pages = Vec::new();
    // Page 1 links to page 2, which gives the page size.
    if let Some(per_page) = first.pagination.next_start.filter(|&n| n > 0) {
        let mut start = first.pagination.last_start;
        while start > 0 && pages.len() + 1 < max_pages {
            pages.push(topic_page(&app, &client, topic_id, start).await?);
            start = start.saturating_sub(per_page);
        }
    }
    pages.push(first);
    // `pages` runs from the last page back to page 1; read each newest first.
    let releases = pages
        .iter()
        .flat_map(|page| page.posts.iter().rev())
        .filter_map(game::parse_release)
        .collect();
    let mut pages_read: Vec<u32> = pages.iter().map(|p| p.pagination.start).collect();
    pages_read.sort_unstable();

    Ok(GameDetail {
        topic_id,
        title,
        info,
        releases,
        pages_read,
        total_pages,
    })
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
