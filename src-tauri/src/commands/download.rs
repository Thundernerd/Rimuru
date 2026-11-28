use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;
use tauri::ipc::Channel;
use tokio::task::AbortHandle;

use super::settings;
use crate::cache::Cache;
use crate::download::{self, DownloadError, Hosts, Progress};
use crate::error::{Error, Result};
use crate::extract::ExtractError;
use crate::jdownloader::{self, Endpoints, Outcome};

/// The downloads running now, by the id the frontend gave them, so they can be cancelled.
#[derive(Default)]
pub struct Downloads(Mutex<HashMap<u32, AbortHandle>>);

/// The HTTP client for JDownloader on this computer.
pub struct JdClient(pub reqwest::Client);

/// Downloads the file a post's download link points to into the folder set in Settings,
/// sending progress on `on_progress`. Returns where the file went.
#[tauri::command]
pub async fn download_file(
    cache: State<'_, Cache>,
    downloads: State<'_, Downloads>,
    id: u32,
    url: String,
    on_progress: Channel<Progress>,
) -> Result<PathBuf> {
    let dest = settings::load(&cache)
        .extract_dir
        .map(PathBuf::from)
        .ok_or(ExtractError::NoDestination)?;
    // One client, so one cookie jar, per download.
    let client = download::client().map_err(DownloadError::from)?;
    let task = tauri::async_runtime::spawn(async move {
        download::download(&client, &Hosts::default(), &url, &dest, |p| {
            let _ = on_progress.send(p);
        })
        .await
    });
    downloads
        .0
        .lock()
        .unwrap()
        .insert(id, task.inner().abort_handle());
    let result = task.await;
    downloads.0.lock().unwrap().remove(&id);
    match result {
        Ok(outcome) => outcome.map_err(Error::from),
        // Only an aborted task ends without an outcome.
        Err(_) => Err(DownloadError::Cancelled.into()),
    }
}

/// Cancels a running download; its partial file is removed.
#[tauri::command]
pub async fn cancel_download(downloads: State<'_, Downloads>, id: u32) -> Result<()> {
    if let Some(handle) = downloads.0.lock().unwrap().remove(&id) {
        handle.abort();
    }
    Ok(())
}

/// Hands a link to a running JDownloader (the opt-in fallback for links `download_file` can't
/// fetch), saving into the folder set in Settings. With JDownloader's API on, sends progress on
/// `on_progress` and returns where the file went; with only Click'n'Load, returns at once as
/// handed off. Cancelling stops the polling and leaves JDownloader alone.
#[tauri::command]
pub async fn send_to_jdownloader(
    cache: State<'_, Cache>,
    client: State<'_, JdClient>,
    downloads: State<'_, Downloads>,
    id: u32,
    url: String,
    package: Option<String>,
    source: Option<String>,
    on_progress: Channel<Progress>,
) -> Result<Outcome> {
    let settings = settings::load(&cache);
    let dest = settings.extract_dir.ok_or(ExtractError::NoDestination)?;
    // One client, so one cookie jar, per download.
    let client = download::client().map_err(DownloadError::from)?;
    let endpoints = Endpoints::local(settings.jdownloader_port);
    let package = package
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "Rimuru".into());
    let source = jdownloader::source_or_default(source);
    let task = tauri::async_runtime::spawn(async move {
        jdownloader::send(&client, &endpoints, &url, &dest, &package, &source, |p| {
            let _ = on_progress.send(p);
        })
        .await
    });
    downloads
        .0
        .lock()
        .unwrap()
        .insert(id, task.inner().abort_handle());
    let result = task.await;
    downloads.0.lock().unwrap().remove(&id);
    match result {
        Ok(outcome) => outcome.map_err(Error::from),
        Err(_) => Err(DownloadError::Cancelled.into()),
    }
}
