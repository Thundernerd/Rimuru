//! Parsing of cs.rin.ru's phpBB 3 pages.
//!
//! The site runs phpBB 3 with "rinDark", a theme based on subsilver2: forum
//! listings and posts are tables. Tests run against real pages saved with
//! `save_snapshot` in `tests/fixtures/` (personal data replaced).

use std::collections::HashSet;
use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

macro_rules! selector {
    ($name:ident, $css:expr) => {
        static $name: LazyLock<Selector> = LazyLock::new(|| Selector::parse($css).unwrap());
    };
}

selector!(TR, "tr");
selector!(SECTION_HEADER, "td.row3");
selector!(TOPIC_TITLE, "a.topictitle");
selector!(PROFILE_LINK, r#"a[href*="mode=viewprofile"]"#);
selector!(TOPIC_DETAILS, "p.topicdetails");
selector!(HEADING_LINK, "h2 a");
selector!(TITLE, "title");
selector!(NAV, "td.nav");
selector!(START_LINK, r#"a[href*="start="]"#);
selector!(LOGOUT_LINK, r#"a[href*="mode=logout"]"#);
selector!(POST_TABLE, "table.tablebg");
selector!(POST_ANCHOR, r#"a[name^="p"]"#);
selector!(POST_AUTHOR, "b.postauthor");
selector!(POST_HEADER, "td.gensmall");
selector!(POST_BODY, "div.postbody");

// ---------------------------------------------------------------- types

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ForumPage {
    pub forum_id: u64,
    pub title: Option<String>,
    pub topics: Vec<TopicSummary>,
    pub pagination: Pagination,
}

/// The listing's section a topic appears in. Everything except `Normal` is
/// repeated at the top of every page of the forum.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TopicKind {
    GlobalAnnouncement,
    Announcement,
    Sticky,
    Normal,
}

impl TopicKind {
    pub fn is_pinned(self) -> bool {
        self != TopicKind::Normal
    }

    fn from_section(header: &str) -> Self {
        match header.trim().to_ascii_lowercase().as_str() {
            "global announcements" => TopicKind::GlobalAnnouncement,
            "announcements" => TopicKind::Announcement,
            "stickies" => TopicKind::Sticky,
            _ => TopicKind::Normal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopicSummary {
    pub id: u64,
    pub kind: TopicKind,
    /// The full title as shown, e.g. `[Info] Nine Sols [Early Access]`.
    pub title: String,
    /// The title without leading and trailing `[...]` tags, e.g. `Nine Sols`.
    pub name: String,
    /// Leading tags, e.g. `["Info"]`.
    pub prefixes: Vec<String>,
    /// Trailing tags, e.g. `["Early Access"]`.
    pub labels: Vec<String>,
    pub url: String,
    pub author: Option<String>,
    /// As displayed by the forum, e.g. `Monday, 29 Jun 2026, 01:18`.
    pub created: Option<String>,
    pub replies: Option<u32>,
    pub views: Option<u32>,
    /// As displayed by the forum, e.g. `Today, 12:21`.
    pub last_post_at: Option<String>,
    pub last_poster: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TopicPage {
    pub topic_id: u64,
    pub title: Option<String>,
    pub posts: Vec<Post>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Post {
    pub id: u64,
    pub author: Option<String>,
    pub subject: Option<String>,
    /// As displayed by the forum, e.g. `Friday, 13 Nov 2026, 13:57`.
    pub posted: Option<String>,
    /// The post body as untrusted forum HTML. Sanitise before rendering.
    pub content_html: String,
    /// The post body as text, one line per line break or block.
    pub content_text: String,
    /// The same text as `content_text`, line by line.
    pub lines: Vec<PostLine>,
    pub links: Vec<PostLink>,
    pub images: Vec<PostImage>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PostLine {
    pub text: String,
    /// Every word on the line is inside a quote of another post.
    pub quoted: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PostImage {
    pub src: String,
    pub in_quote: bool,
    /// Index into `Post::lines` of the line the image sits on (or before).
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PostLink {
    pub url: String,
    /// Host without `www.`, e.g. `gofile.io`.
    pub host: String,
    pub kind: LinkKind,
    pub text: String,
    /// The full line of the post the link sits on, e.g.
    /// `Gofile | VikingFile` or `Store Page: https://store.steampowered.com/...`.
    pub context: String,
    /// Index into `Post::lines` of the line the link sits on.
    pub line: usize,
    /// Inside a quote of another post, so usually a repeat.
    pub in_quote: bool,
    /// Wrapped in the forum's "Unsafe filehosts" warning.
    pub flagged_unsafe: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    SteamStore,
    SteamDb,
    /// Another page on cs.rin.ru.
    Forum,
    External,
}

/// phpBB pages by `start=` offset (items, not page numbers).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Pagination {
    pub start: u32,
    pub next_start: Option<u32>,
    pub previous_start: Option<u32>,
    pub last_start: u32,
    pub page: Option<u32>,
    pub total_pages: Option<u32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Session {
    LoggedIn { username: Option<String> },
    LoggedOut,
}

// ---------------------------------------------------------------- session

/// True for the site's JavaScript "Security check" interstitial.
pub fn is_security_check(html: &str) -> bool {
    html.contains("Security check") && html.contains("securitytoken")
}

/// Signed in when the page has a logout link: `Logout [ name ]`.
pub fn parse_session(html: &str) -> Session {
    let doc = Html::parse_document(html);
    let Some(logout) = doc.select(&LOGOUT_LINK).next() else {
        return Session::LoggedOut;
    };
    let text = text_of(logout);
    let username = text
        .split_once('[')
        .and_then(|(_, rest)| rest.rsplit_once(']'))
        .map(|(name, _)| name.trim().to_string())
        .filter(|name| !name.is_empty());
    Session::LoggedIn { username }
}

// ---------------------------------------------------------------- forum

/// Parses `viewforum.php?f=<forum_id>&start=<start>`.
///
/// Section header rows (`td.row3`) set the [`TopicKind`] of the rows below.
pub fn parse_forum(html: &str, page_url: &Url, forum_id: u64, start: u32) -> ForumPage {
    let doc = Html::parse_document(html);
    ForumPage {
        forum_id,
        title: page_title(&doc),
        topics: parse_topic_rows(&doc, page_url),
        pagination: parse_pagination(&doc, page_url, start, |u| {
            query_u64(u, "f") == Some(forum_id)
        }),
    }
}

/// Where each column is in a topic listing, read from its header row
/// (`<th>Topics</th><th>Replies</th>...`), so listings that order their
/// columns differently (forum pages, search results) parse alike.
#[derive(Debug, Default, Clone)]
struct Columns {
    replies: Option<usize>,
    author: Option<usize>,
    views: Option<usize>,
    last_post: Option<usize>,
}

impl Columns {
    /// The forum page's layout: icon, title, replies, author, views, last post.
    fn forum_default() -> Self {
        Self {
            replies: Some(2),
            author: Some(3),
            views: Some(4),
            last_post: Some(5),
        }
    }

    /// From a row of `<th>`s, counting `colspan`; `None` if it isn't one.
    fn from_header(row: ElementRef) -> Option<Self> {
        let cells: Vec<ElementRef> = row.children().filter_map(ElementRef::wrap).collect();
        if cells.is_empty() || cells.iter().any(|c| c.value().name() != "th") {
            return None;
        }
        let mut columns = Self::default();
        let mut index = 0;
        for cell in cells {
            let span: usize = cell
                .value()
                .attr("colspan")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            let slot = match text_of(cell).to_ascii_lowercase().as_str() {
                "replies" => &mut columns.replies,
                "author" => &mut columns.author,
                "views" => &mut columns.views,
                "last post" => &mut columns.last_post,
                _ => {
                    index += span;
                    continue;
                }
            };
            *slot = Some(index);
            index += span;
        }
        (columns.replies.is_some() || columns.author.is_some()).then_some(columns)
    }
}

/// Topic rows (`tr`s with an `a.topictitle`) of a forum page or search
/// results, in order, without duplicates.
fn parse_topic_rows(doc: &Html, page_url: &Url) -> Vec<TopicSummary> {
    let mut kind = TopicKind::Normal;
    let mut columns = Columns::forum_default();
    let mut seen = HashSet::new();
    let mut topics = Vec::new();

    for row in doc.select(&TR) {
        if let Some(header) = Columns::from_header(row) {
            columns = header;
            continue;
        }
        if let Some(header) = row.select(&SECTION_HEADER).next() {
            kind = TopicKind::from_section(&text_of(header));
            continue;
        }
        let Some(link) = row.select(&TOPIC_TITLE).next() else {
            continue;
        };
        let Some(url) = link
            .value()
            .attr("href")
            .and_then(|h| page_url.join(h).ok())
        else {
            continue;
        };
        let Some(id) = query_u64(&url, "t") else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }

        // Cell positions, counting colspan, to match the header.
        let mut cells: Vec<Option<ElementRef>> = Vec::new();
        for cell in row
            .children()
            .filter_map(ElementRef::wrap)
            .filter(|el| el.value().name() == "td")
        {
            let span: usize = cell
                .value()
                .attr("colspan")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            cells.push(Some(cell));
            cells.extend(std::iter::repeat_n(None, span.saturating_sub(1)));
        }
        let cell = |i: Option<usize>| i.and_then(|i| cells.get(i).copied().flatten());
        let number = |i: Option<usize>| cell(i).and_then(|c| parse_count(&text_of(c)));
        let last_post = cell(columns.last_post);

        let title = text_of(link);
        let (prefixes, name, labels) = split_title(&title);
        topics.push(TopicSummary {
            id,
            kind,
            name,
            prefixes,
            labels,
            title,
            url: clean_url(url),
            author: cell(columns.author)
                .and_then(|c| c.select(&PROFILE_LINK).next())
                .map(text_of),
            created: link
                .value()
                .attr("title")
                .and_then(|t| t.strip_prefix("Posted:"))
                .map(|t| t.trim().to_string()),
            replies: number(columns.replies),
            views: number(columns.views),
            last_post_at: last_post
                .and_then(|c| c.select(&TOPIC_DETAILS).next())
                .map(text_of)
                .filter(|t| !t.is_empty()),
            last_poster: last_post
                .and_then(|c| c.select(&PROFILE_LINK).next())
                .map(text_of),
        });
    }
    topics
}

// ---------------------------------------------------------------- search

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchPage {
    pub query: String,
    pub topics: Vec<TopicSummary>,
    /// From "Search found N matches".
    pub total_matches: Option<u32>,
    pub pagination: Pagination,
}

/// Messages phpBB shows instead of results that mean "try again", rather
/// than "nothing found".
const SEARCH_REFUSALS: &[&str] = &[
    "cannot use search at this time",
    "You must specify at least one word",
    "You cannot search for that many",
    "search was terminated",
    "Sorry but you are not permitted to use the search",
];

/// Parses `search.php?keywords=...&sr=topics` results. Returns the forum's
/// message instead when it refused the search (e.g. searching again too
/// soon after the last search).
pub fn parse_search(
    html: &str,
    page_url: &Url,
    query: &str,
    start: u32,
) -> std::result::Result<SearchPage, String> {
    let doc = Html::parse_document(html);
    let topics = parse_topic_rows(&doc, page_url);
    if topics.is_empty() {
        let text = doc.root_element().text().collect::<String>();
        if let Some(message) = SEARCH_REFUSALS
            .iter()
            .find_map(|phrase| sentence_with(&text, phrase))
        {
            return Err(message);
        }
    }
    let total_matches = doc.root_element().text().find_map(|t| {
        let rest = t.split_once("Search found ")?.1;
        let digits: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == ',')
            .collect();
        digits.replace(',', "").parse().ok()
    });
    Ok(SearchPage {
        query: query.to_string(),
        topics,
        total_matches,
        pagination: parse_pagination(&doc, page_url, start, |u| {
            u.query_pairs()
                .any(|(k, _)| k == "keywords" || k == "search_id")
        }),
    })
}

/// The sentence of `text` that contains `phrase`, tidied up.
fn sentence_with(text: &str, phrase: &str) -> Option<String> {
    let at = text.find(phrase)?;
    let begin = text[..at].rfind(['.', '!', '?', '\n']).map_or(0, |i| i + 1);
    let end = text[at..]
        .find(['.', '!', '?', '\n'])
        .map_or(text.len(), |i| at + i + 1);
    Some(
        text[begin..end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Splits `[Info] Name [VR] [Early Access]` into prefixes, name and labels.
fn split_title(title: &str) -> (Vec<String>, String, Vec<String>) {
    let mut rest = title.trim();
    let mut prefixes = Vec::new();
    while let Some(after) = rest.strip_prefix('[') {
        let Some((tag, tail)) = after.split_once(']') else {
            break;
        };
        if tail.trim().is_empty() {
            break; // keep a name that is only a bracketed tag
        }
        prefixes.push(tag.trim().to_string());
        rest = tail.trim_start();
    }
    let mut labels = Vec::new();
    while let Some(before) = rest.strip_suffix(']') {
        let Some((head, tag)) = before.rsplit_once('[') else {
            break;
        };
        if head.trim().is_empty() {
            break; // keep a name that is only a bracketed tag
        }
        labels.insert(0, tag.trim().to_string());
        rest = head.trim_end();
    }
    (prefixes, rest.to_string(), labels)
}

// ---------------------------------------------------------------- topic

/// Parses `viewtopic.php?t=<topic_id>&start=<start>`.
///
/// Each post is its own `table.tablebg` holding an `<a name="p123">` anchor,
/// `b.postauthor`, a `td.gensmall` header with subject and date, and
/// `div.postbody` blocks: the first is the post, a second is the signature.
pub fn parse_topic(html: &str, page_url: &Url, topic_id: u64, start: u32) -> TopicPage {
    let doc = Html::parse_document(html);
    let mut posts = Vec::new();

    for table in doc.select(&POST_TABLE) {
        let Some(id) = table
            .select(&POST_ANCHOR)
            .find_map(|a| a.value().attr("name")?.strip_prefix('p')?.parse().ok())
        else {
            continue;
        };
        let Some(body) = table.select(&POST_BODY).next() else {
            continue;
        };

        let header = table
            .select(&POST_HEADER)
            .map(text_of)
            .find(|t| t.contains("Posted:"))
            .unwrap_or_default();
        let posted = header
            .split_once("Posted:")
            .map(|(_, rest)| rest.trim().to_string())
            .filter(|t| !t.is_empty());
        let subject = header
            .split_once("Post subject:")
            .map(|(_, rest)| {
                rest.split("Posted:")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .filter(|t| !t.is_empty());

        let walked = Walker::run(body, page_url);
        posts.push(Post {
            id,
            author: table.select(&POST_AUTHOR).next().map(text_of),
            subject,
            posted,
            content_html: body.inner_html().trim().to_string(),
            content_text: walked.text,
            lines: walked.lines,
            links: walked.links,
            images: walked.images,
        });
    }

    TopicPage {
        topic_id,
        title: page_title(&doc),
        posts,
        pagination: parse_pagination(&doc, page_url, start, |u| {
            query_u64(u, "t") == Some(topic_id)
        }),
    }
}

/// Turns a post body into lines of text and collects its links and images,
/// each tied to the line it sits on.
struct Walker<'a> {
    base: &'a Url,
    lines: Vec<PostLine>,
    current: String,
    current_quoted: bool,
    links: Vec<(PostLink, usize)>,
    images: Vec<PostImage>,
}

#[derive(Clone, Copy, Default)]
struct Scope {
    in_quote: bool,
    flagged_unsafe: bool,
}

struct Walked {
    text: String,
    lines: Vec<PostLine>,
    links: Vec<PostLink>,
    images: Vec<PostImage>,
}

impl<'a> Walker<'a> {
    fn run(body: ElementRef, base: &'a Url) -> Walked {
        let mut walker = Walker {
            base,
            lines: Vec::new(),
            current: String::new(),
            current_quoted: true,
            links: Vec::new(),
            images: Vec::new(),
        };
        walker.walk(body, Scope::default());
        walker.break_line();

        let lines = walker.lines;
        let links = walker
            .links
            .into_iter()
            .map(|(mut link, line)| {
                link.context = lines.get(line).map(|l| l.text.clone()).unwrap_or_default();
                link
            })
            .collect();
        Walked {
            text: lines
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            lines,
            links,
            images: walker.images,
        }
    }

    fn break_line(&mut self) {
        let text = self
            .current
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            self.lines.push(PostLine {
                text,
                quoted: self.current_quoted,
            });
        }
        self.current.clear();
        self.current_quoted = true;
    }

    fn push_text(&mut self, text: &str, scope: Scope) {
        if !text.trim().is_empty() && !scope.in_quote {
            self.current_quoted = false;
        }
        self.current.push_str(text);
    }

    fn walk(&mut self, el: ElementRef, scope: Scope) {
        let name = el.value().name();
        let has_class = |c: &str| el.value().classes().any(|x| x == c);
        if matches!(name, "script" | "style" | "video" | "audio") || has_class("link_unsafe_reveal")
            // The "Unsafe filehosts - [READ ME] ..." note next to flagged
            // links; the flag itself is kept on the link.
            || has_class("link_unsafe_note")
        {
            return;
        }
        let scope = Scope {
            in_quote: scope.in_quote || has_class("quotecontent") || has_class("quotetitle"),
            flagged_unsafe: scope.flagged_unsafe || has_class("link_unsafe"),
        };
        let block = matches!(
            name,
            "br" | "div"
                | "p"
                | "li"
                | "ul"
                | "ol"
                | "tr"
                | "table"
                | "blockquote"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
        );
        if block {
            self.break_line();
        }

        if name == "img"
            && let Some(src) = el.value().attr("src").and_then(|s| self.base.join(s).ok())
        {
            self.images.push(PostImage {
                src: src.to_string(),
                in_quote: scope.in_quote,
                line: self.lines.len(),
            });
        }

        let link = (name == "a")
            .then(|| el.value().attr("href"))
            .flatten()
            .and_then(|href| self.base.join(href).ok())
            .filter(|url| matches!(url.scheme(), "http" | "https"));

        for child in el.children() {
            if let Some(text) = child.value().as_text() {
                self.push_text(text, scope);
            } else if let Some(child) = ElementRef::wrap(child) {
                self.walk(child, scope);
            }
        }

        if let Some(url) = link {
            let host = url
                .host_str()
                .unwrap_or("")
                .trim_start_matches("www.")
                .to_string();
            let kind = match host.as_str() {
                "store.steampowered.com" => LinkKind::SteamStore,
                "steamdb.info" => LinkKind::SteamDb,
                "cs.rin.ru" => LinkKind::Forum,
                _ => LinkKind::External,
            };
            // The line isn't finished yet; its index is where it will land.
            let line = self.lines.len();
            self.links.push((
                PostLink {
                    url: url.to_string(),
                    host,
                    kind,
                    text: text_of(el),
                    context: String::new(),
                    line,
                    in_quote: scope.in_quote,
                    flagged_unsafe: scope.flagged_unsafe,
                },
                line,
            ));
        }
        if block {
            self.break_line();
        }
    }
}

// ---------------------------------------------------------------- shared

fn text_of(el: ElementRef) -> String {
    el.text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn query_u64(url: &Url, key: &str) -> Option<u64> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse().ok())
}

fn parse_count(text: &str) -> Option<u32> {
    let plain = text.replace(',', "");
    plain.trim().parse().ok()
}

/// Drops the `sid` parameter phpBB adds to links when cookies are missing.
fn clean_url(mut url: Url) -> String {
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "sid")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.query_pairs_mut().clear().extend_pairs(pairs);
    url.to_string()
}

fn page_title(doc: &Html) -> Option<String> {
    doc.select(&HEADING_LINK)
        .next()
        .map(text_of)
        .or_else(|| {
            // "CS RIN - Steam Underground • View forum - Main Forum"
            let title = text_of(doc.select(&TITLE).next()?);
            Some(
                title
                    .rsplit_once(" - ")
                    .map_or(title.clone(), |(_, t)| t.to_string()),
            )
        })
        .filter(|s| !s.is_empty())
}

/// `start=` offsets from links to the same listing (same script, and
/// `same_listing` holds), plus the "Page X of Y" label.
fn parse_pagination(
    doc: &Html,
    page_url: &Url,
    start: u32,
    same_listing: impl Fn(&Url) -> bool,
) -> Pagination {
    let mut offsets: Vec<u32> = doc
        .select(&START_LINK)
        .filter_map(|a| page_url.join(a.value().attr("href")?).ok())
        .filter(|u| u.path() == page_url.path() && same_listing(u))
        .filter_map(|u| query_u64(&u, "start").and_then(|s| u32::try_from(s).ok()))
        .collect();
    offsets.push(0);
    offsets.sort_unstable();
    offsets.dedup();

    // "Page 1 of 630"
    let (page, total_pages) = doc
        .select(&NAV)
        .map(text_of)
        .find_map(|t| {
            let rest = t.split_once("Page ")?.1;
            let (page, total) = rest.split_once(" of ")?;
            let total: String = total.chars().take_while(char::is_ascii_digit).collect();
            Some((page.trim().parse().ok()?, total.parse().ok()?))
        })
        .map_or((None, None), |(p, t)| (Some(p), Some(t)));

    Pagination {
        start,
        next_start: offsets.iter().copied().find(|&s| s > start),
        previous_start: offsets.iter().rev().copied().find(|&s| s < start),
        last_start: offsets.last().copied().unwrap_or(0).max(start),
        page,
        total_pages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORUM: &str = include_str!("../tests/fixtures/viewforum_php_f_10.html");
    const TOPIC: &str = include_str!("../tests/fixtures/viewtopic_php_t_159639.html");

    fn url(path: &str) -> Url {
        Url::parse("https://cs.rin.ru/forum/")
            .unwrap()
            .join(path)
            .unwrap()
    }

    fn forum() -> ForumPage {
        parse_forum(FORUM, &url("viewforum.php?f=10&start=0"), 10, 0)
    }

    fn topic() -> TopicPage {
        parse_topic(TOPIC, &url("viewtopic.php?t=159639&start=0"), 159639, 0)
    }

    fn post(page: &TopicPage, id: u64) -> &Post {
        page.posts.iter().find(|p| p.id == id).unwrap()
    }

    #[test]
    fn forum_sections() {
        let page = forum();
        assert_eq!(page.title.as_deref(), Some("Main Forum"));
        let count = |k| page.topics.iter().filter(|t| t.kind == k).count();
        assert_eq!(page.topics.len(), 108);
        assert_eq!(count(TopicKind::Normal), 91);
        assert!(count(TopicKind::GlobalAnnouncement) > 0);
        assert!(count(TopicKind::Sticky) > 0);
        // Pinned rows come first, then the normal topics.
        assert!(page.topics[..17].iter().all(|t| t.kind.is_pinned()));
    }

    #[test]
    fn forum_pinned_row() {
        let t = &forum().topics[0];
        assert_eq!(t.id, 159345);
        assert_eq!(t.kind, TopicKind::GlobalAnnouncement);
        assert_eq!(t.prefixes, ["Important"]);
        assert_eq!(t.name, "How to protect against malicious filehosters");
        assert_eq!(t.labels, ["26.11.26"]);
        assert_eq!(t.url, "https://cs.rin.ru/forum/viewtopic.php?f=10&t=159345");
        assert_eq!(t.author.as_deref(), Some("RessourectoR"));
        assert_eq!(t.created.as_deref(), Some("Wednesday, 17 Jun 2026, 18:44"));
        assert_eq!((t.replies, t.views), (Some(0), Some(256672)));
        assert_eq!(t.last_post_at.as_deref(), Some("Today, 12:21"));
        assert_eq!(t.last_poster.as_deref(), Some("RessourectoR"));
    }

    #[test]
    fn forum_normal_rows() {
        let page = forum();
        let normal: Vec<_> = page.topics.iter().filter(|t| !t.kind.is_pinned()).collect();
        assert_eq!(normal[0].title, "[Info] Slotbound");
        assert_eq!(normal[0].prefixes, ["Info"]);
        assert_eq!(normal[0].name, "Slotbound");
        let sm2 = normal
            .iter()
            .find(|t| t.name.starts_with("Warhammer"))
            .unwrap();
        assert_eq!(sm2.name, "Warhammer 40,000: Space Marine 2");
        assert_eq!(sm2.labels, ["Offline Only Cracked"]);
        for t in &normal {
            assert!(t.author.is_some(), "no author for {}", t.title);
            assert!(
                t.replies.is_some() && t.views.is_some(),
                "no counts for {}",
                t.title
            );
            assert!(
                t.created.is_some() && t.last_post_at.is_some(),
                "no dates for {}",
                t.title
            );
        }
    }

    #[test]
    fn forum_pagination() {
        let p = forum().pagination;
        assert_eq!((p.page, p.total_pages), (Some(1), Some(630)));
        assert_eq!(
            (p.previous_start, p.next_start, p.last_start),
            (None, Some(100), 62900)
        );
    }

    /// The layout of subsilver2's search results: a forum column, and
    /// author before replies (unlike forum pages). Not yet checked against
    /// cs.rin.ru's own search page.
    const SEARCH: &str = r#"
      <table class="tablebg">
        <tr><td class="cat" colspan="7"><span class="nav">Search found 27 matches</span></td></tr>
        <tr>
          <th>&nbsp;</th>
          <th>&nbsp;Forum&nbsp;</th>
          <th>&nbsp;Topics&nbsp;</th>
          <th>&nbsp;Author&nbsp;</th>
          <th>&nbsp;Replies&nbsp;</th>
          <th>&nbsp;Views&nbsp;</th>
          <th>&nbsp;Last post&nbsp;</th>
        </tr>
        <tr>
          <td class="row1"><img src="x.gif" /></td>
          <td class="row1"><a href="./viewforum.php?f=10" class="forumlink">Main Forum</a></td>
          <td class="row1"><a title="Posted: Monday, 29 Jun 2026, 01:18" href="./viewtopic.php?f=10&amp;t=159639" class="topictitle">[Info] Slotbound</a></td>
          <td class="row2"><p class="topicauthor"><a href="./memberlist.php?mode=viewprofile&amp;u=1">pixieflitter</a></p></td>
          <td class="row1"><p class="topicdetails">34</p></td>
          <td class="row2"><p class="topicdetails">5120</p></td>
          <td class="row1"><p class="topicdetails">Today, 10:00</p><p class="topicdetails"><a href="./memberlist.php?mode=viewprofile&amp;u=2">DicDale</a></p></td>
        </tr>
      </table>
      <table><tr><td class="nav">Page <strong>1</strong> of <strong>2</strong></td></tr></table>
      <a href="./search.php?keywords=slot&amp;fid%5B%5D=10&amp;sr=topics&amp;sf=titleonly&amp;start=25">2</a>
      <a href="./viewforum.php?f=10&amp;start=100">elsewhere</a>"#;

    #[test]
    fn search_results() {
        let url = url("search.php?keywords=slot&fid[]=10&sr=topics&sf=titleonly");
        let page = parse_search(SEARCH, &url, "slot", 0).unwrap();
        assert_eq!(page.total_matches, Some(27));
        assert_eq!(page.topics.len(), 1);
        let t = &page.topics[0];
        assert_eq!((t.id, t.name.as_str()), (159639, "Slotbound"));
        // Columns come from the header row, not fixed positions.
        assert_eq!(t.author.as_deref(), Some("pixieflitter"));
        assert_eq!((t.replies, t.views), (Some(34), Some(5120)));
        assert_eq!(t.last_post_at.as_deref(), Some("Today, 10:00"));
        assert_eq!(t.last_poster.as_deref(), Some("DicDale"));
        let p = &page.pagination;
        assert_eq!(
            (p.next_start, p.page, p.total_pages),
            (Some(25), Some(1), Some(2))
        );
    }

    #[test]
    fn search_messages() {
        let url = url("search.php?keywords=slot");
        let flood = "<div><p>Sorry but you cannot use search at this time. Please try again in a few minutes.</p></div>";
        assert_eq!(
            parse_search(flood, &url, "slot", 0).unwrap_err(),
            "Sorry but you cannot use search at this time."
        );
        let none = "<p>No suitable matches were found.</p>";
        let page = parse_search(none, &url, "zzz", 0).unwrap();
        assert!(page.topics.is_empty());
    }

    #[test]
    fn split_titles() {
        assert_eq!(split_title("Plain"), (vec![], "Plain".into(), vec![]));
        assert_eq!(
            split_title("[Info] [Old] Game [VR] [Early Access]"),
            (
                vec!["Info".into(), "Old".into()],
                "Game".into(),
                vec!["VR".into(), "Early Access".into()]
            )
        );
        assert_eq!(
            split_title("[Info] [Only Tag]"),
            (vec!["Info".into()], "[Only Tag]".into(), vec![])
        );
    }

    #[test]
    fn topic_posts() {
        let page = topic();
        assert_eq!(page.title.as_deref(), Some("[Info] Slotbound"));
        assert_eq!(page.posts.len(), 15);
        let first = &page.posts[0];
        assert_eq!(first.id, 3540229);
        assert_eq!(first.author.as_deref(), Some("pixieflitter"));
        assert_eq!(first.subject.as_deref(), Some("Slotbound"));
        assert_eq!(first.posted.as_deref(), Some("Monday, 29 Jun 2026, 01:18"));
        assert!(first.content_text.contains("\nGenre(s): Indie, Strategy\n"));
        // The signature is a second div.postbody and must not be included.
        assert!(!first.content_text.contains("_________________"));
        let p = &page.pagination;
        assert_eq!(
            (p.page, p.total_pages, p.next_start),
            (Some(1), Some(3), Some(15))
        );
    }

    #[test]
    fn topic_links() {
        let page = topic();

        let store = &post(&page, 3540229).links;
        assert_eq!(store[0].kind, LinkKind::SteamStore);
        assert!(store[0].context.starts_with("Store Page:"));
        assert_eq!(store[1].kind, LinkKind::SteamDb);

        let csf = &post(&page, 3626548).links;
        assert_eq!(csf[0].url, "https://gofile.io/d/GROz2x3u");
        assert_eq!(
            (csf[0].host.as_str(), csf[0].text.as_str()),
            ("gofile.io", "Gofile")
        );
        assert_eq!(csf[0].context, "Gofile | VikingFile");
        assert_eq!(csf[1].host, "vikingfile.com");
        assert!(csf.iter().all(|l| !l.in_quote));
        // "Make your own cracks with ... RAC | SAC | CAC" is this author's
        // signature, a second div.postbody, so its links are not included.
        assert_eq!(csf.len(), 2);

        let zei = &post(&page, 3627087).links;
        let all = zei.iter().find(|l| l.host == "zeigames.com").unwrap();
        assert!(all.flagged_unsafe);
        assert!(all.context.starts_with("Download:"));
        let filesyard = zei.iter().find(|l| l.host == "filesyard.com").unwrap();
        assert!(!filesyard.flagged_unsafe);
    }

    #[test]
    fn session_states() {
        assert_eq!(
            parse_session(FORUM),
            Session::LoggedIn {
                username: Some("TestUser".into())
            }
        );
        let out = r#"<a href="./ucp.php?mode=login">Login</a>"#;
        assert_eq!(parse_session(out), Session::LoggedOut);
    }

    #[test]
    fn detects_security_check() {
        let page = r#"<title>CS RIN - Security check</title>
          <script>document.cookie = "securitytoken=abc; path=/; secure";</script>"#;
        assert!(is_security_check(page));
        assert!(!is_security_check(FORUM));
    }
}
