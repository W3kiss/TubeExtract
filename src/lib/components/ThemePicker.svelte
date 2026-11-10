<script lang="ts">
	import { settings, setTheme } from '$lib/settings.svelte';
	import { THEMES } from '$lib/themes';
	import { t } from '$lib/i18n';
	import Icon from './Icon.svelte';

	interface Props {
		compact?: boolean;
	}
	let { compact = false }: Props = $props();
</script>

<div class="grid gap-3 {compact ? 'grid-cols-3 min-[820px]:grid-cols-5' : 'grid-cols-2 min-[900px]:grid-cols-3 min-[1180px]:grid-cols-4'}">
	{#each THEMES as th (th.id)}
		{@const sel = settings.theme === th.id}
		<button
			class="rounded-2xl border {compact ? 'p-2' : 'p-2.5'} text-left transition {sel ? 'sel' : 'border-white/10 bg-white/[.02] hover:border-white/25'}"
			onclick={() => setTheme(th.id)}
			aria-pressed={sel}
			title={t(`themeDesc.${th.id}`)}
		>
			<div class="relative overflow-hidden rounded-xl border {compact ? 'h-[68px]' : 'h-[92px]'}" style:background={th.bg} style:border-color="rgb({th.fg} / .12)">
				<div class="absolute top-2 bottom-2 left-2 w-[22%] rounded-lg border" style="background: rgb({th.fg} / .05); border-color: rgb({th.fg} / .12)"></div>
				<div class="absolute top-2 right-2 left-[31%] h-[28%] rounded-lg border" style="background: rgb({th.fg} / .05); border-color: rgb({th.fg} / .12)"></div>
				<div class="absolute bottom-2 left-[31%] h-[30%] w-[22%] rounded-lg border" style="background: rgb({th.fg} / .05); border-color: rgb({th.fg} / .12)"></div>
				<div class="absolute right-2.5 bottom-[19%] h-[7px] w-[34%] rounded-full" style:background={th.accent}></div>
				{#if sel}
					<span class="acc-bg absolute top-1.5 right-1.5 grid h-5 w-5 place-items-center rounded-full shadow-lg"><Icon name="check" class="h-3 w-3" stroke={3} /></span>
				{/if}
			</div>
			<p class="mt-2 text-[13px] font-semibold">{t(`theme.${th.id}`)}</p>
			{#if !compact}
				<p class="mt-0.5 min-h-[28px] text-[11px] leading-snug text-ink-400">{t(`themeDesc.${th.id}`)}</p>
			{/if}
		</button>
	{/each}
</div>
