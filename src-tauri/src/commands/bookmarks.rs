//! Mirrors follows to the user's phpBB bookmarks on cs.rin.ru. The local follow
//! store stays the source of truth; bookmarks only hold which topics are followed.

use tauri::{AppHandle, State};

use super::library::{self, Follow};
use crate::cache::Cache;
use crate::client::ForumClient;
use crate::error::{Error, Result};
use crate::forum_window;
use crate::phpbb::{self, Session, TopicSummary};

/// Stops reading the bookmarks list after this many pages.
const MAX_BOOKMARK_PAGES: usize = 20;

fn signed_out() -> Error {
    Error::ForumMessage("sign in to the forum to sync bookmarks".into())
}

/// Bookmarks the topic on the forum, or removes its bookmark, unless it is
/// already so. phpBB's bookmark link toggles, so the topic page is read first.
#[tauri::command]
pub async fn set_forum_bookmark(
    app: AppHandle,
    client: State<'_, ForumClient>,
    topic_id: u64,
    bookmarked: bool,
) -> Result<bool> {
    let path = format!("viewtopic.php?t={topic_id}");
    let html = client.get(&app, &path).await?;
    let link = phpbb::parse_bookmark_link(&html, &forum_window::forum_url(&path)?)
        .ok_or_else(signed_out)?;
    if link.bookmarked == bookmarked {
        return Ok(bookmarked);
    }
    if !link.url.as_str().starts_with(forum_window::BASE_URL) {
        return Err(Error::ForumMessage(
            "the bookmark link points off the forum".into(),
        ));
    }
    client.get(&app, link.url.as_str()).await?;
    Ok(bookmarked)
}

/// The topics bookmarked on the forum account, read from the User Control Panel.
#[tauri::command]
pub async fn forum_bookmarks(
    app: AppHandle,
    client: State<'_, ForumClient>,
) -> Result<Vec<TopicSummary>> {
    read_bookmarks(&app, &client).await
}

/// Follows every topic bookmarked on the forum that isn't followed yet.
#[tauri::command]
pub async fn import_forum_bookmarks(
    app: AppHandle,
    client: State<'_, ForumClient>,
    cache: State<'_, Cache>,
) -> Result<Vec<Follow>> {
    let topics = read_bookmarks(&app, &client).await?;
    Ok(library::add_follows(&cache, topics))
}

async fn read_bookmarks(app: &AppHandle, client: &ForumClient) -> Result<Vec<TopicSummary>> {
    let mut topics: Vec<TopicSummary> = Vec::new();
    let mut start = 0;
    for _ in 0..MAX_BOOKMARK_PAGES {
        let path = format!("ucp.php?i=main&mode=bookmarks&start={start}");
        let html = client.get(app, &path).await?;
        if phpbb::parse_session(&html) == Session::LoggedOut {
            return Err(signed_out());
        }
        let page = phpbb::parse_bookmarks(&html, &forum_window::forum_url(&path)?, start);
        for topic in page.topics {
            if !topics.iter().any(|t| t.id == topic.id) {
                topics.push(topic);
            }
        }
        match page.pagination.next_start {
            Some(next) if next > start => start = next,
            _ => break,
        }
    }
    Ok(topics)
}
