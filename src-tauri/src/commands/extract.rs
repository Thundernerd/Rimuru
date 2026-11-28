use std::path::PathBuf;

use tauri::State;

use super::settings;
use crate::cache::Cache;
use crate::error::{Error, Result};
use crate::extract::{self, ExtractError};

/// Extracts the archive at `path` into the folder set in Settings, trying the forum's
/// password on a protected archive. Returns where its files went.
#[tauri::command]
pub async fn extract_archive(cache: State<'_, Cache>, path: String) -> Result<PathBuf> {
    let dest = settings::load(&cache)
        .extract_dir
        .map(PathBuf::from)
        .ok_or(ExtractError::NoDestination)?;
    let archive = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || {
        extract::extract(&archive, &dest, extract::PASSWORD)
    })
    .await
    .map_err(|e| ExtractError::Archive(e.to_string()))?
    .map_err(Error::from)
}
