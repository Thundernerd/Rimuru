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
use crate::phpbb::{self, ForumPage, SearchPage, Session, TopicPage, TopicSummary};
use crate::steam::{self, Artwork, SteamApp, SteamClient, SteamMatch};

#[tauri::command]
pub async fn forum_session(app: AppHandle, client: State<'_, ForumClient>) -> Result<Session> {
    let html = client.get(&app, "index.php").await?;
    Ok(phpbb::parse_session(&html))
}

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

const MAIN_FORUM_ID: u64 = 10;
const TOPIC_MAX_AGE: Duration = Duration::from_secs(24 * 3600);
const STEAM_APP_CACHE: &str = "steam_app_v2";
const STEAM_APP_MAX_AGE: Duration = Duration::from_secs(3 * 24 * 3600);
const STEAM_MATCH_MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);
const ARTWORK_CACHE: &str = "steam_artwork_v2";
const STEAM_ARTWORK_MAX_AGE: Duration = Duration::from_secs(14 * 24 * 3600);
const STEAM_NO_MATCH_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameFirstPage {
    pub topic_id: u64,
    pub title: Option<String>,
    pub info: Option<GameInfo>,
    pub releases: Vec<Release>,
    pub total_pages: Option<u32>,
    pub per_page: Option<u32>,
    pub last_start: u32,
}

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
    if let Some(hit) = cache.get("topic_first_v2", &key, version.as_deref(), TOPIC_MAX_AGE) {
        return Ok(hit);
    }
    let page = topic_page(&app, &client, topic_id, 0).await?;
    let game = page.title.as_deref().map(phpbb::topic_name);
    let first = GameFirstPage {
        topic_id,
        title: page.title.clone(),
        info: page.posts.first().and_then(game::parse_game_info),
        releases: page
            .posts
            .iter()
            .rev()
            .filter_map(|p| game::parse_release(p, game.as_deref()))
            .collect(),
        total_pages: page.pagination.total_pages,
        per_page: page.pagination.next_start.filter(|&n| n > 0),
        last_start: page.pagination.last_start,
    };
    cache.put("topic_first_v2", &key, version.as_deref(), &first);
    Ok(first)
}

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
    if let Some(hit) = cache.get("topic_releases_v2", &key, version.as_deref(), TOPIC_MAX_AGE) {
        return Ok(hit);
    }
    let page = topic_page(&app, &client, topic_id, start).await?;
    let game = page.title.as_deref().map(phpbb::topic_name);
    let releases: Vec<Release> = page
        .posts
        .iter()
        .rev()
        .filter_map(|p| game::parse_release(p, game.as_deref()))
        .collect();
    cache.put("topic_releases_v2", &key, version.as_deref(), &releases);
    Ok(releases)
}

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

#[tauri::command]
pub async fn remember_steam_match(
    cache: State<'_, Cache>,
    name: String,
    found: SteamMatch,
) -> Result<()> {
    cache.put("steam_match", &steam::normalize_name(&name), None, &found);
    Ok(())
}

const VISIT_MAX_AGE: Duration = Duration::from_secs(100 * 365 * 24 * 3600);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Visit {
    pub replies: Option<u32>,
    pub release_ids: Vec<u64>,
    pub visited_at: u64,
    #[serde(default)]
    pub topic: Option<TopicSummary>,
}

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

#[tauri::command]
pub async fn record_visit(
    cache: State<'_, Cache>,
    topic: TopicSummary,
    release_ids: Vec<u64>,
) -> Result<Visit> {
    let key = topic.id.to_string();
    let mut ids: Vec<u64> = cache
        .get::<Visit>("visit", &key, None, VISIT_MAX_AGE)
        .map(|v| v.release_ids)
        .unwrap_or_default();
    ids.extend(release_ids);
    ids.sort_unstable();
    ids.dedup();
    let visit = Visit {
        replies: topic.replies,
        release_ids: ids,
        visited_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        topic: Some(topic),
    };
    cache.put("visit", &key, None, &visit);
    Ok(visit)
}

#[tauri::command]
pub async fn recent_visits(cache: State<'_, Cache>, limit: Option<usize>) -> Result<Vec<Visit>> {
    Ok(cache
        .list::<Visit>("visit", VISIT_MAX_AGE)
        .into_iter()
        .filter(|v| v.topic.is_some())
        .take(limit.unwrap_or(50))
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Follow {
    pub topic: TopicSummary,
    pub followed_at: u64,
}

#[tauri::command]
pub async fn follow_topic(cache: State<'_, Cache>, topic: TopicSummary) -> Result<Follow> {
    let key = topic.id.to_string();
    let followed_at = cache
        .get::<Follow>("follow", &key, None, VISIT_MAX_AGE)
        .map(|f| f.followed_at)
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });
    let follow = Follow { topic, followed_at };
    cache.put("follow", &key, None, &follow);
    Ok(follow)
}

#[tauri::command]
pub async fn unfollow_topic(cache: State<'_, Cache>, topic_id: u64) -> Result<()> {
    cache.delete("follow", &topic_id.to_string());
    Ok(())
}

#[tauri::command]
pub async fn followed_topics(cache: State<'_, Cache>) -> Result<Vec<Follow>> {
    let mut follows = cache.list::<Follow>("follow", VISIT_MAX_AGE);
    follows.sort_by_key(|f| std::cmp::Reverse(f.followed_at));
    Ok(follows)
}

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
