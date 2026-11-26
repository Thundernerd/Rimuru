use std::collections::HashMap;
use std::time::Duration;

use tauri::State;

use crate::cache::Cache;
use crate::error::Result;
use crate::steam::{self, Artwork, SteamApp, SteamClient, SteamMatch};

pub const STEAM_APP_CACHE: &str = "steam_app_v2";
pub const STEAM_APP_MAX_AGE: Duration = Duration::from_secs(3 * 24 * 3600);
pub const STEAM_MATCH_CACHE: &str = "steam_match";
pub const STEAM_MATCH_MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);
pub const STEAM_NO_MATCH_CACHE: &str = "steam_no_match";
pub const STEAM_NO_MATCH_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);
pub const ARTWORK_CACHE: &str = "steam_artwork_v2";
pub const STEAM_ARTWORK_MAX_AGE: Duration = Duration::from_secs(14 * 24 * 3600);

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
    if let Some(hit) = cache.get::<SteamMatch>(STEAM_MATCH_CACHE, &key, None, STEAM_MATCH_MAX_AGE) {
        return Ok(Some(hit));
    }
    if cache
        .get::<()>(STEAM_NO_MATCH_CACHE, &key, None, STEAM_NO_MATCH_MAX_AGE)
        .is_some()
    {
        return Ok(None);
    }
    let found = steam.find(&name).await?;
    match &found {
        Some(m) => cache.put(STEAM_MATCH_CACHE, &key, None, m),
        None => cache.put(STEAM_NO_MATCH_CACHE, &key, None, &()),
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
    cache.put(
        STEAM_MATCH_CACHE,
        &steam::normalize_name(&name),
        None,
        &found,
    );
    Ok(())
}
