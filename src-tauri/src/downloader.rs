//! yt-dlp integration: metadata probing, downloads with live progress, ID3 tagging.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::deps;
use crate::error::{self, AppError};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct DownloadState {
    pid: Mutex<Option<u32>>,
    cancelled: AtomicBool,
    busy: AtomicBool,
}

struct BusyGuard<'a>(&'a DownloadState);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        *self.0.pid.lock().unwrap() = None;
        self.0.busy.store(false, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Types exchanged with the frontend
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QualityOption {
    pub key: String,
    pub label: String,
    pub height: u32,
    pub fps: u32,
    pub approx_size: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub webpage_url: String,
    pub extractor: String,
    pub has_video: bool,
    pub qualities: Vec<QualityOption>,
    pub audio_size: Option<u64>,
    pub subtitle_langs: Vec<String>,
    pub auto_caption_langs: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityChoice {
    pub height: u32,
    pub fps: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOptions {
    pub url: String,
    pub site: String,
    /// "audio" | "video"
    pub mode: String,
    pub quality: Option<QualityChoice>,
    pub subtitles: bool,
    pub sub_langs: String,
    pub auto_subs: bool,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub output_dir: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub phase: String,
    pub percent: Option<f64>,
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    pub stream: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    pub file_path: String,
    pub tags_written: bool,
    pub cover_embedded: bool,
}

// ---------------------------------------------------------------------------
// Process helpers
// ---------------------------------------------------------------------------

fn ytdlp_command(app: &AppHandle) -> Result<Command, AppError> {
    let bin = deps::bin_dir(app)?;
    let exe = deps::exe_path(app, "yt-dlp")?;
    if !exe.exists() {
        return Err(AppError::new("setup", "yt-dlp is not installed. Restart the app to install components."));
    }
    let mut cmd = Command::new(exe);

    // Put our bin dir first on PATH so yt-dlp finds deno (JS runtime) and ffmpeg.
    let mut paths = vec![bin.clone()];
    if let Some(p) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&p));
    }
    if let Ok(joined) = std::env::join_paths(paths) {
        cmd.env("PATH", joined);
    }
    cmd.env("PYTHONIOENCODING", "utf-8").env("PYTHONUTF8", "1");
    cmd.arg("--ignore-config")
        .args(["--encoding", "utf-8"])
        .arg("--no-colors")
        .arg("--ffmpeg-location")
        .arg(&bin);

    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    #[cfg(unix)]
    cmd.process_group(0); // so cancel can kill yt-dlp *and* its ffmpeg children
    Ok(cmd)
}

fn spawn_error(e: std::io::Error) -> AppError {
    AppError::new("setup", "Could not start yt-dlp.").with_details(e.to_string())
}

fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill").args(["-TERM", &format!("-{pid}")]).status();
    }
}

/// Reads a pipe and forwards every line. Splits on `\r` too, because ffmpeg
/// redraws its progress line with carriage returns.
fn spawn_line_reader<R>(reader: R, is_stderr: bool, tx: mpsc::UnboundedSender<(bool, String)>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut reader = reader;
        let mut chunk = [0u8; 8192];
        let mut line = Vec::<u8>::new();
        loop {
            match reader.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    for &b in &chunk[..n] {
                        if b == b'\n' || b == b'\r' {
                            if !line.is_empty() {
                                let _ = tx.send((is_stderr, String::from_utf8_lossy(&line).into_owned()));
                                line.clear();
                            }
                        } else {
                            line.push(b);
                        }
                    }
                }
            }
        }
        if !line.is_empty() {
            let _ = tx.send((is_stderr, String::from_utf8_lossy(&line).into_owned()));
        }
    });
}

// ---------------------------------------------------------------------------
// Time parsing
// ---------------------------------------------------------------------------

/// Parses `HH:MM:SS`, `MM:SS` or `SS` (each optionally with `.fraction`) into seconds.
pub fn parse_time(input: &str) -> Option<f64> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() > 3 {
        return None;
    }
    let mut total = 0.0;
    for (i, p) in parts.iter().enumerate() {
        let v: f64 = p.parse().ok()?;
        if v < 0.0 || (i > 0 && v >= 60.0) {
            return None;
        }
        total = total * 60.0 + v;
    }
    Some(total)
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

fn quality_label(height: u32, width: Option<u32>, fps: u32) -> String {
    // Vertical videos (TikTok, Shorts) are labelled by their short side.
    let short = width.map(|w| w.min(height)).unwrap_or(height);
    let tag = match short {
        s if s >= 4000 => " 8K",
        s if s >= 2000 => " 4K",
        s if s >= 1400 => " 2K",
        _ => "",
    };
    let fps_str = if fps > 30 { format!(" {fps}fps") } else { String::new() };
    format!("{short}p{fps_str}{tag}")
}

pub async fn fetch_info(app: &AppHandle, url: &str) -> Result<VideoInfo, AppError> {
    let mut cmd = ytdlp_command(app)?;
    cmd.args(["-J", "--no-playlist", "--no-warnings", "--skip-download", "--", url]);

    let output = tokio::time::timeout(Duration::from_secs(120), cmd.output())
        .await
        .map_err(|_| AppError::new("network", "Timed out while fetching video information. Check your connection."))?
        .map_err(spawn_error)?;

    if !output.status.success() {
        return Err(error::from_ytdlp_stderr(&String::from_utf8_lossy(&output.stderr)));
    }
    let v: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| AppError::internal("Could not read the response from yt-dlp.").with_details(e.to_string()))?;

    if v["_type"].as_str() == Some("playlist") {
        return Err(AppError::new(
            "invalid_url",
            "This is a playlist/album link. Please paste a link to a single video or track.",
        ));
    }

    let str_of = |k: &str| v[k].as_str().map(str::to_string);
    let formats = v["formats"].as_array().cloned().unwrap_or_default();

    // (height, fps) -> (width, max size)
    let mut qualities: BTreeMap<(u32, u32), (Option<u32>, Option<u64>)> = BTreeMap::new();
    let mut audio_size: Option<u64> = None;
    let mut has_video = false;
    for f in &formats {
        let size = f["filesize"].as_u64().or_else(|| f["filesize_approx"].as_u64());
        let vcodec = f["vcodec"].as_str().unwrap_or("");
        let acodec = f["acodec"].as_str().unwrap_or("");
        if f["ext"].as_str() == Some("mhtml") || f["format_note"].as_str() == Some("storyboard") {
            continue;
        }
        if vcodec == "none" {
            if acodec != "none" {
                audio_size = audio_size.max(size);
            }
            continue;
        }
        let Some(height) = f["height"].as_u64().filter(|h| *h > 0) else { continue };
        has_video = true;
        let fps = f["fps"].as_f64().map(|x| x.round() as u32).unwrap_or(0);
        let width = f["width"].as_u64().map(|w| w as u32);
        let entry = qualities.entry((height as u32, fps)).or_insert((width, None));
        entry.1 = entry.1.max(size);
    }
    if formats.is_empty() {
        has_video = v["vcodec"].as_str().map(|c| c != "none").unwrap_or(false) || v["height"].is_u64();
    }

    let mut qualities: Vec<QualityOption> = qualities
        .into_iter()
        .rev()
        .map(|((height, fps), (width, size))| QualityOption {
            key: format!("{height}-{fps}"),
            label: quality_label(height, width, fps),
            height,
            fps,
            approx_size: size.map(|s| s + audio_size.unwrap_or(0)),
        })
        .collect();
    // Collapse near-duplicate fps values (e.g. 24/25/30 at the same height) that would show identical labels.
    qualities.dedup_by(|a, b| a.label == b.label);

    let mut subtitle_langs: Vec<String> = v["subtitles"]
        .as_object()
        .map(|o| o.keys().filter(|k| k.as_str() != "live_chat").cloned().collect())
        .unwrap_or_default();
    subtitle_langs.sort();
    let auto_caption_langs = v["automatic_captions"].as_object().map(|o| o.len()).unwrap_or(0);

    Ok(VideoInfo {
        id: str_of("id").unwrap_or_default(),
        title: str_of("track").or_else(|| str_of("title")).unwrap_or_else(|| "Untitled".into()),
        uploader: str_of("artist")
            .or_else(|| str_of("uploader"))
            .or_else(|| str_of("channel"))
            .or_else(|| str_of("creator"))
            .unwrap_or_else(|| "Unknown".into()),
        thumbnail: str_of("thumbnail"),
        duration: v["duration"].as_f64(),
        webpage_url: str_of("webpage_url").unwrap_or_else(|| url.to_string()),
        extractor: str_of("extractor_key").unwrap_or_default(),
        has_video,
        qualities,
        audio_size,
        subtitle_langs,
        auto_caption_langs,
    })
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

const PROGRESS_PREFIX: &str = "TEPROG";

fn na_u64(s: Option<&str>) -> Option<u64> {
    s.and_then(|x| x.parse::<f64>().ok()).filter(|x| x.is_finite() && *x >= 0.0).map(|x| x as u64)
}
fn na_f64(s: Option<&str>) -> Option<f64> {
    s.and_then(|x| x.parse::<f64>().ok()).filter(|x| x.is_finite())
}

/// Maps a yt-dlp post-processor tag to a phase label for the UI.
fn postprocess_phase(line: &str) -> Option<&'static str> {
    let tag = line.strip_prefix('[')?.split(']').next()?;
    Some(match tag {
        "Merger" => "Merging video and audio",
        "ExtractAudio" => "Converting to MP3 320 kbps",
        "ThumbnailsConvertor" => "Preparing cover art",
        "EmbedThumbnail" => "Embedding cover art",
        "Metadata" => "Writing metadata",
        "SubtitlesConvertor" => "Converting subtitles to SRT",
        "EmbedSubtitle" => "Embedding subtitles",
        "VideoRemuxer" | "VideoConvertor" => "Remuxing video",
        "FixupM3u8" | "FixupM4a" | "FixupTimestamp" | "FixupDuration" | "FixupStretched" => "Fixing up container",
        "MoveFiles" => "Finishing",
        _ => return None,
    })
}

fn is_subtitle_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    [".vtt", ".srt", ".ass", ".ssa", ".json3", ".srv1", ".srv2", ".srv3", ".ttml", ".lrc"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// Parses ffmpeg's `time=HH:MM:SS.xx` progress field.
fn ffmpeg_time(line: &str) -> Option<f64> {
    let idx = line.find("time=")?;
    let rest = &line[idx + 5..];
    let token = rest.split_whitespace().next()?;
    parse_time(token)
}

pub async fn download(
    app: AppHandle,
    state: &DownloadState,
    opts: DownloadOptions,
) -> Result<DownloadResult, AppError> {
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err(AppError::internal("Another download is already running."));
    }
    let _guard = BusyGuard(state);
    state.cancelled.store(false, Ordering::SeqCst);

    let url = crate::sites::validate(&opts.url, &opts.site).map_err(|m| AppError::new("invalid_url", m))?;
    let audio_mode = opts.mode == "audio" || opts.site == "soundcloud";

    // --- time range -----------------------------------------------------
    let start = match opts.start_time.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(parse_time(s).ok_or_else(|| AppError::new("format", format!("Invalid start time \"{s}\". Use HH:MM:SS.")))?),
        None => None,
    };
    let end = match opts.end_time.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(parse_time(s).ok_or_else(|| AppError::new("format", format!("Invalid end time \"{s}\". Use HH:MM:SS.")))?),
        None => None,
    };
    if let (Some(s), Some(e)) = (start, end) {
        if e <= s {
            return Err(AppError::new("format", "End time must be after the start time."));
        }
    }
    if let (Some(s), Some(d)) = (start, opts.duration) {
        if s >= d {
            return Err(AppError::new("format", "Start time is beyond the end of the media."));
        }
    }
    let trimmed = start.is_some() || end.is_some();
    let clip_secs = if trimmed {
        let s = start.unwrap_or(0.0);
        end.or(opts.duration).map(|e| (e - s).max(0.1))
    } else {
        opts.duration
    };

    // --- output -----------------------------------------------------------
    let out_dir = PathBuf::from(&opts.output_dir);
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| AppError::internal(format!("Cannot create output folder: {e}")))?;
    let print_file = std::env::temp_dir().join(format!(
        "tubeextract-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
    ));

    let mut cmd = ytdlp_command(&app)?;
    cmd.args(["--no-playlist", "--newline", "--no-mtime", "--progress"])
        .arg("--progress-template")
        .arg(format!(
            "download:{PROGRESS_PREFIX} %(progress.downloaded_bytes)s %(progress.total_bytes)s \
             %(progress.total_bytes_estimate)s %(progress.speed)s %(progress.eta)s"
        ))
        .arg("-P")
        .arg(&out_dir)
        .arg("-o")
        .arg(if trimmed { "%(title).150B [%(id)s] (clip).%(ext)s" } else { "%(title).150B [%(id)s].%(ext)s" })
        .arg("--print-to-file")
        .arg("after_move:filepath")
        .arg(&print_file);

    if audio_mode {
        cmd.args(["-f", "ba/b", "-x", "--audio-format", "mp3", "--audio-quality", "320K"])
            // Album art: grab the thumbnail, convert to JPEG, crop it square and embed it.
            .args(["--embed-thumbnail", "--convert-thumbnails", "jpg"])
            .arg("--ppa")
            .arg("ThumbnailsConvertor+ffmpeg_o:-c:v mjpeg -qmin 1 -qscale:v 2 -vf crop=\"'if(gt(ih,iw),iw,ih)':'if(gt(iw,ih),ih,iw)'\"")
            // ID3: title & artist parsed from the video title / uploader.
            .arg("--embed-metadata")
            .args(["--parse-metadata", "%(track,title)s:%(meta_title)s"])
            .args(["--parse-metadata", "%(artist,uploader,channel)s:%(meta_artist)s"]);
    } else {
        let selector = match &opts.quality {
            Some(q) if q.fps > 0 => format!(
                "bv*[height={h}][fps>{lo}][fps<{hi}]+ba/bv*[height={h}]+ba/b[height={h}]/bv*[height<={h}]+ba/b[height<={h}]/b",
                h = q.height,
                lo = q.fps.saturating_sub(2),
                hi = q.fps + 2
            ),
            Some(q) => format!("bv*[height={h}]+ba/b[height={h}]/bv*[height<={h}]+ba/b[height<={h}]/b", h = q.height),
            None => "bv*+ba/b".to_string(),
        };
        cmd.args(["-f", &selector, "--merge-output-format", "mp4", "--embed-metadata"]);

        if opts.subtitles {
            let langs = if opts.sub_langs.trim().is_empty() { "en,ru" } else { opts.sub_langs.trim() };
            cmd.args(["--write-subs", "--sub-langs", langs, "--convert-subs", "srt", "--embed-subs"])
                // A single failing subtitle track (e.g. HTTP 429) must not abort the video.
                .arg("--ignore-errors");
            if opts.auto_subs {
                cmd.arg("--write-auto-subs");
            }
        }
    }

    if trimmed {
        let s = start.unwrap_or(0.0);
        let e = end.map(|e| format!("{e:.3}")).unwrap_or_else(|| "inf".into());
        cmd.arg("--download-sections").arg(format!("*{s:.3}-{e}")).arg("--force-keyframes-at-cuts");
    }
    cmd.arg("--").arg(url.as_str());

    // --- run --------------------------------------------------------------
    let mut child = cmd.spawn().map_err(spawn_error)?;
    *state.pid.lock().unwrap() = child.id();

    let (tx, mut rx) = mpsc::unbounded_channel::<(bool, String)>();
    spawn_line_reader(child.stdout.take().expect("piped stdout"), false, tx.clone());
    spawn_line_reader(child.stderr.take().expect("piped stderr"), true, tx);

    let mut progress = DownloadProgress {
        phase: "Starting…".into(),
        ..Default::default()
    };
    let _ = app.emit("download-progress", progress.clone());
    let mut last_emit = Instant::now();
    let mut stderr_tail: Vec<String> = Vec::new();
    let mut saw_template_progress = false;

    while let Some((is_stderr, line)) = rx.recv().await {
        let mut force = false;

        if let Some(rest) = line.strip_prefix(PROGRESS_PREFIX) {
            let mut it = rest.split_whitespace();
            let downloaded = na_u64(it.next());
            let total = na_u64(it.next()).or(na_u64(it.next()));
            let speed = na_f64(it.next());
            let eta = na_u64(it.next());
            progress.downloaded = downloaded;
            progress.total = total;
            progress.speed = speed;
            progress.eta = eta;
            progress.percent = match (downloaded, total) {
                (Some(d), Some(t)) if t > 0 => Some((d as f64 / t as f64 * 100.0).min(100.0)),
                _ => None,
            };
            saw_template_progress = true;
        } else if let Some(dest) = line.strip_prefix("[download] Destination: ") {
            if is_subtitle_file(dest) {
                progress.phase = "Downloading subtitles".into();
            } else {
                progress.stream += 1;
                progress.phase = if audio_mode {
                    "Downloading audio".into()
                } else if progress.stream == 1 {
                    "Downloading video".into()
                } else {
                    "Downloading audio".into()
                };
                if trimmed {
                    progress.phase.push_str(" segment");
                }
            }
            progress.percent = Some(0.0);
            progress.speed = None;
            progress.eta = None;
            // Each stream may use a different downloader (native vs ffmpeg for sections).
            saw_template_progress = false;
            force = true;
        } else if line.starts_with("[info] Downloading subtitles") || line.starts_with("[info] Writing video subtitles") {
            progress.phase = "Downloading subtitles".into();
            force = true;
        } else if let Some(phase) = postprocess_phase(&line) {
            if progress.phase != phase {
                progress.phase = phase.into();
                progress.percent = None;
                progress.speed = None;
                progress.eta = None;
                force = true;
            }
        } else if is_stderr && !saw_template_progress && line.contains("time=") {
            // ffmpeg-based section download: derive percent from the timestamp.
            if let (Some(t), Some(len)) = (ffmpeg_time(&line), clip_secs) {
                progress.percent = Some((t / len * 100.0).clamp(0.0, 100.0));
                progress.speed = None;
            }
        } else if is_stderr {
            stderr_tail.push(line.clone());
            if stderr_tail.len() > 60 {
                stderr_tail.remove(0);
            }
        }

        if force || last_emit.elapsed() >= Duration::from_millis(80) {
            last_emit = Instant::now();
            let _ = app.emit("download-progress", progress.clone());
        }
    }

    let status = child.wait().await.map_err(|e| AppError::internal(e.to_string()))?;
    let print_contents = tokio::fs::read_to_string(&print_file).await.unwrap_or_default();
    let _ = tokio::fs::remove_file(&print_file).await;

    if state.cancelled.load(Ordering::SeqCst) {
        return Err(AppError::new("cancelled", "Download cancelled."));
    }
    let reported = print_contents
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.exists());

    // With --ignore-errors (subtitles) yt-dlp exits non-zero on a minor failure
    // even though the media file was produced; only fail if nothing was saved.
    let file_path = match (status.success(), reported) {
        (_, Some(p)) => p,
        (false, None) => return Err(error::from_ytdlp_stderr(&stderr_tail.join("\n"))),
        (true, None) => return Err(AppError::internal("yt-dlp finished but did not report the output file.")),
    };

    // --- ID3 tags ---------------------------------------------------------
    let mut tags_written = false;
    let mut cover_embedded = false;
    if audio_mode && file_path.extension().and_then(|e| e.to_str()) == Some("mp3") {
        let _ = app.emit(
            "download-progress",
            DownloadProgress { phase: "Writing ID3 tags".into(), ..progress.clone() },
        );
        let (title, artist) = (opts.title.clone(), opts.artist.clone());
        let path = file_path.clone();
        if let Ok(Ok((t, c))) = tokio::task::spawn_blocking(move || write_id3(&path, title, artist)).await {
            tags_written = t;
            cover_embedded = c;
        }
    }

    Ok(DownloadResult {
        file_path: file_path.to_string_lossy().into_owned(),
        tags_written,
        cover_embedded,
    })
}

/// Writes Title/Artist frames (keeping the embedded cover). Returns `(tags_written, has_cover)`.
fn write_id3(path: &Path, title: Option<String>, artist: Option<String>) -> Result<(bool, bool), String> {
    use id3::{Tag, TagLike, Version};
    let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());
    let mut changed = false;
    if let Some(t) = title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        tag.set_title(t);
        changed = true;
    }
    if let Some(a) = artist.map(|a| a.trim().to_string()).filter(|a| !a.is_empty()) {
        tag.set_artist(a.clone());
        if tag.album_artist().is_none() {
            tag.set_album_artist(a);
        }
        changed = true;
    }
    if changed {
        tag.write_to_path(path, Version::Id3v24).map_err(|e| e.to_string())?;
    }
    let has_cover = tag.pictures().next().is_some();
    Ok((changed || tag.title().is_some(), has_cover))
}

pub fn cancel(state: &DownloadState) -> bool {
    if !state.busy.load(Ordering::SeqCst) {
        return false;
    }
    state.cancelled.store(true, Ordering::SeqCst);
    if let Some(pid) = *state.pid.lock().unwrap() {
        kill_tree(pid);
    }
    true
}

pub async fn version(app: &AppHandle) -> Result<String, AppError> {
    let mut cmd = ytdlp_command(app)?;
    cmd.arg("--version");
    let out = tokio::time::timeout(Duration::from_secs(30), cmd.output())
        .await
        .map_err(|_| AppError::internal("yt-dlp did not respond."))?
        .map_err(spawn_error)?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub async fn self_update(app: &AppHandle) -> Result<String, AppError> {
    let mut cmd = ytdlp_command(app)?;
    cmd.arg("-U");
    let out = tokio::time::timeout(Duration::from_secs(180), cmd.output())
        .await
        .map_err(|_| AppError::new("network", "Updating yt-dlp timed out."))?
        .map_err(spawn_error)?;
    let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if !out.status.success() {
        return Err(error::from_ytdlp_stderr(&text));
    }
    let summary = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .last()
        .unwrap_or("yt-dlp is up to date")
        .to_string();
    Ok(summary)
}
