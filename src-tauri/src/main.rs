// CsRinRu Desktop App - Main Entry Point
// A desktop application for browsing and installing games from cs.rin.ru forum

mod scraper;
mod models;
mod parser;
mod database;
mod commands;

use scraper::ForumScraper;
use models::{Game, GameDetail, Release};
use commands::AppState;

fn main() {
    println!("Starting CsRinRu Desktop App...");
    
    tauri::Builder::default()
        .manage(AppState {
            scraper: ForumScraper::new("https://cs.rin.ru").expect("Failed to initialize scraper"),
        })
        .invoke_handler(tauri::generate_handler![
            commands::fetch_game_feed,
            commands::fetch_game_detail,
            commands::search_games,
            commands::export_games
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
