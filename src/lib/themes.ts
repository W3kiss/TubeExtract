export type ThemeId =
	| 'mono'
	| 'star'
	| 'turquoise'
	| 'violet'
	| 'emerald'
	| 'rose'
	| 'amber'
	| 'graphite'
	| 'midnight'
	| 'forest'
	| 'wine'
	| 'amoled'
	| 'light';

export interface Theme {
	id: ThemeId;
	/** Page background. */
	bg: string;
	/** Foreground "ink" as space-separated RGB channels (used for text, borders, overlays). */
	fg: string;
	/** Accent colour: primary button, progress, active toggles. */
	accent: string;
	/** Text colour on top of the accent. */
	onAccent: string;
	light?: boolean;
}

const DARK = '255 255 255';

export const THEMES: Theme[] = [
	{ id: 'mono', bg: '#030304', fg: DARK, accent: '#ffffff', onAccent: '#050505' },
	{ id: 'star', bg: '#000000', fg: DARK, accent: '#ffffff', onAccent: '#000000' },
	{ id: 'turquoise', bg: '#020809', fg: DARK, accent: '#22d3ee', onAccent: '#01171c' },
	{ id: 'violet', bg: '#06050d', fg: DARK, accent: '#a78bfa', onAccent: '#12092e' },
	{ id: 'emerald', bg: '#020906', fg: DARK, accent: '#34d399', onAccent: '#012015' },
	{ id: 'rose', bg: '#0a0407', fg: DARK, accent: '#fb7185', onAccent: '#2a0610' },
	{ id: 'amber', bg: '#090703', fg: DARK, accent: '#fbbf24', onAccent: '#251800' },
	{ id: 'graphite', bg: '#0e0f11', fg: DARK, accent: '#e5e7eb', onAccent: '#15161a' },
	{ id: 'midnight', bg: '#030615', fg: DARK, accent: '#60a5fa', onAccent: '#05173a' },
	{ id: 'forest', bg: '#030905', fg: DARK, accent: '#4ade80', onAccent: '#04200e' },
	{ id: 'wine', bg: '#0a0306', fg: DARK, accent: '#f87171', onAccent: '#2b0707' },
	{ id: 'amoled', bg: '#000000', fg: DARK, accent: '#d4d4d8', onAccent: '#000000' },
	{ id: 'light', bg: '#f4f5f8', fg: '17 19 28', accent: '#2563eb', onAccent: '#ffffff', light: true }
];

export const THEME_IDS = THEMES.map((t) => t.id);

export const themeById = (id: ThemeId): Theme => THEMES.find((t) => t.id === id) ?? THEMES[1];

function hexToChannels(hex: string): string {
	const h = hex.replace('#', '');
	return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)).join(' ');
}

/** Writes the theme's colours as CSS variables on <html>; app.css derives everything else from them. */
export function applyTheme(id: ThemeId) {
	if (typeof document === 'undefined') return;
	const t = themeById(id);
	const root = document.documentElement;
	root.dataset.theme = t.id;
	root.style.setProperty('--fg', t.fg);
	root.style.setProperty('--bgc', hexToChannels(t.bg));
	root.style.setProperty('--accent', t.accent);
	root.style.setProperty('--on-accent', t.onAccent);
	root.style.colorScheme = t.light ? 'light' : 'dark';
}
