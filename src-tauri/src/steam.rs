//! Game details and artwork from Steam's public store API (no key needed).
//!
//! - `appdetails?appids=<id>`: full details for one app.
//! - `storesearch?term=<name>`: search by name, used to match forum topics
//!   to Steam apps for card images.
//!
//! These endpoints are unofficial and rate limited (commonly ~200 requests
//! per 5 minutes), so requests are made one at a time, spaced out, capped
//! below that, and paused after a 429/403. Results are cached by the caller.
//! This client never sends the forum's cookies.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use url::Url;

use crate::error::{Error, Result};

const MIN_SPACING: Duration = Duration::from_millis(250);
const WINDOW: Duration = Duration::from_secs(300);
const MAX_PER_WINDOW: usize = 150;
const BACKOFF: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SteamMatch {
    pub app_id: u64,
    pub name: String,
    /// 231×87 capsule image.
    pub capsule_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SteamApp {
    pub app_id: u64,
    pub name: String,
    pub store_url: String,
    pub header_image: Option<String>,
    pub capsule_image: Option<String>,
    pub short_description: Option<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub release_date: Option<String>,
    pub coming_soon: bool,
    /// `windows`, `macos`, `linux`.
    pub platforms: Vec<String>,
    pub genres: Vec<String>,
    /// Steam's categories, e.g. "Single-player", "Steam Achievements".
    pub categories: Vec<String>,
    pub languages: Vec<String>,
    pub screenshots: Vec<Screenshot>,
    pub is_free: bool,
    /// e.g. `$8.99`, in US dollars.
    pub price: Option<String>,
    pub metacritic: Option<u32>,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Screenshot {
    pub thumbnail: String,
    pub full: String,
}

struct Pacer {
    recent: VecDeque<Instant>,
    blocked_until: Option<Instant>,
}

impl Pacer {
    /// Waits for a free slot and claims it, or fails while backing off.
    async fn wait(&mut self) -> Result<()> {
        if self.blocked_until.is_some_and(|t| Instant::now() < t) {
            return Err(Error::SteamRateLimited);
        }
        loop {
            let now = Instant::now();
            while self
                .recent
                .front()
                .is_some_and(|t| now.duration_since(*t) > WINDOW)
            {
                self.recent.pop_front();
            }
            let until = if self.recent.len() >= MAX_PER_WINDOW {
                self.recent.front().map(|t| *t + WINDOW)
            } else {
                self.recent.back().map(|t| *t + MIN_SPACING)
            };
            match until {
                Some(t) if t > now => tokio::time::sleep(t - now).await,
                _ => break,
            }
        }
        self.recent.push_back(Instant::now());
        Ok(())
    }
}

pub struct SteamClient {
    http: reqwest::Client,
    pacer: Mutex<Pacer>,
}

impl SteamClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()?,
            pacer: Mutex::new(Pacer {
                recent: VecDeque::new(),
                blocked_until: None,
            }),
        })
    }

    async fn get_json(&self, url: Url) -> Result<Value> {
        self.pacer.lock().await.wait().await?;
        let response = self.http.get(url).send().await?;
        let status = response.status().as_u16();
        if status == 429 || status == 403 {
            self.pacer.lock().await.blocked_until = Some(Instant::now() + BACKOFF);
            return Err(Error::SteamRateLimited);
        }
        if !response.status().is_success() {
            return Err(Error::Steam(format!("HTTP {status}")));
        }
        let body = response.bytes().await?;
        serde_json::from_slice(&body).map_err(|e| Error::Steam(format!("unexpected response: {e}")))
    }

    /// Full store details; `None` if Steam has no such app (or region-locks it).
    pub async fn app_details(&self, app_id: u64) -> Result<Option<SteamApp>> {
        let mut url = Url::parse("https://store.steampowered.com/api/appdetails")?;
        url.query_pairs_mut()
            .append_pair("appids", &app_id.to_string())
            .append_pair("l", "english")
            .append_pair("cc", "us");
        Ok(parse_app_details(&self.get_json(url).await?, app_id))
    }

    /// The Steam app whose name matches `name`, if any.
    pub async fn find(&self, name: &str) -> Result<Option<SteamMatch>> {
        for term in search_terms(name) {
            let mut url = Url::parse("https://store.steampowered.com/api/storesearch/")?;
            url.query_pairs_mut()
                .append_pair("term", &term)
                .append_pair("l", "english")
                .append_pair("cc", "US");
            if let Some(found) = pick_match(&self.get_json(url).await?, &term) {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------- parsing

fn strings(value: &Value, field: &str) -> Vec<String> {
    value[field]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().or_else(|| v["description"].as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn string(value: &Value, field: &str) -> Option<String> {
    value[field]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// "English<strong>*</strong>, French<br><strong>*</strong>languages with full audio support"
fn parse_languages(html: &str) -> Vec<String> {
    let list = html.split("<br>").next().unwrap_or("");
    let mut text = String::new();
    let mut in_tag = false;
    for c in list.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text.split(',')
        .map(|s| s.replace('*', "").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn parse_app_details(json: &Value, app_id: u64) -> Option<SteamApp> {
    let entry = &json[app_id.to_string()];
    if entry["success"].as_bool() != Some(true) {
        return None;
    }
    let d = &entry["data"];
    let platforms = [("windows", "windows"), ("mac", "macos"), ("linux", "linux")]
        .into_iter()
        .filter(|(key, _)| d["platforms"][key].as_bool() == Some(true))
        .map(|(_, name)| name.to_string())
        .collect();
    let screenshots = d["screenshots"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    Some(Screenshot {
                        thumbnail: s["path_thumbnail"].as_str()?.to_string(),
                        full: s["path_full"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(SteamApp {
        app_id,
        name: string(d, "name").unwrap_or_default(),
        store_url: format!("https://store.steampowered.com/app/{app_id}"),
        header_image: string(d, "header_image"),
        capsule_image: string(d, "capsule_image"),
        short_description: string(d, "short_description"),
        developers: strings(d, "developers"),
        publishers: strings(d, "publishers"),
        release_date: string(&d["release_date"], "date"),
        coming_soon: d["release_date"]["coming_soon"].as_bool().unwrap_or(false),
        platforms,
        genres: strings(d, "genres"),
        categories: strings(d, "categories"),
        languages: d["supported_languages"]
            .as_str()
            .map(parse_languages)
            .unwrap_or_default(),
        screenshots,
        is_free: d["is_free"].as_bool().unwrap_or(false),
        price: string(&d["price_overview"], "final_formatted"),
        metacritic: d["metacritic"]["score"].as_u64().map(|s| s as u32),
        website: string(d, "website"),
    })
}

/// Lowercase letters and digits only, with `&` as "and" and trademark
/// signs dropped: "Call of Duty®: Modern Warfare® 2" → "callofdutymodernwarfare2".
pub fn normalize_name(name: &str) -> String {
    name.replace('&', " and ")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The name as on the forum, then without "(…)" parts: forum titles add
/// things like "(Remake)" or "(2017)" that Steam's names may not have.
fn search_terms(name: &str) -> Vec<String> {
    let mut terms = vec![name.trim().to_string()];
    let mut stripped = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if depth == 0 => stripped.push(c),
            _ => {}
        }
    }
    let stripped = stripped.split_whitespace().collect::<Vec<_>>().join(" ");
    if !stripped.is_empty() && stripped != terms[0] {
        terms.push(stripped);
    }
    terms
}

/// Only an app whose normalized name equals the term's counts as a match;
/// search also returns DLC, soundtracks and look-alikes.
fn pick_match(json: &Value, term: &str) -> Option<SteamMatch> {
    let wanted = normalize_name(term);
    json["items"].as_array()?.iter().find_map(|item| {
        let name = item["name"].as_str()?;
        let app_id = item["id"].as_u64()?;
        (item["type"].as_str() == Some("app") && normalize_name(name) == wanted).then(|| {
            SteamMatch {
                app_id,
                name: name.to_string(),
                capsule_image: item["tiny_image"].as_str().map(str::to_string),
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn app_details() {
        let body = json!({ "4459590": { "success": true, "data": {
            "name": "Slotbound",
            "header_image": "https://cdn.example/header.jpg",
            "short_description": "A roguelike auto-battler.",
            "developers": ["Optima Arch"],
            "publishers": ["Optima Arch"],
            "release_date": { "coming_soon": false, "date": "Nov 12, 2026" },
            "platforms": { "windows": true, "mac": true, "linux": false },
            "genres": [{ "id": "23", "description": "Indie" }, { "id": "2", "description": "Strategy" }],
            "categories": [{ "id": 2, "description": "Single-player" }],
            "supported_languages": "English<strong>*</strong>, Korean, French<br><strong>*</strong>languages with full audio support",
            "screenshots": [{ "id": 0, "path_thumbnail": "t.jpg", "path_full": "f.jpg" }],
            "is_free": false,
            "price_overview": { "final_formatted": "$8.99" },
            "metacritic": null
        }}});
        let app = parse_app_details(&body, 4459590).unwrap();
        assert_eq!(app.name, "Slotbound");
        assert_eq!(app.store_url, "https://store.steampowered.com/app/4459590");
        assert_eq!(app.release_date.as_deref(), Some("Nov 12, 2026"));
        assert_eq!(app.platforms, ["windows", "macos"]);
        assert_eq!(app.genres, ["Indie", "Strategy"]);
        assert_eq!(app.categories, ["Single-player"]);
        assert_eq!(app.languages, ["English", "Korean", "French"]);
        assert_eq!(app.screenshots[0].full, "f.jpg");
        assert_eq!(app.price.as_deref(), Some("$8.99"));
        assert_eq!(app.metacritic, None);

        let missing = json!({ "1": { "success": false } });
        assert_eq!(parse_app_details(&missing, 1), None);
    }

    #[test]
    fn names() {
        assert_eq!(
            normalize_name("Call of Duty®: Modern Warfare® 2"),
            "callofdutymodernwarfare2"
        );
        assert_eq!(
            normalize_name("Workers & Resources: Soviet Republic"),
            normalize_name("Workers and Resources - Soviet Republic")
        );
        assert_eq!(
            search_terms("Silent Hill 2 (Remake)"),
            ["Silent Hill 2 (Remake)", "Silent Hill 2"]
        );
        assert_eq!(search_terms("Slotbound"), ["Slotbound"]);
    }

    #[test]
    fn matching() {
        // Shape of a real storesearch response for "Silent Hill 2".
        let body = json!({ "total": 2, "items": [
            { "type": "app", "id": 2124490, "name": "SILENT HILL 2", "tiny_image": "https://cdn.example/capsule.jpg" },
            { "type": "app", "id": 4423570, "name": "FATAL FRAME II x SILENT HILL f Costume Set", "tiny_image": "x" }
        ]});
        let found = pick_match(&body, "Silent Hill 2").unwrap();
        assert_eq!(
            (found.app_id, found.capsule_image.as_deref()),
            (2124490, Some("https://cdn.example/capsule.jpg"))
        );
        // Look-alikes don't count.
        assert_eq!(pick_match(&body, "Silent Hill"), None);
        assert_eq!(
            pick_match(&json!({ "total": 0, "items": [] }), "Nothing"),
            None
        );
    }

    #[tokio::test(start_paused = true)]
    async fn pacer_spaces_and_caps_requests() {
        let mut pacer = Pacer {
            recent: VecDeque::new(),
            blocked_until: None,
        };
        let start = Instant::now();
        for _ in 0..3 {
            pacer.wait().await.unwrap();
        }
        assert!(Instant::now() - start >= MIN_SPACING * 2);

        pacer.blocked_until = Some(Instant::now() + BACKOFF);
        assert!(matches!(pacer.wait().await, Err(Error::SteamRateLimited)));
    }
}
