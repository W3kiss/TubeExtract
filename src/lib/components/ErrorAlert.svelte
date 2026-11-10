<script lang="ts">
	import type { AppError } from '$lib/api';
	import { t } from '$lib/i18n';
	import Icon from './Icon.svelte';

	interface Props {
		error: AppError;
		ondismiss: () => void;
		onretry?: () => void;
	}
	let { error, ondismiss, onretry }: Props = $props();
	let showDetails = $state(false);

	const muted = $derived(error.kind === 'cancelled');
</script>

<div
	role="alert"
	class="rise flex gap-4 rounded-3xl border p-4 {muted ? 'border-white/10 bg-white/[.03]' : 'border-red-400/25 bg-red-500/[.07]'}"
>
	<div
		class="grid h-10 w-10 shrink-0 place-items-center rounded-xl border {muted
			? 'border-white/10 bg-white/5 text-ink-300'
			: 'border-red-400/30 bg-red-500/15 text-red-400'}"
	>
		<Icon name={error.kind === 'network' ? 'wifi-off' : 'warn'} />
	</div>
	<div class="min-w-0 flex-1">
		<p class="eyebrow {muted ? '' : '!text-red-400'}">{muted ? t('err.notice') : t('err.error')}</p>
		<p class="mt-1 font-semibold {muted ? 'text-ink-100' : 'text-red-400'}">{t(`err.${error.kind}`)}</p>
		<p class="mt-0.5 text-sm {muted ? 'text-ink-300' : 'text-ink-300'}">{error.message}</p>
		{#if error.details}
			<button class="mt-2 font-mono text-[11px] text-ink-400 underline-offset-2 hover:text-ink-100 hover:underline" onclick={() => (showDetails = !showDetails)}>
				{showDetails ? t('err.hideDetails') : t('err.showDetails')}
			</button>
			{#if showDetails}
				<pre
					class="mt-2 max-h-40 overflow-auto rounded-xl border border-white/8 bg-black/50 p-3 font-mono text-xs whitespace-pre-wrap text-ink-300 select-text">{error.details}</pre>
			{/if}
		{/if}
	</div>
	<div class="flex shrink-0 items-start gap-2">
		{#if onretry && !muted}
			<button class="btn !border-red-400/30 !bg-red-500/15 !py-1.5 !text-red-400 hover:!bg-red-500/25" onclick={onretry}>
				<Icon name="refresh" class="h-3.5 w-3.5" /> {t('err.retry')}
			</button>
		{/if}
		<button class="rounded-lg p-1.5 text-ink-400 transition hover:bg-white/5 hover:text-ink-100" onclick={ondismiss} aria-label={t('err.dismiss')}>
			<Icon name="x" class="h-4 w-4" />
		</button>
	</div>
</div>
