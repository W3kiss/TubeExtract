<script lang="ts">
	import '../app.css';
	import { settings, loadSettings } from '$lib/settings.svelte';
	import { applyTheme } from '$lib/themes';

	let { children } = $props();

	// Apply saved theme/language before the first paint (the app is client-only: ssr = false).
	loadSettings();

	$effect(() => {
		applyTheme(settings.theme);
		document.documentElement.lang = settings.lang;
	});
</script>

{@render children()}
