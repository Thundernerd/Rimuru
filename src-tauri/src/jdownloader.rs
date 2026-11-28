//! Handing links Rimuru can't download itself to a running JDownloader 2.
//!
//! This is the opt-in fallback for [`crate::download::DownloadError::Unsupported`] links. It is
//! called from Rust only: JDownloader rejects requests that carry an `Origin` header, which a
//! webview adds.
//!
//! Two doors are tried, in order:
//! 1. JDownloader's deprecated local API (`RemoteAPI.deprecatedapienabled`, port 3128 by
//!    default), which takes the link and reports progress.
//! 2. Click'n'Load on port 9666, which only takes the link. JDownloader asks the user to allow
//!    it once, and progress then shows in JDownloader itself.

use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use url::Url;

use crate::download::Progress;

/// Where Click'n'Load listens.
pub const CLICK_N_LOAD_PORT: u16 = 9666;
/// How often a running job is polled.
pub const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// The forum topic Click'n'Load requests name as their source when the link has no topic.
const FALLBACK_SOURCE: &str = "https://cs.rin.ru/forum/";

#[derive(Debug, thiserror::Error)]
pub enum JdError {
    #[error("JDownloader isn't running. Start it, or open the link in the browser instead")]
    NotRunning,

    #[error("JDownloader didn't take the link: {0}")]
    Rejected(String),
}

impl JdError {
    pub fn kind(&self) -> &'static str {
        match self {
            JdError::NotRunning => "jdownloader_not_running",
            JdError::Rejected(_) => "jdownloader_rejected",
        }
    }
}

pub type Result<T> = std::result::Result<T, JdError>;

/// How JDownloader can be reached right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Api,
    ClickNLoadOnly,
    NotRunning,
}

/// What came of sending a link.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Outcome {
    /// JDownloader finished the download; this is where the file is.
    Done { path: PathBuf },
    /// Only Click'n'Load was available: the link is in JDownloader, which shows the progress.
    HandedOff {
        #[serde(rename = "handedOff")]
        handed_off: bool,
    },
}

/// Where JDownloader's two doors are, so tests can point them at a local server.
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub api: String,
    pub click_n_load: String,
}

impl Endpoints {
    pub fn local(api_port: u16) -> Self {
        Self {
            api: format!("http://127.0.0.1:{api_port}"),
            click_n_load: format!("http://127.0.0.1:{CLICK_N_LOAD_PORT}"),
        }
    }
}

/// The HTTP client for JDownloader. It never uses a system proxy, since it only talks to
/// 127.0.0.1, and sets no `Origin` header.
pub fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(15))
        .build()
}

/// Finds out which door JDownloader has open.
pub async fn probe(client: &reqwest::Client, ep: &Endpoints) -> Mode {
    if let Ok(r) = client.get(format!("{}/jd/getVersion", ep.api)).send().await {
        if r.status().is_success() {
            return Mode::Api;
        }
    }
    if let Ok(r) = client
        .get(format!("{}/jdcheck.js", ep.click_n_load))
        .send()
        .await
    {
        if r.status().is_success() {
            return Mode::ClickNLoadOnly;
        }
    }
    Mode::NotRunning
}

fn transport(e: reqwest::Error) -> JdError {
    if e.is_connect() || e.is_timeout() {
        JdError::NotRunning
    } else {
        JdError::Rejected(e.to_string())
    }
}

/// One call to the local API, and its `data`. The request shape lives only here: the method is
/// GET and every parameter is a query parameter, strings as they are and anything else as JSON.
/// This is the part to fix if JDownloader turns out to want it otherwise.
async fn call(client: &reqwest::Client, base: &str, path: &str, params: Value) -> Result<Value> {
    let mut url =
        Url::parse(&format!("{base}/{path}")).map_err(|e| JdError::Rejected(e.to_string()))?;
    if let Some(map) = params.as_object() {
        let mut query = url.query_pairs_mut();
        for (k, v) in map {
            match v {
                Value::String(s) => query.append_pair(k, s),
                other => query.append_pair(k, &other.to_string()),
            };
        }
    }
    let response = client.get(url).send().await.map_err(transport)?;
    let status = response.status();
    if !status.is_success() {
        return Err(JdError::Rejected(format!("it answered with HTTP {status}")));
    }
    let bytes = response.bytes().await.map_err(transport)?;
    let body: Value = serde_json::from_slice(&bytes)
        .map_err(|e| JdError::Rejected(format!("its answer wasn't understood: {e}")))?;
    Ok(body.get("data").cloned().unwrap_or(Value::Null))
}

/// Hands a link to JDownloader's API, to be saved into `dest`. Returns the job to poll.
pub async fn send_api(
    client: &reqwest::Client,
    ep: &Endpoints,
    link: &str,
    dest: &str,
    package: &str,
) -> Result<i64> {
    let data = call(
        client,
        &ep.api,
        "linkgrabberv2/addLinks",
        json!({
            "links": link,
            "destinationFolder": dest,
            "packageName": package,
            "autostart": true,
            "overwritePackagizerRules": true,
        }),
    )
    .await?;
    data.get("id")
        .and_then(|id| id.as_i64().or_else(|| id.as_str()?.parse().ok()))
        .ok_or_else(|| JdError::Rejected("it gave no job for the link".into()))
}

/// Hands a link to Click'n'Load. JDownloader asks the user to allow it the first time.
pub async fn send_click_n_load(
    client: &reqwest::Client,
    ep: &Endpoints,
    link: &str,
    dest: &str,
    package: &str,
    source: &str,
) -> Result<()> {
    let body = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("urls", link)
        .append_pair("dir", dest)
        .append_pair("package", package)
        .append_pair("source", source)
        .finish();
    let response = client
        .post(format!("{}/flash/add", ep.click_n_load))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(transport)?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(JdError::Rejected(format!(
            "Click'n'Load answered with HTTP {}",
            response.status()
        )))
    }
}

/// Follows a job until its files are downloaded, reporting progress. Returns the file, or the
/// package folder when the link held several. A link JDownloader finds offline, or no file at
/// all, ends the wait with [`JdError::Rejected`].
pub async fn poll(
    client: &reqwest::Client,
    ep: &Endpoints,
    job: i64,
    interval: Duration,
    mut on_progress: impl FnMut(Progress),
) -> Result<PathBuf> {
    loop {
        let links = call(
            client,
            &ep.api,
            "downloadsV2/queryLinks",
            json!({
                "jobUUIDs": [job],
                "bytesLoaded": true,
                "bytesTotal": true,
                "finished": true,
                "status": true,
                "availability": true,
                "packageUUID": true,
                "name": true,
            }),
        )
        .await?;
        let links = links.as_array().cloned().unwrap_or_default();
        if links.is_empty() {
            if !crawling(client, ep, job).await? {
                return Err(JdError::Rejected(
                    "it found no file at that link, or the link is offline".into(),
                ));
            }
        } else {
            if let Some(offline) = links
                .iter()
                .find(|l| l["availability"].as_str() == Some("OFFLINE"))
            {
                let name = offline["name"].as_str().unwrap_or("the file");
                return Err(JdError::Rejected(format!("{name} is offline")));
            }
            let sum = |key: &str| links.iter().filter_map(|l| l[key].as_u64()).sum::<u64>();
            let total = sum("bytesTotal");
            on_progress(Progress {
                received: sum("bytesLoaded"),
                total: (total > 0).then_some(total),
            });
            if links.iter().all(|l| l["finished"].as_bool() == Some(true)) {
                return save_path(client, ep, &links).await;
            }
        }
        tokio::time::sleep(interval).await;
    }
}

/// Whether JDownloader is still looking into a job's link.
async fn crawling(client: &reqwest::Client, ep: &Endpoints, job: i64) -> Result<bool> {
    let jobs = call(
        client,
        &ep.api,
        "linkgrabberv2/queryLinkCrawlerJobs",
        json!({ "jobUUIDs": [job] }),
    )
    .await?;
    Ok(jobs.as_array().is_some_and(|jobs| {
        jobs.iter()
            .any(|j| j["crawling"].as_bool() == Some(true) || j["checking"].as_bool() == Some(true))
    }))
}

async fn save_path(client: &reqwest::Client, ep: &Endpoints, links: &[Value]) -> Result<PathBuf> {
    let package = links[0]["packageUUID"].clone();
    let packages = call(
        client,
        &ep.api,
        "downloadsV2/queryPackages",
        json!({ "packageUUIDs": [package], "saveTo": true }),
    )
    .await?;
    let folder = packages
        .get(0)
        .and_then(|p| p["saveTo"].as_str())
        .ok_or_else(|| JdError::Rejected("it didn't say where the file went".into()))?;
    let mut path = PathBuf::from(folder);
    if let [only] = links {
        if let Some(name) = only["name"].as_str() {
            path.push(name);
        }
    }
    Ok(path)
}

/// The Click'n'Load source for a link: its topic's URL.
pub fn source_or_default(source: Option<String>) -> String {
    source
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| FALLBACK_SOURCE.into())
}

/// Sends a link to JDownloader in whichever way it can be reached, and follows it when it can.
pub async fn send(
    client: &reqwest::Client,
    ep: &Endpoints,
    link: &str,
    dest: &str,
    package: &str,
    source: &str,
    on_progress: impl FnMut(Progress),
) -> Result<Outcome> {
    match probe(client, ep).await {
        Mode::NotRunning => Err(JdError::NotRunning),
        Mode::ClickNLoadOnly => {
            send_click_n_load(client, ep, link, dest, package, source).await?;
            Ok(Outcome::HandedOff { handed_off: true })
        }
        Mode::Api => {
            let job = send_api(client, ep, link, dest, package).await?;
            let path = poll(client, ep, job, POLL_INTERVAL, on_progress).await?;
            Ok(Outcome::Done { path })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    struct Seen {
        method: String,
        path: String,
        body: String,
        origin: Option<String>,
    }

    /// A local server answering `(status, body)` per request path; unmatched paths get 404.
    /// Returns its base URL and the requests it saw.
    fn serve(
        route: impl Fn(&Seen, usize) -> (u16, String) + Send + Sync + 'static,
    ) -> (String, Arc<Mutex<Vec<Seen>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let route = Arc::new(route);
        let log = seen.clone();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                let mut words = line.split_whitespace();
                let method = words.next().unwrap_or("").to_string();
                let path = words.next().unwrap_or("").to_string();
                let (mut length, mut origin) = (0usize, None);
                loop {
                    let mut h = String::new();
                    if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
                        break;
                    }
                    if let Some((k, v)) = h.trim_end().split_once(':') {
                        if k.eq_ignore_ascii_case("content-length") {
                            length = v.trim().parse().unwrap_or(0);
                        }
                        if k.eq_ignore_ascii_case("origin") {
                            origin = Some(v.trim().to_string());
                        }
                    }
                }
                let mut body = vec![0; length];
                let _ = reader.read_exact(&mut body);
                let request = Seen {
                    method,
                    path,
                    body: String::from_utf8_lossy(&body).into_owned(),
                    origin,
                };
                let n = {
                    let mut log = log.lock().unwrap();
                    log.push(request.clone());
                    log.len()
                };
                let (status, body) = route(&request, n);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        (base, seen)
    }

    fn data(value: Value) -> (u16, String) {
        (200, json!({ "data": value }).to_string())
    }

    fn query(path: &str) -> Vec<(String, String)> {
        Url::parse(&format!("http://x{path}"))
            .unwrap()
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    fn param(path: &str, key: &str) -> Option<String> {
        query(path)
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// An address nothing listens on.
    fn closed() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", l.local_addr().unwrap())
    }

    fn eps(api: &str, cnl: &str) -> Endpoints {
        Endpoints {
            api: api.into(),
            click_n_load: cnl.into(),
        }
    }

    #[tokio::test]
    async fn probe_tells_the_three_modes_apart() {
        let c = client().unwrap();
        let (api, _) = serve(|r, _| {
            if r.path == "/jd/getVersion" {
                data(json!(12345))
            } else {
                (404, String::new())
            }
        });
        let (cnl, _) = serve(|r, _| {
            if r.path == "/jdcheck.js" {
                (200, "jdownloader=true;".into())
            } else {
                (404, String::new())
            }
        });
        assert_eq!(probe(&c, &eps(&api, &cnl)).await, Mode::Api);
        assert_eq!(probe(&c, &eps(&closed(), &cnl)).await, Mode::ClickNLoadOnly);
        assert_eq!(
            probe(&c, &eps(&closed(), &closed())).await,
            Mode::NotRunning
        );
    }

    #[tokio::test]
    async fn add_links_sends_the_folder_and_asks_for_autostart() {
        let (api, seen) = serve(|_, _| data(json!({ "id": 777 })));
        let c = client().unwrap();
        let job = send_api(
            &c,
            &eps(&api, &closed()),
            "https://host/f?a=1&b=2",
            "/games",
            "Pkg",
        )
        .await
        .unwrap();
        assert_eq!(job, 777);
        let seen = seen.lock().unwrap();
        let path = &seen[0].path;
        assert!(path.starts_with("/linkgrabberv2/addLinks?"), "{path}");
        assert_eq!(param(path, "links").unwrap(), "https://host/f?a=1&b=2");
        assert_eq!(param(path, "destinationFolder").unwrap(), "/games");
        assert_eq!(param(path, "packageName").unwrap(), "Pkg");
        assert_eq!(param(path, "autostart").unwrap(), "true");
        assert_eq!(param(path, "overwritePackagizerRules").unwrap(), "true");
        assert_eq!(param(path, "autoExtract"), None);
        assert_eq!(seen[0].origin, None);
    }

    #[tokio::test]
    async fn poll_reports_progress_and_returns_the_file() {
        let (api, _) = serve(|r, n| {
            if r.path.starts_with("/downloadsV2/queryLinks") {
                let done = n >= 2;
                data(json!([{
                    "bytesLoaded": if done { 100 } else { 40 },
                    "bytesTotal": 100,
                    "finished": done,
                    "name": "game.zip",
                    "packageUUID": 5,
                    "availability": "ONLINE",
                }]))
            } else if r.path.starts_with("/downloadsV2/queryPackages") {
                data(json!([{ "saveTo": "/games" }]))
            } else {
                (404, String::new())
            }
        });
        let c = client().unwrap();
        let mut seen = Vec::new();
        let path = poll(
            &c,
            &eps(&api, &closed()),
            9,
            Duration::from_millis(5),
            |p| seen.push(p),
        )
        .await
        .unwrap();
        assert_eq!(path, PathBuf::from("/games/game.zip"));
        assert_eq!(seen.first().unwrap().received, 40);
        assert_eq!(
            seen.last().unwrap(),
            &Progress {
                received: 100,
                total: Some(100)
            }
        );
    }

    #[tokio::test]
    async fn poll_ends_when_the_link_is_offline_or_unknown() {
        let (offline, _) = serve(|_, _| {
            data(json!([{ "name": "x.zip", "availability": "OFFLINE", "finished": false }]))
        });
        let c = client().unwrap();
        let err = poll(
            &c,
            &eps(&offline, &closed()),
            1,
            Duration::from_millis(5),
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind(), "jdownloader_rejected");
        assert!(err.to_string().contains("offline"));

        // No links and no crawling job: JDownloader found nothing, so the wait ends.
        let (empty, _) = serve(|_, _| data(json!([])));
        let err = poll(
            &c,
            &eps(&empty, &closed()),
            1,
            Duration::from_millis(5),
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind(), "jdownloader_rejected");
    }

    #[tokio::test]
    async fn poll_keeps_waiting_while_the_link_is_crawled() {
        let (api, _) = serve(|r, n| {
            if r.path.starts_with("/downloadsV2/queryLinks") {
                if n < 4 {
                    data(json!([]))
                } else {
                    data(
                        json!([{ "bytesLoaded": 1, "bytesTotal": 1, "finished": true,
                        "name": "a.zip", "packageUUID": 1 }]),
                    )
                }
            } else if r.path.starts_with("/linkgrabberv2/queryLinkCrawlerJobs") {
                data(json!([{ "crawling": true }]))
            } else {
                data(json!([{ "saveTo": "/d" }]))
            }
        });
        let c = client().unwrap();
        let path = poll(
            &c,
            &eps(&api, &closed()),
            1,
            Duration::from_millis(5),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(path, PathBuf::from("/d/a.zip"));
    }

    #[tokio::test]
    async fn send_with_nothing_running_is_not_running() {
        let c = client().unwrap();
        let err = send(
            &c,
            &eps(&closed(), &closed()),
            "https://h/f",
            "/d",
            "P",
            "s",
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind(), "jdownloader_not_running");
        let err = send_api(&c, &eps(&closed(), &closed()), "l", "/d", "P")
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "jdownloader_not_running");
    }

    #[tokio::test]
    async fn api_errors_are_rejections() {
        let (api, _) = serve(|_, _| (500, "{}".into()));
        let c = client().unwrap();
        let err = send_api(&c, &eps(&api, &closed()), "l", "/d", "P")
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "jdownloader_rejected");
    }

    #[tokio::test]
    async fn click_n_load_posts_the_form_and_hands_off() {
        let (cnl, seen) = serve(|r, _| {
            if r.path == "/jdcheck.js" {
                (200, "jdownloader=true;".into())
            } else {
                (200, "success".into())
            }
        });
        let c = client().unwrap();
        let outcome = send(
            &c,
            &eps(&closed(), &cnl),
            "https://host/f",
            "/games",
            "Pkg",
            "https://cs.rin.ru/forum/viewtopic.php?t=1",
            |_| panic!("no progress without the API"),
        )
        .await
        .unwrap();
        assert_eq!(outcome, Outcome::HandedOff { handed_off: true });
        let seen = seen.lock().unwrap();
        let post = seen.iter().find(|s| s.method == "POST").unwrap();
        assert_eq!(post.path, "/flash/add");
        assert_eq!(post.origin, None);
        let form: Vec<(String, String)> = url::form_urlencoded::parse(post.body.as_bytes())
            .into_owned()
            .collect();
        let get = |k: &str| form.iter().find(|(n, _)| n == k).unwrap().1.clone();
        assert_eq!(get("urls"), "https://host/f");
        assert_eq!(get("dir"), "/games");
        assert_eq!(get("package"), "Pkg");
        assert_eq!(get("source"), "https://cs.rin.ru/forum/viewtopic.php?t=1");
    }

    #[test]
    fn outcomes_serialize_as_the_frontend_reads_them() {
        let done = Outcome::Done {
            path: "/d/a.zip".into(),
        };
        assert_eq!(
            serde_json::to_value(done).unwrap(),
            json!({ "path": "/d/a.zip" })
        );
        let off = Outcome::HandedOff { handed_off: true };
        assert_eq!(
            serde_json::to_value(off).unwrap(),
            json!({ "handedOff": true })
        );
    }
}
