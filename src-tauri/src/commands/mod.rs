pub mod forum;
pub mod game;
pub mod library;
pub mod steam;

pub const MAIN_FORUM_ID: u64 = 10;

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
