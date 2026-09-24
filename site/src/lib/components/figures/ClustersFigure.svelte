<script lang="ts">
	/**
	 * Semantic clusters of one input's sentences over a grid of thresholds
	 * (EP-0012, M5).
	 *
	 * The generator ran `embed_and_cluster` at every grid value, so the control
	 * snaps to them and the browser never recomputes a cluster. Each sentence
	 * is a row; every pair whose cosine cleared the threshold is an arc on the
	 * left, labelled with the score; the marker before a row says which
	 * cluster it is in, and a hollow marker that it is in none. The twin lists
	 * the clusters and edges at every grid value.
	 *
	 * Stepping the threshold is the only motion: the old edges fade out and
	 * the new fade in, about 0.6s. Under reduced motion the new state appears
	 * at once.
	 */
	import { fade } from 'svelte/transition';
	import type { ClustersFigureFile } from '$lib/types';
	import FigureFrame from './FigureFrame.svelte';
	import { enter, leave } from './motion';

	let {
		id,
		file,
		threshold,
		dataUrl
	}: { id: string; file: ClustersFigureFile; threshold: number; dataUrl: string } = $props();

	const ROW = 30;
	const TOP = 14;
	const LEVEL = 34;
	const FONT = 11;
	const ADVANCE = FONT * 0.6;

	const data = $derived(file.data);
	let current = $derived(threshold);
	const point = $derived(data.grid.find((g) => Math.abs(g.threshold - current) < 1e-9) ?? data.grid[0]);

	const clusterOf = $derived.by(() => {
		const m = new Map<number, number>();
		point.clusters.forEach((c, k) => c.members.forEach((i) => m.set(i, k)));
		return m;
	});

	const layout = $derived.by(() => {
		const edges = point.clusters
			.flatMap((c, k) => c.edges.map((e) => ({ ...e, k })))
			.sort((p, q) => p.b - p.a - (q.b - q.a) || p.a - q.a);
		// Levels, as in the parse figure: shorter spans nearer the rows.
		const placed: { a: number; b: number; level: number }[] = [];
		const laid = edges.map((e) => {
			let level = 1;
			for (const p of placed) if (p.a < e.b && e.a < p.b) level = Math.max(level, p.level + 1);
			placed.push({ a: e.a, b: e.b, level });
			return { ...e, level };
		});
		return { laid };
	});
	// Enough room for the deepest nesting any grid value needs, so the rows
	// do not move as the reader steps.
	const maxLevel = $derived.by(() => {
		let deepest = 1;
		for (const g of data.grid) {
			const es = g.clusters.flatMap((c) => c.edges).sort((p, q) => p.b - p.a - (q.b - q.a) || p.a - q.a);
			const placed: { a: number; b: number; level: number }[] = [];
			for (const e of es) {
				let level = 1;
				for (const p of placed) if (p.a < e.b && e.a < p.b) level = Math.max(level, p.level + 1);
				placed.push({ a: e.a, b: e.b, level });
				deepest = Math.max(deepest, level);
			}
		}
		return deepest;
	});

	const X0 = $derived(24 + maxLevel * LEVEL + 20);
	const textX = $derived(X0 + 50);
	const width = $derived(
		Math.ceil(textX + Math.max(...data.sentences.map((s) => [...s.text].length)) * ADVANCE + 16)
	);
	const height = $derived(TOP + data.sentences.length * ROW + 6);
	const rowY = (i: number) => TOP + i * ROW + ROW / 2;

	function arc(a: number, b: number, level: number) {
		const y1 = rowY(a);
		const y2 = rowY(b);
		const x = X0 - 6;
		const bulge = x - level * LEVEL;
		return {
			d: `M${x},${y1}C${bulge},${y1} ${bulge},${y2} ${x},${y2}`,
			// The cubic reaches three quarters of the way to its control points.
			lx: x - 0.75 * (x - bulge),
			ly: (y1 + y2) / 2
		};
	}

	const members = (m: number[]) => `{${m.map((i) => i + 1).join(', ')}}`;
	const edgeText = (e: { a: number; b: number; score: number }) => `${e.a + 1}-${e.b + 1} (${e.score.toFixed(4)})`;
	const fmt = (t: number) => t.toFixed(2);
</script>

<FigureFrame {id} kind="clusters" title="Semantic clusters as the threshold moves" {file} {dataUrl}>
	{#snippet controls()}
		<div class="picker" role="group" aria-label="Similarity threshold">
			<span class="picker-label" aria-hidden="true">Threshold</span>
			{#each data.grid as g (g.threshold)}
				<button
					type="button"
					aria-pressed={Math.abs(g.threshold - current) < 1e-9}
					aria-label="Threshold {fmt(g.threshold)}"
					onclick={() => (current = g.threshold)}>{fmt(g.threshold).slice(1)}</button
				>
			{/each}
		</div>
	{/snippet}

	<ol class="sentences" aria-label="The sentences, numbered as in the figure">
		{#each data.sentences as s (s.index)}
			<li><span class="n">{s.index + 1}</span>{s.text}</li>
		{/each}
	</ol>

	<!-- A region that scrolls must take focus, or it cannot be scrolled from
	     the keyboard (WCAG 2.1.1). -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-twin tall" tabindex="0" role="region" aria-label="Clusters and edges at every threshold">
		<table data-twin-for={id}>
			<thead><tr><th>Threshold</th><th>Clusters</th><th>Edges (cosine)</th></tr></thead>
			<tbody>
				{#each data.grid as g (g.threshold)}
					<tr class:current={Math.abs(g.threshold - current) < 1e-9}>
						<td class="num">{fmt(g.threshold)}</td>
						<td>{g.clusters.length ? g.clusters.map((c) => members(c.members)).join(' ') : 'none'}</td>
						<td class="edges">{g.clusters.flatMap((c) => c.edges).map(edgeText).join(', ') || 'none'}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<p class="state">
		At <strong>{fmt(current)}</strong>{#if Math.abs(current - data.default_threshold) < 1e-9}, matra's default{/if}:
		{point.clusters.length} {point.clusters.length === 1 ? 'cluster' : 'clusters'}, {data.sentences.length -
			clusterOf.size} of {data.sentences.length} sentences in none.
	</p>

	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-scroll" tabindex="0" role="region" aria-label="Clusters at threshold {fmt(current)}">
		<svg {width} {height} viewBox="0 0 {width} {height}" role="img" aria-label="Sentences with the similarities that cleared {fmt(current)} drawn as arcs" data-clusters-for={id} data-threshold={fmt(current)}>
			<title>Sentence similarities above {fmt(current)}</title>
			{#each data.sentences as s, i (s.index)}
				<text class="num" x={X0 + 38} y={rowY(i)}>{s.index + 1}</text>
				<text class="sent" class:none={!clusterOf.has(i)} x={textX} y={rowY(i)} font-size={FONT}>{s.text}</text>
			{/each}
			{#key current}
				<g in:fade={enter()} out:fade={leave()}>
					{#each layout.laid as e (`${e.a}-${e.b}`)}
						{@const g = arc(e.a, e.b, e.level)}
						<g class="edge c{e.k % 4}" data-a={e.a + 1} data-b={e.b + 1} data-score={e.score}>
							<path d={g.d} />
							<text class="score" x={g.lx} y={g.ly}>{e.score.toFixed(2)}</text>
						</g>
					{/each}
					{#each data.sentences as s, i (s.index)}
						{@const k = clusterOf.get(i)}
						<circle
							class="mark {k === undefined ? 'none' : `c${k % 4}`}"
							data-row={i + 1}
							data-cluster={k === undefined ? '' : k + 1}
							cx={X0 + 10}
							cy={rowY(i)}
							r="5"
						/>
					{/each}
				</g>
			{/key}
		</svg>
	</div>

	{#snippet legend()}
		<ul class="fig-legend" aria-label="Markers">
			<li><svg width="12" height="12" aria-hidden="true"><circle class="mark c0" cx="6" cy="6" r="5" /></svg>in a cluster (colour per cluster)</li>
			<li><svg width="12" height="12" aria-hidden="true"><circle class="mark none" cx="6" cy="6" r="5" /></svg>in no cluster</li>
		</ul>
	{/snippet}

	{#snippet note()}
		Each arc is a pair of sentences whose cosine similarity cleared the threshold, labelled with the
		score. A cluster is a connected group, so two sentences can share one through a third without
		their own pair clearing the bar. The embeddings are fixed; only the threshold moves, over the
		values matra was run at. matra ships {fmt(data.default_threshold)} as a starting point, not a verdict:
		which threshold is right depends on the text and the question.
	{/snippet}
</FigureFrame>

<style>
	:global(.mx-figure[data-figure='clusters']) {
		--c0: light-dark(#b3401e, #f0915f);
		--c1: light-dark(#2b6a8c, #7fb9d8);
		--c2: light-dark(#2f7a4f, #7cc79a);
		--c3: light-dark(#6d4f9e, #b69ae0);
	}

	.picker {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.2rem;
	}

	.picker-label {
		margin-right: 0.35rem;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.picker button {
		min-width: 2.3rem;
		height: 2rem;
		padding: 0 0.35rem;
		font: 600 0.78rem var(--font-mono);
		color: var(--text-muted);
		background: transparent;
		border: 1px solid var(--border);
		border-radius: 6px;
		cursor: pointer;
	}

	.picker button:hover {
		color: var(--text);
		border-color: var(--border-strong);
	}

	.picker button[aria-pressed='true'] {
		color: var(--accent);
		background: var(--accent-soft);
		border-color: var(--accent);
	}

	:global(html:not([data-js])) .picker {
		display: none;
	}

	.sentences {
		margin: 0 0 0.8rem;
		padding: 0;
		list-style: none;
		font-size: 0.92rem;
		line-height: 1.5;
	}

	.sentences li {
		margin: 0.1rem 0;
	}

	.sentences .n,
	.n {
		display: inline-block;
		min-width: 1.8em;
		font: 600 0.78rem var(--font-mono);
		color: var(--text-muted);
	}

	.tall {
		max-height: 14rem;
		overflow-y: auto;
	}

	tr.current td {
		color: var(--text);
		font-weight: 700;
		background: var(--accent-soft);
	}

	td.edges {
		white-space: normal;
		min-width: 16rem;
	}

	.state {
		margin: 0.2rem 0 0.5rem;
		font-size: 0.9rem;
		color: var(--text-muted);
	}

	.state strong {
		color: var(--text);
		font-family: var(--font-mono);
	}

	svg {
		font-family: var(--font-mono);
	}

	text.num {
		font-size: 11px;
		font-weight: 600;
		fill: var(--text-muted);
		text-anchor: end;
		dominant-baseline: middle;
	}

	.sent {
		fill: var(--text);
		dominant-baseline: middle;
	}

	.sent.none {
		fill: var(--text-muted);
	}

	.c0 {
		--c: var(--c0);
	}
	.c1 {
		--c: var(--c1);
	}
	.c2 {
		--c: var(--c2);
	}
	.c3 {
		--c: var(--c3);
	}

	.edge path {
		fill: none;
		stroke: var(--c);
		stroke-width: 1.6px;
	}

	.score {
		font-size: 10px;
		font-weight: 600;
		fill: var(--c);
		text-anchor: middle;
		dominant-baseline: middle;
		paint-order: stroke;
		stroke: var(--bg-raised);
		stroke-width: 4px;
		stroke-linejoin: round;
	}

	.mark {
		fill: var(--c);
		stroke: var(--c);
		stroke-width: 1.5px;
	}

	.mark.none {
		fill: none;
		stroke: var(--text-muted);
	}
</style>
