//! Commands for the main window. All are `async`: reading the forum window's
//! cookies from a synchronous command deadlocks on Windows.

use std::collections::HashMap;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::client::ForumClient;
use crate::error::{Error, Result};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::forum_window;
use crate::game::{self, GameInfo, Release};
use crate::phpbb::{self, ForumPage, SearchPage, Session, TopicPage};
use crate::steam::{self, Artwork, SteamApp, SteamClient, SteamMatch};

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

/// Searches topic titles, like the forum's "Search this forum" box: in the
/// main game forum, or in all forums with `all_forums`. Not cached: results
/// change, and the forum limits how often a user may search.
#[tauri::command]
pub async fn search_topics(
    app: AppHandle,
    client: State<'_, ForumClient>,
    query: String,
    start: Option<u32>,
    all_forums: Option<bool>,
) -> Result<SearchPage> {
    let start = start.unwrap_or(0);
    let mut url = forum_window::forum_url("search.php")?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("keywords", query.trim());
        if !all_forums.unwrap_or(false) {
            q.append_pair("fid[]", &MAIN_FORUM_ID.to_string());
        }
        q.append_pair("sr", "topics")
            .append_pair("sf", "titleonly")
            .append_pair("start", &start.to_string());
    }
    let path = format!("search.php?{}", url.query().unwrap_or_default());
    let html = client.get(&app, &path).await?;
    phpbb::parse_search(&html, &url, query.trim(), start).map_err(Error::ForumMessage)
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
/// The forum's main game listing ("Main Forum").
const MAIN_FORUM_ID: u64 = 10;
const TOPIC_MAX_AGE: Duration = Duration::from_secs(24 * 3600);
/// Bumped when `SteamApp` gains fields (v2: trailers).
const STEAM_APP_CACHE: &str = "steam_app_v2";
const STEAM_APP_MAX_AGE: Duration = Duration::from_secs(3 * 24 * 3600);
const STEAM_MATCH_MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);
/// Bumped when `Artwork` gains fields, so older entries are fetched again.
const ARTWORK_CACHE: &str = "steam_artwork_v2";
const STEAM_ARTWORK_MAX_AGE: Duration = Duration::from_secs(14 * 24 * 3600);
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
    if let Some(hit) = cache.get::<Option<SteamApp>>(STEAM_APP_CACHE, &key, None, STEAM_APP_MAX_AGE)
    {
        return Ok(hit);
    }
    let details = steam.app_details(app_id).await?;
    cache.put(STEAM_APP_CACHE, &key, None, &details);
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

/// Card artwork for many apps at once, from cache where possible and one
/// Steam request per [`steam::ARTWORK_BATCH`] uncached apps. Apps Steam
/// has no artwork for come back without images.
#[tauri::command]
pub async fn steam_artwork(
    steam: State<'_, SteamClient>,
    cache: State<'_, Cache>,
    app_ids: Vec<u64>,
) -> Result<HashMap<u64, Artwork>> {
    let mut found = HashMap::new();
    let mut missing = Vec::new();
    for id in app_ids {
        match cache.get::<Artwork>(ARTWORK_CACHE, &id.to_string(), None, STEAM_ARTWORK_MAX_AGE) {
            Some(art) => {
                found.insert(id, art);
            }
            None => missing.push(id),
        }
    }
    for batch in missing.chunks(steam::ARTWORK_BATCH) {
        let mut fetched = steam.artwork(batch).await?;
        for &id in batch {
            let art = fetched.remove(&id).unwrap_or_default();
            cache.put(ARTWORK_CACHE, &id.to_string(), None, &art);
            found.insert(id, art);
        }
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

/// Kept for good: visits are the user's own history, not cached data.
const VISIT_MAX_AGE: Duration = Duration::from_secs(100 * 365 * 24 * 3600);

/// What was there when a game topic was last opened.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Visit {
    /// The topic's reply count at the time.
    pub replies: Option<u32>,
    /// Post ids of the releases seen.
    pub release_ids: Vec<u64>,
    /// Unix seconds.
    pub visited_at: u64,
}

/// Last visits for the given topics; unvisited topics are left out.
#[tauri::command]
pub async fn topic_visits(
    cache: State<'_, Cache>,
    topic_ids: Vec<u64>,
) -> Result<HashMap<u64, Visit>> {
    Ok(topic_ids
        .into_iter()
        .filter_map(|id| {
            Some((
                id,
                cache.get("visit", &id.to_string(), None, VISIT_MAX_AGE)?,
            ))
        })
        .collect())
}

/// Records a visit to a topic, keeping release ids seen on earlier visits
/// (a visit may not have read every page). Returns the new visit.
#[tauri::command]
pub async fn record_visit(
    cache: State<'_, Cache>,
    topic_id: u64,
    replies: Option<u32>,
    release_ids: Vec<u64>,
) -> Result<Visit> {
    let key = topic_id.to_string();
    let mut ids: Vec<u64> = cache
        .get::<Visit>("visit", &key, None, VISIT_MAX_AGE)
        .map(|v| v.release_ids)
        .unwrap_or_default();
    ids.extend(release_ids);
    ids.sort_unstable();
    ids.dedup();
    let visit = Visit {
        replies,
        release_ids: ids,
        visited_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    cache.put("visit", &key, None, &visit);
    Ok(visit)
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
