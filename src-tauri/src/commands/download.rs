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

/// The downloads running now, by the id the frontend gave them, so they can be cancelled.
#[derive(Default)]
pub struct Downloads(Mutex<HashMap<u32, AbortHandle>>);

/// The HTTP client for file hosts, kept apart from the forum's.
pub struct DownloadClient(pub reqwest::Client);

/// Downloads the file a post's download link points to into the folder set in Settings,
/// sending progress on `on_progress`. Returns where the file went.
#[tauri::command]
pub async fn download_file(
    cache: State<'_, Cache>,
    client: State<'_, DownloadClient>,
    downloads: State<'_, Downloads>,
    id: u32,
    url: String,
    on_progress: Channel<Progress>,
) -> Result<PathBuf> {
    let dest = settings::load(&cache)
        .extract_dir
        .map(PathBuf::from)
        .ok_or(ExtractError::NoDestination)?;
    let client = client.0.clone();
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
