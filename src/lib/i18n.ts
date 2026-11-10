import { settings, type Lang } from './settings.svelte';
import { en, type Dict, type Key } from './locales/en';
import { ru } from './locales/ru';
import { uk } from './locales/uk';
import { de } from './locales/de';
import { es } from './locales/es';
import { fr } from './locales/fr';
import { pl } from './locales/pl';
import { tr } from './locales/tr';

const dicts: Record<Lang, Dict> = { en, ru, uk, de, es, fr, pl, tr };

export type { Key };

/** Translates a key into the current interface language. Reactive: re-evaluates when the language changes. */
export function t(key: Key, params?: Record<string, string | number>): string {
	const s = dicts[settings.lang]?.[key] ?? en[key] ?? key;
	return params ? s.replace(/\{(\w+)\}/g, (_, k: string) => String(params[k] ?? '')) : s;
}

/** BCP-47 locale for date formatting. */
export const locale = () => settings.lang;
