// Forum Scraper Module
// Handles fetching and parsing of cs.rin.ru forum

use crate::models::{Game, GameDetail};
use reqwest::Client;
use scraper::{Html, Selector};
use std::error::Error;

pub struct ForumScraper {
    client: Client,
    base_url: String,
    game_feed_url: String,
}

pub struct GameFeed {
    pub games: Vec<Game>,
    pub pagination: PaginationInfo,
    pub last_updated: String,
}

pub struct PaginationInfo {
    pub current_page: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_previous: bool,
}

impl ForumScraper {
    pub fn new(base_url: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            game_feed_url: format!("{}/forum/viewforum.php?f=10", base_url),
        })
    }
    
    /// Fetches the game feed from the forum
    pub async fn fetch_game_feed(&self) -> Result<GameFeed, Box<dyn Error>> {
        let response = self.client.get(&self.game_feed_url).send().await?;
        let body = response.text().await?;
        
        let document = Html::parse_document(&body);
        let games = self.parse_games(&document)?;
        let pagination = self.parse_pagination(&document)?;
        
        Ok(GameFeed {
            games,
            pagination,
            last_updated: self.extract_last_updated(&body)?,
        })
    }
    
    /// Parses individual game entries from the forum HTML
    fn parse_games(&self, document: &scraper::Html) -> Result<Vec<Game>, Box<dyn Error>> {
        // Selector for forum thread rows
        let thread_selector = Selector::parse("tr.topic").unwrap();
        
        document
            .select(&thread_selector)
            .map(|element| self.parse_game_row(element))
            .collect()
    }
    
    /// Parses a single game row from the forum
    fn parse_game_row(&self, element: scraper::ElementRef) -> Result<Game, Box<dyn Error>> {
        // Extract thread title and URL
        let title_selector = Selector::parse("td.title a.topictitle").unwrap();
        let title_elem = element.select(&title_selector).next().ok_or("Missing title")?;
        
        let title = title_elem.text().collect::<String>();
        let thread_url = title_elem
            .value()
            .attr("href")
            .map(|h| format!("{}{}", self.base_url, h))
            .unwrap_or_default();
        
        // Extract author and date
        let author = self.extract_author(&element)?;
        let post_date = self.extract_post_date(&element)?;
        
        // Generate a unique ID for the game
        let id = format!("game-{}", thread_url.replace('/', "-"));
        
        Ok(Game::new(id, title, author, thread_url, post_date))
    }
    
    /// Extracts the author name from a game row
    fn extract_author(&self, element: &scraper::ElementRef) -> Result<String, Box<dyn Error>> {
        let author_selector = Selector::parse("td.author a").unwrap();
        let author_elem = element.select(&author_selector).next();
        
        Ok(author_elem
            .map(|e| e.text().collect::<String>())
            .unwrap_or_else(|| "Unknown".to_string()))
    }
    
    /// Extracts the post date from a game row
    fn extract_post_date(&self, element: &scraper::ElementRef) -> Result<String, Box<dyn Error>> {
        let date_selector = Selector::parse("td.date").unwrap();
        let date_elem = element.select(&date_selector).next();
        
        Ok(date_elem
            .map(|e| e.text().collect::<String>())
            .unwrap_or_else(|| String::new()))
    }
    
    /// Parses pagination information from the forum page
    fn parse_pagination(&self, document: &scraper::Html) -> Result<PaginationInfo, Box<dyn Error>> {
        let page_selector = Selector::parse("div.pagination").unwrap();
        
        // Extract current page and total pages
        let current_page = self.extract_current_page(document);
        let has_next = document.select(&page_selector).any(|e| {
            e.text().any(|t| t.contains("next") || t.contains("Next"))
        });
        
        Ok(PaginationInfo {
            current_page,
            total_pages: current_page, // Will be refined with full pagination parsing
            has_next,
            has_previous: current_page > 1,
        })
    }
    
    /// Extracts current page number from the forum
    fn extract_current_page(&self, document: &scraper::Html) -> u32 {
        // Implementation for extracting page number from pagination controls
        1 // Default to first page
    }
    
    /// Extracts the last updated timestamp from the forum
    fn extract_last_updated(&self, body: &str) -> Result<String, Box<dyn Error>> {
        // Extract timestamp from page metadata or footer
        Ok(format!("{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")))
    }
    
    /// Fetches detailed information for a specific game thread
    pub async fn fetch_game_detail(&self, game: &Game) -> Result<GameDetail, Box<dyn Error>> {
        let response = self.client.get(&game.thread_url).send().await?;
        let body = response.text().await?;
        
        let document = Html::parse_document(&body);
        let posts = self.parse_forum_posts(&document, game)?;
        let releases = self.extract_releases(&posts);
        let images = self.fetch_game_images(game).await?;
        
        Ok(GameDetail {
            game: game.clone(),
            posts,
            releases,
            images,
        })
    }
    
    /// Parses forum posts from a game thread
    fn parse_forum_posts(
        &self,
        document: &scraper::Html,
        game: &Game,
    ) -> Result<Vec<crate::models::ForumPost>, Box<dyn Error>> {
        // Implementation for parsing individual posts within a thread
        // This handles the forum post structure extraction
        Ok(Vec::new())
    }
    
    /// Extracts release information from forum posts
    fn extract_releases(&self, posts: &[crate::models::ForumPost]) -> Vec<crate::models::Release> {
        // Implementation for identifying and grouping release posts
        posts.iter()
            .filter(|post| post.is_release_post)
            .map(|post| self.parse_release_from_post(post))
            .collect()
    }
    
    /// Creates a release from a forum post
    fn parse_release_from_post(&self, post: &crate::models::ForumPost) -> crate::models::Release {
        // Extract release version, date, and download links from post content
        crate::models::Release::new(String::new(), post.date.clone())
    }
    
    /// Fetches game images from external database
    async fn fetch_game_images(&self, game: &Game) -> Result<Vec<crate::models::Image>, Box<dyn Error>> {
        // Implementation for matching games with external database images
        Ok(Vec::new())
    }
}
