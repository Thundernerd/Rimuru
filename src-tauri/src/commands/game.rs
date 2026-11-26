use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::forum::topic_page;
use crate::cache::Cache;
use crate::client::ForumClient;
use crate::error::Result;
use crate::game::{self, GameInfo, Release};
use crate::phpbb;

pub const TOPIC_FIRST_CACHE: &str = "topic_first_v2";
pub const TOPIC_RELEASES_CACHE: &str = "topic_releases_v2";
pub const TOPIC_MAX_AGE: Duration = Duration::from_secs(24 * 3600);

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
    if let Some(hit) = cache.get(TOPIC_FIRST_CACHE, &key, version.as_deref(), TOPIC_MAX_AGE) {
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
    cache.put(TOPIC_FIRST_CACHE, &key, version.as_deref(), &first);
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
    if let Some(hit) = cache.get(
        TOPIC_RELEASES_CACHE,
        &key,
        version.as_deref(),
        TOPIC_MAX_AGE,
    ) {
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
    cache.put(TOPIC_RELEASES_CACHE, &key, version.as_deref(), &releases);
    Ok(releases)
}
