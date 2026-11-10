<script lang="ts">
	import type { DownloadProgress } from '$lib/api';
	import { formatBytes, formatDuration, formatSpeed } from '$lib/format';
	import { t } from '$lib/i18n';

	interface Props {
		progress: DownloadProgress | null;
		oncancel: () => void;
		cancelling: boolean;
	}
	let { progress, oncancel, cancelling }: Props = $props();

	const percent = $derived(progress?.percent ?? null);
	const indeterminate = $derived(percent == null);
</script>

<div class="panel rise p-5">
	<div class="mb-4 flex items-center justify-between gap-4">
		<div class="flex min-w-0 items-center gap-3">
			<span class="relative flex h-2 w-2">
				<span class="acc-fill absolute inline-flex h-full w-full animate-ping rounded-full opacity-60"></span>
				<span class="acc-fill relative inline-flex h-2 w-2 rounded-full"></span>
			</span>
			<span class="eyebrow !text-ink-100 truncate">{cancelling ? t('prog.cancelling') : (progress?.phase ?? t('prog.starting'))}</span>
		</div>
		<span class="font-mono text-3xl font-semibold tracking-tight tabular-nums">
			{indeterminate ? '' : `${percent!.toFixed(1)}%`}
		</span>
	</div>

	<div class="relative h-2 overflow-hidden rounded-full bg-white/8">
		{#if indeterminate}
			<div class="animate-indeterminate acc-fill absolute inset-y-0 w-2/5 rounded-full"></div>
		{:else}
			<div
				class="bar-shimmer acc-fill acc-glow relative h-full overflow-hidden rounded-full transition-[width] duration-200 ease-out"
				style:width="{Math.max(1, percent!)}%"
			></div>
		{/if}
	</div>

	<div class="mt-5 flex items-end justify-between gap-4">
		<dl class="grid grid-cols-3 gap-8">
			<div>
				<dt class="eyebrow">{t('prog.speed')}</dt>
				<dd class="mt-1 font-mono text-sm font-medium tabular-nums">{formatSpeed(progress?.speed)}</dd>
			</div>
			<div>
				<dt class="eyebrow">{t('prog.size')}</dt>
				<dd class="mt-1 font-mono text-sm font-medium tabular-nums">
					{#if progress?.total}
						{formatBytes(progress.downloaded)} / {formatBytes(progress.total)}
					{:else}
						{formatBytes(progress?.downloaded)}
					{/if}
				</dd>
			</div>
			<div>
				<dt class="eyebrow">{t('prog.left')}</dt>
				<dd class="mt-1 font-mono text-sm font-medium tabular-nums">{progress?.eta != null ? formatDuration(progress.eta) : '—'}</dd>
			</div>
		</dl>
		<button
			class="btn hover:!border-red-400/40 hover:!bg-red-500/10 hover:!text-red-400"
			onclick={oncancel}
			disabled={cancelling}
		>
			{t('prog.cancel')}
		</button>
	</div>
</div>
