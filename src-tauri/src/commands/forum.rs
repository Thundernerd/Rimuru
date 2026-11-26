use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use super::MAIN_FORUM_ID;
use crate::client::ForumClient;
use crate::error::{Error, Result};
use crate::forum_window;
use crate::phpbb::{self, ForumPage, SearchPage, Session, TopicPage};

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
    sort: Option<String>,
) -> Result<ForumPage> {
    let start = start.unwrap_or(0);
    let order = match sort.as_deref() {
        Some("replies") => "&sk=r&sd=d",
        Some("views") => "&sk=v&sd=d",
        _ => "",
    };
    let path = format!("viewforum.php?f={forum_id}&start={start}{order}");
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

pub(super) async fn topic_page(
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
