//! Parsing of phpBB 3 pages.
//!
//! Written against the two stock phpBB 3 layouts (prosilver's `li.row` lists
//! and subsilver2's tables) because cs.rin.ru's theme has not been checked
//! yet. Selectors lean on things every phpBB theme keeps: `a.topictitle`,
//! `viewtopic.php?...t=` links, `memberlist.php?mode=viewprofile` links and
//! `start=` pagination. Verify against saved pages (see `save_snapshot`) and
//! add those as test fixtures.

use std::collections::HashSet;
use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use url::Url;

macro_rules! selector {
    ($name:ident, $css:expr) => {
        static $name: LazyLock<Selector> = LazyLock::new(|| Selector::parse($css).unwrap());
    };
}

selector!(TOPIC_TITLE, "a.topictitle");
selector!(PROFILE_LINK, r#"a[href*="mode=viewprofile"]"#);
selector!(DD_POSTS, "dd.posts");
selector!(DD_VIEWS, "dd.views");
selector!(TD, "td");
selector!(HEADING, "h2");
selector!(HEADING_LINK, "h2 a");
selector!(TITLE, "title");
selector!(START_LINK, r#"a[href*="start="]"#);
selector!(LOGOUT_LINK, r#"a[href*="mode=logout"]"#);
selector!(
    LOGGED_IN_USERNAME,
    "#username_logged_in .username, #username_logged_in .username-coloured"
);
selector!(POST_CONTENT, "div.content, div.postbody");
selector!(PRO_CONTENT, "div.content");
selector!(SUB_CONTENT, "div.postbody");
selector!(POST_ANCHOR, r#"a[name^="p"], a[id^="p"]"#);
selector!(PRO_AUTHOR, "p.author");
selector!(SUB_AUTHOR, "b.postauthor, .postauthor");
selector!(TIME, "time[datetime]");

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ForumPage {
    pub forum_id: u64,
    pub title: Option<String>,
    pub topics: Vec<TopicSummary>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TopicSummary {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub author: Option<String>,
    pub last_poster: Option<String>,
    pub replies: Option<u32>,
    pub views: Option<u32>,
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
    pub id: Option<u64>,
    pub author: Option<String>,
    /// ISO timestamp when the theme provides `<time datetime>`, otherwise the
    /// date text as displayed.
    pub posted: Option<String>,
    pub content_html: String,
    pub content_text: String,
}

/// phpBB pages by `start=` offset (items, not page numbers).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Pagination {
    pub start: u32,
    pub next_start: Option<u32>,
    pub previous_start: Option<u32>,
    pub last_start: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Session {
    LoggedIn { username: Option<String> },
    LoggedOut,
}

/// True for the site's JavaScript "Security check" interstitial.
pub fn is_security_check(html: &str) -> bool {
    html.contains("Security check") && html.contains("securitytoken")
}

pub fn parse_session(html: &str) -> Session {
    let doc = Html::parse_document(html);
    let Some(logout) = doc.select(&LOGOUT_LINK).next() else {
        return Session::LoggedOut;
    };
    // phpBB 3.1+: <li id="username_logged_in"> ... <span class="username">name</span>
    // phpBB 3.0:  "Logout [ name ]"
    let username = doc
        .select(&LOGGED_IN_USERNAME)
        .next()
        .map(text_of)
        .or_else(|| {
            let text = text_of(logout);
            let inner = text
                .split_once('[')?
                .1
                .rsplit_once(']')?
                .0
                .trim()
                .to_string();
            (!inner.is_empty()).then_some(inner)
        })
        .filter(|name| !name.is_empty());
    Session::LoggedIn { username }
}

/// Parses `viewforum.php?f=<forum_id>&start=<start>`.
pub fn parse_forum(html: &str, page_url: &Url, forum_id: u64, start: u32) -> ForumPage {
    let doc = Html::parse_document(html);
    let mut seen = HashSet::new();
    let mut topics = Vec::new();

    for link in doc.select(&TOPIC_TITLE) {
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
            continue; // announcements can be listed twice
        }

        let row = topic_row(link);
        let profiles: Vec<String> = row
            .map(|r| r.select(&PROFILE_LINK).map(text_of).collect())
            .unwrap_or_default();
        let (replies, views) = row.map(row_counts).unwrap_or((None, None));

        topics.push(TopicSummary {
            id,
            title: text_of(link),
            url: clean_url(url),
            author: profiles.first().cloned(),
            last_poster: if profiles.len() > 1 {
                profiles.last().cloned()
            } else {
                None
            },
            replies,
            views,
        });
    }

    ForumPage {
        forum_id,
        title: page_title(&doc),
        topics,
        pagination: parse_pagination(&doc, page_url, start, "f", forum_id),
    }
}

/// Parses `viewtopic.php?t=<topic_id>&start=<start>`.
pub fn parse_topic(html: &str, page_url: &Url, topic_id: u64, start: u32) -> TopicPage {
    let doc = Html::parse_document(html);
    let mut seen_containers = HashSet::new();
    let mut posts = Vec::new();

    for found in doc.select(&POST_CONTENT) {
        // A post's container is the nearest div.post (prosilver) or table
        // (subsilver2). Its body is the first div.content (prosilver, where
        // div.postbody wraps the whole post) or else the first div.postbody
        // (subsilver2, where later ones are signatures).
        let container = post_container(found).unwrap_or(found);
        if !seen_containers.insert(container.id()) {
            continue;
        }
        let content = container
            .select(&PRO_CONTENT)
            .next()
            .or_else(|| container.select(&SUB_CONTENT).next())
            .unwrap_or(found);

        let id = container.value().id().and_then(post_id).or_else(|| {
            container.select(&POST_ANCHOR).find_map(|a| {
                a.value()
                    .attr("name")
                    .or(a.value().attr("id"))
                    .and_then(post_id)
            })
        });

        let author = container
            .select(&SUB_AUTHOR)
            .next()
            .map(text_of)
            .or_else(|| container.select(&PROFILE_LINK).next().map(text_of))
            .filter(|s| !s.is_empty());

        let posted = container
            .select(&TIME)
            .next()
            .and_then(|t| t.value().attr("datetime").map(str::to_string))
            .or_else(|| {
                // phpBB 3.0 prosilver: "by name » Mon Jan 01, 2024 12:00 pm"
                let text = text_of(container.select(&PRO_AUTHOR).next()?);
                Some(text.rsplit_once('»')?.1.trim().to_string())
            })
            .filter(|s| !s.is_empty());

        posts.push(Post {
            id,
            author,
            posted,
            content_html: content.inner_html().trim().to_string(),
            content_text: text_of(content),
        });
    }

    TopicPage {
        topic_id,
        title: page_title(&doc),
        posts,
        pagination: parse_pagination(&doc, page_url, start, "t", topic_id),
    }
}

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

fn has_class(el: &ElementRef, class: &str) -> bool {
    el.value().classes().any(|c| c == class)
}

/// prosilver: `<li class="row">`, subsilver2: `<tr>`.
fn topic_row(link: ElementRef) -> Option<ElementRef> {
    link.ancestors().filter_map(ElementRef::wrap).find(|el| {
        let name = el.value().name();
        name == "tr" || (name == "li" && has_class(el, "row"))
    })
}

fn post_container(content: ElementRef) -> Option<ElementRef> {
    content.ancestors().filter_map(ElementRef::wrap).find(|el| {
        let name = el.value().name();
        (name == "div" && has_class(el, "post")) || name == "table"
    })
}

fn post_id(raw: &str) -> Option<u64> {
    raw.strip_prefix('p')?.parse().ok()
}

fn first_number(text: &str) -> Option<u32> {
    let digits: String = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .filter(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// (replies, views) from `dd.posts`/`dd.views`, or from the first two
/// purely numeric table cells.
fn row_counts(row: ElementRef) -> (Option<u32>, Option<u32>) {
    let dd = |sel: &Selector| {
        row.select(sel)
            .next()
            .map(text_of)
            .and_then(|t| first_number(&t))
    };
    if let (Some(r), v) = (dd(&DD_POSTS), dd(&DD_VIEWS)) {
        return (Some(r), v);
    }
    let mut numbers = row.select(&TD).map(text_of).filter_map(|t| {
        let plain = t.replace(',', "");
        (!plain.is_empty() && plain.chars().all(|c| c.is_ascii_digit()))
            .then(|| plain.parse().ok())
            .flatten()
    });
    (numbers.next(), numbers.next())
}

fn page_title(doc: &Html) -> Option<String> {
    doc.select(&HEADING_LINK)
        .next()
        .or_else(|| doc.select(&HEADING).next())
        .or_else(|| doc.select(&TITLE).next())
        .map(text_of)
        .filter(|s| !s.is_empty())
}

/// Collects `start=` offsets from links that point at the same forum/topic.
fn parse_pagination(doc: &Html, page_url: &Url, start: u32, key: &str, id: u64) -> Pagination {
    let mut offsets: Vec<u32> = doc
        .select(&START_LINK)
        .filter_map(|a| page_url.join(a.value().attr("href")?).ok())
        .filter(|u| u.path() == page_url.path() && query_u64(u, key) == Some(id))
        .filter_map(|u| query_u64(&u, "start").and_then(|s| u32::try_from(s).ok()))
        .collect();
    offsets.push(0);
    offsets.sort_unstable();
    offsets.dedup();

    Pagination {
        start,
        next_start: offsets.iter().copied().find(|&s| s > start),
        previous_start: offsets.iter().rev().copied().find(|&s| s < start),
        last_start: offsets.last().copied().unwrap_or(0).max(start),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(path: &str) -> Url {
        Url::parse("https://cs.rin.ru/forum/")
            .unwrap()
            .join(path)
            .unwrap()
    }

    const PROSILVER_FORUM: &str = r#"
      <h2 class="forum-title"><a href="./viewforum.php?f=10">Main Forum</a></h2>
      <div class="pagination">
        <a href="./viewforum.php?f=10&amp;start=50">2</a>
        <a href="./viewforum.php?f=10&amp;start=100">3</a>
      </div>
      <ul class="topiclist topics">
        <li class="row bg1">
          <dl>
            <dt><a href="./viewtopic.php?f=10&amp;t=111&amp;sid=abc" class="topictitle">Game One</a>
              <br />by <a href="./memberlist.php?mode=viewprofile&amp;u=5" class="username">alice</a></dt>
            <dd class="posts">1,234 <dfn>Replies</dfn></dd>
            <dd class="views">56,789 <dfn>Views</dfn></dd>
            <dd class="lastpost"><span>by <a href="./memberlist.php?mode=viewprofile&amp;u=6">bob</a></span></dd>
          </dl>
        </li>
        <li class="row bg2">
          <dl><dt><a href="./viewtopic.php?f=10&amp;t=222" class="topictitle">Game Two</a></dt></dl>
        </li>
      </ul>"#;

    #[test]
    fn prosilver_forum() {
        let page = parse_forum(PROSILVER_FORUM, &url("viewforum.php?f=10"), 10, 0);
        assert_eq!(page.title.as_deref(), Some("Main Forum"));
        assert_eq!(page.topics.len(), 2);
        let t = &page.topics[0];
        assert_eq!(t.id, 111);
        assert_eq!(t.title, "Game One");
        assert_eq!(t.url, "https://cs.rin.ru/forum/viewtopic.php?f=10&t=111");
        assert_eq!(t.author.as_deref(), Some("alice"));
        assert_eq!(t.last_poster.as_deref(), Some("bob"));
        assert_eq!((t.replies, t.views), (Some(1234), Some(56789)));
        assert_eq!(page.topics[1].author, None);
        assert_eq!(
            page.pagination,
            Pagination {
                start: 0,
                next_start: Some(50),
                previous_start: None,
                last_start: 100
            }
        );
    }

    const SUBSILVER_FORUM: &str = r#"
      <h2><a href="./viewforum.php?f=10">Main Forum</a></h2>
      <table class="tablebg">
        <tr>
          <td class="row1"><img src="icon.gif" /></td>
          <td class="row1"><a href="./viewtopic.php?f=10&amp;t=333" class="topictitle">Game Three</a></td>
          <td class="row2"><p class="topicauthor"><a href="./memberlist.php?mode=viewprofile&amp;u=7">carol</a></p></td>
          <td class="row1"><p class="topicdetails">12</p></td>
          <td class="row2"><p class="topicdetails">3400</p></td>
          <td class="row1"><p class="topicdetails"><a href="./memberlist.php?mode=viewprofile&amp;u=8">dave</a></p></td>
        </tr>
      </table>
      <a href="./viewforum.php?f=10&amp;start=0">1</a>
      <a href="./viewforum.php?f=10&amp;start=100">3</a>"#;

    #[test]
    fn subsilver_forum() {
        let page = parse_forum(SUBSILVER_FORUM, &url("viewforum.php?f=10&start=50"), 10, 50);
        let t = &page.topics[0];
        assert_eq!((t.id, t.title.as_str()), (333, "Game Three"));
        assert_eq!(t.author.as_deref(), Some("carol"));
        assert_eq!(t.last_poster.as_deref(), Some("dave"));
        assert_eq!((t.replies, t.views), (Some(12), Some(3400)));
        assert_eq!(page.pagination.previous_start, Some(0));
        assert_eq!(page.pagination.next_start, Some(100));
    }

    #[test]
    fn prosilver_topic() {
        let html = r#"
          <h2 class="topic-title"><a href="./viewtopic.php?t=111">Game One</a></h2>
          <div id="p900" class="post bg2"><div class="inner">
            <div class="postbody">
              <p class="author">by <strong><a href="./memberlist.php?mode=viewprofile&amp;u=5">alice</a></strong>
                 » <time datetime="2024-01-02T03:04:05+00:00">Tue Jan 02</time></p>
              <div class="content">Download <b>here</b></div>
              <div class="signature">sig</div>
            </div>
          </div></div>
          <div id="p901" class="post bg1"><div class="inner"><div class="postbody">
            <p class="author">by <a href="./memberlist.php?mode=viewprofile&amp;u=6">bob</a> » Wed Jan 03, 2024 1:00 pm</p>
            <div class="content">Thanks</div>
          </div></div></div>"#;
        let page = parse_topic(html, &url("viewtopic.php?t=111"), 111, 0);
        assert_eq!(page.title.as_deref(), Some("Game One"));
        assert_eq!(page.posts.len(), 2);
        let p = &page.posts[0];
        assert_eq!(p.id, Some(900));
        assert_eq!(p.author.as_deref(), Some("alice"));
        assert_eq!(p.posted.as_deref(), Some("2024-01-02T03:04:05+00:00"));
        assert_eq!(p.content_text, "Download here");
        assert_eq!(p.content_html, "Download <b>here</b>");
        assert_eq!(
            page.posts[1].posted.as_deref(),
            Some("Wed Jan 03, 2024 1:00 pm")
        );
    }

    #[test]
    fn subsilver_topic() {
        let html = r#"
          <table class="tablebg">
            <tr><td><a name="p42"></a><b class="postauthor">carol</b></td></tr>
            <tr><td><div class="postbody">First post</div><div class="postbody">signature</div></td></tr>
          </table>
          <table class="tablebg">
            <tr><td><a name="p43"></a><b class="postauthor">dave</b></td></tr>
            <tr><td><div class="postbody">Reply</div></td></tr>
          </table>"#;
        let page = parse_topic(html, &url("viewtopic.php?t=5"), 5, 0);
        let got: Vec<_> = page
            .posts
            .iter()
            .map(|p| (p.id, p.author.clone().unwrap(), p.content_text.clone()))
            .collect();
        assert_eq!(
            got,
            vec![
                (Some(42), "carol".into(), "First post".into()),
                (Some(43), "dave".into(), "Reply".into())
            ]
        );
    }

    #[test]
    fn session_states() {
        let new = r#"<li id="username_logged_in"><span class="username">alice</span></li>
                     <a href="./ucp.php?mode=logout&amp;sid=x">Logout</a>"#;
        assert_eq!(
            parse_session(new),
            Session::LoggedIn {
                username: Some("alice".into())
            }
        );

        let old = r#"<a href="./ucp.php?mode=logout&amp;sid=x">Logout [ bob ]</a>"#;
        assert_eq!(
            parse_session(old),
            Session::LoggedIn {
                username: Some("bob".into())
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
        assert!(!is_security_check(
            "<title>CS RIN - Steam Underground</title>"
        ));
    }
}
