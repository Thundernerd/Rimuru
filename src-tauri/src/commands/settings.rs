use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::PERMANENT_KINDS;
use super::library::VISIT_MAX_AGE;
use crate::cache::Cache;
use crate::error::Result;
use crate::forum_window;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub link_browser: Option<String>,
    pub refresh_minutes: u32,
    pub notifications: bool,
    pub dev_tools: bool,
    pub sync_bookmarks: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            link_browser: None,
            refresh_minutes: 15,
            notifications: true,
            dev_tools: false,
            sync_bookmarks: false,
        }
    }
}

#[tauri::command]
pub async fn get_settings(cache: State<'_, Cache>) -> Result<Settings> {
    Ok(cache
        .get("setting", "app", None, VISIT_MAX_AGE)
        .unwrap_or_default())
}

#[tauri::command]
pub async fn set_settings(cache: State<'_, Cache>, settings: Settings) -> Result<Settings> {
    let settings = Settings {
        link_browser: settings
            .link_browser
            .map(|b| b.trim().to_string())
            .filter(|b| !b.is_empty()),
        refresh_minutes: settings.refresh_minutes.clamp(1, 24 * 60),
        ..settings
    };
    cache.put("setting", "app", None, &settings);
    Ok(settings)
}

#[tauri::command]
pub async fn clear_cache(cache: State<'_, Cache>) -> Result<usize> {
    Ok(cache.prune(&[], PERMANENT_KINDS))
}

#[tauri::command]
pub async fn sign_out(app: AppHandle) -> Result<()> {
    forum_window::sign_out(&app)
}
