<script lang="ts">
	/**
	 * The mark in the page's own inks: the bar, strokes and arcs in the
	 * current text colour, the root in Emergence. `mono` puts the root in the
	 * ink too and carries the hierarchy with opacity. The geometry is
	 * $lib/mark, generated from matra's parse of the motto.
	 */
	import type { MarkLayout } from '$lib/mark';

	let {
		layout,
		mono = false,
		label = 'matra',
		height,
		decorative = false,
		bar = true
	}: {
		layout: MarkLayout;
		mono?: boolean;
		label?: string;
		/** Rendered height in px; the width follows the layout's aspect. */
		height?: number;
		decorative?: boolean;
		/** Draw the headline bar; off where the page's own rule is the bar. */
		bar?: boolean;
	} = $props();

	const h = $derived(height ?? layout.height);
	const w = $derived((h * layout.width) / layout.height);
	const quiet = $derived(mono ? 0.6 : 1);
</script>

<svg
	class="mark"
	class:mono
	viewBox="0 0 {layout.width} {layout.height}"
	width={w}
	height={h}
	role={decorative ? undefined : 'img'}
	aria-label={decorative ? undefined : label}
	aria-hidden={decorative ? 'true' : undefined}
	focusable="false"
>
	<g fill="none" stroke="currentColor" stroke-linecap="square">
		{#if bar}
			<line
				stroke-width={layout.weight.bar}
				x1={layout.bar.x1}
				x2={layout.bar.x2}
				y1={layout.bar.y}
				y2={layout.bar.y}
			/>
		{/if}
		{#each layout.strokes.filter((s) => !s.root) as s (`${s.id}-${s.y1}`)}
			<line stroke-width={layout.weight.stroke} opacity={quiet} x1={s.x} x2={s.x} y1={s.y1} y2={s.y2} />
		{/each}
		{#each layout.arcs as a (`${a.head}-${a.dependent}`)}
			<path stroke-width={layout.weight.arc} opacity={mono ? 0.8 : 1} d={a.d} />
		{/each}
		{#each layout.strokes.filter((s) => s.root) as s (s.id)}
			<line class="root" stroke-width={layout.weight.root} x1={s.x} x2={s.x} y1={s.y1} y2={s.y2} />
		{/each}
	</g>
	{#if layout.words.length}
		<g class="words" font-size={layout.wordSize}>
			{#each layout.words as word (word.id)}
				<text x={word.x} y={word.y}>{word.text}</text>
			{/each}
		</g>
	{/if}
</svg>

<style>
	.mark {
		display: block;
		overflow: visible;
	}

	.root {
		stroke: var(--emergence);
	}

	.mono .root {
		stroke: currentColor;
	}

	.words {
		fill: currentColor;
		font-family: var(--font-mono);
	}
</style>
