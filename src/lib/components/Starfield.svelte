<script lang="ts">
	import { settings } from '$lib/settings.svelte';
	import { themeById } from '$lib/themes';

	let canvas = $state<HTMLCanvasElement>();

	interface Star {
		x: number;
		y: number;
		r: number;
		a: number;
		speed: number;
		phase: number;
		/** current pull offset towards the cursor (Star theme) */
		ox: number;
		oy: number;
	}

	const PULL_RADIUS = 190;

	$effect(() => {
		const el = canvas;
		const ctx = el?.getContext('2d');
		if (!el || !ctx) return;

		const theme = themeById(settings.theme);
		const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		// "Star" theme: stars drift towards the cursor and brighten.
		const follow = settings.theme === 'star' && !reduce;
		const rgb = theme.fg.split(' ').join(',');
		const baseAlpha = theme.light ? 0.55 : 1;

		let w = 0;
		let h = 0;
		let stars: Star[] = [];
		let raf = 0;
		let last = 0;
		let mx = -1e4;
		let my = -1e4;

		function resize() {
			const dpr = Math.min(window.devicePixelRatio || 1, 2);
			w = window.innerWidth;
			h = window.innerHeight;
			el!.width = w * dpr;
			el!.height = h * dpr;
			ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
			const count = Math.min(260, Math.round((w * h) / 9000));
			stars = Array.from({ length: count }, () => ({
				x: Math.random() * w,
				y: Math.random() * h,
				r: Math.random() < 0.1 ? 1.5 : 0.6 + Math.random() * 0.6,
				a: 0.15 + Math.random() * 0.55,
				speed: 0.4 + Math.random() * 1.2,
				phase: Math.random() * Math.PI * 2,
				ox: 0,
				oy: 0
			}));
			draw(performance.now());
		}

		function draw(t: number) {
			ctx!.clearRect(0, 0, w, h);
			ctx!.fillStyle = `rgb(${rgb})`;
			for (const s of stars) {
				let boost = 0;
				if (follow) {
					const dx = mx - s.x;
					const dy = my - s.y;
					const d = Math.hypot(dx, dy);
					let tx = 0;
					let ty = 0;
					if (d < PULL_RADIUS && d > 1) {
						const k = 1 - d / PULL_RADIUS;
						const pull = Math.min(d * 0.7, 64) * k * 1.4;
						tx = (dx / d) * pull;
						ty = (dy / d) * pull;
						boost = k;
					}
					s.ox += (tx - s.ox) * 0.1;
					s.oy += (ty - s.oy) * 0.1;
				}
				const twinkle = reduce ? 1 : 0.55 + 0.45 * Math.sin((t / 1000) * s.speed + s.phase);
				ctx!.globalAlpha = Math.min(1, (s.a * twinkle + boost * 0.7) * baseAlpha);
				ctx!.beginPath();
				ctx!.arc(s.x + s.ox, s.y + s.oy, s.r * (1 + boost * 0.9), 0, Math.PI * 2);
				ctx!.fill();
			}
		}

		function frame(t: number) {
			raf = requestAnimationFrame(frame);
			if (t - last < (follow ? 14 : 40)) return;
			last = t;
			draw(t);
		}

		const onMove = (e: PointerEvent) => {
			mx = e.clientX;
			my = e.clientY;
		};
		const onLeave = () => {
			mx = my = -1e4;
		};

		resize();
		window.addEventListener('resize', resize);
		if (follow) {
			window.addEventListener('pointermove', onMove);
			document.addEventListener('pointerleave', onLeave);
		}
		if (!reduce) raf = requestAnimationFrame(frame);

		return () => {
			cancelAnimationFrame(raf);
			window.removeEventListener('resize', resize);
			window.removeEventListener('pointermove', onMove);
			document.removeEventListener('pointerleave', onLeave);
		};
	});
</script>

<canvas bind:this={canvas} class="pointer-events-none fixed inset-0 h-full w-full" aria-hidden="true"></canvas>
