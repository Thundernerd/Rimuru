//! What a game topic means: the game's details from the first post, and the
//! releases (posts with download links) from the replies.
//!
//! Works on parsed [`Post`]s, so it doesn't touch HTML.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::phpbb::{LinkKind, Post, PostLink};

// ---------------------------------------------------------------- game info

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GameInfo {
    pub title: Option<String>,
    pub header_image: Option<String>,
    pub store_url: Option<String>,
    pub steamdb_url: Option<String>,
    pub steam_app_id: Option<u64>,
    pub genres: Vec<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub languages: Vec<String>,
    /// `windows`, `macos`, `linux`, from the "Operating system(s)" icons.
    pub platforms: Vec<String>,
    pub features: Vec<String>,
    /// The "About The Game" text, one paragraph per line.
    pub description: Option<String>,
    pub system_requirements: Vec<RequirementSet>,
    /// Any other `Key: value` lines from the details section.
    pub fields: Vec<Field>,
    /// Images other than the header and platform icons, e.g. screenshots.
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequirementSet {
    /// e.g. `Minimum` or `Recommended`; empty if the post has no label.
    pub label: String,
    pub items: Vec<Field>,
}

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Intro,
    Details,
    Features,
    About,
    Requirements,
}

fn section_heading(line: &str) -> Option<Section> {
    match line
        .trim()
        .trim_end_matches(':')
        .to_ascii_lowercase()
        .as_str()
    {
        "details" => Some(Section::Details),
        "features" => Some(Section::Features),
        "about the game" | "about this game" | "description" => Some(Section::About),
        "system requirements" => Some(Section::Requirements),
        _ => None,
    }
}

/// `Key: value`, with a short key so prose containing a colon isn't split.
fn key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    (!key.is_empty() && key.len() <= 40 && !key.contains("http")).then(|| (key, value.trim()))
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Reads the game details a topic's first post usually holds: a title, then
/// "Details" (`Store Page:`, `Genre(s):`, ...), "Features", "About The
/// Game" and "System Requirements". Returns `None` when the post has none
/// of those, so it isn't a game info post.
pub fn parse_game_info(post: &Post) -> Option<GameInfo> {
    let mut info = GameInfo::default();
    let mut section = Section::Intro;
    let mut found_sections = false;
    let mut about = Vec::new();
    let links_on = |i: usize| {
        post.links
            .iter()
            .filter(move |l| l.line == i && !l.in_quote)
    };
    let mut platform_icons = Vec::new();

    for (i, line) in post.lines.iter().enumerate().filter(|(_, l)| !l.quoted) {
        let text = line.text.as_str();
        if let Some(s) = section_heading(text) {
            section = s;
            found_sections = true;
            continue;
        }
        match section {
            Section::Intro => {
                if info.title.is_none() {
                    info.title = Some(text.to_string());
                }
            }
            Section::Details => {
                let Some((key, value)) = key_value(text) else {
                    continue;
                };
                let link = |kind| links_on(i).find(|l| l.kind == kind).map(|l| l.url.clone());
                match key.to_ascii_lowercase().as_str() {
                    "store page" => info.store_url = link(LinkKind::SteamStore),
                    "steamdb page" => info.steamdb_url = link(LinkKind::SteamDb),
                    "genre(s)" | "genre" | "genres" => info.genres = split_list(value),
                    "developer" | "developer(s)" => info.developer = Some(value.to_string()),
                    "publisher" | "publisher(s)" => info.publisher = Some(value.to_string()),
                    "release date" => info.release_date = Some(value.to_string()),
                    "language(s)" | "languages" => info.languages = split_list(value),
                    "operating system(s)" | "operating systems" | "platforms" => {
                        platform_icons.push(i);
                    }
                    _ if !value.is_empty() => info.fields.push(Field {
                        key: key.to_string(),
                        value: value.to_string(),
                    }),
                    _ => {}
                }
            }
            Section::Features => info.features.push(text.to_string()),
            Section::About => about.push(text.to_string()),
            Section::Requirements => match key_value(text) {
                Some((label, "")) => info.system_requirements.push(RequirementSet {
                    label: label.to_string(),
                    items: Vec::new(),
                }),
                Some((key, value)) => {
                    if info.system_requirements.is_empty() {
                        info.system_requirements.push(RequirementSet {
                            label: String::new(),
                            items: Vec::new(),
                        });
                    }
                    let set = info.system_requirements.last_mut().unwrap();
                    set.items.push(Field {
                        key: key.to_string(),
                        value: value.to_string(),
                    });
                }
                None => {}
            },
        }
    }
    if !found_sections {
        return None;
    }

    if let Some(url) = &info.store_url {
        info.steam_app_id = STEAM_APP_ID.captures(url).and_then(|c| c[1].parse().ok());
    }
    info.description = (!about.is_empty()).then(|| about.join("\n"));

    // The platform icons sit on the "Operating system(s):" line. The header
    // image is the first other image. Spacer images pad the feature list.
    for image in post.images.iter().filter(|img| !img.in_quote) {
        let src = image.src.as_str();
        if src.contains("spacer.gif") || src.contains("/public/images/") {
            continue;
        }
        let platform = ["win", "mac", "linux"]
            .into_iter()
            .find(|p| src.contains(&format!("icon_platform_{p}")));
        if let Some(platform) = platform {
            let name = match platform {
                "win" => "windows",
                "mac" => "macos",
                _ => "linux",
            };
            if platform_icons.contains(&image.line) || !info.platforms.contains(&name.into()) {
                info.platforms.push(name.into());
            }
        } else if info.header_image.is_none() {
            info.header_image = Some(src.to_string());
        } else {
            info.images.push(src.to_string());
        }
    }
    info.platforms.dedup();
    Some(info)
}

// ---------------------------------------------------------------- releases

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Release {
    pub post_id: u64,
    pub author: Option<String>,
    pub posted: Option<String>,
    /// The post's first line, e.g. `Slotbound (v1.0.0.6 | Build 26665715) (149 MB)`.
    pub title: Option<String>,
    /// The post's own lines (no quotes), for showing alongside the links.
    pub details: Vec<String>,
    pub version: Option<String>,
    pub build: Option<String>,
    pub size: Option<String>,
    /// e.g. `Clean Steam Files`, `Pre-installed`.
    pub tags: Vec<String>,
    pub links: Vec<DownloadLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadLink {
    pub url: String,
    pub host: String,
    /// The link's text, or the host when the text is just the URL.
    pub label: String,
    /// Wrapped in the forum's "Unsafe filehosts" warning.
    pub flagged_unsafe: bool,
}

/// External sites that show up in posts but aren't downloads: stores,
/// communities, wikis, video and image hosts. Subdomains match too.
const NOT_DOWNLOAD_HOSTS: &[&str] = &[
    // stores and launchers
    "steampowered.com",
    "steamstatic.com",
    "epicgames.com",
    "gog.com",
    "ubisoft.com",
    "ea.com",
    "xbox.com",
    "microsoft.com",
    "humblebundle.com",
    "itch.io",
    // communities and docs
    "steamcommunity.com",
    "discord.gg",
    "discord.com",
    "github.com",
    "reddit.com",
    "fmhy.net",
    "pcgamingwiki.com",
    "wikipedia.org",
    "twitter.com",
    "x.com",
    // media
    "youtube.com",
    "youtu.be",
    "twitch.tv",
    // images
    "imgur.com",
    "ibb.co",
    "imgbb.com",
    "postimg.cc",
    "postimages.org",
    "prnt.sc",
    "prntscr.com",
    "gyazo.com",
    "imagebam.com",
    "tinypic.com",
];

static STEAM_APP_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/app/(\d+)").unwrap());
static BUILD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bbuild\b\s*:?\s*(\d{5,})").unwrap());
static VERSION_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*version\s*:\s*([^\[|]+)").unwrap());
static VERSION_V: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bv(\d+(?:\.\d+)+[a-z]?)\b").unwrap());
static SIZE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d+(?:[.,]\d+)?)\s*(KB|MB|GB|TB|KiB|MiB|GiB|TiB)\b").unwrap()
});
static CSF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)clean steam files|\bCSF\b").unwrap());
static PRE_INSTALLED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)pre-?installed").unwrap());

fn is_download(link: &PostLink) -> bool {
    let host = link.host.as_str();
    let listed = NOT_DOWNLOAD_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")));
    // Other sites' forums, e.g. forums.focus-home.com.
    let forum = host.starts_with("forum.") || host.starts_with("forums.");
    link.kind == LinkKind::External && !link.in_quote && !listed && !forum
}

/// A post is a release when it has download links of its own, i.e. outside
/// quotes and not to Steam, the forum or community sites.
pub fn parse_release(post: &Post) -> Option<Release> {
    let links: Vec<DownloadLink> = post
        .links
        .iter()
        .filter(|l| is_download(l))
        .map(|l| DownloadLink {
            url: l.url.clone(),
            host: l.host.clone(),
            label: if l.text.is_empty() || l.text.starts_with("http") {
                l.host.clone()
            } else {
                l.text.clone()
            },
            flagged_unsafe: l.flagged_unsafe,
        })
        .collect();
    if links.is_empty() {
        return None;
    }

    let details: Vec<String> = post
        .lines
        .iter()
        .filter(|l| !l.quoted)
        .map(|l| l.text.clone())
        .collect();
    let find = |re: &Regex| {
        details
            .iter()
            .find_map(|l| Some(re.captures(l)?[1].trim().to_string()))
    };

    let mut tags = Vec::new();
    if details.iter().any(|l| CSF.is_match(l)) {
        tags.push("Clean Steam Files".to_string());
    }
    if details.iter().any(|l| PRE_INSTALLED.is_match(l)) {
        tags.push("Pre-installed".to_string());
    }

    Some(Release {
        post_id: post.id,
        author: post.author.clone(),
        posted: post.posted.clone(),
        title: details.first().cloned(),
        version: find(&VERSION_LINE).or_else(|| find(&VERSION_V)),
        build: find(&BUILD),
        size: details
            .iter()
            .find_map(|l| SIZE.captures(l).map(|c| format!("{} {}", &c[1], &c[2]))),
        tags,
        details,
        links,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phpbb::{PostLink, TopicPage, parse_topic};
    use url::Url;

    fn topic() -> TopicPage {
        let html = include_str!("../tests/fixtures/viewtopic_php_t_159639.html");
        let url = Url::parse("https://cs.rin.ru/forum/viewtopic.php?t=159639&start=0").unwrap();
        parse_topic(html, &url, 159639, 0)
    }

    #[test]
    fn game_info_from_first_post() {
        let page = topic();
        let info = parse_game_info(&page.posts[0]).expect("first post is game info");
        assert_eq!(info.title.as_deref(), Some("Slotbound"));
        assert_eq!(
            info.store_url.as_deref(),
            Some("https://store.steampowered.com/app/4459590")
        );
        assert_eq!(
            info.steamdb_url.as_deref(),
            Some("https://steamdb.info/app/4459590")
        );
        assert_eq!(info.steam_app_id, Some(4459590));
        assert_eq!(info.genres, ["Indie", "Strategy"]);
        assert_eq!(info.developer.as_deref(), Some("Optima Arch"));
        assert_eq!(info.publisher.as_deref(), Some("Optima Arch"));
        assert_eq!(info.release_date.as_deref(), Some("Coming soon"));
        assert_eq!(info.languages.len(), 9);
        assert_eq!(info.languages[0], "English");
        assert_eq!(info.platforms, ["windows"]);
        assert_eq!(info.features[0], "Single-player");
        assert!(info.features.contains(&"Family Sharing".to_string()));
        assert!(
            info.description
                .as_deref()
                .unwrap()
                .starts_with("Master your luck")
        );
        assert!(
            info.header_image
                .as_deref()
                .unwrap()
                .contains("/header.jpg")
        );
        assert!(!info.images.is_empty());

        let labels: Vec<_> = info
            .system_requirements
            .iter()
            .map(|s| s.label.as_str())
            .collect();
        assert_eq!(labels, ["Minimum", "Recommended"]);
        let min = &info.system_requirements[0].items;
        assert_eq!(
            (min[0].key.as_str(), min[0].value.as_str()),
            ("OS", "Windows 10 (64-bit)")
        );
        // Requirement lines don't leak into the details' extra fields.
        assert!(info.fields.iter().all(|f| f.key != "OS"));
    }

    fn fixture(topic_id: u64) -> TopicPage {
        let html =
            std::fs::read_to_string(format!("tests/fixtures/viewtopic_php_t_{topic_id}.html"))
                .unwrap();
        let url = Url::parse(&format!(
            "https://cs.rin.ru/forum/viewtopic.php?t={topic_id}&start=0"
        ))
        .unwrap();
        parse_topic(&html, &url, topic_id, 0)
    }

    #[test]
    fn game_info_across_topics() {
        // (topic, title, app id, developer, has requirements)
        let cases = [
            (135719, "Wizordum", 1715590, "Emberheart Games", true),
            (
                80048,
                "Star Wars Battlefront II (2017)",
                1237950,
                "DICE",
                true,
            ),
            (
                91289,
                "Workers & Resources: Soviet Republic",
                784150,
                "3Division",
                false,
            ),
            (91823, "World War Z", 699130, "Saber Interactive Inc", true),
        ];
        for (topic_id, title, app_id, developer, has_reqs) in cases {
            let page = fixture(topic_id);
            let info =
                parse_game_info(&page.posts[0]).unwrap_or_else(|| panic!("no info in {topic_id}"));
            assert_eq!(info.title.as_deref(), Some(title));
            assert_eq!(info.steam_app_id, Some(app_id), "{title}");
            assert_eq!(info.developer.as_deref(), Some(developer), "{title}");
            assert!(
                info.header_image.as_deref().unwrap().contains("header.jpg"),
                "{title}"
            );
            assert!(
                !info.genres.is_empty() && !info.languages.is_empty(),
                "{title}"
            );
            assert!(info.description.is_some(), "{title}");
            assert_eq!(!info.system_requirements.is_empty(), has_reqs, "{title}");
        }
        // Extra detail lines are kept as fields.
        let swbf = parse_game_info(&fixture(80048).posts[0]).unwrap();
        assert_eq!(swbf.fields[0].key, "Steam Release Date");
    }

    #[test]
    fn releases_across_topics() {
        let releases = |t| {
            fixture(t)
                .posts
                .iter()
                .filter_map(parse_release)
                .collect::<Vec<_>>()
        };

        let wizordum = releases(135719);
        let hosts: Vec<_> = wizordum.iter().map(|r| r.links[0].host.as_str()).collect();
        assert_eq!(
            hosts,
            [
                "privatebin.rinuploads.org",
                "1fichier.com",
                "1fichier.com",
                "pixeldrain.com"
            ]
        );
        assert!(wizordum[0].links[0].flagged_unsafe);
        // The forum's unsafe-filehost note doesn't end up in the text.
        assert!(
            wizordum
                .iter()
                .all(|r| r.details.iter().all(|l| !l.contains("[READ ME]")))
        );
        let csf = &wizordum[3];
        assert_eq!(csf.build.as_deref(), Some("16222355"));
        assert_eq!(csf.tags, ["Clean Steam Files"]);

        // Store pages and other sites' forums are not releases.
        assert!(releases(91823).is_empty());
        assert!(releases(80048).is_empty());
    }

    #[test]
    fn image_hosts_are_not_downloads() {
        let link = |host: &str| PostLink {
            url: format!("https://{host}/x"),
            host: host.into(),
            kind: LinkKind::External,
            text: String::new(),
            context: String::new(),
            line: 0,
            in_quote: false,
            flagged_unsafe: false,
        };
        for host in [
            "ibb.co",
            "i.ibb.co",
            "imgur.com",
            "i.imgur.com",
            "postimg.cc",
            "prnt.sc",
        ] {
            assert!(!is_download(&link(host)), "{host}");
        }
        for host in ["gofile.io", "pixeldrain.com", "1fichier.com"] {
            assert!(is_download(&link(host)), "{host}");
        }
    }

    #[test]
    fn replies_are_not_game_info() {
        let page = topic();
        assert!(page.posts[1..].iter().all(|p| parse_game_info(p).is_none()));
    }

    #[test]
    fn releases() {
        let page = topic();
        let releases: Vec<_> = page.posts.iter().filter_map(parse_release).collect();
        let by_author = |a: &str| {
            releases
                .iter()
                .find(|r| r.author.as_deref() == Some(a))
                .unwrap()
        };
        let hosts = |r: &Release| r.links.iter().map(|l| l.host.clone()).collect::<Vec<_>>();

        assert_eq!(
            releases.len(),
            4,
            "{:#?}",
            releases.iter().map(|r| &r.author).collect::<Vec<_>>()
        );
        // The game info post links only to Steam, so it isn't a release.
        assert!(releases.iter().all(|r| r.post_id != page.posts[0].id));

        let kektin = by_author("kektin.com");
        assert_eq!(hosts(kektin), ["kektin.com", "fileditchhot.st"]);
        assert_eq!(kektin.build.as_deref(), Some("26643956"));
        assert_eq!(kektin.size.as_deref(), Some("179.99 MB"));
        assert_eq!(kektin.tags, ["Pre-installed"]);

        let csf = by_author("userunicode");
        assert_eq!(hosts(csf), ["gofile.io", "vikingfile.com"]);
        assert_eq!(csf.links[0].label, "Gofile");
        assert_eq!(csf.build.as_deref(), Some("26735285"));
        assert_eq!(csf.tags, ["Clean Steam Files"]);
        // Signature links (gbe_fork on GitHub, RAC/SAC on the forum) are not included.
        assert_eq!(csf.links.len(), 2);

        let pretend = by_author("pretendme");
        assert_eq!(pretend.version.as_deref(), Some("1.0.0.4"));
        assert_eq!(pretend.build.as_deref(), Some("26651617"));
        assert_eq!(pretend.links[0].label, "gofile.io");

        let zei = by_author("DicDale");
        assert_eq!(hosts(zei), ["zeigames.com", "filesyard.com"]);
        assert!(zei.links[0].flagged_unsafe && !zei.links[1].flagged_unsafe);
        assert_eq!(
            zei.title.as_deref(),
            Some("Slotbound (v1.0.0.6 | Build 26665715) (149 MB)")
        );
        assert_eq!(zei.version.as_deref(), Some("1.0.0.6"));
        assert_eq!(zei.build.as_deref(), Some("26665715"));
        assert_eq!(zei.size.as_deref(), Some("149 MB"));
    }
}
