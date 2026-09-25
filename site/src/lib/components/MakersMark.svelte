<script lang="ts">
	/**
	 * The maker's mark: one circle with three straight arms, and nothing else.
	 * It is carved, not lit: a stroke in the page's own muted ink, hanging from
	 * the footer's rule by its upper arm the way a vowel sign hangs from the
	 * headline. It does not glow, move or link. It appears once, here.
	 *
	 * Geometry on the 9-grid: the circle's radius is U/3; each arm runs from the
	 * circle to 2U from the centre, at 120 degrees from straight up. The upper
	 * arm ends at y = 0, where the rule is; the lower two end at 3U.
	 */
	const U = 9;
	const r = U / 3;
	const reach = 2 * U;
	const cx = 2 * U;
	const cy = reach;
	const arms = [-90, 30, 150].map((deg) => {
		const a = (deg * Math.PI) / 180;
		return {
			x1: cx + r * Math.cos(a),
			y1: cy + r * Math.sin(a),
			x2: cx + reach * Math.cos(a),
			y2: cy + reach * Math.sin(a)
		};
	});
	let { height = 18 }: { height?: number } = $props();
</script>

<svg
	class="makers-mark"
	viewBox="0 0 {4 * U} {3 * U}"
	width={(height * 4) / 3}
	{height}
	role="img"
	aria-label="maker's mark"
>
	<g fill="none" stroke="currentColor" stroke-width="1.25" vector-effect="non-scaling-stroke">
		<circle {cx} {cy} {r} vector-effect="non-scaling-stroke" />
		{#each arms as a, i (i)}
			<line x1={a.x1} y1={a.y1} x2={a.x2} y2={a.y2} vector-effect="non-scaling-stroke" />
		{/each}
	</g>
</svg>

<style>
	.makers-mark {
		display: block;
		color: var(--text-muted);
		overflow: visible;
	}
</style>
