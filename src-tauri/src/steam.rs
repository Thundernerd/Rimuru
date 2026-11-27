use std::collections::{HashMap, VecDeque};
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
    pub platforms: Vec<String>,
    pub genres: Vec<String>,
    pub categories: Vec<String>,
    pub languages: Vec<String>,
    pub screenshots: Vec<Screenshot>,
    pub is_free: bool,
    pub price: Option<String>,
    pub metacritic: Option<u32>,
    pub website: Option<String>,
    #[serde(default)]
    pub movies: Vec<Movie>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Movie {
    pub name: String,
    pub thumbnail: Option<String>,
    pub hls: Option<String>,
    pub mp4: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Artwork {
    pub portrait: Option<String>,
    pub header: Option<String>,
    pub capsule: Option<String>,
    pub hero: Option<String>,
}

pub const ARTWORK_BATCH: usize = 50;

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

    pub async fn app_details(&self, app_id: u64) -> Result<Option<SteamApp>> {
        let mut url = Url::parse("https://store.steampowered.com/api/appdetails")?;
        url.query_pairs_mut()
            .append_pair("appids", &app_id.to_string())
            .append_pair("l", "english")
            .append_pair("cc", "us");
        Ok(parse_app_details(&self.get_json(url).await?, app_id))
    }

    pub async fn artwork(&self, app_ids: &[u64]) -> Result<HashMap<u64, Artwork>> {
        let ids: Vec<Value> = app_ids
            .iter()
            .map(|id| serde_json::json!({ "appid": id }))
            .collect();
        let input = serde_json::json!({
            "ids": ids,
            "context": { "language": "english", "country_code": "US" },
            "data_request": { "include_assets": true },
        });
        let mut url = Url::parse("https://api.steampowered.com/IStoreBrowseService/GetItems/v1")?;
        url.query_pairs_mut()
            .append_pair("input_json", &input.to_string());
        Ok(parse_artwork(&self.get_json(url).await?))
    }

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

/// Steam's store API sends some text fields HTML-entity encoded, such as
/// `&amp;` for `&`. Decodes them into plain text.
fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    scraper::Html::parse_fragment(text)
        .root_element()
        .text()
        .collect()
}

fn decoded(value: &Value, field: &str) -> Option<String> {
    string(value, field)
        .map(|s| decode_entities(&s).trim().to_string())
        .filter(|s| !s.is_empty())
}

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
        name: decoded(d, "name").unwrap_or_default(),
        store_url: format!("https://store.steampowered.com/app/{app_id}"),
        header_image: string(d, "header_image"),
        capsule_image: string(d, "capsule_image"),
        short_description: decoded(d, "short_description"),
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
        movies: parse_movies(&d["movies"]),
    })
}

pub fn parse_artwork(json: &Value) -> HashMap<u64, Artwork> {
    const ASSET_BASE: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";
    let Some(items) = json["response"]["store_items"].as_array() else {
        return HashMap::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let app_id = item["appid"].as_u64()?;
            let assets = &item["assets"];
            let format = assets["asset_url_format"].as_str()?;
            let url = |key: &str| {
                let file = assets[key].as_str().filter(|f| !f.is_empty())?;
                Some(format!(
                    "{ASSET_BASE}{}",
                    format.replace("${FILENAME}", file)
                ))
            };
            Some((
                app_id,
                Artwork {
                    portrait: url("library_capsule"),
                    header: url("header"),
                    capsule: url("small_capsule"),
                    hero: url("library_hero"),
                },
            ))
        })
        .collect()
}

fn parse_movies(movies: &Value) -> Vec<Movie> {
    let Some(list) = movies.as_array() else {
        return Vec::new();
    };
    let mut movies: Vec<(bool, Movie)> = list
        .iter()
        .map(|m| {
            let movie = Movie {
                name: string(m, "name").unwrap_or_else(|| "Trailer".into()),
                thumbnail: string(m, "thumbnail"),
                hls: string(m, "hls_h264"),
                mp4: string(&m["mp4"], "max").or_else(|| string(&m["mp4"], "480")),
            };
            (m["highlight"].as_bool().unwrap_or(false), movie)
        })
        .filter(|(_, m)| m.hls.is_some() || m.mp4.is_some())
        .collect();
    movies.sort_by_key(|(highlight, _)| !highlight);
    movies.into_iter().map(|(_, m)| m).collect()
}

pub fn normalize_name(name: &str) -> String {
    name.replace('&', " and ")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

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
            "metacritic": null,
            "movies": [
                { "id": 1, "name": "Teaser", "thumbnail": "t1.jpg", "mp4": { "480": "a.mp4", "max": "b.mp4" } },
                { "id": 2, "name": "Official Trailer", "thumbnail": "t2.jpg",
                  "hls_h264": "https://video.example/hls_264_master.m3u8", "dash_h264": "x.mpd", "highlight": true },
                { "id": 3, "name": "Broken" }
            ]
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
        let names: Vec<_> = app.movies.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Official Trailer", "Teaser"]);
        assert_eq!(
            app.movies[0].hls.as_deref(),
            Some("https://video.example/hls_264_master.m3u8")
        );
        assert_eq!(app.movies[1].mp4.as_deref(), Some("b.mp4"));

        assert_eq!(
            app.short_description.as_deref(),
            Some("A roguelike auto-battler.")
        );

        let missing = json!({ "1": { "success": false } });
        assert_eq!(parse_app_details(&missing, 1), None);
    }

    #[test]
    fn decodes_entities() {
        let body = json!({ "7": { "success": true, "data": {
            "name": "Sword &amp; Sorcery",
            "short_description": "Fast &amp; furious &quot;co-op&quot; &#39;fun&#39; &lt;3 &#x263A;"
        }}});
        let app = parse_app_details(&body, 7).unwrap();
        assert_eq!(app.name, "Sword & Sorcery");
        assert_eq!(
            app.short_description.as_deref(),
            Some("Fast & furious \"co-op\" 'fun' <3 \u{263A}")
        );
        assert_eq!(
            decode_entities("Plain text, no entities"),
            "Plain text, no entities"
        );
        assert_eq!(decode_entities("Rock & roll"), "Rock & roll");
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
        let body = json!({ "total": 2, "items": [
            { "type": "app", "id": 2124490, "name": "SILENT HILL 2", "tiny_image": "https://cdn.example/capsule.jpg" },
            { "type": "app", "id": 4423570, "name": "FATAL FRAME II x SILENT HILL f Costume Set", "tiny_image": "x" }
        ]});
        let found = pick_match(&body, "Silent Hill 2").unwrap();
        assert_eq!(
            (found.app_id, found.capsule_image.as_deref()),
            (2124490, Some("https://cdn.example/capsule.jpg"))
        );
        assert_eq!(pick_match(&body, "Silent Hill"), None);
        assert_eq!(
            pick_match(&json!({ "total": 0, "items": [] }), "Nothing"),
            None
        );
    }

    #[test]
    fn artwork() {
        let body = json!({ "response": { "store_items": [
            { "appid": 4459590, "assets": {
                "asset_url_format": "steam/apps/4459590/${FILENAME}?t=1795587406",
                "library_capsule": "192507d5/library_capsule.jpg",
                "header": "560f439f/header.jpg",
                "small_capsule": "d64e41eb/capsule_231x87.jpg",
                "library_hero": "a1b2c3d4/library_hero.jpg"
            }},
            { "appid": 2124490, "assets": {
                "asset_url_format": "steam/apps/2124490/${FILENAME}?t=1744248682",
                "library_capsule": "library_600x900.jpg",
                "header": "header.jpg"
            }},
            { "appid": 10, "assets": { "asset_url_format": "steam/apps/10/${FILENAME}", "header": "header.jpg" } },
            { "appid": 11 }
        ]}});
        let art = parse_artwork(&body);
        assert_eq!(
            art[&4459590].portrait.as_deref(),
            Some(
                "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/4459590/192507d5/library_capsule.jpg?t=1795587406"
            )
        );
        assert!(
            art[&2124490]
                .portrait
                .as_deref()
                .unwrap()
                .ends_with("/2124490/library_600x900.jpg?t=1744248682")
        );
        assert_eq!(art[&2124490].capsule, None);
        assert!(
            art[&4459590]
                .hero
                .as_deref()
                .unwrap()
                .ends_with("/a1b2c3d4/library_hero.jpg?t=1795587406")
        );
        assert_eq!(art[&2124490].hero, None);
        assert_eq!(art[&10].portrait, None);
        assert!(art[&10].header.is_some());
        assert!(!art.contains_key(&11));
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
