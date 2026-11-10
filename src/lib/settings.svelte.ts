import { THEME_IDS, applyTheme, type ThemeId } from './themes';

export const LANGS = [
	{ id: 'ru', code: 'RU', name: 'Русский' },
	{ id: 'en', code: 'EN', name: 'English' },
	{ id: 'uk', code: 'UK', name: 'Українська' },
	{ id: 'de', code: 'DE', name: 'Deutsch' },
	{ id: 'es', code: 'ES', name: 'Español' },
	{ id: 'fr', code: 'FR', name: 'Français' },
	{ id: 'pl', code: 'PL', name: 'Polski' },
	{ id: 'tr', code: 'TR', name: 'Türkçe' }
] as const;

export type Lang = (typeof LANGS)[number]['id'];

const KEY = 'tubeextract.settings.v1';
const isLang = (v: unknown): v is Lang => LANGS.some((l) => l.id === v);
const isTheme = (v: unknown): v is ThemeId => THEME_IDS.includes(v as ThemeId);

class Settings {
	lang = $state<Lang>('en');
	theme = $state<ThemeId>('star');
	/** false until the user finishes the first-launch welcome window */
	welcomed = $state(true);
}

export const settings = new Settings();

function detectLang(): Lang {
	const code = (typeof navigator !== 'undefined' ? navigator.language : 'en').slice(0, 2).toLowerCase();
	return isLang(code) ? code : 'en';
}

function persist() {
	try {
		localStorage.setItem(KEY, JSON.stringify({ lang: settings.lang, theme: settings.theme, welcomed: settings.welcomed }));
	} catch {
		/* storage unavailable — settings just won't persist */
	}
}

/** Reads saved settings (falling back to the system language) and applies the theme right away. */
export function loadSettings() {
	let saved: { lang?: unknown; theme?: unknown; welcomed?: unknown } | null = null;
	try {
		saved = JSON.parse(localStorage.getItem(KEY) ?? 'null');
	} catch {
		/* ignore */
	}
	settings.lang = isLang(saved?.lang) ? saved.lang : detectLang();
	settings.theme = isTheme(saved?.theme) ? saved.theme : 'star';
	// No saved settings → first launch: show the welcome window.
	settings.welcomed = saved?.welcomed === true;
	applyTheme(settings.theme);
	document.documentElement.lang = settings.lang;
}

export function finishWelcome() {
	settings.welcomed = true;
	persist();
}

export function setLang(lang: Lang) {
	settings.lang = lang;
	document.documentElement.lang = lang;
	persist();
}

export function setTheme(theme: ThemeId) {
	settings.theme = theme;
	applyTheme(theme);
	persist();
}
