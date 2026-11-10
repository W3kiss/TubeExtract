use serde::Serialize;

/// Error payload sent to the GUI. `kind` lets the UI pick an icon / headline.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    /// "network" | "invalid_url" | "unavailable" | "format" | "cancelled" | "setup" | "internal"
    pub kind: &'static str,
    pub message: String,
    pub details: Option<String>,
}

impl AppError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self { kind, message: message.into(), details: None }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        let d = details.into();
        if !d.trim().is_empty() {
            self.details = Some(d);
        }
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self::internal(message)
    }
}

/// Turns yt-dlp's stderr into a short, human readable error.
pub fn from_ytdlp_stderr(stderr: &str) -> AppError {
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|l| l.starts_with("ERROR:"))
        .collect();
    let details = if errors.is_empty() {
        stderr.lines().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
    } else {
        errors.join("\n")
    };
    let hay = details.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| hay.contains(n));

    let err = if has(&[
        "getaddrinfo",
        "failed to resolve",
        "name or service not known",
        "temporary failure in name resolution",
        "network is unreachable",
        "connection reset",
        "connection refused",
        "connection aborted",
        "timed out",
        "no route to host",
        "remote end closed connection",
        "urlopen error",
        "ssl:",
    ]) {
        AppError::new("network", "Network error — check your internet connection and try again.")
    } else if has(&["unsupported url"]) {
        AppError::new("invalid_url", "This link isn't supported or doesn't point to a video/track.")
    } else if has(&["private video", "this video is private", "is private"]) {
        AppError::new("unavailable", "This content is private.")
    } else if has(&["sign in to confirm", "age-restricted", "confirm your age", "login required", "log in"]) {
        AppError::new("unavailable", "The platform requires signing in to access this content (age-restricted or bot check).")
    } else if has(&["not available in your country", "geo restrict", "geo-restrict"]) {
        AppError::new("unavailable", "This content is not available in your region.")
    } else if has(&["video unavailable", "has been removed", "not available", "does not exist", "http error 404"]) {
        AppError::new("unavailable", "This content is unavailable or has been removed.")
    } else if has(&["requested format is not available", "no video formats"]) {
        AppError::new("format", "The selected quality isn't available for this video. Pick another one.")
    } else if has(&["http error 403", "forbidden"]) {
        AppError::new("network", "The server refused the request (HTTP 403). Try updating yt-dlp.")
    } else if has(&["http error 429", "too many requests"]) {
        AppError::new("network", "Too many requests — the platform is rate-limiting you. Wait a bit and retry.")
    } else if has(&["ffmpeg", "ffprobe"]) && has(&["not found", "not installed"]) {
        AppError::new("setup", "FFmpeg is missing. Restart the app to re-install components.")
    } else if has(&["no space left", "disk full"]) {
        AppError::new("internal", "Not enough disk space in the output folder.")
    } else if has(&["permission denied", "access is denied"]) {
        AppError::new("internal", "Cannot write to the output folder (permission denied).")
    } else {
        AppError::new("internal", "yt-dlp failed to process this link.")
    };
    err.with_details(details)
}
