<script lang="ts">
	import type { SiteId } from '$lib/sites';

	interface Props {
		id: SiteId;
		class?: string;
	}
	let { id, class: cls = 'h-5 w-5' }: Props = $props();
</script>

<!-- Simple brand marks drawn with currentColor, so the caller tints them with the platform colour. -->
<svg class={cls} viewBox="0 0 24 24" aria-hidden="true">
	{#if id === 'youtube'}
		<path
			fill="currentColor"
			fill-rule="evenodd"
			d="M7.2 4.5h9.6c2.6 0 4.7 2 4.7 4.6v5.8c0 2.6-2.1 4.6-4.7 4.6H7.2c-2.6 0-4.7-2-4.7-4.6V9.1c0-2.6 2.1-4.6 4.7-4.6Zm2.6 4.2v6.6l5.7-3.3Z"
		/>
	{:else if id === 'vk'}
		<g fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round">
			<path d="M2.8 8.2 6 15.8l3.2-7.6" />
			<path d="M13.2 8v7.8M19.8 8l-5.4 4.4 5.4 3.4" />
		</g>
	{:else if id === 'rutube'}
		<g fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
			<rect x="3" y="3" width="18" height="18" rx="5.5" />
			<path d="M8.6 16.6V8.4h3.9a2.4 2.4 0 0 1 0 4.8H8.6M12.3 13.2l2.9 3.4" />
		</g>
		<circle cx="17.6" cy="6.6" r="1.1" fill="currentColor" />
	{:else if id === 'tiktok'}
		<g fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round">
			<path d="M14 3.5v11.1a3.7 3.7 0 1 1-3.7-3.7" />
			<path d="M14 3.5c.3 2.5 2 4.2 4.7 4.5" />
		</g>
	{:else}
		<path fill="currentColor" d="M9 18.6h8.2a3.5 3.5 0 0 0 .5-6.96A5.6 5.6 0 0 0 9 10.7Z" />
		<path d="M5.2 13.3v5.3M7.2 11.8v6.8" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
	{/if}
</svg>
