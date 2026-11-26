//! Commands for the main window. All are `async`: reading the forum window's
//! cookies from a synchronous command deadlocks on Windows.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::client::ForumClient;
use crate::error::Result;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::forum_window;
use crate::game::{self, GameInfo, Release};
use crate::phpbb::{self, ForumPage, Session, TopicPage};
use crate::steam::{self, SteamApp, SteamClient, SteamMatch};

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

/// How long cached data stays usable. Forum entries are also keyed by the
/// topic's reply count, so new replies invalidate them sooner.
const TOPIC_MAX_AGE: Duration = Duration::from_secs(24 * 3600);
const STEAM_APP_MAX_AGE: Duration = Duration::from_secs(3 * 24 * 3600);
const STEAM_MATCH_MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);
const STEAM_NO_MATCH_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameFirstPage {
    pub topic_id: u64,
    pub title: Option<String>,
    /// From the topic's first post, when it holds the game's details.
    pub info: Option<GameInfo>,
    /// Releases on page 1, newest first.
    pub releases: Vec<Release>,
    pub total_pages: Option<u32>,
    /// Posts per page, when the topic has more than one page.
    pub per_page: Option<u32>,
    /// `start=` of the topic's last page.
    pub last_start: u32,
}

/// Page 1 of a game topic: the game's details and the first releases.
/// `replies` (from the forum listing) keys the cache: a topic with new
/// replies is fetched again.
#[tauri::command]
pub async fn game_first_page(
    app: AppHandle,
    client: State<'_, ForumClient>,
    cache: State<'_, Cache>,
    topic_id: u64,
    replies: Option<u32>,
) -> Result<GameFirstPage> {
    let key = topic_id.to_string();
    let version = replies.map(|r| r.to_string());
    if let Some(hit) = cache.get("topic_first", &key, version.as_deref(), TOPIC_MAX_AGE) {
        return Ok(hit);
    }
    let page = topic_page(&app, &client, topic_id, 0).await?;
    let first = GameFirstPage {
        topic_id,
        title: page.title.clone(),
        info: page.posts.first().and_then(game::parse_game_info),
        releases: page
            .posts
            .iter()
            .rev()
            .filter_map(game::parse_release)
            .collect(),
        total_pages: page.pagination.total_pages,
        per_page: page.pagination.next_start.filter(|&n| n > 0),
        last_start: page.pagination.last_start,
    };
    cache.put("topic_first", &key, version.as_deref(), &first);
    Ok(first)
}

/// Releases on one page of a topic, newest first. Cached like
/// [`game_first_page`].
#[tauri::command]
pub async fn game_releases(
    app: AppHandle,
    client: State<'_, ForumClient>,
    cache: State<'_, Cache>,
    topic_id: u64,
    start: u32,
    replies: Option<u32>,
) -> Result<Vec<Release>> {
    let key = format!("{topic_id}:{start}");
    let version = replies.map(|r| r.to_string());
    if let Some(hit) = cache.get("topic_releases", &key, version.as_deref(), TOPIC_MAX_AGE) {
        return Ok(hit);
    }
    let page = topic_page(&app, &client, topic_id, start).await?;
    let releases: Vec<Release> = page
        .posts
        .iter()
        .rev()
        .filter_map(game::parse_release)
        .collect();
    cache.put("topic_releases", &key, version.as_deref(), &releases);
    Ok(releases)
}

/// Steam store details for an app; `None` if Steam doesn't have it.
#[tauri::command]
pub async fn steam_app(
    steam: State<'_, SteamClient>,
    cache: State<'_, Cache>,
    app_id: u64,
) -> Result<Option<SteamApp>> {
    let key = app_id.to_string();
    if let Some(hit) = cache.get::<Option<SteamApp>>("steam_app", &key, None, STEAM_APP_MAX_AGE) {
        return Ok(hit);
    }
    let details = steam.app_details(app_id).await?;
    cache.put("steam_app", &key, None, &details);
    Ok(details)
}

/// The Steam app a game name matches, for card images. Misses are cached
/// too (for less long), so unmatched names aren't searched over and over.
#[tauri::command]
pub async fn steam_match(
    steam: State<'_, SteamClient>,
    cache: State<'_, Cache>,
    name: String,
) -> Result<Option<SteamMatch>> {
    let key = steam::normalize_name(&name);
    if let Some(hit) = cache.get::<SteamMatch>("steam_match", &key, None, STEAM_MATCH_MAX_AGE) {
        return Ok(Some(hit));
    }
    if cache
        .get::<()>("steam_no_match", &key, None, STEAM_NO_MATCH_MAX_AGE)
        .is_some()
    {
        return Ok(None);
    }
    let found = steam.find(&name).await?;
    match &found {
        Some(m) => cache.put("steam_match", &key, None, m),
        None => cache.put("steam_no_match", &key, None, &()),
    }
    Ok(found)
}

/// Records the Steam app a game name belongs to, overriding a name-search
/// result. Used when a topic's first post links its Steam store page,
/// which is more reliable than matching by name.
#[tauri::command]
pub async fn remember_steam_match(
    cache: State<'_, Cache>,
    name: String,
    found: SteamMatch,
) -> Result<()> {
    cache.put("steam_match", &steam::normalize_name(&name), None, &found);
    Ok(())
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
