<script lang="ts">
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { t } from '$lib/i18n';
	import { spotlight } from '$lib/actions';
	import { LANGS } from '$lib/settings.svelte';
	import { THEMES } from '$lib/themes';
	import Icon from './Icon.svelte';

	const USER = 'w3kiss';
	const PROFILE_URL = `https://github.com/${USER}`;
	const REPOS_URL = `https://github.com/${USER}?tab=repositories`;
	const TECH = ['Tauri 2', 'Rust', 'Svelte 5', 'TypeScript', 'yt-dlp', 'FFmpeg'];

	// The avatar is GitHub's own, so changing it on GitHub changes it here.
	// The hourly `v` value only keeps the webview cache from serving a stale picture for too long.
	const avatar = `${PROFILE_URL}.png?size=320&v=${Math.floor(Date.now() / 3_600_000)}`;
	let avatarFailed = $state(false);
	let stats = $state<{ repos: number; followers: number } | null>(null);

	$effect(() => {
		const ctrl = new AbortController();
		fetch(`https://api.github.com/users/${USER}`, {
			signal: ctrl.signal,
			headers: { Accept: 'application/vnd.github+json' }
		})
			.then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
			.then((j) => (stats = { repos: j.public_repos, followers: j.followers }))
			.catch(() => {
				/* offline or rate-limited: tiles just show a dash */
			});
		return () => ctrl.abort();
	});

	async function open(url: string) {
		try {
			await openUrl(url);
		} catch {
			window.open(url, '_blank', 'noopener');
		}
	}

	const tiles = $derived([
		{ n: stats?.repos, k: 'author.statRepos' as const },
		{ n: stats?.followers, k: 'author.statFollowers' as const },
		{ n: LANGS.length, k: 'author.statLangs' as const },
		{ n: THEMES.length, k: 'author.statThemes' as const }
	]);

	const features = [
		{ icon: 'shield', title: 'author.f1t', text: 'author.f1x' },
		{ icon: 'bolt', title: 'author.f2t', text: 'author.f2x' },
		{ icon: 'settings', title: 'author.f3t', text: 'author.f3x' }
	] as const;
</script>

<div class="mx-auto max-w-[860px] space-y-4 pt-2">
	<!-- Author card -->
	<section class="panel rise relative overflow-hidden px-8 pt-10 pb-9 text-center" use:spotlight>
		<!-- orbit rings around the avatar (centre = avatar centre) -->
		<div class="pointer-events-none absolute top-[96px] left-1/2 -translate-x-1/2 -translate-y-1/2" aria-hidden="true">
			<div class="relative grid h-[320px] w-[320px] place-items-center rounded-full border border-white/[.06]">
				<div class="h-[228px] w-[228px] rounded-full border border-white/[.08]"></div>
				<div class="absolute inset-0 animate-spin" style="animation-duration: 28s">
					<span class="acc-fill acc-glow absolute top-[-3px] left-1/2 h-1.5 w-1.5 -translate-x-1/2 rounded-full"></span>
				</div>
			</div>
		</div>

		<div class="relative mx-auto h-[112px] w-[112px] rounded-full border border-white/15 p-[3px] shadow-2xl shadow-black/60">
			{#if avatarFailed}
				<div class="acc-bg grid h-full w-full place-items-center rounded-full text-4xl font-bold uppercase">{USER[0]}</div>
			{:else}
				<img
					src={avatar}
					alt={USER}
					class="h-full w-full rounded-full object-cover"
					referrerpolicy="no-referrer"
					onerror={() => (avatarFailed = true)}
				/>
			{/if}
		</div>

		<p class="eyebrow relative mt-5 flex items-center justify-center gap-3 !text-[10px] !tracking-[.3em]">
			<span class="h-px w-9 bg-white/15"></span>{t('author.role')}<span class="h-px w-9 bg-white/15"></span>
		</p>
		<h2 class="relative mt-2 text-[56px] leading-none font-bold tracking-[-0.045em]">{USER}</h2>
		<p class="relative mx-auto mt-4 max-w-[520px] text-[15px] leading-relaxed text-ink-300">{t('author.desc')}</p>

		<div class="relative mt-6 flex flex-wrap items-center justify-center gap-3">
			<button class="btn btn-primary !rounded-full !px-5" onclick={() => open(PROFILE_URL)}>
				<Icon name="user" class="h-4 w-4" />
				{t('author.profile')}
				<Icon name="external" class="h-3.5 w-3.5 opacity-70" />
			</button>
			<button class="btn !rounded-full !px-5" onclick={() => open(REPOS_URL)}>
				<Icon name="layers" class="h-4 w-4" />
				{t('author.repos')}
				<Icon name="external" class="h-3.5 w-3.5 opacity-70" />
			</button>
		</div>
	</section>

	<!-- Stats -->
	<div class="rise grid grid-cols-2 gap-3 min-[900px]:grid-cols-4" style="animation-delay: 60ms">
		{#each tiles as s (s.k)}
			<div class="rounded-2xl border border-white/8 bg-white/[.03] px-4 py-5 text-center">
				<p class="text-[32px] leading-none font-bold tracking-tight tabular-nums">{s.n ?? '—'}</p>
				<p class="mt-2 text-xs text-ink-400">{t(s.k)}</p>
			</div>
		{/each}
	</div>

	<!-- Features -->
	<div class="rise grid gap-3 min-[900px]:grid-cols-3" style="animation-delay: 120ms">
		{#each features as f (f.title)}
			<div class="panel p-5" use:spotlight>
				<div class="tile"><Icon name={f.icon} /></div>
				<h3 class="mt-5 text-[15px] font-semibold tracking-tight">{t(f.title)}</h3>
				<p class="mt-1.5 text-[13px] leading-relaxed text-ink-400">{t(f.text)}</p>
			</div>
		{/each}
	</div>

	<!-- Tech -->
	<div class="rise pt-4 text-center" style="animation-delay: 180ms">
		<p class="eyebrow !text-[10px] !tracking-[.3em]">{t('author.tech')}</p>
		<div class="mt-4 flex flex-wrap items-center justify-center gap-2">
			{#each TECH as name (name)}
				<span class="rounded-full border border-white/10 bg-white/[.03] px-3.5 py-1.5 text-xs text-ink-300">{name}</span>
			{/each}
		</div>
		<p class="mt-7 text-xs text-ink-500">{t('author.thanks')}</p>
	</div>
</div>
