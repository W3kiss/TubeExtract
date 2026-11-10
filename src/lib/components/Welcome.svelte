<script lang="ts">
	import { finishWelcome } from '$lib/settings.svelte';
	import { t } from '$lib/i18n';
	import { spotlight } from '$lib/actions';
	import Icon from './Icon.svelte';
	import LanguagePicker from './LanguagePicker.svelte';
	import ThemePicker from './ThemePicker.svelte';
</script>

<!-- First-launch window: language + theme. Everything previews live behind the blur. -->
<div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4 backdrop-blur-xl" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
	<div class="panel rise max-h-full w-[min(960px,96vw)] overflow-y-auto bg-ink-900/90 p-7 shadow-2xl shadow-black/70" use:spotlight>
		<div class="flex items-start gap-4">
			<div class="acc-bg grid h-12 w-12 shrink-0 place-items-center rounded-2xl">
				<Icon name="download" class="h-6 w-6" stroke={2.4} />
			</div>
			<div class="min-w-0 flex-1">
				<p class="eyebrow flex items-center gap-2 !text-[10px] !text-ink-300">
					<span class="acc-fill h-1.5 w-1.5 rounded-full"></span>
					TubeExtract / {t('welcome.eyebrow')}
				</p>
				<h2 id="welcome-title" class="mt-1.5 text-[28px] leading-tight font-bold tracking-tight">{t('welcome.title')}</h2>
				<p class="mt-1 text-sm text-ink-400">{t('welcome.sub')}</p>
			</div>
		</div>

		<div class="mt-6 flex items-center gap-3">
			<div class="tile !h-9 !w-9"><Icon name="globe" class="h-4 w-4" /></div>
			<div>
				<h3 class="text-[14px] font-semibold tracking-tight">{t('set.langTitle')}</h3>
				<p class="text-xs text-ink-400">{t('set.langSub')}</p>
			</div>
		</div>
		<div class="mt-3"><LanguagePicker compact /></div>

		<div class="mt-6 flex items-center gap-3">
			<div class="tile !h-9 !w-9"><Icon name="palette" class="h-4 w-4" /></div>
			<div>
				<h3 class="text-[14px] font-semibold tracking-tight">{t('set.themeTitle')}</h3>
				<p class="text-xs text-ink-400">{t('set.themeSub')}</p>
			</div>
		</div>
		<div class="mt-3"><ThemePicker compact /></div>

		<button class="btn btn-primary mt-7 w-full !rounded-2xl !py-3.5 !text-[15px]" onclick={finishWelcome}>
			{t('welcome.cta')}
			<Icon name="arrow" class="h-4 w-4" stroke={2.4} />
		</button>
	</div>
</div>
