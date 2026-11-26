// Forum Parser Module
// Handles parsing of forum posts and extraction of release information

use crate::models::{ForumPost, Release, DownloadLink};
use regex::Regex;
use std::collections::HashMap;

pub struct ForumParser;

impl ForumParser {
    /// Parses a forum post to extract content and identify if it's a release post
    pub fn parse_post(content: &str) -> ForumPost {
        let post = ForumPost {
            id: Self::generate_post_id(content),
            author: Self::extract_author(content),
            content: content.to_string(),
            date: Self::extract_date(content),
            is_release_post: Self::is_release_post(content),
            download_links: Self::extract_download_links(content),
        };
        
        post
    }
    
    /// Generates a unique ID for a post
    fn generate_post_id(content: &str) -> String {
        // Use content hash or position to generate unique ID
        format!("post-{}", content.chars().take(8).collect::<String>())
    }
    
    /// Extracts author name from post content
    fn extract_author(content: &str) -> String {
        // Look for author patterns in the HTML
        let author_re = Regex::new(r#"author[^>]*>([^<]+)"#).unwrap();
        author_re
            .captures(content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "Unknown Author".to_string())
    }
    
    /// Extracts date from post content
    fn extract_date(content: &str) -> String {
        let date_re = Regex::new(r#"\b\d{1,2}[./-]\d{1,2}[./-]\d{2,4}\b"#).unwrap();
        date_re
            .find(content)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| String::new())
    }
    
    /// Determines if a post contains release information
    fn is_release_post(content: &str) -> bool {
        let release_keywords = [
            "release",
            "v\d+\.\d+",
            "version \d+",
            "download",
            "update",
            "patch",
        ];
        
        content.to_lowercase()
            .split_whitespace()
            .any(|word| release_keywords.iter().any(|keyword| {
                word.contains(keyword.to_lowercase().as_str())
            }))
    }
    
    /// Extracts download links from post content
    fn extract_download_links(content: &str) -> Vec<String> {
        let link_re = Regex::new(r#"href\s*=\s*["']([^"']+\.?(?:rar|zip|7z|exe|dmg|deb|rpm)["'])"#).unwrap();
        
        link_re
            .captures_iter(content)
            .map(|cap| cap.get(1).unwrap().as_str().to_string())
            .collect()
    }
    
    /// Parses download links with metadata extraction
    pub fn parse_download_links(links: &[String]) -> Vec<DownloadLink> {
        links.iter().map(|link| Self::parse_single_link(link)).collect()
    }
    
    /// Parses a single download link with metadata
    fn parse_single_link(link: &str) -> DownloadLink {
        let file_name = Self::extract_file_name(link);
        let file_type = Self::extract_file_type(&file_name);
        
        DownloadLink {
            url: link.clone(),
            file_name,
            file_type,
            file_size: None, // Size would be extracted from additional metadata
            mirror: link.contains("mirror") || link.contains("cdn"),
        }
    }
    
    /// Extracts file name from URL
    fn extract_file_name(url: &str) -> String {
        url.split('/').last().unwrap_or(url).to_string()
    }
    
    /// Determines file type from file name extension
    fn extract_file_type(file_name: &str) -> String {
        file_name.split('.').last().unwrap_or("unknown").to_string()
    }
    
    /// Groups download links by file type
    pub fn group_links_by_type(links: &[DownloadLink]) -> HashMap<String, Vec<String>> {
        let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
        
        for link in links {
            grouped
                .entry(link.file_type.clone())
                .or_insert_with(Vec::new)
                .push(link.url.clone());
        }
        
        grouped
    }
}
