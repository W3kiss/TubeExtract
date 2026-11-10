mod deps;
mod downloader;
mod error;
mod sites;

use downloader::{DownloadOptions, DownloadResult, DownloadState, VideoInfo};
use error::AppError;
use tauri::{AppHandle, Manager, State};

/// Are yt-dlp / ffmpeg / deno present in the app data directory?
#[tauri::command]
fn check_dependencies(app: AppHandle) -> Result<deps::DepsStatus, AppError> {
    Ok(deps::status(&app)?)
}

/// Downloads every missing component, emitting `setup-progress` events.
#[tauri::command]
async fn install_dependencies(app: AppHandle) -> Result<(), AppError> {
    deps::install_missing(app)
        .await
        .map_err(|e| {
            let kind = if e.contains("network") { "network" } else { "setup" };
            AppError::new(kind, "Could not download the required components.").with_details(e)
        })
}

#[tauri::command]
async fn fetch_metadata(app: AppHandle, url: String, site: String) -> Result<VideoInfo, AppError> {
    let parsed = sites::validate(&url, &site).map_err(|m| AppError::new("invalid_url", m))?;
    downloader::fetch_info(&app, parsed.as_str()).await
}

/// Runs a download to completion; progress arrives through `download-progress` events.
#[tauri::command]
async fn start_download(
    app: AppHandle,
    state: State<'_, DownloadState>,
    options: DownloadOptions,
) -> Result<DownloadResult, AppError> {
    downloader::download(app, &state, options).await
}

#[tauri::command]
fn cancel_download(state: State<'_, DownloadState>) -> bool {
    downloader::cancel(&state)
}

#[tauri::command]
fn default_download_dir(app: AppHandle) -> Option<String> {
    app.path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .ok()
        .map(|p| p.join("TubeExtract").to_string_lossy().into_owned())
}

#[tauri::command]
async fn ytdlp_version(app: AppHandle) -> Result<String, AppError> {
    downloader::version(&app).await
}

#[tauri::command]
async fn update_ytdlp(app: AppHandle) -> Result<String, AppError> {
    downloader::self_update(&app).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(DownloadState::default())
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            install_dependencies,
            fetch_metadata,
            start_download,
            cancel_download,
            default_download_dir,
            ytdlp_version,
            update_ytdlp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TubeExtract");
}
