/** Feeds the pointer position to CSS (`--mx` / `--my`) so `.panel` can draw a spotlight. */
export function spotlight(node: HTMLElement) {
	const move = (e: PointerEvent) => {
		const r = node.getBoundingClientRect();
		node.style.setProperty('--mx', `${e.clientX - r.left}px`);
		node.style.setProperty('--my', `${e.clientY - r.top}px`);
	};
	node.addEventListener('pointermove', move);
	return {
		destroy() {
			node.removeEventListener('pointermove', move);
		}
	};
}
