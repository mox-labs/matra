<script lang="ts">
	/**
	 * The paragraph measures of one document, as small multiples (EP-0012, M4).
	 *
	 * One panel per measure, sharing the paragraph axis, so a reader sees where
	 * in the document each moves and whether they move together. A paragraph
	 * matra declined to measure (the field is `None`) is a gap with a cross, not
	 * a zero. A panel carries a reference line only where matra computes a
	 * document value: readability's is `Corpus::mean_readability`. matra has no
	 * document-level lexical density or compression ratio, so those panels have
	 * none. The table of every value leads.
	 *
	 * The marks are neutral ink. Pointing at or focusing a paragraph's row
	 * draws a rule through its three values and lights them in Spark;
	 * pointing at a value lights its row. At once; nothing else moves.
	 */
	import { scaleLinear } from 'd3-scale';
	import type { MetricsFigureFile, MetricsParagraph } from '$lib/types';
	import FigureFrame from './FigureFrame.svelte';
	import { roving } from './motion';

	let { id, file, dataUrl }: { id: string; file: MetricsFigureFile; dataUrl: string } = $props();

	/** The paragraph the reader points at. */
	let active = $state<number | null>(null);

	type Metric = 'readability_grade' | 'lexical_density' | 'compression_ratio';
	const PANELS: { metric: Metric; title: string; hint: string; reference?: 'mean_readability' }[] = [
		{ metric: 'readability_grade', title: 'Readability grade', hint: 'Flesch-Kincaid; higher reads harder', reference: 'mean_readability' },
		{ metric: 'lexical_density', title: 'Lexical density', hint: 'content words over all words' },
		{ metric: 'compression_ratio', title: 'Compression ratio', hint: 'compressed over original bytes; lower repeats more' }
	];

	// Fits the column on a desktop with the card's padding; narrower screens scroll.
	const W = 640;
	const LEFT = 46;
	const RIGHT = 70;
	const PANEL_H = 84;
	const TITLE_H = 26;
	const GAP = 18;
	const AXIS_H = 26;

	const paragraphs = $derived(file.data.paragraphs);
	const doc = $derived(file.data.document);
	const x = $derived(
		scaleLinear()
			.domain([1, Math.max(2, paragraphs.length)])
			.range([LEFT + 10, W - RIGHT - 10])
	);
	const height = $derived(PANELS.length * (TITLE_H + PANEL_H + GAP) + AXIS_H);
	const every = $derived(paragraphs.length > 30 ? 5 : paragraphs.length > 15 ? 2 : 1);

	function panel(i: number) {
		const p = PANELS[i];
		const top = i * (TITLE_H + PANEL_H + GAP) + TITLE_H;
		const values = paragraphs.map((q) => q[p.metric]).filter((v): v is number => v !== null);
		const ref = p.reference ? doc[p.reference] : null;
		const all = ref === null ? values : [...values, ref];
		const lo = Math.min(...all);
		const hi = Math.max(...all);
		const pad = (hi - lo || Math.abs(hi) || 1) * 0.12;
		const y = scaleLinear()
			.domain([lo - pad, hi + pad])
			.range([top + PANEL_H - 12, top + 4])
			.nice(4);
		// The line breaks at every unmeasured paragraph rather than bridging it.
		const runs: MetricsParagraph[][] = [];
		let run: MetricsParagraph[] = [];
		for (const q of paragraphs) {
			if (q[p.metric] === null) {
				if (run.length) runs.push(run);
				run = [];
			} else run.push(q);
		}
		if (run.length) runs.push(run);
		const path = runs
			.map((r) => r.map((q, k) => `${k ? 'L' : 'M'}${x(q.index)},${y(q[p.metric] as number)}`).join(''))
			.join('');
		return { ...p, top, y, ref, path, ticks: y.ticks(3) };
	}
	const panels = $derived(PANELS.map((_, i) => panel(i)));

	const fmt = (v: number | null) => (v === null ? 'none' : v.toFixed(2));
	const tick = (v: number, metric: Metric) => (metric === 'readability_grade' ? v.toFixed(0) : v.toFixed(2));

	let overflowing = $state<boolean | null>(null);
	function measure(node: HTMLElement) {
		const update = () => (overflowing = node.scrollWidth > node.clientWidth + 1);
		const ro = new ResizeObserver(update);
		ro.observe(node);
		update();
		return () => ro.disconnect();
	}
</script>

<FigureFrame {id} kind="metrics" title="Paragraph measures across one document" {file} {dataUrl}>
	<dl class="doc">
		<div><dt>Mean readability</dt><dd data-doc="mean_readability">{fmt(doc.mean_readability)}</dd></div>
		<div><dt>Vocabulary TTR</dt><dd>{fmt(doc.vocabulary_ttr)}</dd></div>
		<div><dt>Nominalization ratio</dt><dd>{fmt(doc.nominalization_ratio)}</dd></div>
		<div><dt>Passive ratio</dt><dd>{fmt(doc.passive_ratio)}</dd></div>
	</dl>

	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-scroll" tabindex="0" role="region" aria-label="Measures by paragraph" {@attach measure}>
		<svg width={W} {height} viewBox="0 0 {W} {height}" role="img" aria-label="Readability grade, lexical density and compression ratio for each of {paragraphs.length} paragraphs" data-points-for={id}>
			<title>Paragraph measures across the document</title>
			{#if panels.length}
				<line
					class="cursor"
					class:on={active !== null}
					x1={x(active ?? 1)}
					x2={x(active ?? 1)}
					y1={panels[0].top - 4}
					y2={panels[panels.length - 1].top + PANEL_H - 4}
				/>
			{/if}
			{#each panels as p (p.metric)}
				<g class="panel">
					<text class="title" x={LEFT - 38} y={p.top - 10}>{p.title}<tspan class="hint" dx="8">{p.hint}</tspan></text>
					{#each p.ticks as t (t)}
						<line class="grid" x1={LEFT} x2={W - RIGHT} y1={p.y(t)} y2={p.y(t)} />
						<text class="tick" x={LEFT - 6} y={p.y(t)}>{tick(t, p.metric)}</text>
					{/each}
					{#if p.ref !== null}
						<line class="ref" data-reference={p.metric} data-value={p.ref} x1={LEFT} x2={W - RIGHT} y1={p.y(p.ref)} y2={p.y(p.ref)} />
						<text class="ref-label" x={W - RIGHT + 6} y={p.y(p.ref)}>mean {p.ref.toFixed(1)}</text>
					{/if}
					<path class="trace" d={p.path} />
					{#each paragraphs as q (q.index)}
						{#if q[p.metric] === null}
							<path class="none" data-metric={p.metric} data-paragraph={q.index} data-value="" d="M{x(q.index) - 3},{p.top + PANEL_H - 15}l6,6m0,-6l-6,6" />
						{:else}
							<!-- Pointer-only: the table's rows give the keyboard the same. -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<circle
								class="dot"
								class:lit={active === q.index}
								data-metric={p.metric}
								data-paragraph={q.index}
								data-value={q[p.metric]}
								cx={x(q.index)}
								cy={p.y(q[p.metric] as number)}
								r="3.2"
								onpointerenter={() => (active = q.index)}
								onpointerleave={() => (active = null)}
							/>
						{/if}
					{/each}
				</g>
			{/each}
			<g class="axis">
				{#each paragraphs as q (q.index)}
					{#if (q.index - 1) % every === 0}
						<text class="tick x" x={x(q.index)} y={height - 8}>{q.index}</text>
					{/if}
				{/each}
				<text class="tick x label" x={LEFT - 38} y={height - 8}>¶</text>
			</g>
		</svg>
	</div>
	{#if overflowing ?? false}
		<p class="fig-hint" aria-hidden="true">Scroll sideways to see every paragraph.</p>
	{/if}

	{#snippet twin()}
		<!-- A region that scrolls must take focus, or it cannot be scrolled from
		     the keyboard (WCAG 2.1.1). -->
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<div class="fig-twin tall" tabindex="0" role="region" aria-label="Every paragraph's measures">
			<table data-twin-for={id}>
				<thead>
					<tr><th>#</th><th>Opens with</th><th>Words</th><th>Grade</th><th>Density</th><th>Compression</th></tr>
				</thead>
				<tbody {@attach roving}>
					{#each paragraphs as q (q.index)}
						<tr
							data-row={q.index}
							class:lit={active === q.index}
							onpointerenter={() => (active = q.index)}
							onpointerleave={() => (active = null)}
							onfocus={() => (active = q.index)}
							onblur={() => (active = null)}
						>
							<td class="num">{q.index}</td>
							<td class="opening">{q.opening}…</td>
							<td class="num">{q.words}</td>
							<td>{fmt(q.readability_grade)}</td>
							<td>{fmt(q.lexical_density)}</td>
							<td>{fmt(q.compression_ratio)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/snippet}

	{#snippet note()}
		One panel per paragraph measure, paragraphs in document order along the bottom. A cross marks a
		paragraph matra declined to measure: the field is <code>None</code> because the paragraph falls
		outside the measure's applicability conditions above, and the line breaks there rather than
		inventing a value. The dashed line is the document's mean readability as matra computes it
		(<code>Corpus::mean_readability</code>); matra computes no document-level lexical density or
		compression ratio, so those panels have no line.
	{/snippet}
</FigureFrame>

<style>
	.tall {
		max-height: 17rem;
		overflow-y: auto;
	}

	.tall thead th {
		position: sticky;
		top: 0;
		background: var(--bg-raised);
	}

	td.opening {
		max-width: 16rem;
		overflow: hidden;
		text-overflow: ellipsis;
		font-family: var(--font-sans);
		color: var(--text-muted);
	}

	.doc {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem 1.4rem;
		margin: 0 0 0.9rem;
		font-size: 0.82rem;
	}

	.doc div {
		display: flex;
		gap: 0.45em;
	}

	.doc dt {
		color: var(--text-muted);
	}

	.doc dd {
		margin: 0;
		font-family: var(--font-mono);
		font-weight: 600;
	}

	svg {
		font-family: var(--font-sans);
		--m: var(--mark);
	}

	.cursor {
		stroke: var(--spark);
		stroke-width: 1px;
		opacity: 0;
		pointer-events: none;
	}

	.cursor.on {
		opacity: 1;
	}

	.dot.lit {
		fill: var(--spark);
		stroke: var(--spark);
	}

	tbody tr:focus-visible {
		outline: 2px solid var(--spark);
		outline-offset: -2px;
	}

	tbody tr.lit td {
		color: var(--spark);
	}

	.title {
		font-size: 12.5px;
		font-weight: 650;
		fill: var(--text);
	}

	.hint {
		font-size: 11px;
		font-weight: 400;
		fill: var(--text-muted);
	}

	.grid {
		stroke: var(--border);
		stroke-width: 1px;
	}

	.tick {
		font: 10.5px var(--font-mono);
		fill: var(--text-muted);
		text-anchor: end;
		dominant-baseline: middle;
	}

	.tick.x {
		text-anchor: middle;
	}

	.tick.label {
		text-anchor: start;
	}

	.ref {
		stroke: var(--text);
		stroke-width: 1.2px;
		stroke-dasharray: 5 4;
		opacity: 0.75;
	}

	.ref-label {
		font: 600 10.5px var(--font-mono);
		fill: var(--text);
		dominant-baseline: middle;
	}

	.trace {
		fill: none;
		stroke: var(--m);
		stroke-width: 1.4px;
		opacity: 0.55;
	}

	.dot {
		fill: var(--m);
		stroke: var(--bg-raised);
		stroke-width: 1.5px;
	}

	.none {
		fill: none;
		stroke: var(--text-muted);
		stroke-width: 1.4px;
		stroke-linecap: round;
	}
</style>
