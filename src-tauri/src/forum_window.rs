use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::{Error, Result};

pub const LABEL: &str = "forum";
pub const BASE_URL: &str = "https://cs.rin.ru/forum/";

pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";

pub const PAGE_LOADED_EVENT: &str = "forum-page-loaded";

pub fn forum_url(path: &str) -> Result<Url> {
    Ok(Url::parse(BASE_URL)?.join(path)?)
}

pub fn create(app: &AppHandle) -> Result<WebviewWindow> {
    let window =
        WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(forum_url("index.php")?))
            .title("cs.rin.ru")
            .inner_size(1100.0, 800.0)
            .visible(false)
            .user_agent(USER_AGENT)
            .on_page_load(|window, payload| {
                if payload.event() == PageLoadEvent::Finished {
                    let _ = window.app_handle().emit_to(
                        "main",
                        PAGE_LOADED_EVENT,
                        payload.url().to_string(),
                    );
                }
            })
            .build()?;
    Ok(window)
}

fn get(app: &AppHandle) -> Result<WebviewWindow> {
    app.get_webview_window(LABEL).ok_or(Error::NoForumWindow)
}

pub fn show(app: &AppHandle, path: &str) -> Result<()> {
    let window = get(app)?;
    window.navigate(forum_url(path)?)?;
    window.show()?;
    window.set_focus()?;
    Ok(())
}

pub fn hide(app: &AppHandle) -> Result<()> {
    get(app)?.hide()?;
    Ok(())
}

pub fn refresh(app: &AppHandle) -> Result<()> {
    get(app)?.navigate(forum_url("index.php")?)?;
    Ok(())
}

pub fn cookie_header(app: &AppHandle, url: &Url) -> Result<String> {
    let cookies = get(app)?.cookies_for_url(url.clone())?;
    Ok(cookies
        .iter()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<_>>()
        .join("; "))
}
