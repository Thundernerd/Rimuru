//! The hidden "forum" webview window.
//!
//! It is a real browser pointed at cs.rin.ru: it passes the site's JavaScript
//! security check by itself and is where the user signs in. Its cookie store
//! is then the source of truth for the session that [`crate::client`] uses.
//!
//! This window loads a remote site, so it must never be given IPC access:
//! keep it out of every capability in `capabilities/`.

use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::{Error, Result};

pub const LABEL: &str = "forum";
pub const BASE_URL: &str = "https://cs.rin.ru/forum/";

/// Shared by the webview and the HTTP client so the site sees one browser.
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";

/// Event sent to the main window whenever the forum window finishes loading a page.
pub const PAGE_LOADED_EVENT: &str = "forum-page-loaded";

pub fn forum_url(path: &str) -> Result<Url> {
    Ok(Url::parse(BASE_URL)?.join(path)?)
}

/// Creates the forum window, hidden, and starts loading the forum index.
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

/// Shows the forum window on the given page, e.g. `ucp.php?mode=login`.
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

/// Reloads the forum index in the background so the webview runs the
/// security check again.
pub fn refresh(app: &AppHandle) -> Result<()> {
    get(app)?.navigate(forum_url("index.php")?)?;
    Ok(())
}

/// The webview's cookies for `url` as a `Cookie:` header value. Includes
/// HttpOnly cookies such as the phpBB session id.
///
/// Must not be called from a synchronous command or event handler: it
/// deadlocks on Windows (see the docs for `WebviewWindow::cookies`).
pub fn cookie_header(app: &AppHandle, url: &Url) -> Result<String> {
    let cookies = get(app)?.cookies_for_url(url.clone())?;
    Ok(cookies
        .iter()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<_>>()
        .join("; "))
}
