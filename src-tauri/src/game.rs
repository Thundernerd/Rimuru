use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::phpbb::{LinkKind, Post, PostLink};
use crate::steam;

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
    pub platforms: Vec<String>,
    pub features: Vec<String>,
    pub description: Option<String>,
    pub system_requirements: Vec<RequirementSet>,
    pub fields: Vec<Field>,
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequirementSet {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Release {
    pub post_id: u64,
    pub author: Option<String>,
    pub posted: Option<String>,
    pub title: Option<String>,
    pub details: Vec<String>,
    pub version: Option<String>,
    pub build: Option<String>,
    pub size: Option<String>,
    pub tags: Vec<String>,
    pub links: Vec<DownloadLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadLink {
    pub url: String,
    pub host: String,
    pub label: String,
    pub flagged_unsafe: bool,
}

const NOT_DOWNLOAD_HOSTS: &[&str] = &[
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
    "youtube.com",
    "youtu.be",
    "twitch.tv",
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
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[^a-z0-9])v(\d+(?:\.\d+)+[a-z]?)\.?(?:[^a-z0-9.]|$)").unwrap());
static SIZE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d+(?:[.,]\d+)?)\s*(KB|MB|GB|TB|KiB|MiB|GiB|TiB)\b").unwrap()
});
static CSF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)clean steam files|\bCSF\b").unwrap());
static PRE_INSTALLED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)pre-?installed").unwrap());
static GOG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bGOG\b").unwrap());
static URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://\S+").unwrap());
static LEAD_IN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:(?:hi|hello|hey|yo|greetings)(?:\s+(?:all|everyone|guys))?\b[\s,!.:-]*)?(?:(?:here(?:'s|\s+is|\s+are)?(?:\s+you\s+go)?|as\s+requested|uploaded)\b[\s,!.:-]*)?(?:(?:the|a|an|my)\s+)?",
    )
    .unwrap()
});
static SKIP_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:thanks?\b|thank\s+you|enjoy\b|with\s+the\s+usual\s+password|the\s+password\b|password\s*:|have\s+fun|cheers\b)").unwrap()
});
static RELEASE_MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)clean steam files|\bCSF\b|pre-?installed|\bGOG\b|\brepack\b|\bportable\b|\bbuild\b|\bversion\b|(?:^|[^a-z0-9])v\d+(?:\.\d+)+|\bwin(?:32|64)\b|\bupdate\b|\bhotfix\b|\bdlcs?\b|\bcrack(?:ed)?\b|\bfix\b|\bbranch\b").unwrap()
});
static FILE_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\S+\.(?:exe|zip|rar|7z|iso|bin|tar|gz)$").unwrap());

fn tidy_line(line: &str) -> String {
    let without_urls = URL.replace_all(line, " ");
    let trimmed = without_urls
        .trim_start_matches(|c: char| !(c.is_alphanumeric() || c == '[' || c == '('))
        .trim();
    let rest = LEAD_IN.replace(trimmed, "");
    rest.trim()
        .trim_end_matches(|c: char| matches!(c, ':' | '-' | '|' | '~' | '⚠') || c.is_whitespace())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn release_title(details: &[String], game: Option<&str>) -> Option<String> {
    let game_key = game.map(steam::normalize_name).filter(|k| !k.is_empty());
    let mut best: Option<(f32, String)> = None;
    for (index, line) in details.iter().enumerate() {
        if SKIP_LINE.is_match(line.trim()) {
            continue;
        }
        let text = tidy_line(line);
        if text.chars().filter(|c| c.is_alphanumeric()).count() < 3 {
            continue;
        }
        let mentions_game = game_key
            .as_ref()
            .is_some_and(|k| steam::normalize_name(&text).contains(k.as_str()));
        let mut score = -0.25 * index as f32;
        if mentions_game {
            score += 5.0;
        }
        if RELEASE_MARKER.is_match(&text) {
            score += 3.0;
        }
        if FILE_NAME.is_match(&text) {
            score -= 6.0;
        }
        if text.len() > 100 {
            score -= 3.0;
        }
        if text.ends_with('?') {
            score -= 3.0;
        }
        if score >= 1.0 && best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, text));
        }
    }
    let game = game.map(str::trim).filter(|g| !g.is_empty());
    match (best, game) {
        (Some((_, text)), Some(game)) => {
            let has_game = steam::normalize_name(&text).contains(&steam::normalize_name(game));
            Some(if has_game {
                text
            } else {
                format!("{game} — {text}")
            })
        }
        (Some((_, text)), None) => Some(text),
        (None, game) => game.map(str::to_string),
    }
}

fn is_download(link: &PostLink) -> bool {
    let host = link.host.as_str();
    let listed = NOT_DOWNLOAD_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")));
    let forum = host.starts_with("forum.") || host.starts_with("forums.");
    link.kind == LinkKind::External && !link.in_quote && !listed && !forum
}

pub fn parse_release(post: &Post, game: Option<&str>) -> Option<Release> {
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
    if details.iter().any(|l| GOG.is_match(l)) {
        tags.push("GOG".to_string());
    }

    Some(Release {
        post_id: post.id,
        author: post.author.clone(),
        posted: post.posted.clone(),
        title: release_title(&details, game),
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
        let swbf = parse_game_info(&fixture(80048).posts[0]).unwrap();
        assert_eq!(swbf.fields[0].key, "Steam Release Date");
    }

    #[test]
    fn release_titles() {
        let titles = |t| {
            let page = fixture(t);
            let game = page.title.as_deref().map(crate::phpbb::topic_name);
            page.posts
                .iter()
                .filter_map(|p| parse_release(p, game.as_deref()))
                .map(|r| (r.author.unwrap_or_default(), r.title.unwrap_or_default()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            titles(159639),
            [
                ("kektin.com".into(), "Slotbound".into()),
                ("userunicode".into(), "Slotbound (Clean Steam Files)".into()),
                ("pretendme".into(), "Slotbound (Clean Steam Files)".into()),
                (
                    "DicDale".into(),
                    "Slotbound (v1.0.0.6 | Build 26665715) (149 MB)".into()
                ),
            ]
        );
        assert_eq!(
            titles(135719),
            [
                ("SasKP".into(), "Wizordum".into()),
                ("irukanjji".into(), "Wizordum — GOG version".into()),
                ("irukanjji".into(), "Wizordum — GOG version".into()),
                (
                    "Dude Dude".into(),
                    "Wizordum [Win64] [Branch: Public] (Clean Steam Files)".into()
                ),
            ]
        );
    }

    #[test]
    fn title_lines() {
        let title = |lines: &[&str]| {
            let lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            release_title(&lines, Some("Nine Sols"))
        };
        assert_eq!(
            title(&["Hi", "Nine Sols v1.2 repack"]).as_deref(),
            Some("Nine Sols v1.2 repack")
        );
        assert_eq!(
            title(&["Hello guys! Here is the update v1.3"]).as_deref(),
            Some("Nine Sols — update v1.3")
        );
        assert_eq!(
            title(&["Thanks to X for the crack", "https://gofile.io/d/abc"]).as_deref(),
            Some("Nine Sols")
        );
        assert_eq!(
            title(&["Does anyone have the DLC?", "Nine Sols + DLC"]).as_deref(),
            Some("Nine Sols + DLC")
        );
    }

    #[test]
    fn gog_tag_and_file_versions() {
        let page = fixture(135719);
        let gog = page
            .posts
            .iter()
            .filter_map(|p| parse_release(p, Some("Wizordum")))
            .find(|r| r.author.as_deref() == Some("irukanjji"))
            .unwrap();
        assert_eq!(gog.tags, ["GOG"]);
        assert_eq!(gog.version.as_deref(), Some("0.2.0.4"));
    }

    #[test]
    fn releases_across_topics() {
        let releases = |t| {
            let page = fixture(t);
            let game = page.title.as_deref().map(crate::phpbb::topic_name);
            page.posts
                .iter()
                .filter_map(|p| parse_release(p, game.as_deref()))
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
        assert!(
            wizordum
                .iter()
                .all(|r| r.details.iter().all(|l| !l.contains("[READ ME]")))
        );
        let csf = &wizordum[3];
        assert_eq!(csf.build.as_deref(), Some("16222355"));
        assert_eq!(csf.tags, ["Clean Steam Files"]);

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
        let releases: Vec<_> = page
            .posts
            .iter()
            .filter_map(|p| parse_release(p, Some("Slotbound")))
            .collect();
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
