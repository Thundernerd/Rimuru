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
    pub extract_dir: Option<String>,
    pub jdownloader: bool,
    pub jdownloader_port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            link_browser: None,
            refresh_minutes: 15,
            notifications: true,
            dev_tools: false,
            sync_bookmarks: false,
            extract_dir: None,
            jdownloader: false,
            jdownloader_port: 3128,
        }
    }
}

pub fn load(cache: &Cache) -> Settings {
    cache
        .get("setting", "app", None, VISIT_MAX_AGE)
        .unwrap_or_default()
}

#[tauri::command]
pub async fn get_settings(cache: State<'_, Cache>) -> Result<Settings> {
    Ok(load(&cache))
}

#[tauri::command]
pub async fn set_settings(cache: State<'_, Cache>, settings: Settings) -> Result<Settings> {
    let settings = Settings {
        link_browser: settings
            .link_browser
            .map(|b| b.trim().to_string())
            .filter(|b| !b.is_empty()),
        extract_dir: settings
            .extract_dir
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty()),
        refresh_minutes: settings.refresh_minutes.clamp(1, 24 * 60),
        jdownloader_port: settings.jdownloader_port.max(1),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_dir_defaults_to_none_and_round_trips() {
        let old: Settings = serde_json::from_str(r#"{"refresh_minutes": 30}"#).unwrap();
        assert_eq!(old.extract_dir, None);
        assert_eq!(old.refresh_minutes, 30);

        let settings = Settings {
            extract_dir: Some("/games/extracted".into()),
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), settings);
    }

    #[test]
    fn jdownloader_is_off_on_port_3128_by_default_and_round_trips() {
        let old: Settings = serde_json::from_str(r#"{"refresh_minutes": 30}"#).unwrap();
        assert!(!old.jdownloader);
        assert_eq!(old.jdownloader_port, 3128);

        let settings = Settings {
            jdownloader: true,
            jdownloader_port: 4000,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), settings);
    }
}
