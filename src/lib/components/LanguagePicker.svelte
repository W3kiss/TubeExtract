<script lang="ts">
	import { LANGS, settings, setLang } from '$lib/settings.svelte';

	interface Props {
		compact?: boolean;
	}
	let { compact = false }: Props = $props();
</script>

<div class="grid gap-3 {compact ? 'grid-cols-2 min-[700px]:grid-cols-4' : 'grid-cols-2 min-[1180px]:grid-cols-4'}">
	{#each LANGS as l (l.id)}
		{@const sel = settings.lang === l.id}
		<button
			class="flex items-center gap-3 rounded-2xl border px-3.5 {compact ? 'py-2.5' : 'py-3'} text-left transition {sel
				? 'sel'
				: 'border-white/10 bg-white/[.02] hover:border-white/25 hover:bg-white/[.05]'}"
			onclick={() => setLang(l.id)}
			aria-pressed={sel}
		>
			<span class="chip !px-2 !py-1 !text-[10.5px]">{l.code}</span>
			<span class="text-[15px] font-semibold">{l.name}</span>
		</button>
	{/each}
</div>
