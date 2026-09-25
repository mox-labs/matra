<script lang="ts">
	/**
	 * TextRank over one document (EP-0012, M5): every sentence's score in
	 * document order, the summary's sentences marked, and the summary itself.
	 *
	 * matra returns each sentence's score and position. The similarity graph
	 * those scores come from is internal to matra, so it is not drawn: drawing
	 * it would mean computing it a second time here.
	 *
	 * The summary's bars are Emergence, the converged result, and each is also
	 * numbered above and listed below; the rest are neutral. Pointing at or
	 * focusing a row lights its bar in Spark, and pointing at a bar lights its
	 * row, at once. Nothing else moves.
	 */
	import { scaleLinear } from 'd3-scale';
	import type { TextrankFigureFile } from '$lib/types';
	import FigureFrame from './FigureFrame.svelte';
	import { roving } from './motion';

	let { id, file, dataUrl }: { id: string; file: TextrankFigureFile; dataUrl: string } = $props();

	/** The sentence the reader points at, by position. */
	let active = $state<number | null>(null);

	const W = 640;
	const LEFT = 44;
	const RIGHT = 12;
	const TOP = 18;
	const H = 170;
	const AXIS = 30;

	const rows = $derived(file.data.sentences);
	const summary = $derived(rows.filter((r) => r.summary));
	const layout = $derived.by(() => {
		const n = rows.length;
		const step = (W - LEFT - RIGHT) / Math.max(1, n);
		const max = Math.max(...rows.map((r) => r.score));
		const y = scaleLinear().domain([0, max]).range([TOP + H, TOP]).nice(4);
		// Alternate bands mark where one paragraph gives way to the next.
		const bands: { x: number; w: number; p: number | null; odd: boolean }[] = [];
		rows.forEach((r, i) => {
			const last = bands[bands.length - 1];
			if (last && last.p === r.paragraph) last.w += step;
			else bands.push({ x: LEFT + i * step, w: step, p: r.paragraph, odd: bands.length % 2 === 1 });
		});
		return { step, y, bands, ticks: y.ticks(4), bar: Math.max(1.5, step - 1.5) };
	});
	const x = (i: number) => LEFT + i * layout.step;
	const height = TOP + H + AXIS;
	const fmt = (v: number) => v.toFixed(4);
</script>

<FigureFrame {id} kind="textrank" title="TextRank: the summary and the scores it came from" {file} {dataUrl}>
	<!-- A region that scrolls must take focus, or it cannot be scrolled from
	     the keyboard (WCAG 2.1.1). -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-twin tall" tabindex="0" role="region" aria-label="Every sentence's TextRank score">
		<table data-twin-for={id}>
			<thead><tr><th>#</th><th>¶</th><th>Score</th><th>Summary</th><th>Sentence</th></tr></thead>
			<tbody {@attach roving}>
				{#each rows as r (r.position)}
					<tr
						class:picked={r.summary}
						class:lit={active === r.position}
						data-row={r.position}
						onpointerenter={() => (active = r.position)}
						onpointerleave={() => (active = null)}
						onfocus={() => (active = r.position)}
						onblur={() => (active = null)}
					>
						<td class="num">{r.position + 1}</td>
						<td class="num">{r.paragraph ?? ''}</td>
						<td>{fmt(r.score)}</td>
						<td>{r.summary ? 'yes' : ''}</td>
						<td class="text">{r.text}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-scroll" tabindex="0" role="region" aria-label="TextRank score of every sentence">
		<svg class:active={active !== null} width={W} {height} viewBox="0 0 {W} {height}" role="img" aria-label="TextRank scores of {rows.length} sentences in document order; the {summary.length} summary sentences are marked" data-bars-for={id}>
			<title>TextRank score of each sentence, in document order</title>
			{#each layout.bands as b, i (i)}
				{#if b.odd}<rect class="band" x={b.x} y={TOP} width={b.w} height={H} />{/if}
			{/each}
			{#each layout.ticks as t (t)}
				<line class="grid" x1={LEFT} x2={W - RIGHT} y1={layout.y(t)} y2={layout.y(t)} />
				<text class="tick" x={LEFT - 6} y={layout.y(t)}>{t.toFixed(3)}</text>
			{/each}
			{#each rows as r, i (r.position)}
				<!-- Pointer-only: the table's rows give the keyboard the same. -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<rect
					class="hit"
					x={x(i)}
					y={TOP}
					width={layout.step}
					height={H}
					onpointerenter={() => (active = r.position)}
					onpointerleave={() => (active = null)}
				/>
				<rect
					class="bar"
					class:picked={r.summary}
					class:lit={active === r.position}
					data-position={r.position + 1}
					data-score={r.score}
					data-summary={r.summary ? 'yes' : ''}
					x={x(i) + 0.75}
					y={layout.y(r.score)}
					width={layout.bar}
					height={TOP + H - layout.y(r.score)}
				/>
				{#if r.summary}
					<text class="pick" x={x(i) + layout.step / 2} y={layout.y(r.score) - 6}>{r.position + 1}</text>
				{/if}
			{/each}
			<text class="axis" x={LEFT} y={height - 8}>sentence 1</text>
			<text class="axis end" x={W - RIGHT} y={height - 8}>sentence {rows.length}</text>
			<text class="axis mid" x={(LEFT + W - RIGHT) / 2} y={height - 8}>document order; bands alternate by paragraph</text>
		</svg>
	</div>

	<ol class="summary" aria-label="The summary">
		{#each summary as s (s.position)}
			<li><span class="n">{s.position + 1}</span>{s.text}</li>
		{/each}
	</ol>

	{#snippet legend()}
		<ul class="fig-legend" aria-label="Bars">
			<li><span class="swatch picked" aria-hidden="true"></span>in the summary ({file.data.n} sentences, matra's default)</li>
			<li><span class="swatch" aria-hidden="true"></span>not picked</li>
		</ul>
	{/snippet}

	{#snippet note()}
		TextRank scores each sentence by how much it shares with the others, and the summary is the top
		{file.data.n} returned in document order, as listed above. The scores come from a similarity
		graph between sentences that matra builds and does not return, so the figure shows the scores and
		not the graph.
	{/snippet}
</FigureFrame>

<style>
	/* The summary is the converged result: Emergence, and numbered. The rest
	   are the quiet neutral mark. */
	:global(.mx-figure[data-figure='textrank']) {
		--pick: var(--emergence);
		--rest: var(--mark-quiet);
	}

	.hit {
		fill: transparent;
	}

	.bar {
		transition: opacity var(--duration-fast) linear;
		pointer-events: none;
	}

	.active .bar:not(.lit) {
		opacity: 0.45;
	}

	.bar.lit {
		fill: var(--spark);
	}

	tbody tr:focus-visible {
		outline: 2px solid var(--spark);
		outline-offset: -2px;
	}

	tbody tr.lit td {
		color: var(--spark);
	}

	@media (prefers-reduced-motion: reduce) {
		.bar {
			transition: none;
		}
	}

	.tall {
		max-height: 15rem;
		overflow-y: auto;
	}

	.tall thead th {
		position: sticky;
		top: 0;
		background: var(--bg-raised);
	}

	td.text {
		font-family: var(--font-read);
		white-space: normal;
		min-width: 18rem;
		color: var(--text-muted);
	}

	tr.picked td {
		color: var(--text);
		font-weight: 600;
	}

	tr.picked td.text {
		color: var(--text);
	}

	svg {
		font: 10.5px var(--font-mono);
	}

	.band {
		fill: var(--bg-subtle);
	}

	.grid {
		stroke: var(--border);
	}

	.tick {
		fill: var(--text-muted);
		text-anchor: end;
		dominant-baseline: middle;
	}

	.bar:not(.lit) {
		fill: var(--rest);
	}

	.bar.picked:not(.lit) {
		fill: var(--pick);
	}

	.pick {
		font-weight: 700;
		fill: var(--pick);
		text-anchor: middle;
	}

	.axis {
		font-family: var(--font-sans);
		fill: var(--text-muted);
	}

	.axis.end {
		text-anchor: end;
	}

	.axis.mid {
		text-anchor: middle;
	}

	.summary {
		margin: 0.8rem 0 0;
		padding: 0.6rem 0.9rem;
		list-style: none;
		border-left: 3px solid var(--pick);
		background: var(--bg-subtle);
		border-radius: 0;
	}

	.summary li {
		margin: 0.25rem 0;
		font-size: 0.94rem;
		line-height: 1.5;
	}

	.summary .n {
		display: inline-block;
		min-width: 2.2em;
		font: 700 0.78rem var(--font-mono);
		color: var(--pick);
	}

	.swatch {
		display: inline-block;
		width: 0.6em;
		height: 0.9em;
		margin-right: 0.45em;
		vertical-align: -0.1em;
		background: var(--rest);
	}

	.swatch.picked {
		background: var(--pick);
	}
</style>
