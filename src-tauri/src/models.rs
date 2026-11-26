// Data Models for the CsRinRu Desktop App

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a game entry in the forum feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub author: String,
    pub thread_url: String,
    pub post_date: String,
    pub is_expanded: bool,
}

/// Detailed game information with parsed posts and releases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDetail {
    pub game: Game,
    pub posts: Vec<ForumPost>,
    pub releases: Vec<Release>,
    pub images: Vec<String>,
}

/// Represents a forum post within a game thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumPost {
    pub id: String,
    pub author: String,
    pub content: String,
    pub date: String,
    pub is_release_post: bool,
    pub download_links: Vec<String>,
}

/// Represents a game release with download links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub release_date: String,
    pub description: String,
    pub download_links: Vec<DownloadLink>,
    pub file_types: HashMap<String, Vec<String>>,
}

/// Represents a download link with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadLink {
    pub url: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: Option<String>,
    pub mirror: bool,
}

/// Database match result for game metadata and images
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMetadata {
    pub external_id: String,
    pub database_source: String,
    pub images: Vec<Image>,
    pub tags: Vec<String>,
}

/// Represents an image from an external database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub url: String,
    pub thumbnail_url: String,
    pub source: String,
    pub cached: bool,
}

impl Game {
    /// Creates a new game from forum data
    pub fn new(id: String, title: String, author: String, thread_url: String, post_date: String) -> Self {
        Self {
            id,
            title,
            author,
            thread_url,
            post_date,
            is_expanded: false,
        }
    }
    
    /// Marks the game as expanded with parsed posts
    pub fn expand(mut self) -> Self {
        self.is_expanded = true;
        self
    }
}

impl Release {
    /// Creates a new release entry
    pub fn new(version: String, release_date: String) -> Self {
        Self {
            version,
            release_date,
            description: String::new(),
            download_links: Vec::new(),
            file_types: HashMap::new(),
        }
    }
    
    /// Adds a download link to the release
    pub fn add_download_link(&mut self, link: DownloadLink) {
        self.download_links.push(link);
    }
}
