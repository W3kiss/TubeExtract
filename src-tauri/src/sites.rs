//! Platform detection. Mirrors `src/lib/sites.ts` so the backend never trusts the UI blindly.

use url::Url;

const SITES: &[(&str, &str, &[&str])] = &[
    ("youtube", "YouTube", &["youtube.com", "youtu.be", "youtube-nocookie.com"]),
    ("vk", "VK", &["vk.com", "vk.ru", "vkvideo.ru", "vk.video"]),
    ("rutube", "Rutube", &["rutube.ru"]),
    ("tiktok", "TikTok", &["tiktok.com"]),
    ("soundcloud", "SoundCloud", &["soundcloud.com", "snd.sc"]),
];

fn host_matches(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// Returns the site id a URL belongs to, if any.
pub fn detect(url: &Url) -> Option<&'static str> {
    let host = url.host_str()?.to_ascii_lowercase();
    SITES
        .iter()
        .find(|(_, _, domains)| domains.iter().any(|d| host_matches(&host, d)))
        .map(|(id, _, _)| *id)
}

fn label(id: &str) -> &str {
    SITES.iter().find(|(i, _, _)| *i == id).map(|(_, l, _)| *l).unwrap_or(id)
}

/// Parses `raw` and ensures it belongs to the selected `site`.
pub fn validate(raw: &str, site: &str) -> Result<Url, String> {
    let trimmed = raw.trim();
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let url = Url::parse(&with_scheme).map_err(|_| "This doesn't look like a valid link.".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Only http(s) links are supported.".into());
    }
    match detect(&url) {
        Some(found) if found == site => Ok(url),
        Some(found) => Err(format!(
            "This is a {} link, but the {} tab is selected.",
            label(found),
            label(site)
        )),
        None => Err(format!("This link is not a {} link.", label(site))),
    }
}
