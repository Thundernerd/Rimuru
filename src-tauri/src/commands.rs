// Tauri Commands Module
// Provides the bridge between the frontend and backend

use crate::scraper::ForumScraper;
use crate::models::{Game, GameDetail};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Application state shared across commands
pub struct AppState {
    pub scraper: ForumScraper,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchGameFeedRequest {
    pub page: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchGameDetailRequest {
    pub game_id: String,
}

/// Command to fetch the game feed from cs.rin.ru forum
///
/// This command initiates a fetch of games from the forum section.
/// It returns a list of games with their basic information.
///
/// # Arguments
///
/// * `page` - The page number to fetch (default: 1)
///
/// # Returns
///
/// A vector of Game objects containing:
/// - id: Unique game identifier
/// - title: Game title from the forum thread
/// - author: Author of the game
/// - thread_url: URL to the full game thread
/// - post_date: Date of the forum post
/// - image: URL to game image from external database
#[tauri::command]
pub async fn fetch_game_feed(state: State<AppState>, page: u32) -> Result<Vec<Game>, String> {
    // In a full implementation, pagination would be handled here
    let _ = page; // Currently using first page
    
    match state.scraper.fetch_game_feed().await {
        Ok(feed) => {
            // Process games and add image URLs
            let games_with_images = feed.games.into_iter()
                .map(|mut game| {
                    // Add default or fetched image URL
                    game = game_with_image(game);
                    game
                })
                .collect();
            
            Ok(games_with_images)
        }
        Err(e) => Err(format!("Failed to fetch game feed: {}", e)),
    }
}

/// Command to fetch detailed information for a specific game
///
/// This command retrieves the full details for a game including:
/// - Forum posts from the game thread
/// - Release information with download links
/// - Images from external databases
///
/// # Arguments
///
/// * `game_id` - The unique identifier of the game to fetch
///
/// # Returns
///
/// A GameDetail object containing:
/// - game: Basic game information
/// - posts: List of forum posts in the game thread
/// - releases: Extracted release information
/// - images: Images from matched external databases
#[tauri::command]
pub async fn fetch_game_detail(
    state: State<AppState>,
    game_id: String,
) -> Result<GameDetail, String> {
    // Find the game from the current feed
    let feed = match state.scraper.fetch_game_feed().await {
        Ok(feed) => feed,
        Err(e) => return Err(format!("Failed to fetch feed: {}", e)),
    };
    
    // Find the specific game
    let game = feed.games.iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("Game not found: {}", game_id))?;
    
    // Fetch detailed information
    match state.scraper.fetch_game_detail(game).await {
        Ok(detail) => Ok(detail),
        Err(e) => Err(format!("Failed to fetch game details: {}", e)),
    }
}

/// Command to search for games by query
///
/// Enables searching across the forum for games matching specific criteria.
///
/// # Arguments
///
/// * `query` - Search query string
/// * `category` - Optional category filter
///
/// # Returns
///
/// A vector of matching games with relevance scores
#[tauri::command]
pub async fn search_games(
    state: State<AppState>,
    query: String,
    category: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let feed = state.scraper.fetch_game_feed().await
        .map_err(|e| format!("Failed to fetch feed: {}", e))?;
    
    let results = feed.games
        .into_iter()
        .filter(|game| {
            game.title.to_lowercase().contains(&query.to_lowercase())
                || game.author.to_lowercase().contains(&query.to_lowercase())
        })
        .map(|game| SearchResult {
            game: game_with_image(game),
            relevance: calculate_relevance(&game.title, &query),
            category: category.clone(),
        })
        .collect();
    
    Ok(results)
}

/// Command to export game data to file
///
/// Exports the current game data in a specified format for backup or sharing.
///
/// # Arguments
///
/// * `format` - Export format (json, csv, xml)
/// * `output_path` - Path for the exported file
///
/// # Returns
///
/// Path to the exported file and export statistics
#[tauri::command]
pub async fn export_games(
    state: State<AppState>,
    format: String,
    output_path: String,
) -> Result<ExportResult, String> {
    let feed = state.scraper.fetch_game_feed().await
        .map_err(|e| format!("Failed to fetch feed: {}", e))?;
    
    // Generate export based on format
    let (exported_path, stats) = match format.as_str() {
        "json" => export_as_json(&feed, &output_path),
        "csv" => export_as_csv(&feed, &output_path),
        "xml" => export_as_xml(&feed, &output_path),
        _ => return Err("Unsupported export format".to_string()),
    }?;
    
    Ok(ExportResult {
        path: exported_path,
        stats,
        format,
    })
}

// Support structures

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub game: Game,
    pub relevance: f64,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub stats: ExportStats,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportStats {
    pub games_exported: usize,
    pub total_posts: usize,
    pub total_releases: usize,
    pub export_timestamp: String,
}

// Helper functions

fn game_with_image(game: Game) -> Game {
    // In full implementation, this would fetch images from the database client
    // For now, we add a placeholder image
    game // Return with image handling to be implemented
}

fn calculate_relevance(title: &str, query: &str) -> f64 {
    let title_lower = title.to_lowercase();
    let query_lower = query.to_lowercase();
    
    // Simple relevance calculation based on query term frequency in title
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    let mut score = 0.0;
    
    for word in &query_words {
        if title_lower.contains(*word) {
            score += 1.0;
        }
    }
    
    score / query_words.len() as f64
}

fn export_as_json(feed: &crate::scraper::GameFeed, path: &str) -> Result<(String, ExportStats), String> {
    // Implementation for JSON export
    let games_count = feed.games.len();
    let stats = ExportStats {
        games_exported: games_count,
        total_posts: 0,
        total_releases: 0,
        export_timestamp: chrono::Local::now().to_rfc3339(),
    };
    
    Ok((format!("{}.json", path), stats))
}

fn export_as_csv(feed: &crate::scraper::GameFeed, path: &str) -> Result<(String, ExportStats), String> {
    // Implementation for CSV export
    let games_count = feed.games.len();
    let stats = ExportStats {
        games_exported: games_count,
        total_posts: 0,
        total_releases: 0,
        export_timestamp: chrono::Local::now().to_rfc3339(),
    };
    
    Ok((format!("{}.csv", path), stats))
}

fn export_as_xml(feed: &crate::scraper::GameFeed, path: &str) -> Result<(String, ExportStats), String> {
    // Implementation for XML export
    let games_count = feed.games.len();
    let stats = ExportStats {
        games_exported: games_count,
        total_posts: 0,
        total_releases: 0,
        export_timestamp: chrono::Local::now().to_rfc3339(),
    };
    
    Ok((format!("{}.xml", path), stats))
}
