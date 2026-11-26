use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::header::COOKIE;
use tauri::AppHandle;

use crate::error::{Error, Result};
use crate::{forum_window, phpbb};

const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

pub struct ForumClient {
    http: reqwest::Client,
    last_refresh: Mutex<Option<Instant>>,
}

impl ForumClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(forum_window::USER_AGENT)
            .build()?;
        Ok(Self {
            http,
            last_refresh: Mutex::new(None),
        })
    }

    fn refresh_forum_window(&self, app: &AppHandle) -> Result<()> {
        let mut last = self.last_refresh.lock().unwrap();
        if last.is_some_and(|t| t.elapsed() < REFRESH_INTERVAL) {
            return Ok(());
        }
        *last = Some(Instant::now());
        forum_window::refresh(app)
    }

    pub async fn get(&self, app: &AppHandle, path: &str) -> Result<String> {
        let url = forum_window::forum_url(path)?;
        let cookies = forum_window::cookie_header(app, &url)?;

        let response = self.http.get(url).header(COOKIE, cookies).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if phpbb::is_security_check(&body) {
            self.refresh_forum_window(app)?;
            return Err(Error::SecurityCheck);
        }
        if !status.is_success() {
            return Err(Error::Status(status.as_u16()));
        }
        Ok(body)
    }
}
