<script lang="ts">
	import { tick } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { revealItemInDir } from '@tauri-apps/plugin-opener';

	import {
		api,
		toAppError,
		type AppError,
		type AppStatus,
		type DownloadProgress,
		type DownloadResult,
		type SetupProgress,
		type VideoInfo
	} from '$lib/api';
	import { SITES, checkUrl, siteById, type SiteId } from '$lib/sites';
	import { formatBytes, formatDuration, formatWhen, parseTime, toHms } from '$lib/format';
	import { t, locale } from '$lib/i18n';
	import { settings } from '$lib/settings.svelte';
	import { loadHistory, newId, saveHistory, type HistoryItem } from '$lib/history';
	import { spotlight } from '$lib/actions';
	import SetupOverlay from '$lib/components/SetupOverlay.svelte';
	import ErrorAlert from '$lib/components/ErrorAlert.svelte';
	import ProgressPanel from '$lib/components/ProgressPanel.svelte';
	import Starfield from '$lib/components/Starfield.svelte';
	import Icon, { type IconName } from '$lib/components/Icon.svelte';
	import BrandIcon from '$lib/components/BrandIcon.svelte';
	import Welcome from '$lib/components/Welcome.svelte';
	import AuthorPanel from '$lib/components/AuthorPanel.svelte';
	import LanguagePicker from '$lib/components/LanguagePicker.svelte';
	import ThemePicker from '$lib/components/ThemePicker.svelte';

	// ── Core UI state (Svelte 5 runes) ──────────────────────────────────────
	let status = $state<AppStatus>('loading_components');
	let error = $state<AppError | null>(null);
	/** What failed last, so "Retry" knows what to re-run. */
	let errorSource = $state<'metadata' | 'download' | null>(null);
	let view = $state<'download' | 'history' | 'settings' | 'authors'>('download');

	// Component setup
	let setupMissing = $state<string[]>([]);
	let setupProgress = $state<SetupProgress | null>(null);
	let setupError = $state<AppError | null>(null);
	let updating = $state(false);
	let updateNote = $state<string | null>(null);

	// Input
	let siteId = $state<SiteId>('youtube');
	let urlInput = $state('');
	let urlEl = $state<HTMLInputElement>();
	let mode = $state<'audio' | 'video'>('video');

	// Metadata
	let info = $state<VideoInfo | null>(null);
	let infoUrl = $state<string | null>(null);
	let fetchToken = 0;

	// Options
	let qualityKey = $state('best');
	let subtitles = $state(false);
	let subLangs = $state('en,ru');
	let autoSubs = $state(false);
	let trimEnabled = $state(false);
	let startTime = $state('');
	let endTime = $state('');
	let tagTitle = $state('');
	let tagArtist = $state('');
	let outputDir = $state('');

	// Download
	let progress = $state<DownloadProgress | null>(null);
	let result = $state<DownloadResult | null>(null);
	let cancelling = $state(false);

	// Library
	let history = $state<HistoryItem[]>([]);

	// ── Derived ─────────────────────────────────────────────────────────────
	const site = $derived(siteById(siteId));
	const urlCheck = $derived(checkUrl(urlInput, site));
	const audioOnly = $derived(site.audioOnly || (info !== null && !info.hasVideo));
	const effectiveMode = $derived<'audio' | 'video'>(audioOnly ? 'audio' : mode);
	const busy = $derived(status === 'fetching_metadata' || status === 'downloading');
	const selectedQuality = $derived(info?.qualities.find((q) => q.key === qualityKey) ?? null);

	const startSec = $derived(trimEnabled ? parseTime(startTime) : null);
	const endSec = $derived(trimEnabled ? parseTime(endTime) : null);
	const trimError = $derived.by(() => {
		if (!trimEnabled) return null;
		if (Number.isNaN(startSec)) return t('trim.errStart');
		if (Number.isNaN(endSec)) return t('trim.errEnd');
		if (startSec != null && endSec != null && endSec <= startSec) return t('trim.errOrder');
		const d = info?.duration;
		if (d != null && startSec != null && startSec >= d) return t('trim.errPast', { d: toHms(d) });
		if (d != null && endSec != null && endSec > d + 1) return t('trim.errExceeds', { d: toHms(d) });
		return null;
	});
	const clipLength = $derived.by(() => {
		if (!trimEnabled || trimError) return null;
		if (startSec == null && endSec == null) return null;
		const end = endSec ?? info?.duration ?? null;
		return end == null ? null : end - (startSec ?? 0);
	});

	const canDownload = $derived(
		urlCheck.state === 'ok' &&
			info !== null &&
			infoUrl === urlCheck.url &&
			!trimError &&
			outputDir.length > 0 &&
			!busy
	);

	/** Why the download button is disabled — shown under it so it never feels broken. */
	const blockReason = $derived.by(() => {
		if (canDownload || busy) return null;
		if (urlCheck.state !== 'ok') return t('block.paste');
		if (!info || infoUrl !== urlCheck.url) return status === 'error' ? t('block.fix') : t('block.loading');
		if (trimError) return trimError;
		if (!outputDir) return t('block.folder');
		return null;
	});

	const estimatedSize = $derived.by(() => {
		if (!info) return null;
		let bytes: number | null;
		if (effectiveMode === 'audio') {
			// 320 kbps MP3 ≈ 40 KB/s.
			bytes = info.duration ? info.duration * 40_000 : null;
		} else {
			bytes = selectedQuality?.approxSize ?? info.qualities[0]?.approxSize ?? null;
		}
		if (bytes != null && clipLength != null && info.duration) bytes *= clipLength / info.duration;
		return bytes;
	});

	const statusMeta = $derived.by(() => {
		switch (status) {
			case 'loading_components':
				return { label: t('status.setup'), dot: 'bg-white animate-pulse' };
			case 'fetching_metadata':
				return { label: t('status.fetching'), dot: 'bg-amber-300 animate-pulse' };
			case 'downloading':
				return { label: t('status.downloading'), dot: 'bg-white animate-pulse' };
			case 'error':
				return { label: t('status.error'), dot: 'bg-red-400' };
			default:
				return { label: t('status.ready'), dot: 'bg-emerald-400' };
		}
	});

	const urlMessage = $derived.by(() => {
		const c = urlCheck;
		if (c.state === 'invalid') return t('url.invalid');
		if (c.state === 'mismatch')
			return c.detected ? t('url.mismatch', { found: c.detected.name, site: site.name }) : t('url.notLink', { site: site.name });
		return '';
	});
	const actLabel = $derived(
		effectiveMode === 'audio' ? t('act.mp3') : selectedQuality ? t('act.videoQ', { q: selectedQuality.label }) : t('act.video')
	);

	const percent = $derived(progress?.percent ?? null);
	const RING = 2 * Math.PI * 42;

	// ── Startup: make sure yt-dlp & ffmpeg exist ────────────────────────────
	async function ensureComponents() {
		setupError = null;
		setupProgress = null;
		status = 'loading_components';
		try {
			const deps = await api.checkDependencies();
			if (!deps.ready) {
				setupMissing = deps.missing;
				await api.installDependencies();
				const after = await api.checkDependencies();
				if (!after.ready) throw { kind: 'setup', message: `Still missing: ${after.missing.join(', ')}` };
			}
			status = 'ready';
		} catch (e) {
			setupError = toAppError(e);
		}
	}

	$effect(() => {
		const unlisteners = [
			listen<SetupProgress>('setup-progress', (e) => (setupProgress = e.payload)),
			listen<DownloadProgress>('download-progress', (e) => (progress = e.payload))
		];
		history = loadHistory();
		ensureComponents();
		api.defaultDownloadDir().then((d) => {
			if (d && !outputDir) outputDir = d;
		});
		return () => unlisteners.forEach((p) => p.then((off) => off()));
	});

	// ── Metadata: auto-fetch shortly after a valid link is pasted ───────────
	$effect(() => {
		const check = urlCheck;
		if (status === 'loading_components' || status === 'downloading') return;
		if (check.state !== 'ok' || check.url === infoUrl) return;
		const timer = setTimeout(() => fetchMetadata(check.url), 450);
		return () => clearTimeout(timer);
	});

	// Clear stale metadata as soon as the link changes.
	$effect(() => {
		const current = urlCheck.state === 'ok' ? urlCheck.url : null;
		if (infoUrl && current !== infoUrl && status !== 'fetching_metadata') {
			info = null;
			infoUrl = null;
			result = null;
		}
	});

	async function fetchMetadata(url: string) {
		const token = ++fetchToken;
		status = 'fetching_metadata';
		error = null;
		result = null;
		info = null;
		infoUrl = url;
		try {
			const data = await api.fetchMetadata(url, siteId);
			if (token !== fetchToken) return;
			info = data;
			qualityKey = data.qualities[0]?.key ?? 'best';
			tagTitle = data.title;
			tagArtist = data.uploader;
			status = 'ready';
		} catch (e) {
			if (token !== fetchToken) return;
			info = null;
			showError(toAppError(e), 'metadata');
		}
	}

	function showError(e: AppError, source: 'metadata' | 'download') {
		error = e;
		errorSource = source;
		status = 'error';
	}

	function dismissError() {
		error = null;
		errorSource = null;
		status = 'ready';
	}

	function retry() {
		const source = errorSource;
		dismissError();
		if (source === 'download') startDownload();
		else if (urlCheck.state === 'ok') fetchMetadata(urlCheck.url);
	}

	// ── Actions ─────────────────────────────────────────────────────────────
	function selectSite(id: SiteId) {
		if (busy) return;
		view = 'download';
		if (id === siteId) return;
		siteId = id;
		error = null;
		if (status === 'error') status = 'ready';
	}

	async function focusUrl() {
		view = 'download';
		await tick();
		urlEl?.focus();
		urlEl?.select();
	}

	function onKeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
			e.preventDefault();
			if (status !== 'downloading' && status !== 'loading_components') focusUrl();
		}
	}

	async function pasteFromClipboard() {
		try {
			const text = await navigator.clipboard.readText();
			if (text) urlInput = text.trim();
		} catch {
			/* clipboard permission denied — user can still Ctrl+V */
		}
	}

	async function chooseFolder() {
		const picked = await openDialog({ directory: true, multiple: false, defaultPath: outputDir || undefined });
		if (typeof picked === 'string') outputDir = picked;
	}

	async function startDownload() {
		if (urlCheck.state !== 'ok' || !info) return;
		const snapshot = {
			title: info.title,
			thumbnail: info.thumbnail,
			site: siteId,
			mode: effectiveMode,
			label: effectiveMode === 'audio' ? 'MP3 320k' : (selectedQuality?.label ?? 'Video')
		};
		error = null;
		result = null;
		cancelling = false;
		progress = null;
		status = 'downloading';
		try {
			const done = await api.startDownload({
				url: urlCheck.url,
				site: siteId,
				mode: effectiveMode,
				quality:
					effectiveMode === 'video' && selectedQuality
						? { height: selectedQuality.height, fps: selectedQuality.fps }
						: null,
				subtitles: effectiveMode === 'video' && subtitles,
				subLangs,
				autoSubs,
				startTime: trimEnabled && startTime.trim() ? startTime.trim() : null,
				endTime: trimEnabled && endTime.trim() ? endTime.trim() : null,
				outputDir,
				title: effectiveMode === 'audio' ? tagTitle : null,
				artist: effectiveMode === 'audio' ? tagArtist : null,
				duration: info.duration
			});
			result = done;
			history = [{ id: newId(), path: done.filePath, at: Date.now(), ...snapshot }, ...history];
			saveHistory(history);
			status = 'ready';
		} catch (e) {
			showError(toAppError(e), 'download');
		} finally {
			cancelling = false;
		}
	}

	async function cancelDownload() {
		cancelling = true;
		await api.cancelDownload().catch(() => {});
	}

	async function updateYtdlp() {
		updating = true;
		updateNote = null;
		try {
			updateNote = await api.updateYtdlp();
		} catch (e) {
			updateNote = toAppError(e).message;
		} finally {
			updating = false;
			// transient feedback only — don't leave version text lying around
			setTimeout(() => (updateNote = null), 5000);
		}
	}

	async function reveal(path: string) {
		try {
			await revealItemInDir(path);
		} catch {
			/* file was moved or deleted */
		}
	}

	function removeFromHistory(id: string) {
		history = history.filter((h) => h.id !== id);
		saveHistory(history);
	}

	function clearHistory() {
		history = [];
		saveHistory(history);
	}

	function setTrimFromDuration() {
		if (!info?.duration) return;
		if (!startTime) startTime = '00:00:00';
		if (!endTime) endTime = toHms(info.duration);
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#snippet head(icon: IconName, title: string, sub: string, tag?: string)}
	<div class="flex items-center gap-3">
		<div class="tile"><Icon name={icon} /></div>
		<div class="min-w-0 flex-1">
			<div class="flex items-center gap-2">
				<h2 class="text-[15px] font-semibold tracking-tight">{title}</h2>
				{#if tag}<span class="chip">{tag}</span>{/if}
			</div>
			<p class="truncate text-xs text-ink-400">{sub}</p>
		</div>
	</div>
{/snippet}

<Starfield />

{#if !settings.welcomed}
	<Welcome />
{/if}

{#if status === 'loading_components'}
	<SetupOverlay missing={setupMissing} progress={setupProgress} error={setupError} onretry={ensureComponents} />
{/if}

<div class="relative z-10 flex h-screen gap-5 p-4">
	<!-- ── Sidebar ────────────────────────────────────────────────────── -->
	<aside class="flex w-[268px] shrink-0 flex-col rounded-[28px] border border-white/8 bg-black/55 p-4 shadow-2xl shadow-black/50 backdrop-blur-xl">
		<div class="flex items-center gap-3 rounded-2xl border border-white/8 bg-white/[.03] p-3">
			<div class="acc-bg grid h-11 w-11 shrink-0 place-items-center rounded-xl">
				<Icon name="download" class="h-5 w-5" stroke={2.4} />
			</div>
			<div class="min-w-0 flex-1">
				<p class="leading-tight font-bold tracking-tight">TubeExtract</p>
				<p class="eyebrow mt-1 truncate !text-[9px] !tracking-[.2em]">{t('app.tagline')}</p>
			</div>
			<span class="h-1.5 w-1.5 rounded-full {statusMeta.dot}"></span>
		</div>

		<div class="mt-5 min-h-0 flex-1 overflow-y-auto pr-0.5">
			<p class="eyebrow mb-2 px-2 !text-[9.5px]">{t('nav.platforms')}</p>
			<nav class="flex flex-col gap-1 rounded-2xl border border-white/8 bg-white/[.02] p-1.5" aria-label={t('nav.platforms')}>
				{#each SITES as s (s.id)}
					{@const active = view === 'download' && s.id === siteId}
					<button
						class="group flex items-center gap-3 rounded-xl border px-2.5 py-2 text-left transition
							{active ? 'border-white/12 bg-white/[.07]' : 'border-transparent hover:bg-white/[.04]'}
							disabled:cursor-not-allowed disabled:opacity-40"
						onclick={() => selectSite(s.id)}
						disabled={busy && !active}
						aria-current={active ? 'page' : undefined}
					>
						<span
							class="tile !h-9 !w-9 !rounded-[11px]"
							style:color={s.color}
							style:background={`${s.color}${active ? '26' : '14'}`}
							style:border-color={active ? `${s.color}77` : `${s.color}33`}
							style:box-shadow={active ? `0 0 22px -6px ${s.color}` : 'none'}
						>
							<BrandIcon id={s.id} class="h-[18px] w-[18px]" />
						</span>
						<span class="min-w-0 flex-1">
							<span class="block truncate text-[13px] font-semibold {active ? 'text-ink-100' : 'text-ink-300 group-hover:text-ink-100'}">{s.name}</span>
							<span class="block truncate text-[11px] text-ink-400">{t(`tag.${s.id}`)}</span>
						</span>
						{#if active}<Icon name="arrow" class="h-4 w-4 shrink-0 text-ink-300" />{/if}
					</button>
				{/each}
			</nav>

			<p class="eyebrow mt-5 mb-2 px-2 !text-[9.5px]">{t('nav.library')}</p>
			<nav class="flex flex-col gap-1 rounded-2xl border border-white/8 bg-white/[.02] p-1.5" aria-label={t('nav.library')}>
				<button
					class="group flex w-full items-center gap-3 rounded-xl border px-2.5 py-2 text-left transition
						{view === 'history' ? 'border-white/12 bg-white/[.07]' : 'border-transparent hover:bg-white/[.04]'}"
					onclick={() => (view = 'history')}
					aria-current={view === 'history' ? 'page' : undefined}
				>
					<span class="tile !h-9 !w-9 !rounded-[11px]"><Icon name="history" class="h-[18px] w-[18px]" /></span>
					<span class="min-w-0 flex-1">
						<span class="block truncate text-[13px] font-semibold {view === 'history' ? 'text-ink-100' : 'text-ink-300 group-hover:text-ink-100'}">{t('nav.history')}</span>
						<span class="block truncate text-[11px] text-ink-400">{t('nav.saved', { n: history.length })}</span>
					</span>
					{#if view === 'history'}<Icon name="arrow" class="h-4 w-4 shrink-0 text-ink-300" />{/if}
				</button>
				<button
					class="group flex w-full items-center gap-3 rounded-xl border px-2.5 py-2 text-left transition
						{view === 'settings' ? 'border-white/12 bg-white/[.07]' : 'border-transparent hover:bg-white/[.04]'}"
					onclick={() => (view = 'settings')}
					aria-current={view === 'settings' ? 'page' : undefined}
				>
					<span class="tile !h-9 !w-9 !rounded-[11px]"><Icon name="settings" class="h-[18px] w-[18px]" /></span>
					<span class="min-w-0 flex-1">
						<span class="block truncate text-[13px] font-semibold {view === 'settings' ? 'text-ink-100' : 'text-ink-300 group-hover:text-ink-100'}">{t('nav.settings')}</span>
						<span class="block truncate font-mono text-[10.5px] text-ink-400">{settings.lang.toUpperCase()} · {t(`theme.${settings.theme}`)}</span>
					</span>
					{#if view === 'settings'}<Icon name="arrow" class="h-4 w-4 shrink-0 text-ink-300" />{/if}
				</button>
				<button
					class="group flex w-full items-center gap-3 rounded-xl border px-2.5 py-2 text-left transition
						{view === 'authors' ? 'border-white/12 bg-white/[.07]' : 'border-transparent hover:bg-white/[.04]'}"
					onclick={() => (view = 'authors')}
					aria-current={view === 'authors' ? 'page' : undefined}
				>
					<span class="tile !h-9 !w-9 !rounded-[11px]"><Icon name="user" class="h-[18px] w-[18px]" /></span>
					<span class="min-w-0 flex-1">
						<span class="block truncate text-[13px] font-semibold {view === 'authors' ? 'text-ink-100' : 'text-ink-300 group-hover:text-ink-100'}">{t('nav.authors')}</span>
						<span class="block truncate text-[11px] text-ink-400">{t('nav.authorsSub')}</span>
					</span>
					{#if view === 'authors'}<Icon name="arrow" class="h-4 w-4 shrink-0 text-ink-300" />{/if}
				</button>
			</nav>
		</div>

		<div class="mt-4 rounded-2xl border border-white/8 bg-white/[.03] p-3">
			<div class="flex items-center gap-3">
				<span class="h-2 w-2 shrink-0 rounded-full {statusMeta.dot}"></span>
				<div class="min-w-0 flex-1">
					<p class="text-[13px] leading-tight font-semibold">{statusMeta.label}</p>
				</div>
				<button
					class="grid h-9 w-9 shrink-0 place-items-center rounded-full border border-white/10 text-ink-300 transition hover:bg-white/8 hover:text-ink-100 disabled:opacity-40"
					onclick={updateYtdlp}
					disabled={updating || busy || status === 'loading_components'}
					title={t('update.check')}
					aria-label={t('update.check')}
				>
					<Icon name="refresh" class="h-4 w-4 {updating ? 'animate-spin' : ''}" />
				</button>
			</div>
			{#if updateNote}<p class="mt-2 line-clamp-2 text-[11px] text-ink-400">{updateNote}</p>{/if}
		</div>
	</aside>

	<!-- ── Main panel ─────────────────────────────────────────────────── -->
	<main class="relative min-w-0 flex-1 overflow-y-auto pr-1">
		<div class="mx-auto max-w-[1160px] space-y-5 px-3 pt-4 pb-10">
			{#if view !== 'authors'}
			<!-- Hero -->
			<header class="rise flex items-start justify-between gap-8">
				<div class="min-w-0">
					<p class="eyebrow flex items-center gap-2 !text-[10px] !text-ink-300">
						<span class="acc-fill h-1.5 w-1.5 rounded-full"></span>
						{view === 'download'
							? t('hero.eyebrowDownload', { site: site.name })
							: view === 'history'
								? t('hero.eyebrowHistory')
								: t('hero.eyebrowSettings')}
					</p>
					{#if view === 'download'}
						<h1 class="hero-title mt-4">{t('hero.dl1')}<br /><span class="text-ink-300">{t('hero.dl2')}</span></h1>
						<p class="mt-4 max-w-xl text-sm leading-relaxed text-ink-400">{t('hero.dlText')}</p>
					{:else if view === 'history'}
						<h1 class="hero-title mt-4">{t('hero.h1')}<br /><span class="text-ink-300">{t('hero.h2')}</span></h1>
						<p class="mt-4 max-w-xl text-sm leading-relaxed text-ink-400">{t('hero.hText')}</p>
					{:else}
						<h1 class="hero-title mt-4">{t('hero.s1')}<br /><span class="text-ink-300">{t('hero.s2')}</span></h1>
						<p class="mt-4 max-w-xl text-sm leading-relaxed text-ink-400">{t('hero.sText')}</p>
					{/if}
				</div>

				<!-- Status orb: platform icon when idle, live ring while downloading -->
				<div class="grid-bg relative hidden h-[116px] w-[176px] shrink-0 place-items-center overflow-hidden rounded-2xl border border-white/8 bg-white/[.02] lg:grid">
					<svg class="h-[92px] w-[92px] {status === 'downloading' && percent == null ? 'animate-spin' : ''}" viewBox="0 0 100 100" aria-hidden="true">
						<circle cx="50" cy="50" r="42" fill="none" stroke="currentColor" stroke-opacity="0.1" stroke-width="2" />
						<circle
							cx="50"
							cy="50"
							r="42"
							fill="none"
							style:stroke={status === 'downloading' || view !== 'download' ? 'var(--accent)' : site.color}
							stroke-width="2.5"
							stroke-linecap="round"
							transform="rotate(-90 50 50)"
							stroke-dasharray={RING}
							stroke-dashoffset={status === 'downloading' ? RING * (1 - (percent ?? 25) / 100) : RING * 0.72}
							class="transition-all duration-300"
						/>
					</svg>
					<div class="absolute inset-0 grid place-items-center">
						{#if status === 'downloading' && percent != null}
							<span class="font-mono text-lg font-semibold tabular-nums">{Math.round(percent)}%</span>
						{:else if view === 'history'}
							<Icon name="history" class="h-7 w-7 text-ink-300" />
						{:else if view === 'settings'}
							<Icon name="palette" class="h-7 w-7 text-ink-300" />
						{:else}
							<span style:color={site.color}><BrandIcon id={site.id} class="h-8 w-8" /></span>
						{/if}
					</div>
				</div>
			</header>

			<!-- Stat chips -->
			<div class="rise flex items-stretch gap-3" style="animation-delay: 60ms">
				<div class="flex items-center gap-3 rounded-2xl border border-white/8 bg-white/[.03] py-2.5 pr-5 pl-3">
					<div class="tile !h-9 !w-9"><Icon name="download" class="h-4 w-4" /></div>
					<div>
						<p class="text-lg leading-none font-bold tabular-nums">{history.length}</p>
						<p class="mt-1 text-[10.5px] text-ink-400">{t('stat.downloads')}</p>
					</div>
				</div>
				<div class="flex items-center gap-3 rounded-2xl border border-white/8 bg-white/[.03] py-2.5 pr-5 pl-3">
					<div class="tile !h-9 !w-9"><Icon name="grid" class="h-4 w-4" /></div>
					<div>
						<p class="text-lg leading-none font-bold tabular-nums">{SITES.length}</p>
						<p class="mt-1 text-[10.5px] text-ink-400">{t('stat.platforms')}</p>
					</div>
				</div>
				<div class="flex items-center gap-3 rounded-2xl border border-white/8 bg-white/[.03] py-2.5 pr-5 pl-3">
					<div class="tile !h-9 !w-9"><Icon name="music" class="h-4 w-4" /></div>
					<div>
						<p class="text-lg leading-none font-bold">{t('stat.mp3')}</p>
						<p class="mt-1 text-[10.5px] text-ink-400">{t('stat.mp3sub')}</p>
					</div>
				</div>
				<div class="hidden flex-1 items-center justify-end gap-3 rounded-2xl border border-white/8 bg-white/[.015] px-5 min-[1180px]:flex">
					<span class="h-px w-8 bg-white/15"></span>
					<span class="eyebrow !text-[9.5px] !tracking-[.12em]">{t('stat.local')}</span>
				</div>
			</div>

			{/if}

			{#if view === 'download'}
				<!-- URL input -->
				<section class="panel rise p-5" style="animation-delay: 100ms" use:spotlight>
					{@render head('link', t('url.title'), t('url.sub'), site.name)}
					<div
						class="mt-4 flex items-center gap-2 rounded-2xl border bg-black/45 p-1.5 pl-4 transition
							{urlCheck.state === 'ok'
							? 'border-emerald-400/35'
							: urlCheck.state === 'mismatch' || urlCheck.state === 'invalid'
								? 'border-amber-400/40'
								: 'border-white/10 focus-within:border-white/30'}"
					>
						<span class="shrink-0" style:color={site.color}><BrandIcon id={site.id} class="h-[18px] w-[18px]" /></span>
						<input
							bind:this={urlEl}
							class="min-w-0 flex-1 bg-transparent py-2 text-[15px] outline-none placeholder:text-ink-500"
							placeholder={site.placeholder}
							bind:value={urlInput}
							disabled={status === 'downloading'}
							spellcheck="false"
							autocomplete="off"
						/>
						{#if urlInput}
							<button class="rounded-lg p-2 text-ink-400 transition hover:bg-white/5 hover:text-ink-100" onclick={() => (urlInput = '')} aria-label={t('url.clear')} disabled={status === 'downloading'}>
								<Icon name="x" class="h-4 w-4" />
							</button>
						{:else}
							<span class="kbd mr-1">Ctrl K</span>
						{/if}
						<button class="btn" onclick={pasteFromClipboard} disabled={status === 'downloading'}>{t('url.paste')}</button>
					</div>

					<div class="mt-2 min-h-5 px-1 text-sm">
						{#if urlCheck.state === 'mismatch' || urlCheck.state === 'invalid'}
							<p class="flex flex-wrap items-center gap-2 text-amber-400">
								<Icon name="warn" class="h-4 w-4 shrink-0" />
								{urlMessage}
								{#if urlCheck.state === 'mismatch' && urlCheck.detected}
									{@const target = urlCheck.detected}
									<button
										class="rounded-md border border-amber-400/25 bg-amber-400/10 px-2 py-0.5 text-xs font-semibold text-amber-400 transition hover:bg-amber-400/20"
										onclick={() => selectSite(target.id)}>{t('url.switch', { site: target.name })}</button
									>
								{/if}
							</p>
						{:else if status === 'fetching_metadata'}
							<p class="flex items-center gap-2 text-ink-400">
								<span class="h-3.5 w-3.5 animate-spin rounded-full border-2 border-ink-600 border-t-white"></span>
								{t('url.fetching')}
							</p>
						{:else if urlCheck.state === 'ok'}
							<p class="flex items-center gap-1.5 text-emerald-400/90"><Icon name="check" class="h-4 w-4" stroke={2.4} /> {t('url.valid', { site: site.name })}</p>
						{/if}
					</div>
				</section>

				{#if error}
					<ErrorAlert {error} ondismiss={dismissError} onretry={error.kind === 'invalid_url' ? undefined : retry} />
				{/if}

				<!-- Metadata card -->
				{#if status === 'fetching_metadata'}
					<div class="panel flex animate-pulse gap-5 p-4">
						<div class="h-[117px] w-52 rounded-2xl bg-white/6"></div>
						<div class="flex-1 space-y-3 py-3">
							<div class="h-4 w-3/4 rounded bg-white/6"></div>
							<div class="h-3 w-1/3 rounded bg-white/6"></div>
							<div class="h-3 w-1/4 rounded bg-white/6"></div>
						</div>
					</div>
				{:else if info}
					<div class="panel rise flex gap-5 p-4" use:spotlight>
						<div class="relative h-[117px] w-52 shrink-0 overflow-hidden rounded-2xl border border-white/8 bg-ink-800">
							{#if info.thumbnail}
								<img src={info.thumbnail} alt="" class="h-full w-full object-cover" referrerpolicy="no-referrer" />
							{/if}
							{#if info.duration}
								<span class="absolute right-2 bottom-2 rounded-md bg-black/80 px-1.5 py-0.5 font-mono text-[11px] text-white">{formatDuration(info.duration)}</span>
							{/if}
						</div>
						<div class="min-w-0 flex-1 py-1">
							<p class="eyebrow flex items-center gap-1.5 !text-[9.5px]"><span style:color={site.color}><BrandIcon id={site.id} class="h-3.5 w-3.5" /></span>{info.extractor}</p>
							<p class="mt-1.5 line-clamp-2 text-lg leading-snug font-semibold tracking-tight select-text">{info.title}</p>
							<p class="mt-1 truncate text-sm text-ink-400">{info.uploader}</p>
							<div class="mt-3 flex flex-wrap gap-1.5">
								{#if info.qualities[0]}<span class="chip">{t('meta.upTo', { q: info.qualities[0].label })}</span>{/if}
								{#if info.subtitleLangs.length}<span class="chip">{t('meta.subs', { n: info.subtitleLangs.length })}</span>{/if}
								{#if !info.hasVideo}<span class="chip">{t('meta.audioOnly')}</span>{/if}
							</div>
						</div>
					</div>
				{/if}

				<div class="grid items-start gap-5 min-[1180px]:grid-cols-2">
					<!-- Format -->
					<section class="panel rise p-5" style="animation-delay: 140ms" use:spotlight>
						{@render head('layers', t('fmt.title'), effectiveMode === 'audio' ? t('fmt.subAudio') : t('fmt.subVideo'), effectiveMode === 'audio' ? t('mode.audio') : t('mode.video'))}

						<div class="relative mt-4 grid grid-cols-2 rounded-xl border border-white/8 bg-black/45 p-1" role="radiogroup" aria-label={t('fmt.title')}>
							<div
								class="acc-fill acc-glow absolute inset-y-1 w-[calc(50%-4px)] rounded-lg transition-transform duration-300 ease-out"
								style:transform={effectiveMode === 'video' ? 'translateX(100%)' : 'translateX(0)'}
								style:left="4px"
							></div>
							<button
								role="radio"
								aria-checked={effectiveMode === 'audio'}
								class="relative z-10 flex items-center justify-center gap-2 rounded-lg py-2.5 text-sm font-semibold transition
									{effectiveMode === 'audio' ? 'acc-ink' : 'text-ink-400 hover:text-ink-100'}"
								onclick={() => (mode = 'audio')}
								disabled={status === 'downloading'}
							>
								<Icon name="music" class="h-4 w-4" />
								{t('mode.audio')} · MP3
							</button>
							<button
								role="radio"
								aria-checked={effectiveMode === 'video'}
								class="relative z-10 flex items-center justify-center gap-2 rounded-lg py-2.5 text-sm font-semibold transition
									{effectiveMode === 'video' ? 'acc-ink' : 'text-ink-400 hover:text-ink-100'}
									disabled:cursor-not-allowed disabled:opacity-40"
								onclick={() => (mode = 'video')}
								disabled={audioOnly || status === 'downloading'}
								title={audioOnly ? t('fmt.audioOnlyTip', { site: site.name }) : undefined}
							>
								<Icon name="video" class="h-4 w-4" />
								{t('mode.video')}
							</button>
						</div>

						<div class="mt-5 grid grid-cols-2 gap-4">
							{#if effectiveMode === 'video'}
								<label class="col-span-2 block">
									<span class="eyebrow mb-1.5 block !text-[9.5px]">{t('fmt.resolution')}</span>
									<select class="field" bind:value={qualityKey} disabled={!info || status === 'downloading'}>
										{#if !info}
											<option value="best">{t('fmt.loadLink')}</option>
										{:else if info.qualities.length === 0}
											<option value="best">{t('fmt.best')}</option>
										{:else}
											{#each info.qualities as q (q.key)}
												<option value={q.key}>{q.label}{q.approxSize ? ` · ~${formatBytes(q.approxSize)}` : ''}</option>
											{/each}
										{/if}
									</select>
								</label>

								<div class="col-span-2">
									<span class="eyebrow mb-1.5 block !text-[9.5px]">{t('fmt.subtitles')}</span>
									<label class="flex cursor-pointer items-center gap-3 rounded-[14px] border border-white/10 bg-black/45 px-3.5 py-2.5 text-sm">
										<input type="checkbox" class="chk" bind:checked={subtitles} disabled={status === 'downloading'} />
										<span>{t('fmt.subsEmbed')}</span>
									</label>
								</div>

								{#if subtitles}
									<label class="col-span-2 block">
										<span class="eyebrow mb-1.5 block !text-[9.5px]">{t('fmt.langs')}</span>
										<input class="field font-mono" bind:value={subLangs} placeholder="en,ru" spellcheck="false" disabled={status === 'downloading'} />
										{#if info}
											<span class="mt-1.5 block truncate text-[11px] text-ink-400">
												{info.subtitleLangs.length
													? t('fmt.available', { list: `${info.subtitleLangs.slice(0, 12).join(', ')}${info.subtitleLangs.length > 12 ? '…' : ''}` })
													: t('fmt.noSubs')}
											</span>
										{/if}
									</label>
									<label class="col-span-2 flex cursor-pointer items-center gap-3 px-1 text-sm text-ink-300">
										<input type="checkbox" class="chk" bind:checked={autoSubs} disabled={status === 'downloading'} />
										{t('fmt.autoSubs')}
										{#if info?.autoCaptionLangs}<span class="font-mono text-xs text-ink-400">({info.autoCaptionLangs})</span>{/if}
									</label>
								{/if}
							{:else}
								<label class="block">
									<span class="eyebrow mb-1.5 block !text-[9.5px]">{t('fmt.tagTitle')}</span>
									<input class="field" bind:value={tagTitle} placeholder={t('fmt.titlePh')} disabled={!info || status === 'downloading'} />
								</label>
								<label class="block">
									<span class="eyebrow mb-1.5 block !text-[9.5px]">{t('fmt.tagArtist')}</span>
									<input class="field" bind:value={tagArtist} placeholder={t('fmt.artistPh')} disabled={!info || status === 'downloading'} />
								</label>
								<p class="col-span-2 flex items-center gap-2 text-xs text-ink-400">
									<Icon name="image" class="h-3.5 w-3.5" />
									{t('fmt.cover')}
								</p>
							{/if}
						</div>
					</section>

					<div class="space-y-5">
						<!-- Trim -->
						<section class="panel rise p-5" style="animation-delay: 180ms" use:spotlight>
							<div class="flex items-center gap-3">
								<div class="min-w-0 flex-1">{@render head('scissors', t('trim.title'), t('trim.sub'))}</div>
								<button
									role="switch"
									aria-checked={trimEnabled}
									aria-label={t('trim.enable')}
									class="relative h-6 w-11 shrink-0 rounded-full border border-white/10 transition {trimEnabled ? 'acc-fill' : 'bg-white/8'} disabled:opacity-50"
									onclick={() => {
										trimEnabled = !trimEnabled;
										if (trimEnabled) setTrimFromDuration();
									}}
									disabled={status === 'downloading'}
								>
									<span class="absolute top-0.5 left-0.5 h-[18px] w-[18px] rounded-full shadow transition-transform {trimEnabled ? 'acc-knob translate-x-5' : 'bg-white'}"></span>
								</button>
							</div>

							{#if trimEnabled}
								<div class="mt-4 grid grid-cols-[1fr_auto_1fr] items-end gap-3">
									<label class="block">
										<span class="eyebrow mb-1.5 block !text-[9.5px]">{t('trim.start')}</span>
										<input
											class="field font-mono {Number.isNaN(startSec) ? '!border-amber-400/50' : ''}"
											bind:value={startTime}
											placeholder="00:00:00"
											disabled={status === 'downloading'}
										/>
									</label>
									<span class="pb-3 text-ink-500"><Icon name="arrow" class="h-4 w-4" /></span>
									<label class="block">
										<span class="eyebrow mb-1.5 block !text-[9.5px]">{t('trim.end')}</span>
										<input
											class="field font-mono {Number.isNaN(endSec) ? '!border-amber-400/50' : ''}"
											bind:value={endTime}
											placeholder={info?.duration ? toHms(info.duration) : 'HH:MM:SS'}
											disabled={status === 'downloading'}
										/>
									</label>
								</div>
								<p class="mt-2 text-xs {trimError ? 'text-amber-400' : 'text-ink-400'}">
									{#if trimError}
										{trimError}
									{:else if clipLength != null}
										{t('trim.len', { len: formatDuration(clipLength) })}
									{:else}
										{t('trim.hint')}
									{/if}
								</p>
							{/if}
						</section>

						<!-- Save to -->
						<section class="panel rise p-5" style="animation-delay: 220ms" use:spotlight>
							<div class="flex items-center gap-3">
								<div class="tile"><Icon name="folder" /></div>
								<div class="min-w-0 flex-1">
									<p class="eyebrow !text-[9.5px]">{t('save.to')}</p>
									<p class="mt-0.5 truncate font-mono text-[13px] select-text" title={outputDir}>{outputDir || t('save.choose')}</p>
								</div>
								<button class="btn" onclick={chooseFolder} disabled={status === 'downloading'}>{t('save.change')}</button>
							</div>
						</section>
					</div>
				</div>

				<!-- Action -->
				<section class="space-y-4">
					{#if status === 'downloading'}
						<ProgressPanel {progress} {cancelling} oncancel={cancelDownload} />
					{:else}
						<button class="btn btn-primary w-full !rounded-2xl !py-4 !text-base" onclick={startDownload} disabled={!canDownload}>
							<Icon name="download" class="h-5 w-5" stroke={2.4} />
							{actLabel}
							{#if estimatedSize}<span class="font-mono text-sm font-medium opacity-60">· ~{formatBytes(estimatedSize)}</span>{/if}
						</button>
						{#if blockReason}
							<p class="eyebrow text-center !text-[10px]">{blockReason}</p>
						{/if}
					{/if}

					{#if result}
						<div class="panel rise flex items-center gap-4 !border-emerald-400/25 !bg-emerald-400/[.06] p-4">
							<div class="grid h-11 w-11 shrink-0 place-items-center rounded-xl border border-emerald-400/30 bg-emerald-400/15 text-emerald-400">
								<Icon name="check" class="h-5 w-5" stroke={2.6} />
							</div>
							<div class="min-w-0 flex-1">
								<p class="font-semibold text-emerald-400">{t('res.done')}</p>
								<p class="truncate font-mono text-xs text-ink-300 select-text" title={result.filePath}>{result.filePath}</p>
								{#if result.tagsWritten}
									<p class="mt-0.5 text-xs text-ink-400">
										{t('res.tags')} · {result.coverEmbedded ? t('res.cover') : t('res.noCover')}
									</p>
								{/if}
							</div>
							<button class="btn !border-emerald-400/25 !bg-emerald-400/10 !text-emerald-400 hover:!bg-emerald-400/20" onclick={() => reveal(result!.filePath)}>
								<Icon name="folder" class="h-4 w-4" /> {t('res.show')}
							</button>
						</div>
					{/if}
				</section>
			{:else if view === 'history'}
				<!-- ── History ─────────────────────────────────────────── -->
				<div class="flex items-center gap-3 pt-2">
					<div class="tile"><Icon name="history" /></div>
					<div class="flex flex-1 items-center gap-2">
						<h2 class="text-[15px] font-semibold tracking-tight">{t('hist.title')}</h2>
						<span class="chip">{history.length}</span>
					</div>
					<span class="hidden h-px flex-[2] bg-white/8 min-[1180px]:block"></span>
					{#if history.length}
						<button class="btn !py-1.5" onclick={clearHistory}><Icon name="trash" class="h-4 w-4" /> {t('hist.clear')}</button>
					{/if}
				</div>

				{#if history.length === 0}
					<div class="panel rise grid place-items-center gap-3 px-6 py-16 text-center">
						<div class="tile !h-14 !w-14 !rounded-2xl"><Icon name="inbox" class="h-6 w-6" /></div>
						<p class="font-semibold">{t('hist.emptyTitle')}</p>
						<p class="max-w-sm text-sm text-ink-400">{t('hist.emptyText')}</p>
						<button class="btn btn-primary mt-2" onclick={focusUrl}>{t('hist.paste')} <span class="kbd !border-current/20 !bg-transparent !text-current opacity-70">Ctrl K</span></button>
					</div>
				{:else}
					<div class="grid gap-4 min-[1180px]:grid-cols-2">
						{#each history as h (h.id)}
							{@const hs = siteById(h.site)}
							<article class="panel rise flex gap-4 p-4" use:spotlight>
								<div class="relative h-[76px] w-[132px] shrink-0 overflow-hidden rounded-xl border border-white/8 bg-ink-800">
									<div class="absolute inset-0 grid place-items-center text-ink-500"><Icon name={h.mode === 'audio' ? 'music' : 'video'} class="h-6 w-6" /></div>
									{#if h.thumbnail}
										<img src={h.thumbnail} alt="" class="relative h-full w-full object-cover" referrerpolicy="no-referrer" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />
									{/if}
								</div>
								<div class="flex min-w-0 flex-1 flex-col">
									<p class="line-clamp-2 text-sm leading-snug font-semibold select-text">{h.title}</p>
									<div class="mt-2 flex flex-wrap items-center gap-1.5">
										<span class="chip" style:color={hs.color}><BrandIcon id={hs.id} class="h-3 w-3" />{hs.name}</span>
										<span class="chip">{h.label}</span>
										<span class="text-[11px] text-ink-400">{formatWhen(h.at, locale())}</span>
									</div>
									<p class="mt-2 truncate font-mono text-[11px] text-ink-500 select-text" title={h.path}>{h.path}</p>
									<div class="mt-3 flex items-center gap-2">
										<button class="btn !px-3 !py-1.5 !text-xs" onclick={() => reveal(h.path)}><Icon name="folder" class="h-3.5 w-3.5" /> {t('res.show')}</button>
										<button class="grid h-8 w-8 place-items-center rounded-lg text-ink-400 transition hover:bg-red-500/10 hover:text-red-400" onclick={() => removeFromHistory(h.id)} aria-label={t('hist.remove')} title={t('hist.remove')}>
											<Icon name="trash" class="h-4 w-4" />
										</button>
									</div>
								</div>
							</article>
						{/each}
					</div>
				{/if}
			{:else if view === 'settings'}
				<!-- ── Settings ────────────────────────────────────────── -->
				<section class="rise space-y-3 pt-2" style="animation-delay: 100ms">
					<div class="flex items-center gap-3">
						<div class="tile"><Icon name="globe" /></div>
						<div>
							<h2 class="text-[15px] font-semibold tracking-tight">{t('set.langTitle')}</h2>
							<p class="text-xs text-ink-400">{t('set.langSub')}</p>
						</div>
					</div>
					<LanguagePicker />
					<p class="px-1 text-xs text-ink-500">{t('set.note')}</p>
				</section>

				<section class="rise space-y-3 pt-3" style="animation-delay: 160ms">
					<div class="flex items-center gap-3">
						<div class="tile"><Icon name="palette" /></div>
						<div>
							<h2 class="text-[15px] font-semibold tracking-tight">{t('set.themeTitle')}</h2>
							<p class="text-xs text-ink-400">{t('set.themeSub')}</p>
						</div>
					</div>
					<ThemePicker />
				</section>
			{:else}
				<AuthorPanel />
			{/if}
		</div>
	</main>
</div>
