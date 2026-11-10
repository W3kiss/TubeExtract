export function formatBytes(bytes?: number | null): string {
	if (bytes == null || !Number.isFinite(bytes)) return '—';
	const units = ['B', 'KB', 'MB', 'GB', 'TB'];
	let v = bytes;
	let i = 0;
	while (v >= 1024 && i < units.length - 1) {
		v /= 1024;
		i++;
	}
	return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export const formatSpeed = (bps?: number | null) => (bps ? `${formatBytes(bps)}/s` : '—');

export function formatDuration(seconds?: number | null): string {
	if (seconds == null || !Number.isFinite(seconds)) return '—';
	const s = Math.max(0, Math.round(seconds));
	const h = Math.floor(s / 3600);
	const m = Math.floor((s % 3600) / 60);
	const sec = s % 60;
	const pad = (n: number) => n.toString().padStart(2, '0');
	return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${m}:${pad(sec)}`;
}

/**
 * Parses HH:MM:SS, MM:SS or SS (optionally with fractional seconds).
 * Returns `null` for empty input and `NaN` for malformed input.
 */
export function parseTime(input: string): number | null {
	const s = input.trim();
	if (!s) return null;
	if (!/^\d+(:\d{1,2}){0,2}(\.\d+)?$/.test(s)) return NaN;
	const parts = s.split(':').map(Number);
	if (parts.slice(1).some((p) => p >= 60)) return NaN;
	return parts.reduce((acc, p) => acc * 60 + p, 0);
}

export function toHms(seconds: number): string {
	const s = Math.max(0, Math.floor(seconds));
	const pad = (n: number) => n.toString().padStart(2, '0');
	return `${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
}

/** Localised date + time, e.g. "12 Nov 2026, 14:05". */
export function formatWhen(ts: number, locale: string): string {
	try {
		return new Date(ts).toLocaleString(locale, { dateStyle: 'medium', timeStyle: 'short' });
	} catch {
		return new Date(ts).toLocaleString();
	}
}
