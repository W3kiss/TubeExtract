# TubeExtract

Desktop GUI downloader for YouTube, VK, Rutube, TikTok and SoundCloud.
Built with Tauri 2 (Rust), SvelteKit 2, Svelte 5 runes, TypeScript and Tailwind CSS 4.

## Download

Grab **TubeExtract.exe** (portable) or the `*-setup.exe` installer from the
[Releases](https://github.com/W3kiss/TubeExtract/releases) page. Windows 10/11 with WebView2 (preinstalled).

## Features

- YouTube, VK, Rutube, TikTok and SoundCloud: video (choose quality, subtitles) or MP3 320 kbps with ID3 tags and album art
- Trim a segment before downloading, download history, Ctrl+K to focus the link field
- 8 interface languages (RU, EN, UK, DE, ES, FR, PL, TR) and 13 colour themes, chosen on first launch


## Run

```bash
npm install
npm run tauri dev      # development
npm run tauri build    # installer in src-tauri/target/release/bundle
```

On first launch the app downloads its components into the app data folder
(`%APPDATA%\com.tubeextract.app\bin` on Windows, `~/Library/Application Support/com.tubeextract.app/bin`
on macOS, `~/.local/share/com.tubeextract.app/bin` on Linux):

| Component | Source |
|---|---|
| yt-dlp | github.com/yt-dlp/yt-dlp releases |
| ffmpeg / ffprobe | github.com/eugeneware/ffmpeg-static releases |
| deno | github.com/denoland/deno releases (yt-dlp needs a JS runtime for YouTube) |

Delete that folder to force a re-install.

## Layout

- `src-tauri/src/deps.rs`: detects the platform, downloads the binaries and streams `setup-progress` events
- `src-tauri/src/downloader.rs`: metadata (`yt-dlp -J`), downloads with `download-progress` events, cancel, ID3 tagging
- `src-tauri/src/sites.rs`: URL validation per platform (mirrors `src/lib/sites.ts`)
- `src-tauri/src/error.rs`: turns yt-dlp's stderr into readable error messages
- `src/routes/+page.svelte`: the main UI and its state (`loading_components | ready | fetching_metadata | downloading | error`)
