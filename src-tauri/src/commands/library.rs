use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::unix_now;
use crate::cache::Cache;
use crate::error::Result;
use crate::game::Release;
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

/// Follows each topic not followed yet, as of now; returns the new Follows.
/// Topics already followed keep their Follow, `followed_at` included.
pub fn add_follows(cache: &Cache, topics: Vec<TopicSummary>) -> Vec<Follow> {
    let now = unix_now();
    topics
        .into_iter()
        .filter_map(|topic| {
            let key = topic.id.to_string();
            if cache
                .get::<Follow>("follow", &key, None, VISIT_MAX_AGE)
                .is_some()
            {
                return None;
            }
            let follow = Follow {
                topic,
                followed_at: now,
            };
            cache.put("follow", &key, None, &follow);
            Some(follow)
        })
        .collect()
}

#[tauri::command]
pub async fn followed_topics(cache: State<'_, Cache>) -> Result<Vec<Follow>> {
    let mut follows = cache.list::<Follow>("follow", VISIT_MAX_AGE);
    follows.sort_by_key(|f| std::cmp::Reverse(f.followed_at));
    Ok(follows)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Owned {
    pub topic_id: u64,
    pub post_id: u64,
    pub build: Option<String>,
    pub version: Option<String>,
    pub title: Option<String>,
    pub marked_at: u64,
}

#[tauri::command]
pub async fn mark_owned(cache: State<'_, Cache>, topic_id: u64, release: Release) -> Result<Owned> {
    let owned = Owned {
        topic_id,
        post_id: release.post_id,
        build: release.build,
        version: release.version,
        title: release.title,
        marked_at: unix_now(),
    };
    cache.put("owned", &topic_id.to_string(), None, &owned);
    Ok(owned)
}

#[tauri::command]
pub async fn clear_owned(cache: State<'_, Cache>, topic_id: u64) -> Result<()> {
    cache.delete("owned", &topic_id.to_string());
    Ok(())
}

#[tauri::command]
pub async fn owned_releases(cache: State<'_, Cache>) -> Result<Vec<Owned>> {
    Ok(cache.list::<Owned>("owned", VISIT_MAX_AGE))
}

#[tauri::command]
pub async fn unnotified(cache: State<'_, Cache>, post_ids: Vec<u64>) -> Result<Vec<u64>> {
    Ok(post_ids
        .into_iter()
        .filter(|id| {
            cache
                .get::<u64>("notified", &id.to_string(), None, VISIT_MAX_AGE)
                .is_none()
        })
        .collect())
}

#[tauri::command]
pub async fn mark_notified(cache: State<'_, Cache>, post_ids: Vec<u64>) -> Result<()> {
    for id in post_ids {
        cache.put("notified", &id.to_string(), None, &unix_now());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phpbb::TopicKind;

    fn topic(id: u64) -> TopicSummary {
        TopicSummary {
            id,
            kind: TopicKind::Normal,
            title: format!("Game {id}"),
            name: format!("Game {id}"),
            prefixes: vec![],
            labels: vec![],
            url: format!("https://cs.rin.ru/forum/viewtopic.php?t={id}"),
            author: None,
            created: None,
            replies: None,
            views: None,
            last_post_at: None,
            last_poster: None,
        }
    }

    #[test]
    fn add_follows_keeps_existing_follows() {
        let cache = Cache::in_memory();
        let old = Follow {
            topic: topic(1),
            followed_at: 5,
        };
        cache.put("follow", "1", None, &old);

        let added = add_follows(&cache, vec![topic(1), topic(2), topic(2)]);

        assert_eq!(added.iter().map(|f| f.topic.id).collect::<Vec<_>>(), [2]);
        let one: Follow = cache.get("follow", "1", None, VISIT_MAX_AGE).unwrap();
        assert_eq!(one, old);
        assert!(
            cache
                .get::<Follow>("follow", "2", None, VISIT_MAX_AGE)
                .is_some()
        );
    }
}
