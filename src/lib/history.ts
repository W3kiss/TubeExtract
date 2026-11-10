import type { SiteId } from './sites';

export interface HistoryItem {
	id: string;
	title: string;
	site: SiteId;
	mode: 'audio' | 'video';
	/** e.g. "1080p60" or "MP3 320k" */
	label: string;
	path: string;
	thumbnail: string | null;
	at: number;
}

const KEY = 'tubeextract.history.v1';
const LIMIT = 200;

export function loadHistory(): HistoryItem[] {
	try {
		const raw = localStorage.getItem(KEY);
		const parsed = raw ? JSON.parse(raw) : [];
		return Array.isArray(parsed) ? parsed : [];
	} catch {
		return [];
	}
}

export function saveHistory(items: HistoryItem[]) {
	try {
		localStorage.setItem(KEY, JSON.stringify(items.slice(0, LIMIT)));
	} catch {
		/* storage unavailable — history simply won't persist */
	}
}

export const newId = () => Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
