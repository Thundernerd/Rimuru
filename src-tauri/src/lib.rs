// CsRinRu Desktop App - Library Root
pub mod scraper;
pub mod models;
pub mod parser;
pub mod database;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_creation() {
        let game = models::Game::new(
            "game-1".to_string(),
            "Test Game".to_string(),
            "Test Author".to_string(),
            "https://cs.rin.ru/forum/showthread.php?t=123".to_string(),
            "2024-01-15".to_string(),
        );
        
        assert_eq!(game.title, "Test Game");
        assert_eq!(game.author, "Test Author");
        assert!(!game.is_expanded);
        
        let expanded_game = game.expand();
        assert!(expanded_game.is_expanded);
    }

    #[test]
    fn test_release_creation() {
        let mut release = models::Release::new("v1.0".to_string(), "2024-01-15".to_string());
        
        assert_eq!(release.version, "v1.0");
        
        let download_link = models::DownloadLink {
            url: "https://example.com/game.zip".to_string(),
            file_name: "game_v1.0.zip".to_string(),
            file_type: "zip".to_string(),
            file_size: Some("125 MB".to_string()),
            mirror: false,
        };
        
        release.add_download_link(download_link);
        assert_eq!(release.download_links.len(), 1);
    }

    #[test]
    fn test_parser_release_detection() {
        let post_content = "
        <div class=\"post\">
            <span class=\"author\">JohnDoe</span>
            <p>Release v2.0 is now available for download!</p>
            <a href=\"https://example.com/game-v2.zip\">Download the latest version</a>
        </div>
        ";
        
        let post = parser::ForumParser::parse_post(post_content);
        
        assert!(post.is_release_post);
        assert!(!post.download_links.is_empty());
    }

    #[test]
    fn test_download_link_extraction() {
        let links = vec![
            "https://cdn.example.com/game-release.zip".to_string(),
            "https://mirror.example.com/game-installer.exe".to_string(),
            "https://dl.example.com/game-patch-7z.7z".to_string(),
        ];
        
        let parsed_links = parser::ForumParser::parse_download_links(&links);
        
        assert_eq!(parsed_links.len(), 3);
        assert_eq!(parsed_links[0].file_type, "zip");
        assert_eq!(parsed_links[1].file_type, "exe");
        assert!(parsed_links[0].mirror);
    }

    #[test]
    fn test_forum_scraper_initialization() {
        let base_url = "https://cs.rin.ru";
        let scraper = scraper::ForumScraper::new(base_url).expect("Failed to create scraper");
        
        assert_eq!(scraper.game_feed_url, "https://cs.rin.ru/forum/viewforum.php?f=10");
    }

    #[test]
    fn test_database_client_initialization() {
        let client = database::GameDatabaseClient::new();
        
        assert_eq!(client.sources.len(), 3);
        assert_eq!(client.sources[0].name, "IGDB");
        assert_eq!(client.sources[1].name, "Steam");
        assert_eq!(client.sources[2].name, "GOG");
    }
}
