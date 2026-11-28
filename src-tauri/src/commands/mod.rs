pub mod bookmarks;
pub mod extract;
pub mod forum;
pub mod game;
pub mod library;
pub mod settings;
pub mod steam;

use std::time::Duration;

pub const MAIN_FORUM_ID: u64 = 10;

pub const PERMANENT_KINDS: &[&str] = &["visit", "follow", "owned", "setting", "notified"];

pub fn expiring_kinds() -> Vec<(&'static str, Duration)> {
    vec![
        (game::TOPIC_FIRST_CACHE, game::TOPIC_MAX_AGE),
        (game::TOPIC_RELEASES_CACHE, game::TOPIC_MAX_AGE),
        (steam::STEAM_APP_CACHE, steam::STEAM_APP_MAX_AGE),
        (steam::STEAM_MATCH_CACHE, steam::STEAM_MATCH_MAX_AGE),
        (steam::STEAM_NO_MATCH_CACHE, steam::STEAM_NO_MATCH_MAX_AGE),
        (steam::ARTWORK_CACHE, steam::STEAM_ARTWORK_MAX_AGE),
    ]
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
