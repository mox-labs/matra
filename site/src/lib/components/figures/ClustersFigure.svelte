<script lang="ts">
	/**
	 * Semantic clusters of one input's sentences over a grid of thresholds
	 * (EP-0012, M5), scrubbed by the reader.
	 *
	 * The generator ran `embed_and_cluster` at every grid value; the browser
	 * never recomputes a cluster. Every grid state is drawn once, as a layer,
	 * and the scrub sets each layer's opacity: at a grid value that state alone
	 * shows, and between two values the two blend, so the reader sees an edge
	 * go as the bar passes it. The state line says which runs are blending.
	 *
	 * Motion, per the motion research: the scrub is direct manipulation, the
	 * figure follows the pointer within a frame and linearly (F3: a continuous
	 * change is not eased). On release it settles on the nearest grid value
	 * with a short slow-in, slow-out transition (F3, F5: one stage) that a new
	 * input retargets mid-flight (F2), because it is a CSS transition on a
	 * persistent layer. Arrow keys step the grid. Opacity only. Under reduced
	 * motion states jump. Nothing moves on load.
	 *
	 * Linked highlighting: pointing at a sentence (in the list or the figure)
	 * keeps its cluster and edges and dims the rest; pointing at a row of the
	 * table previews that threshold. Colour is by role and never alone: a
	 * clustered sentence's marker is filled, in Emergence (the converged
	 * role), with its cluster's letter beside it; a hollow marker is in none;
	 * what the reader points at is Spark.
	 */
	import { onMount } from 'svelte';
	import type { ClusterGridPoint, ClustersFigureFile } from '$lib/types';
	import FigureFrame from './FigureFrame.svelte';

	let {
		id,
		file,
		threshold,
		dataUrl
	}: { id: string; file: ClustersFigureFile; threshold: number; dataUrl: string } = $props();

	const ROW = 30;
	const TOP = 14;
	// Nested arcs' labels sit at their apexes, three quarters of a level
	// apart; at 44 a four-character label clears its neighbour's.
	const LEVEL = 44;
	const FONT = 11;
	const ADVANCE = FONT * 0.6;

	const data = $derived(file.data);
	const n = $derived(data.grid.length);
	const indexOf = (t: number) => Math.max(0, data.grid.findIndex((g) => Math.abs(g.threshold - t) < 1e-9));

	/** The scrub position: a grid index, fractional between two runs. It
	 *  starts at the threshold the page asked for, prerendered, and is the
	 *  reader's from then on. */
	// svelte-ignore state_referenced_locally
	let pos = $state(indexOf(threshold));
	let dragging = $state(false);
	let preview = $state<number | null>(null);
	let js = $state(false);
	onMount(() => (js = true));

	const shown = $derived(preview ?? pos);
	const nearest = $derived(Math.min(n - 1, Math.max(0, Math.round(shown))));
	const point = $derived(data.grid[nearest]);
	const between = $derived(Math.abs(shown - nearest) > 0.02);
	const lo = $derived(Math.floor(shown));
	const hi = $derived(Math.min(n - 1, Math.ceil(shown)));

	/** Each layer's weight: 1 at its own grid value, blending linearly to its neighbours. */
	const weight = (i: number) => Math.max(0, 1 - Math.abs(shown - i));

	const clusterIndex = (g: ClusterGridPoint) => {
		const m = new Map<number, number>();
		g.clusters.forEach((c, k) => c.members.forEach((i) => m.set(i, k)));
		return m;
	};

	function levelled(g: ClusterGridPoint) {
		const edges = g.clusters
			.flatMap((c, k) => c.edges.map((e) => ({ ...e, k })))
			.sort((p, q) => p.b - p.a - (q.b - q.a) || p.a - q.a);
		const placed: { a: number; b: number; level: number }[] = [];
		return edges.map((e) => {
			let level = 1;
			for (const p of placed) if (p.a < e.b && e.a < p.b) level = Math.max(level, p.level + 1);
			placed.push({ a: e.a, b: e.b, level });
			return { ...e, level };
		});
	}

	const layers = $derived(data.grid.map((g) => ({ g, laid: levelled(g), cluster: clusterIndex(g) })));
	// Enough room for the deepest nesting any grid value needs, so the rows
	// do not move as the reader scrubs.
	const maxLevel = $derived(Math.max(1, ...layers.flatMap((l) => l.laid.map((e) => e.level))));

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

	/** The sentence the reader is pointing at, and what it lights at the state shown. */
	let focus = $state<number | null>(null);
	const litRows = $derived.by(() => {
		if (focus === null) return null;
		const k = layers[nearest].cluster.get(focus);
		if (k === undefined) return new Set([focus]);
		return new Set(point.clusters[k].members);
	});
	const edgeLit = (e: { a: number; b: number }) => litRows !== null && litRows.has(e.a) && litRows.has(e.b);

	const LETTERS = 'ABCDEFGH';
	const members = (m: number[]) => `{${m.map((i) => i + 1).join(', ')}}`;
	const edgeText = (e: { a: number; b: number; score: number }) => `${e.a + 1}-${e.b + 1} (${e.score.toFixed(4)})`;

	/** Whether the diagram is wider than its frame, so the scroll hint shows. */
	let overflowing = $state<boolean | null>(null);
	function measure(node: HTMLElement) {
		const update = () => (overflowing = node.scrollWidth > node.clientWidth + 1);
		const ro = new ResizeObserver(update);
		ro.observe(node);
		update();
		return () => ro.disconnect();
	}
	// Before any script runs, guess from the width of the column on a desktop.
	const hint = $derived(overflowing ?? width > 882);
	const fmt = (t: number) => t.toFixed(2);
	const thresholdAt = (p: number) => {
		const i = Math.floor(p);
		const j = Math.min(n - 1, i + 1);
		return data.grid[i].threshold + (data.grid[j].threshold - data.grid[i].threshold) * (p - i);
	};

	function settle() {
		dragging = false;
		pos = Math.round(pos);
	}

	function onkeydown(e: KeyboardEvent) {
		const step = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[e.key];
		let next: number | null = step === undefined ? null : Math.round(pos) + step;
		if (e.key === 'Home') next = 0;
		if (e.key === 'End') next = n - 1;
		if (next === null) return;
		e.preventDefault();
		pos = Math.min(n - 1, Math.max(0, next));
	}
</script>

<FigureFrame {id} kind="clusters" title="Semantic clusters as the threshold moves" {file} {dataUrl}>
	{#snippet controls()}
		{#if js}
			<div class="scrub">
				<label class="scrub-label" for="{id}-scrub">threshold</label>
				<div class="track">
					<input
						id="{id}-scrub"
						type="range"
						min="0"
						max={n - 1}
						step="any"
						value={pos}
						aria-valuetext="{fmt(thresholdAt(shown))}{between ? `, between matra's runs at ${fmt(data.grid[lo].threshold)} and ${fmt(data.grid[hi].threshold)}` : ''}"
						oninput={(e) => {
							dragging = true;
							pos = Number((e.currentTarget as HTMLInputElement).value);
						}}
						onchange={settle}
						onpointerup={settle}
						{onkeydown}
					/>
					<div class="ticks" aria-hidden="true">
						{#each data.grid as g, i (g.threshold)}
							<button
								type="button"
								tabindex="-1"
								class:at={i === nearest}
								class:default={Math.abs(g.threshold - data.default_threshold) < 1e-9}
								onclick={() => (pos = i)}>{fmt(g.threshold).slice(1)}</button
							>
						{/each}
					</div>
				</div>
			</div>
		{/if}
	{/snippet}

	<ol class="sentences" aria-label="The sentences, numbered as in the figure">
		{#each data.sentences as s, i (s.index)}
			<!-- Each sentence takes focus, so the keyboard lights a cluster as
			     the pointer does. -->
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<li
				tabindex="0"
				class:lit={litRows?.has(i)}
				class:dim={litRows !== null && !litRows.has(i)}
				onpointerenter={() => (focus = i)}
				onpointerleave={() => (focus = null)}
				onfocus={() => (focus = i)}
				onblur={() => (focus = null)}
			>
				<span class="n">{s.index + 1}</span>{s.text}
			</li>
		{/each}
	</ol>

	<!-- A region that scrolls must take focus, or it cannot be scrolled from
	     the keyboard (WCAG 2.1.1). -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-twin" tabindex="0" role="region" aria-label="Clusters and edges at every threshold">
		<table data-twin-for={id}>
			<thead><tr><th>Threshold</th><th>Clusters</th><th>Edges (cosine)</th></tr></thead>
			<tbody>
				{#each data.grid as g, i (g.threshold)}
					<tr
						class:current={i === nearest}
						onpointerenter={() => js && (preview = i)}
						onpointerleave={() => (preview = null)}
						onclick={() => {
							pos = i;
							preview = null;
						}}
					>
						<td class="num">{fmt(g.threshold)}</td>
						<!-- Each cell wraps between whole items, never inside one, so no
						     column runs off the figure's edge. -->
						<td class="wrap"
							>{#each g.clusters as c, k (k)}{k ? ' ' : ''}<span>{members(c.members)}</span>{:else}none{/each}</td
						>
						<td class="wrap edges"
							>{#each g.clusters.flatMap((c) => c.edges) as e, k (`${e.a}-${e.b}`)}{k ? ', ' : ''}<span
									>{edgeText(e)}</span
								>{:else}none{/each}</td
						>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<p class="state" aria-live="polite">
		{#if between}
			Between matra's runs at <strong>{fmt(data.grid[lo].threshold)}</strong> and
			<strong>{fmt(data.grid[hi].threshold)}</strong>: the figure blends the two. Let go to settle on the nearer.
		{:else}
			At <strong>{fmt(point.threshold)}</strong>{#if Math.abs(point.threshold - data.default_threshold) < 1e-9}, matra's default{/if}:
			{point.clusters.length} {point.clusters.length === 1 ? 'cluster' : 'clusters'}, {data.sentences.length -
				layers[nearest].cluster.size} of {data.sentences.length} sentences in none.
		{/if}
	</p>

	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		class="fig-scroll"
		tabindex="0"
		role="region"
		aria-label="Clusters at threshold {fmt(point.threshold)}"
		{@attach measure}
	>
		<svg
			{width}
			{height}
			viewBox="0 0 {width} {height}"
			role="img"
			aria-label="Sentences with the similarities that cleared {fmt(point.threshold)} drawn as arcs"
			data-clusters-for={id}
			class:dragging
			class:focused={litRows !== null}
		>
			<title>Sentence similarities above {fmt(point.threshold)}</title>
			{#each data.sentences as s, i (s.index)}
				<!-- Pointer-only: the same lighting is on the sentence list above,
				     which the keyboard reaches. -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<g
					class="row"
					class:dim={litRows !== null && !litRows.has(i)}
					onpointerenter={() => (focus = i)}
					onpointerleave={() => (focus = null)}
				>
					<rect class="hit" x={X0} y={rowY(i) - ROW / 2} width={width - X0} height={ROW} />
					<text class="num" x={X0 + 38} y={rowY(i)}>{s.index + 1}</text>
					<text class="sent" class:none={!layers[nearest].cluster.has(i)} x={textX} y={rowY(i)} font-size={FONT}
						>{s.text}</text
					>
				</g>
			{/each}
			{#each layers as layer, li (layer.g.threshold)}
				<g
					class="layer"
					style="opacity: {weight(li)}"
					data-threshold={fmt(layer.g.threshold)}
					data-current={li === Math.round(pos) && preview === null ? 'true' : 'false'}
					aria-hidden={li === nearest ? undefined : 'true'}
				>
					{#each layer.laid as e (`${e.a}-${e.b}`)}
						{@const g = arc(e.a, e.b, e.level)}
						<g class="edge" class:lit={edgeLit(e)} data-a={e.a + 1} data-b={e.b + 1} data-score={e.score}>
							<path d={g.d} />
							<text class="score" x={g.lx} y={g.ly}>{e.score.toFixed(2)}</text>
						</g>
					{/each}
					{#each data.sentences as s, i (s.index)}
						{@const k = layer.cluster.get(i)}
						<g class="marker" class:lit={litRows?.has(i)}>
							{#if k !== undefined}
								<text class="letter" x={X0 + 22} y={rowY(i)}>{LETTERS[k] ?? k + 1}</text>
							{/if}
							<circle
								class="mark"
								class:none={k === undefined}
								data-row={i + 1}
								data-cluster={k === undefined ? '' : k + 1}
								cx={X0 + 10}
								cy={rowY(i)}
								r="5"
							/>
						</g>
					{/each}
				</g>
			{/each}
		</svg>
	</div>
	{#if hint}
		<p class="fig-hint" aria-hidden="true">Scroll sideways to read the whole sentences.</p>
	{/if}

	{#snippet legend()}
		<ul class="fig-legend" aria-label="Markers">
			<li>
				<svg width="26" height="12" aria-hidden="true"
					><circle class="mark" cx="6" cy="6" r="5" /><text class="letter" x="20" y="6">A</text></svg
				>in a cluster, named by its letter
			</li>
			<li><svg width="12" height="12" aria-hidden="true"><circle class="mark none" cx="6" cy="6" r="5" /></svg>in no cluster</li>
			<li><span class="swatch-spark" aria-hidden="true"></span>what you point at, with its cluster</li>
		</ul>
	{/snippet}

	{#snippet note()}
		Each arc is a pair of sentences whose cosine similarity cleared the threshold, labelled with the
		score. A cluster is a connected group, so two sentences can share one through a third without
		their own pair clearing the bar. The embeddings are fixed; only the threshold moves, over the
		values matra was run at, and between two of them the figure blends the two runs rather than
		compute one of its own. matra ships {fmt(data.default_threshold)} as a starting point, not a verdict:
		which threshold is right depends on the text and the question.
	{/snippet}
</FigureFrame>

<style>
	.scrub {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		min-width: min(100%, 24rem);
	}

	.scrub-label {
		font: var(--type-xs) var(--font-mono);
		color: var(--text-muted);
	}

	.track {
		flex: 1;
		min-width: 0;
	}

	.track input {
		display: block;
		width: 100%;
		margin: 0;
		accent-color: var(--spark);
		cursor: grab;
		touch-action: none;
	}

	.track input:active {
		cursor: grabbing;
	}

	.ticks {
		display: flex;
		justify-content: space-between;
	}

	.ticks button {
		all: unset;
		font: var(--type-xs) / 1.4 var(--font-mono);
		color: var(--text-muted);
		cursor: pointer;
		/* The range thumb's half width keeps the first and last labels under
		   their grid values. */
		min-width: 1.6em;
		text-align: center;
	}

	.ticks button.default {
		text-decoration: underline 1px var(--border-strong);
		text-underline-offset: 0.2em;
	}

	.ticks button.at {
		color: var(--spark);
		font-weight: 600;
	}

	/* One number per sentence: the one the figure and the table use, set as
	   text so it matches them, with the list's own marker off. */
	.sentences {
		margin: 0 0 0.8rem;
		padding: 0;
		list-style: none;
		font-size: 0.92rem;
		line-height: 1.5;
	}

	.sentences li {
		margin: 0.1rem 0;
		/* A wrapped sentence hangs under its first word, not its number. */
		padding-inline-start: 2rem;
		text-indent: -2rem;
		transition: opacity var(--duration-fast) linear;
	}

	.sentences .n {
		display: inline-block;
		min-width: 2rem;
		text-indent: 0;
		font: 600 0.78rem var(--font-mono);
		color: var(--text-muted);
	}

	/* The frame keeps table cells on one line; these wrap between items. */
	.fig-twin td.wrap {
		white-space: normal;
	}

	.fig-twin td.wrap span {
		white-space: nowrap;
	}

	.sentences li:focus-visible {
		outline: 2px solid var(--spark);
		outline-offset: 1px;
	}

	.sentences li.dim {
		opacity: 0.35;
	}

	.sentences li.lit .n {
		color: var(--spark);
	}

	.fig-twin tr {
		cursor: pointer;
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

	.hit {
		fill: transparent;
	}

	.row {
		transition: opacity var(--duration-fast) linear;
	}

	.row.dim {
		opacity: 0.35;
	}

	.sent {
		fill: var(--text);
		dominant-baseline: middle;
	}

	.sent.none {
		fill: var(--text-muted);
	}

	/* The layers settle with one eased stage, and follow the scrub directly
	   (no transition) while the reader drags. */
	.layer {
		transition: opacity var(--duration-slow) var(--easing-smooth);
		will-change: opacity;
	}

	.dragging .layer {
		transition: none;
	}

	.edge path {
		fill: none;
		stroke: var(--mark);
		stroke-width: 1.5px;
	}

	.score {
		font-size: 10px;
		font-weight: 600;
		fill: var(--text-muted);
		text-anchor: middle;
		dominant-baseline: middle;
		paint-order: stroke;
		stroke: var(--bg-raised);
		stroke-width: 4px;
		stroke-linejoin: round;
	}

	.focused .edge:not(.lit) {
		opacity: 0.25;
	}

	.edge.lit path {
		stroke: var(--spark);
		stroke-width: 2px;
	}

	.edge.lit .score {
		fill: var(--spark);
	}

	.mark {
		fill: var(--emergence);
		stroke: var(--emergence);
		stroke-width: 1.5px;
	}

	.mark.none {
		fill: none;
		stroke: var(--mark-quiet);
	}

	.marker.lit .mark:not(.none) {
		stroke: var(--spark);
	}

	.letter {
		font-size: 10px;
		font-weight: 600;
		fill: var(--text-muted);
		text-anchor: middle;
		dominant-baseline: middle;
	}

	.swatch-spark {
		display: inline-block;
		width: 0.9em;
		height: 2px;
		margin-inline-end: 0.4em;
		vertical-align: 0.3em;
		background: var(--spark);
	}

	@media (prefers-reduced-motion: reduce) {
		.layer,
		.row,
		.sentences li {
			transition: none;
		}
	}
</style>
