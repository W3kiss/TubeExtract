<script lang="ts">
	import type { AppError, SetupProgress } from '$lib/api';
	import { formatBytes } from '$lib/format';
	import { t } from '$lib/i18n';
	import Icon from './Icon.svelte';

	interface Props {
		missing: string[];
		progress: SetupProgress | null;
		error: AppError | null;
		onretry: () => void;
	}
	let { missing, progress, error, onretry }: Props = $props();

	const overall = $derived(Math.round(progress?.overall ?? 0));
	const componentPercent = $derived(
		progress?.total ? Math.min(100, (progress.downloaded / progress.total) * 100) : null
	);
	const doneLabels = $derived(
		progress ? missing.slice(0, progress.stage === 'done' ? progress.index + 1 : progress.index) : []
	);
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-2xl">
	<div
		class="acc-fill pointer-events-none absolute -top-48 left-1/2 h-[520px] w-[520px] -translate-x-1/2 rounded-full opacity-10 blur-[140px]"
	></div>

	<div class="panel rise w-[500px] bg-ink-900/90 p-8 shadow-2xl shadow-black/70">
		<p class="eyebrow mb-5 flex items-center gap-2">
			<span class="h-1.5 w-1.5 rounded-full {error ? 'bg-red-400' : 'acc-fill animate-pulse'}"></span>
			{t('setup.eyebrow')}
		</p>
		<div class="mb-6 flex items-center gap-4">
			<div class="acc-bg grid h-14 w-14 shrink-0 place-items-center rounded-2xl">
				{#if error}
					<Icon name="warn" class="h-7 w-7" />
				{:else}
					<Icon name="download" class="h-7 w-7 animate-bounce" />
				{/if}
			</div>
			<div>
				<h2 class="text-xl font-bold tracking-tight">
					{error ? t('setup.failed') : t('setup.title')}
				</h2>
				<p class="mt-0.5 text-sm text-ink-400">
					{error ? error.message : t('setup.sub')}
				</p>
			</div>
		</div>

		{#if error}
			{#if error.details}
				<pre
					class="mb-5 max-h-32 overflow-auto rounded-xl border border-white/8 bg-black/50 p-3 font-mono text-xs whitespace-pre-wrap text-red-400 select-text">{error.details}</pre>
			{/if}
			<button class="btn btn-primary w-full !py-3" onclick={onretry}><Icon name="refresh" class="h-4 w-4" /> {t('err.retry')}</button>
		{:else}
			<div class="mb-2 flex items-end justify-between text-sm">
				<span class="text-ink-300">
					{#if progress}
						{progress.stage === 'extracting' ? t('setup.installing') : t('setup.downloading')}
						<span class="font-semibold text-ink-100">{progress.label}</span>
						<span class="font-mono text-xs text-ink-400">({progress.index + 1}/{progress.count})</span>
					{:else}
						{t('setup.preparing')}
					{/if}
				</span>
				<span class="font-mono text-2xl font-semibold tabular-nums">{overall}%</span>
			</div>

			<div class="relative h-2 overflow-hidden rounded-full bg-white/8">
				<div
					class="bar-shimmer acc-fill acc-glow relative h-full overflow-hidden rounded-full transition-[width] duration-300"
					style:width="{Math.max(2, overall)}%"
				></div>
			</div>

			<div class="mt-2 h-4 font-mono text-[11px] text-ink-400 tabular-nums">
				{#if progress?.stage === 'downloading'}
					{formatBytes(progress.downloaded)}
					{#if progress.total}
						{t('setup.of')} {formatBytes(progress.total)} · {componentPercent?.toFixed(0)}%{/if}
				{/if}
			</div>

			<ul class="mt-6 space-y-2">
				{#each missing as name (name)}
					{@const done = doneLabels.includes(name)}
					{@const active = !done && progress?.label === name}
					<li class="flex items-center gap-3 rounded-xl border px-3 py-2 text-sm transition {active ? 'border-white/15 bg-white/[.05]' : 'border-transparent'}">
						<span
							class="grid h-5 w-5 place-items-center rounded-full border text-[10px] transition
								{done ? 'border-emerald-400/50 bg-emerald-400/15 text-emerald-400' : active ? 'border-white/60 bg-white/10' : 'border-ink-600'}"
						>
							{#if done}<Icon name="check" class="h-3 w-3" stroke={3} />{:else if active}<span class="acc-fill h-1.5 w-1.5 animate-ping rounded-full"></span>{/if}
						</span>
						<span class="font-mono text-[13px] {done ? 'text-ink-300' : active ? 'text-ink-100' : 'text-ink-400'}">{name}</span>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>
