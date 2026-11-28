//! Downloading a post's download links into the extract folder.
//!
//! The links on a post are mostly file-host pages, not files. A link is first resolved to a
//! direct file: pixeldrain and gofile through their APIs, and any other link by fetching it and
//! taking the answer as the file unless it is a web page. Hosts that need a wait, a captcha or a
//! login answer with a web page, so they fail with [`DownloadError::Unsupported`] and the link
//! is left to the browser.
//!
//! The file is streamed into `<name>.part` in the destination and renamed once it is complete.
//! On any failure, cancellation included, the `.part` file is removed, so nothing is written.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use reqwest::header::{
    AUTHORIZATION, CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue,
};
use serde::{Deserialize, Serialize};
use url::Url;

/// How long connecting to a host may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// How long a download may go without receiving anything before it is given up.
const STALL_TIMEOUT: Duration = Duration::from_secs(60);
/// How often progress is reported at most.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

/// The website token gofile's own site sends when listing a folder's contents.
const GOFILE_WEBSITE_TOKEN: &str = "4fd6sg89d7s6";

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("{0} doesn't offer a direct download Rimuru can fetch; open it in the browser instead")]
    Unsupported(String),

    #[error("{host} answered with HTTP {status}")]
    Status { host: String, status: u16 },

    #[error("couldn't find the file on {host}: {message}")]
    Resolve { host: String, message: String },

    #[error("the download stalled: nothing arrived for {} seconds", STALL_TIMEOUT.as_secs())]
    Stalled,

    #[error("the download was cancelled")]
    Cancelled,

    #[error("{0} is not a link Rimuru can download")]
    BadUrl(String),

    #[error("the download failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("couldn't write the download: {0}")]
    Io(#[from] io::Error),
}

impl DownloadError {
    pub fn kind(&self) -> &'static str {
        match self {
            DownloadError::Unsupported(_) => "download_unsupported",
            DownloadError::Status { .. } => "download_status",
            DownloadError::Resolve { .. } => "download_resolve",
            DownloadError::Stalled => "download_stalled",
            DownloadError::Cancelled => "download_cancelled",
            DownloadError::BadUrl(_) => "download_bad_url",
            DownloadError::Http(_) => "download_http",
            DownloadError::Io(_) => "io",
        }
    }
}

pub type Result<T> = std::result::Result<T, DownloadError>;

/// How far a download is: bytes received, and the size when the host says it.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Progress {
    pub received: u64,
    pub total: Option<u64>,
}

/// Where the hosts' APIs are, so tests can point them at a local server.
#[derive(Debug, Clone)]
pub struct Hosts {
    pub pixeldrain: String,
    pub gofile_api: String,
}

impl Default for Hosts {
    fn default() -> Self {
        Self {
            pixeldrain: "https://pixeldrain.com".into(),
            gofile_api: "https://api.gofile.io".into(),
        }
    }
}

/// The HTTP client for file hosts. It is separate from the forum's so the forum's cookies never
/// reach a file host.
pub fn client() -> reqwest::Result<reqwest::Client> {
    client_builder().build()
}

fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(crate::forum_window::USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
}

/// A link resolved to the file it points to.
#[derive(Debug, Clone)]
struct DirectFile {
    url: String,
    headers: HeaderMap,
    name: Option<String>,
}

impl DirectFile {
    fn plain(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: HeaderMap::new(),
            name: None,
        }
    }
}

fn host_of(url: &Url) -> String {
    url.host_str().unwrap_or("the host").to_string()
}

fn is_host(url: &Url, domain: &str) -> bool {
    url.host_str()
        .is_some_and(|h| h == domain || h.ends_with(&format!(".{domain}")))
}

/// The pixeldrain file id of a `pixeldrain.com/u/<id>` or `/api/file/<id>` link.
fn pixeldrain_id(url: &Url) -> Option<String> {
    if !is_host(url, "pixeldrain.com") && !is_host(url, "pixeldrain.net") {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        ["u", id, ..] | ["api", "file", id, ..] => Some((*id).to_string()),
        _ => None,
    }
}

/// The gofile content code of a `gofile.io/d/<code>` link.
fn gofile_code(url: &Url) -> Option<String> {
    if !is_host(url, "gofile.io") {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        ["d", code, ..] => Some((*code).to_string()),
        _ => None,
    }
}

#[derive(Deserialize)]
struct GofileAnswer<T> {
    status: String,
    data: Option<T>,
}

#[derive(Deserialize)]
struct GofileAccount {
    token: String,
}

#[derive(Deserialize)]
struct GofileContent {
    #[serde(rename = "type", default)]
    kind: String,
    name: Option<String>,
    link: Option<String>,
    #[serde(default)]
    children: std::collections::HashMap<String, GofileContent>,
}

fn parse_gofile<T: serde::de::DeserializeOwned>(body: String) -> Result<GofileAnswer<T>> {
    serde_json::from_str(&body).map_err(|e| DownloadError::Resolve {
        host: "gofile.io".into(),
        message: format!("its answer wasn't understood: {e}"),
    })
}

async fn resolve_gofile(client: &reqwest::Client, hosts: &Hosts, code: &str) -> Result<DirectFile> {
    let fail = |message: String| DownloadError::Resolve {
        host: "gofile.io".into(),
        message,
    };
    let api = hosts.gofile_api.trim_end_matches('/');

    let account: GofileAnswer<GofileAccount> = parse_gofile(
        client
            .post(format!("{api}/accounts"))
            .send()
            .await?
            .text()
            .await?,
    )?;
    let token = match (account.status.as_str(), account.data) {
        ("ok", Some(data)) => data.token,
        (status, _) => return Err(fail(format!("couldn't get a guest account ({status})"))),
    };

    let contents: GofileAnswer<GofileContent> = parse_gofile(
        client
            .get(format!("{api}/contents/{code}?wt={GOFILE_WEBSITE_TOKEN}"))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?
            .text()
            .await?,
    )?;
    let content = match (contents.status.as_str(), contents.data) {
        ("ok", Some(data)) => data,
        (status, _) => return Err(fail(format!("it answered {status}"))),
    };

    let mut files: Vec<GofileContent> = if content.kind == "file" {
        vec![content]
    } else {
        content
            .children
            .into_values()
            .filter(|c| c.kind == "file")
            .collect()
    };
    let file = match files.len() {
        1 => files.remove(0),
        0 => return Err(fail("the folder has no files".into())),
        n => {
            return Err(fail(format!(
                "the folder has {n} files; open it in the browser to pick one"
            )));
        }
    };
    let link = file
        .link
        .ok_or_else(|| fail("the file has no download link".into()))?;

    let mut headers = HeaderMap::new();
    if let Ok(cookie) = HeaderValue::from_str(&format!("accountToken={token}")) {
        headers.insert(COOKIE, cookie);
    }
    Ok(DirectFile {
        url: link,
        headers,
        name: file.name,
    })
}

/// Resolves a download link to the file it points to.
async fn resolve(client: &reqwest::Client, hosts: &Hosts, link: &Url) -> Result<DirectFile> {
    if let Some(id) = pixeldrain_id(link) {
        let base = hosts.pixeldrain.trim_end_matches('/');
        return Ok(DirectFile::plain(format!("{base}/api/file/{id}?download")));
    }
    if let Some(code) = gofile_code(link) {
        return resolve_gofile(client, hosts, &code).await;
    }
    Ok(DirectFile::plain(link.as_str()))
}

/// Downloads the file a link points to into `dest`, reporting progress, and returns its path.
pub async fn download(
    client: &reqwest::Client,
    hosts: &Hosts,
    link: &str,
    dest: &Path,
    mut on_progress: impl FnMut(Progress),
) -> Result<PathBuf> {
    let link = Url::parse(link.trim()).map_err(|_| DownloadError::BadUrl(link.to_string()))?;
    if !matches!(link.scheme(), "http" | "https") {
        return Err(DownloadError::BadUrl(link.to_string()));
    }
    let host = host_of(&link);
    let file = resolve(client, hosts, &link).await?;

    let mut response = client
        .get(&file.url)
        .headers(file.headers.clone())
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(DownloadError::Status {
            host,
            status: status.as_u16(),
        });
    }
    let disposition = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let is_attachment = disposition.as_deref().is_some_and(|d| {
        d.trim_start()
            .to_ascii_lowercase()
            .starts_with("attachment")
    });
    let is_page = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|t| t.to_ascii_lowercase().contains("text/html"));
    if is_page && !is_attachment {
        return Err(DownloadError::Unsupported(host));
    }

    let name = disposition
        .as_deref()
        .and_then(disposition_filename)
        .or(file.name)
        .or_else(|| url_filename(response.url()))
        .map(|n| sanitize_filename(&n))
        .unwrap_or_else(|| "download".to_string());

    fs::create_dir_all(dest)?;
    let (final_path, mut part) = PartFile::create(dest, &name)?;

    let total = response.content_length();
    let mut received = 0u64;
    let mut last_report = Instant::now();
    on_progress(Progress { received, total });
    loop {
        let chunk = tokio::time::timeout(STALL_TIMEOUT, response.chunk())
            .await
            .map_err(|_| DownloadError::Stalled)??;
        let Some(chunk) = chunk else { break };
        part.file.write_all(&chunk)?;
        received += chunk.len() as u64;
        if last_report.elapsed() >= PROGRESS_INTERVAL {
            last_report = Instant::now();
            on_progress(Progress { received, total });
        }
    }
    if total.is_some_and(|t| received < t) {
        return Err(DownloadError::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the host closed the connection before the file was complete",
        )));
    }
    on_progress(Progress { received, total });
    part.finish(&final_path)?;
    Ok(final_path)
}

/// A `.part` file being written, removed when dropped unless it was finished.
struct PartFile {
    path: PathBuf,
    file: File,
    done: bool,
}

impl PartFile {
    /// Creates the `.part` file for `name` in `dest`, picking a name that neither the part
    /// file nor the finished file takes from an existing file: `name (1).ext` and so on.
    fn create(dest: &Path, name: &str) -> Result<(PathBuf, PartFile)> {
        for n in 0..1000 {
            let candidate = dest.join(numbered(name, n));
            if candidate.exists() {
                continue;
            }
            let path = part_path(&candidate);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok((
                        candidate,
                        PartFile {
                            path,
                            file,
                            done: false,
                        },
                    ));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Err(DownloadError::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("too many files named {name} already"),
        )))
    }

    fn finish(mut self, final_path: &Path) -> Result<()> {
        self.file.flush()?;
        self.file.sync_all()?;
        if final_path.exists() {
            return Err(DownloadError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} appeared while downloading", final_path.display()),
            )));
        }
        fs::rename(&self.path, final_path)?;
        self.done = true;
        Ok(())
    }
}

impl Drop for PartFile {
    fn drop(&mut self) {
        if !self.done {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn part_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// `name` with ` (n)` before its extension, or `name` itself for 0.
fn numbered(name: &str, n: u32) -> String {
    if n == 0 {
        return name.to_string();
    }
    let lower = name.to_ascii_lowercase();
    let split = [".tar.gz", ".tar.bz2", ".tar.xz"]
        .iter()
        .find(|ext| lower.ends_with(*ext) && name.len() > ext.len())
        .map(|ext| name.len() - ext.len())
        .or_else(|| name.rfind('.').filter(|&i| i > 0));
    match split {
        Some(i) => format!("{} ({n}){}", &name[..i], &name[i..]),
        None => format!("{name} ({n})"),
    }
}

/// The file name a `Content-Disposition` header gives, preferring `filename*`.
fn disposition_filename(header: &str) -> Option<String> {
    let mut plain = None;
    for part in header.split(';').map(str::trim) {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if key == "filename*" {
            // RFC 5987: charset'language'percent-encoded
            let encoded = value.splitn(3, '\'').nth(2).unwrap_or(value);
            if let Some(name) = percent_decode(encoded.trim_matches('"')) {
                return Some(name).filter(|n| !n.is_empty());
            }
        } else if key == "filename" {
            plain = Some(value.trim_matches('"').to_string());
        }
    }
    plain.filter(|n| !n.is_empty())
}

/// The last segment of a URL's path, decoded.
fn url_filename(url: &Url) -> Option<String> {
    let last = url.path_segments()?.rfind(|s| !s.is_empty())?;
    percent_decode(last).filter(|n| !n.is_empty())
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(b) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).ok()
}

/// A file name that is safe to create in the destination: no folders, no characters Windows
/// refuses, not hidden and not empty.
pub fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim();
    if cleaned.is_empty() {
        "download".to_string()
    } else {
        cleaned.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;

    /// What the test server answers a request with.
    enum Reply {
        /// A whole HTTP response.
        Full(Vec<u8>),
        /// Headers promising more body than is sent, then the connection is closed.
        Truncated(Vec<u8>),
    }

    struct Request {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
    }

    impl Request {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        }
    }

    fn response(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Reply {
        let mut out = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
            body.len()
        );
        for (k, v) in headers {
            out.push_str(&format!("{k}: {v}\r\n"));
        }
        out.push_str("\r\n");
        let mut bytes = out.into_bytes();
        bytes.extend_from_slice(body);
        Reply::Full(bytes)
    }

    /// Starts a local HTTP server answering each request with `route`, and returns its base URL.
    fn serve(route: impl Fn(&Request, &str) -> Reply + Send + Sync + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let route = Arc::new(route);
        let base_for_thread = base.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let route = route.clone();
                let base = base_for_thread.clone();
                std::thread::spawn(move || handle(stream, &*route, &base));
            }
        });
        base
    }

    fn handle(mut stream: TcpStream, route: &dyn Fn(&Request, &str) -> Reply, base: &str) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return;
        }
        let mut words = line.split_whitespace();
        let method = words.next().unwrap_or("").to_string();
        let path = words.next().unwrap_or("").to_string();
        let mut headers = Vec::new();
        let mut length = 0usize;
        loop {
            let mut h = String::new();
            if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
                break;
            }
            if let Some((k, v)) = h.trim_end().split_once(':') {
                if k.eq_ignore_ascii_case("content-length") {
                    length = v.trim().parse().unwrap_or(0);
                }
                headers.push((k.to_string(), v.trim().to_string()));
            }
        }
        let mut body = vec![0; length];
        let _ = reader.read_exact(&mut body);
        let request = Request {
            method,
            path,
            headers,
        };
        match route(&request, base) {
            Reply::Full(bytes) | Reply::Truncated(bytes) => {
                let _ = stream.write_all(&bytes);
            }
        }
        let _ = stream.flush();
    }

    fn test_client() -> reqwest::Client {
        client_builder().no_proxy().build().unwrap()
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    const ARCHIVE: &[u8] = b"PK\x03\x04 pretend this is a zip";

    #[tokio::test]
    async fn downloads_a_direct_file_into_the_destination() {
        let base = serve(|req, _| match req.path.as_str() {
            "/files/Game%20v1.2.zip" => {
                response("200 OK", &[("Content-Type", "application/zip")], ARCHIVE)
            }
            _ => response("404 Not Found", &[], b""),
        });
        let dir = tempfile::tempdir().unwrap();
        let mut seen = Vec::new();
        let path = download(
            &test_client(),
            &Hosts::default(),
            &format!("{base}/files/Game%20v1.2.zip"),
            dir.path(),
            |p| seen.push(p),
        )
        .await
        .unwrap();
        assert_eq!(path, dir.path().join("Game v1.2.zip"));
        assert_eq!(fs::read(&path).unwrap(), ARCHIVE);
        assert_eq!(entries(dir.path()), vec!["Game v1.2.zip"]);
        let last = *seen.last().unwrap();
        assert_eq!(last.received, ARCHIVE.len() as u64);
        assert_eq!(last.total, Some(ARCHIVE.len() as u64));
    }

    #[tokio::test]
    async fn names_the_file_by_content_disposition_and_keeps_existing_files() {
        let base = serve(|_, _| {
            response(
                "200 OK",
                &[
                    ("Content-Type", "application/octet-stream"),
                    ("Content-Disposition", "attachment; filename=\"Game.zip\""),
                ],
                ARCHIVE,
            )
        });
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Game.zip"), b"older").unwrap();
        let path = download(
            &test_client(),
            &Hosts::default(),
            &format!("{base}/dl?id=7"),
            dir.path(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(path, dir.path().join("Game (1).zip"));
        assert_eq!(fs::read(dir.path().join("Game.zip")).unwrap(), b"older");
        assert_eq!(fs::read(&path).unwrap(), ARCHIVE);
    }

    #[tokio::test]
    async fn a_web_page_is_refused_and_nothing_is_written() {
        let base = serve(|_, _| {
            response(
                "200 OK",
                &[("Content-Type", "text/html; charset=utf-8")],
                b"<html>wait 30 seconds</html>",
            )
        });
        let dir = tempfile::tempdir().unwrap();
        let err = download(
            &test_client(),
            &Hosts::default(),
            &format!("{base}/file/abc"),
            dir.path(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DownloadError::Unsupported(_)), "{err:?}");
        assert!(entries(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn an_error_status_is_refused_and_nothing_is_written() {
        let base = serve(|_, _| response("404 Not Found", &[], b"gone"));
        let dir = tempfile::tempdir().unwrap();
        let err = download(
            &test_client(),
            &Hosts::default(),
            &format!("{base}/x.zip"),
            dir.path(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DownloadError::Status { status: 404, .. }),
            "{err:?}"
        );
        assert!(entries(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn a_dropped_connection_leaves_the_destination_empty() {
        let base = serve(|_, _| {
            let mut bytes = b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n".to_vec();
            bytes.extend_from_slice(ARCHIVE);
            Reply::Truncated(bytes)
        });
        let dir = tempfile::tempdir().unwrap();
        let result = download(
            &test_client(),
            &Hosts::default(),
            &format!("{base}/big.zip"),
            dir.path(),
            |_| {},
        )
        .await;
        assert!(result.is_err(), "{result:?}");
        assert!(entries(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn an_unreachable_host_leaves_the_destination_empty() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let dir = tempfile::tempdir().unwrap();
        let result = download(
            &test_client(),
            &Hosts::default(),
            &format!("http://{addr}/x.zip"),
            dir.path(),
            |_| {},
        )
        .await;
        assert!(matches!(result, Err(DownloadError::Http(_))), "{result:?}");
        assert!(entries(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn a_link_that_is_not_http_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let err = download(
            &test_client(),
            &Hosts::default(),
            "magnet:?xt=urn:btih:abc",
            dir.path(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DownloadError::BadUrl(_)), "{err:?}");
        assert!(entries(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn resolves_a_pixeldrain_page_through_its_api() {
        let base = serve(|req, _| match req.path.as_str() {
            "/api/file/AbC123?download" => response(
                "200 OK",
                &[
                    ("Content-Type", "application/zip"),
                    (
                        "Content-Disposition",
                        "attachment; filename*=UTF-8''Some%20Game.zip",
                    ),
                ],
                ARCHIVE,
            ),
            _ => response("404 Not Found", &[], b""),
        });
        let hosts = Hosts {
            pixeldrain: base,
            ..Hosts::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let path = download(
            &test_client(),
            &hosts,
            "https://pixeldrain.com/u/AbC123",
            dir.path(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(path, dir.path().join("Some Game.zip"));
        assert_eq!(fs::read(&path).unwrap(), ARCHIVE);
    }

    #[tokio::test]
    async fn resolves_a_gofile_folder_with_a_guest_account() {
        let base = serve(|req, base| match (req.method.as_str(), req.path.as_str()) {
            ("POST", "/accounts") => response(
                "200 OK",
                &[("Content-Type", "application/json")],
                br#"{"status":"ok","data":{"id":"g","token":"tok123"}}"#,
            ),
            ("GET", p) if p.starts_with("/contents/xYz?") => {
                if req.header("authorization") != Some("Bearer tok123") {
                    return response("401 Unauthorized", &[], b"");
                }
                let body = format!(
                    r#"{{"status":"ok","data":{{"type":"folder","children":{{"f1":{{"type":"file","name":"Game.7z","link":"{base}/download/web/f1/Game.7z"}}}}}}}}"#
                );
                response(
                    "200 OK",
                    &[("Content-Type", "application/json")],
                    body.as_bytes(),
                )
            }
            ("GET", "/download/web/f1/Game.7z")
                if req.header("cookie") == Some("accountToken=tok123") =>
            {
                response(
                    "200 OK",
                    &[("Content-Type", "application/x-7z-compressed")],
                    ARCHIVE,
                )
            }
            _ => response("403 Forbidden", &[], b""),
        });
        let hosts = Hosts {
            gofile_api: base,
            ..Hosts::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let path = download(
            &test_client(),
            &hosts,
            "https://gofile.io/d/xYz",
            dir.path(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(path, dir.path().join("Game.7z"));
        assert_eq!(fs::read(&path).unwrap(), ARCHIVE);
    }

    #[tokio::test]
    async fn a_gofile_error_is_reported_and_nothing_is_written() {
        let base = serve(|req, _| match req.path.as_str() {
            "/accounts" => response("200 OK", &[], br#"{"status":"ok","data":{"token":"t"}}"#),
            _ => response(
                "200 OK",
                &[],
                br#"{"status":"error-passwordRequired","data":{}}"#,
            ),
        });
        let hosts = Hosts {
            gofile_api: base,
            ..Hosts::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let err = download(
            &test_client(),
            &hosts,
            "https://gofile.io/d/xYz",
            dir.path(),
            |_| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DownloadError::Resolve { .. }), "{err:?}");
        assert!(err.to_string().contains("error-passwordRequired"));
        assert!(entries(dir.path()).is_empty());
    }

    #[test]
    fn recognises_host_links() {
        let id = |s: &str| pixeldrain_id(&Url::parse(s).unwrap());
        assert_eq!(id("https://pixeldrain.com/u/abc"), Some("abc".into()));
        assert_eq!(
            id("https://pixeldrain.com/api/file/abc"),
            Some("abc".into())
        );
        assert_eq!(id("https://pixeldrain.net/u/abc?x=1"), Some("abc".into()));
        assert_eq!(id("https://pixeldrain.com/l/list"), None);
        assert_eq!(id("https://notpixeldrain.com/u/abc"), None);
        let code = |s: &str| gofile_code(&Url::parse(s).unwrap());
        assert_eq!(code("https://gofile.io/d/AbCd"), Some("AbCd".into()));
        assert_eq!(code("https://gofile.io/"), None);
    }

    #[test]
    fn reads_content_disposition_names() {
        assert_eq!(
            disposition_filename("attachment; filename=\"a b.zip\""),
            Some("a b.zip".into())
        );
        assert_eq!(
            disposition_filename("attachment; filename=a.zip"),
            Some("a.zip".into())
        );
        assert_eq!(
            disposition_filename(
                "attachment; filename=\"fallback.zip\"; filename*=UTF-8''r%C3%A9al.zip"
            ),
            Some("réal.zip".into())
        );
        assert_eq!(disposition_filename("inline"), None);
    }

    #[test]
    fn sanitizes_file_names() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("..\\..\\boot.ini"), "boot.ini");
        assert_eq!(sanitize_filename("a:b*c?.zip"), "a_b_c_.zip");
        assert_eq!(sanitize_filename(".."), "download");
        assert_eq!(sanitize_filename(".hidden"), "hidden");
        assert_eq!(sanitize_filename("  "), "download");
    }

    #[test]
    fn numbers_names_before_the_extension() {
        assert_eq!(numbered("Game.zip", 0), "Game.zip");
        assert_eq!(numbered("Game.zip", 2), "Game (2).zip");
        assert_eq!(numbered("Game.tar.gz", 1), "Game (1).tar.gz");
        assert_eq!(numbered("README", 1), "README (1)");
    }
}
