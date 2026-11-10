//! Self-contained dependency management.
//!
//! On first launch the app downloads `yt-dlp`, `ffmpeg`/`ffprobe` and `deno`
//! (the JavaScript runtime yt-dlp needs for YouTube) into
//! `<AppData>/bin`, streaming progress to the GUI via the `setup-progress` event.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// How a downloaded asset is turned into the final executable.
#[derive(Clone, Copy)]
enum Packaging {
    /// The asset is the executable itself.
    Raw,
    /// The asset is a single gzip-compressed executable.
    Gzip,
    /// The asset is a zip archive; extract the entry with this file name.
    Zip(&'static str),
}

struct Component {
    /// Stable id sent to the frontend.
    id: &'static str,
    /// Human readable name for the loading overlay.
    label: &'static str,
    /// Executable file name inside the bin dir.
    file: String,
    url: String,
    packaging: Packaging,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetupProgress {
    pub component: String,
    pub label: String,
    pub index: usize,
    pub count: usize,
    /// "downloading" | "extracting" | "done"
    pub stage: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    /// 0..=100 across all components.
    pub overall: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepsStatus {
    pub ready: bool,
    pub missing: Vec<String>,
    pub bin_dir: String,
}

const EXE: &str = if cfg!(windows) { ".exe" } else { "" };

pub fn bin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Cannot resolve the application data directory: {e}"))?
        .join("bin");
    Ok(dir)
}

pub fn exe_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    Ok(bin_dir(app)?.join(format!("{name}{EXE}")))
}

/// The pre-compiled binaries for the current OS / CPU architecture.
fn components() -> Result<Vec<Component>, String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // yt-dlp — official standalone builds.
    let ytdlp_asset = match (os, arch) {
        ("windows", "aarch64") => "yt-dlp_arm64.exe",
        ("windows", _) => "yt-dlp.exe",
        ("macos", _) => "yt-dlp_macos", // universal binary
        ("linux", "x86_64") => "yt-dlp_linux",
        ("linux", "aarch64") => "yt-dlp_linux_aarch64",
        _ => return Err(format!("Unsupported platform: {os}/{arch}")),
    };

    // ffmpeg / ffprobe — static builds published on GitHub by ffmpeg-static.
    // Windows on ARM runs the x64 build through emulation.
    let ff_platform = match (os, arch) {
        ("windows", _) => "win32-x64",
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", _) => "darwin-x64",
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        _ => return Err(format!("Unsupported platform: {os}/{arch}")),
    };

    // deno — JS runtime yt-dlp uses to solve YouTube's player challenges.
    let deno_target = match (os, arch) {
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("windows", _) => "x86_64-pc-windows-msvc",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", _) => "x86_64-apple-darwin",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("linux", _) => "x86_64-unknown-linux-gnu",
        _ => return Err(format!("Unsupported platform: {os}/{arch}")),
    };

    const FF: &str = "https://github.com/eugeneware/ffmpeg-static/releases/latest/download";
    Ok(vec![
        Component {
            id: "yt-dlp",
            label: "yt-dlp",
            file: format!("yt-dlp{EXE}"),
            url: format!("https://github.com/yt-dlp/yt-dlp/releases/latest/download/{ytdlp_asset}"),
            packaging: Packaging::Raw,
        },
        Component {
            id: "ffmpeg",
            label: "FFmpeg",
            file: format!("ffmpeg{EXE}"),
            url: format!("{FF}/ffmpeg-{ff_platform}.gz"),
            packaging: Packaging::Gzip,
        },
        Component {
            id: "ffprobe",
            label: "FFprobe",
            file: format!("ffprobe{EXE}"),
            url: format!("{FF}/ffprobe-{ff_platform}.gz"),
            packaging: Packaging::Gzip,
        },
        Component {
            id: "deno",
            label: "Deno JS runtime",
            file: format!("deno{EXE}"),
            url: format!("https://github.com/denoland/deno/releases/latest/download/deno-{deno_target}.zip"),
            packaging: Packaging::Zip(if cfg!(windows) { "deno.exe" } else { "deno" }),
        },
    ])
}

fn is_installed(path: &Path) -> bool {
    std::fs::metadata(path).map(|m| m.is_file() && m.len() > 0).unwrap_or(false)
}

pub fn status(app: &AppHandle) -> Result<DepsStatus, String> {
    let dir = bin_dir(app)?;
    let missing = components()?
        .into_iter()
        .filter(|c| !is_installed(&dir.join(&c.file)))
        .map(|c| c.label.to_string())
        .collect::<Vec<_>>();
    Ok(DepsStatus {
        ready: missing.is_empty(),
        missing,
        bin_dir: dir.to_string_lossy().into_owned(),
    })
}

pub async fn install_missing(app: AppHandle) -> Result<(), String> {
    let dir = bin_dir(&app)?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;

    let todo: Vec<Component> = components()?
        .into_iter()
        .filter(|c| !is_installed(&dir.join(&c.file)))
        .collect();
    let count = todo.len();
    if count == 0 {
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .user_agent(concat!("TubeExtract/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    for (index, comp) in todo.iter().enumerate() {
        let emit = |stage: &str, downloaded: u64, total: Option<u64>| {
            let frac = match (stage, total) {
                ("done", _) => 1.0,
                ("extracting", _) => 0.97,
                (_, Some(t)) if t > 0 => (downloaded as f64 / t as f64).min(1.0) * 0.95,
                _ => 0.0,
            };
            let _ = app.emit(
                "setup-progress",
                SetupProgress {
                    component: comp.id.into(),
                    label: comp.label.into(),
                    index,
                    count,
                    stage: stage.into(),
                    downloaded,
                    total,
                    overall: ((index as f64 + frac) / count as f64) * 100.0,
                },
            );
        };

        let download_path = dir.join(format!("{}.download", comp.file));
        download_file(&client, &comp.url, &download_path, |d, t| emit("downloading", d, t))
            .await
            .map_err(|e| format!("Failed to download {}: {e}", comp.label))?;

        emit("extracting", 0, None);
        let final_path = dir.join(&comp.file);
        let packaging = comp.packaging;
        let (src, dst) = (download_path.clone(), final_path.clone());
        tokio::task::spawn_blocking(move || unpack(packaging, &src, &dst))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("Failed to install {}: {e}", comp.label))?;
        let _ = tokio::fs::remove_file(&download_path).await;
        emit("done", 0, None);
    }
    Ok(())
}

/// Streams `url` into `dest`, reporting `(downloaded, total)` at most ~10x per second.
async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| describe_reqwest(&e))?
        .error_for_status()
        .map_err(|e| describe_reqwest(&e))?;
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(dest).await.map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut downloaded = 0u64;
    let mut last = Instant::now() - Duration::from_secs(1);

    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| describe_reqwest(&e))?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        if last.elapsed() >= Duration::from_millis(100) {
            last = Instant::now();
            on_progress(downloaded, total);
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    on_progress(downloaded, total);
    if let Some(t) = total {
        if downloaded < t {
            return Err("connection closed before the download finished".into());
        }
    }
    Ok(())
}

fn describe_reqwest(e: &reqwest::Error) -> String {
    if e.is_connect() || e.is_timeout() {
        "network unreachable — check your internet connection".into()
    } else if let Some(status) = e.status() {
        format!("server responded with HTTP {status}")
    } else {
        e.to_string()
    }
}

/// Writes the executable to a temporary file next to `dst`, then atomically renames it.
fn unpack(packaging: Packaging, src: &Path, dst: &Path) -> Result<(), String> {
    let tmp = dst.with_extension("partial");
    {
        let mut out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        match packaging {
            Packaging::Raw => {
                let mut input = std::fs::File::open(src).map_err(|e| e.to_string())?;
                std::io::copy(&mut input, &mut out).map_err(|e| e.to_string())?;
            }
            Packaging::Gzip => {
                let input = std::fs::File::open(src).map_err(|e| e.to_string())?;
                let mut gz = flate2::read::GzDecoder::new(std::io::BufReader::new(input));
                std::io::copy(&mut gz, &mut out).map_err(|e| format!("corrupt archive: {e}"))?;
            }
            Packaging::Zip(entry_name) => {
                let input = std::fs::File::open(src).map_err(|e| e.to_string())?;
                let mut archive = zip::ZipArchive::new(input).map_err(|e| format!("corrupt archive: {e}"))?;
                let index = (0..archive.len())
                    .find(|&i| {
                        archive
                            .by_index(i)
                            .map(|f| f.name().rsplit('/').next() == Some(entry_name))
                            .unwrap_or(false)
                    })
                    .ok_or_else(|| format!("{entry_name} not found in archive"))?;
                let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
                let mut buf = Vec::with_capacity(entry.size() as usize);
                entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                out.write_all(&buf).map_err(|e| e.to_string())?;
            }
        }
        out.flush().map_err(|e| e.to_string())?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }

    if dst.exists() {
        let _ = std::fs::remove_file(dst);
    }
    std::fs::rename(&tmp, dst).map_err(|e| e.to_string())?;
    Ok(())
}
