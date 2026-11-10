export type SiteId = 'youtube' | 'vk' | 'rutube' | 'tiktok' | 'soundcloud';

export interface Site {
	id: SiteId;
	name: string;
	/** Short badge text shown in the sidebar. */
	badge: string;
	/** One-line description shown under the name in the sidebar. */
	tagline: string;
	/** Accent colour for the tab. */
	color: string;
	domains: string[];
	placeholder: string;
	/** SoundCloud has no video. */
	audioOnly: boolean;
}

export const SITES: Site[] = [
	{
		id: 'youtube',
		tagline: 'Videos, Shorts, music',
		name: 'YouTube',
		badge: 'YT',
		color: '#ff3b3b',
		domains: ['youtube.com', 'youtu.be', 'youtube-nocookie.com'],
		placeholder: 'https://www.youtube.com/watch?v=…',
		audioOnly: false
	},
	{
		id: 'vk',
		tagline: 'Videos & clips',
		name: 'VK',
		badge: 'VK',
		color: '#2f87ff',
		domains: ['vk.com', 'vk.ru', 'vkvideo.ru', 'vk.video'],
		placeholder: 'https://vk.com/video-…',
		audioOnly: false
	},
	{
		id: 'rutube',
		tagline: 'Videos & shows',
		name: 'Rutube',
		badge: 'RT',
		color: '#22c3a6',
		domains: ['rutube.ru'],
		placeholder: 'https://rutube.ru/video/…',
		audioOnly: false
	},
	{
		id: 'tiktok',
		tagline: 'Short clips',
		name: 'TikTok',
		badge: 'TT',
		color: '#ff2d8a',
		domains: ['tiktok.com'],
		placeholder: 'https://www.tiktok.com/@user/video/…',
		audioOnly: false
	},
	{
		id: 'soundcloud',
		tagline: 'Tracks · audio only',
		name: 'SoundCloud',
		badge: 'SC',
		color: '#ff7a1a',
		domains: ['soundcloud.com', 'snd.sc'],
		placeholder: 'https://soundcloud.com/artist/track',
		audioOnly: true
	}
];

export const siteById = (id: SiteId): Site => SITES.find((s) => s.id === id)!;

/** Parses user input into a URL, accepting links without a scheme. */
export function parseUrl(raw: string): URL | null {
	const value = raw.trim();
	if (!value || /\s/.test(value)) return null;
	try {
		const url = new URL(value.includes('://') ? value : `https://${value}`);
		if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
		if (!url.hostname.includes('.')) return null;
		return url;
	} catch {
		return null;
	}
}

/** Which platform does a URL belong to? */
export function detectSite(raw: string): Site | null {
	const url = parseUrl(raw);
	if (!url) return null;
	const host = url.hostname.toLowerCase();
	return SITES.find((s) => s.domains.some((d) => host === d || host.endsWith(`.${d}`))) ?? null;
}

/** Messages are built in the UI (so they can be translated). */
export type UrlCheck =
	| { state: 'empty' }
	| { state: 'invalid' }
	| { state: 'mismatch'; detected: Site | null }
	| { state: 'ok'; url: string };

export function checkUrl(raw: string, site: Site): UrlCheck {
	if (!raw.trim()) return { state: 'empty' };
	const url = parseUrl(raw);
	if (!url) return { state: 'invalid' };
	const detected = detectSite(raw);
	if (detected?.id === site.id) return { state: 'ok', url: url.toString() };
	return { state: 'mismatch', detected };
}
