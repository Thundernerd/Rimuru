// External Database Module
// Handles matching games with online databases for images and metadata

use crate::models::{Game, GameMetadata, Image};
use reqwest::Client;
use std::error::Error;

pub struct GameDatabaseClient {
    client: Client,
    // Support multiple database sources
    sources: Vec<DatabaseSource>,
}

pub struct DatabaseSource {
    pub name: String,
    pub base_url: String,
    pub search_endpoint: String,
}

impl GameDatabaseClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            sources: vec![
                DatabaseSource {
                    name: "IGDB".to_string(),
                    base_url: "https://api.igdb.com/v4".to_string(),
                    search_endpoint: "/games/search".to_string(),
                },
                DatabaseSource {
                    name: "Steam".to_string(),
                    base_url: "https://store.steampowered.com".to_string(),
                    search_endpoint: "/api/store/search".to_string(),
                },
                DatabaseSource {
                    name: "GOG".to_string(),
                    base_url: "https://www.gog.com".to_string(),
                    search_endpoint: "/api/search".to_string(),
                },
            ],
        }
    }
    
    /// Matches a game from the forum with an external database
    pub async fn match_game(&self, game: &Game) -> Result<Option<GameMetadata>, Box<dyn Error>> {
        // Search across configured database sources
        for source in &self.sources {
            if let Some(metadata) = self.search_source(source, game).await? {
                return Ok(Some(metadata));
            }
        }
        
        Ok(None)
    }
    
    /// Searches a specific database source for a game
    async fn search_source(
        &self,
        source: &DatabaseSource,
        game: &Game,
    ) -> Result<Option<GameMetadata>, Box<dyn Error>> {
        // Build search query with game title and author
        let search_query = format!("{} {}", game.title, game.author);
        let search_url = format!("{}{}", source.base_url, source.search_endpoint);
        
        // Execute search request
        let response = self.client
            .get(&search_url)
            .query(&[("q", &search_query)])
            .send()
            .await?;
        
        if response.status().is_success() {
            let body = response.text().await?;
            let metadata = self.parse_search_result(&body, source, game);
            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }
    
    /// Parses the search result from a database source
    fn parse_search_result(
        &self,
        body: &str,
        source: &DatabaseSource,
        game: &Game,
    ) -> GameMetadata {
        GameMetadata {
            external_id: Self::generate_external_id(&game.title, &source.name),
            database_source: source.name.clone(),
            images: Vec::new(),
            tags: self.extract_tags(body),
        }
    }
    
    /// Generates a unique external ID for a game
    fn generate_external_id(title: &str, source: &str) -> String {
        // Create a normalized ID from the title
        let normalized = title.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .replace(" ", "_");
        
        format!("{}-{}", source.to_lowercase(), normalized)
    }
    
    /// Extracts tags from the search result
    fn extract_tags(&self, body: &str) -> Vec<String> {
        // Extract game tags/genres from the database response
        vec!["game".to_string()]
    }
    
    /// Fetches images for a game from external database
    pub async fn fetch_game_images(
        &self,
        game: &Game,
        metadata: &GameMetadata,
    ) -> Result<Vec<Image>, Box<dyn Error>> {
        // Fetch images from the matched database source
        let images_url = format!(
            "{}/games/{}/images",
            metadata.database_source, metadata.external_id
        );
        
        // Implement image fetching logic with caching support
        Ok(vec![Image {
            url: format!("{}/large", images_url),
            thumbnail_url: format!("{}/thumbnail", images_url),
            source: metadata.database_source.clone(),
            cached: false,
        }])
    }
    
    /// Caches game images locally
    pub async fn cache_images(
        &self,
        images: &[Image],
        cache_dir: &str,
    ) -> Result<(), Box<dyn Error>> {
        for image in images {
            if !image.cached {
                // Download and cache the image
                self.download_and_cache_image(image, cache_dir).await?;
            }
        }
        Ok(())
    }
    
    /// Downloads and caches a single image
    async fn download_and_cache_image(
        &self,
        image: &Image,
        cache_dir: &str,
    ) -> Result<(), Box<dyn Error>> {
        // Implementation for downloading images to local cache
        // Returns the local cache path
        Ok(())
    }
}
