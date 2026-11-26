use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::unix_now;
use crate::cache::Cache;
use crate::error::Result;
use crate::phpbb::TopicSummary;

pub const VISIT_MAX_AGE: Duration = Duration::from_secs(100 * 365 * 24 * 3600);

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
        visited_at: unix_now(),
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
        .unwrap_or_else(unix_now);
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
