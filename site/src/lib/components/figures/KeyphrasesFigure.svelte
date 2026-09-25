<script lang="ts">
	/**
	 * RAKE against YAKE over the same text, as a slopegraph (EP-0012, M4).
	 *
	 * The phrases are each method's top few. Each is drawn at its rank under
	 * RAKE on the left and under YAKE on the right, on one log scale, so a
	 * phrase one method ranks first and the other ranks 900th reads as a steep
	 * line. Ranks, not scores: matra's two scores are not comparable with each
	 * other. A line joins a phrase only when both methods rank it; a phrase
	 * only one method produces is a hollow dot on that side, with no line, so a
	 * pair of lists that share little does not become a bundle of lines to
	 * nowhere. The table of every rank and score leads. Static: nothing moves.
	 *
	 * No colour tells the groups apart: a phrase in RAKE's top few is joined by
	 * a solid line, one in YAKE's by a dashed line, one in both by a heavy
	 * line; the side a label sits on already says which method ranks it.
	 * Pointing at or focusing a row lights its line and labels in Spark, and
	 * pointing at a line lights its row, at once.
	 */
	import { scaleLog } from 'd3-scale';
	import type { KeyphrasesFigureFile } from '$lib/types';
	import FigureFrame from './FigureFrame.svelte';
	import { roving } from './motion';

	let { id, file, dataUrl }: { id: string; file: KeyphrasesFigureFile; dataUrl: string } = $props();

	const CHAR = 11 * 0.6; // monospace 11px advance
	const ROW = 15;
	const TOP = 34;
	const SPAN = 300;
	const SLOPE_W = 180;
	const PAD = 12;

	const data = $derived(file.data);
	/** The phrase the reader points at. */
	let active = $state<string | null>(null);
	const inRake = $derived(new Set(data.rake.map((p) => p.phrase)));
	const inYake = $derived(new Set(data.yake.map((p) => p.phrase)));

	const rows = $derived(
		[...data.phrases].sort((a, b) => {
			const ra = Math.min(a.rake?.rank ?? Infinity, a.yake?.rank ?? Infinity);
			const rb = Math.min(b.rake?.rank ?? Infinity, b.yake?.rank ?? Infinity);
			return ra - rb || a.phrase.localeCompare(b.phrase);
		})
	);

	const label = (phrase: string, rank: number) => `${phrase}  ${rank}`;

	const layout = $derived.by(() => {
		const maxRank = Math.max(
			10,
			...data.phrases.flatMap((p) => [p.rake?.rank ?? 1, p.yake?.rank ?? 1])
		);
		const y = scaleLog().domain([1, maxRank]).range([TOP, TOP + SPAN]);
		const leftW = Math.max(...data.rake.map((p) => label(p.phrase, p.rank).length)) * CHAR;
		const rightW = Math.max(...data.yake.map((p) => label(p.phrase, p.rank).length)) * CHAR;
		const L = PAD + leftW + 16;
		const R = L + SLOPE_W;
		const width = Math.ceil(R + 16 + rightW + PAD);

		// Labels sit at their point's height unless that would overlap the
		// label above; then they step down, with a leader back to the point.
		const dodge = (list: { phrase: string; rank: number }[]) => {
			let last = -Infinity;
			return [...list]
				.sort((a, b) => a.rank - b.rank || a.phrase.localeCompare(b.phrase))
				.map((p) => {
					const py = y(p.rank);
					const ly = Math.max(py, last + ROW);
					last = ly;
					return { ...p, py, ly };
				});
		};
		const left = dodge(data.rake);
		const right = dodge(data.yake);
		const bottom = y(y.domain()[1]);
		const height = Math.ceil(Math.max(bottom + 16, ...left.map((l) => l.ly + 10), ...right.map((r) => r.ly + 10)));
		const ticks = [1, 10, 100, 1000, 10000].filter((t) => t <= y.domain()[1]);
		const lines = data.phrases.map((p) => ({
			phrase: p.phrase,
			rake: p.rake?.rank ?? null,
			yake: p.yake?.rank ?? null,
			y1: p.rake ? y(p.rake.rank) : null,
			y2: p.yake ? y(p.yake.rank) : null,
			group: inRake.has(p.phrase) && inYake.has(p.phrase) ? 'both' : inRake.has(p.phrase) ? 'rake' : 'yake'
		}));
		return { y, L, R, width, height, left, right, lines, ticks, bottom };
	});

	let overflowing = $state<boolean | null>(null);
	function measure(node: HTMLElement) {
		const update = () => (overflowing = node.scrollWidth > node.clientWidth + 1);
		const ro = new ResizeObserver(update);
		ro.observe(node);
		update();
		return () => ro.disconnect();
	}
	const hint = $derived(overflowing ?? layout.width > 712);

	const fmtScore = (v: number) => (v >= 100 ? v.toFixed(1) : v.toFixed(3));
</script>

<FigureFrame {id} kind="keyphrases" title="RAKE and YAKE on the same text" {file} {dataUrl}>
	<!-- A region that scrolls must take focus, or it cannot be scrolled from
	     the keyboard (WCAG 2.1.1). -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-twin tall" tabindex="0" role="region" aria-label="Ranks and scores under both methods">
		<table data-twin-for={id}>
			<thead>
				<tr><th>Phrase</th><th>RAKE rank</th><th>RAKE score</th><th>YAKE rank</th><th>YAKE score</th></tr>
			</thead>
			<tbody {@attach roving}>
				{#each rows as r (r.phrase)}
					<tr
						data-row={r.phrase}
						class:lit={active === r.phrase}
						onpointerenter={() => (active = r.phrase)}
						onpointerleave={() => (active = null)}
						onfocus={() => (active = r.phrase)}
						onblur={() => (active = null)}
					>
						<td class="phrase">{r.phrase}</td>
						<td class="num">{r.rake ? r.rake.rank : 'not ranked'}</td>
						<td>{r.rake ? fmtScore(r.rake.score) : ''}</td>
						<td class="num">{r.yake ? r.yake.rank : 'not ranked'}</td>
						<td>{r.yake ? fmtScore(r.yake.score) : ''}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-scroll" tabindex="0" role="region" aria-label="Slopegraph of ranks" {@attach measure}>
		<svg width={layout.width} height={layout.height} viewBox="0 0 {layout.width} {layout.height}" role="img" aria-label="Each phrase's RAKE rank joined to its YAKE rank, on a log scale" data-slopes-for={id}>
			<title>RAKE rank against YAKE rank for the top phrases of each</title>
			<text class="head" x={layout.L} y={14} text-anchor="end">RAKE rank</text>
			<text class="head" x={layout.R} y={14}>YAKE rank</text>
			{#each layout.ticks as t (t)}
				<line class="grid" x1={layout.L} x2={layout.R} y1={layout.y(t)} y2={layout.y(t)} />
			{/each}
			<line class="axis" x1={layout.L} x2={layout.L} y1={layout.y(1) - 6} y2={layout.bottom} />
			<line class="axis" x1={layout.R} x2={layout.R} y1={layout.y(1) - 6} y2={layout.bottom} />

			{#each layout.lines as l (l.phrase)}
				<!-- Pointer-only: the table's rows give the keyboard the same. -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<g
					class="slope g-{l.group}"
					class:lit={active === l.phrase}
					class:dim={active !== null && active !== l.phrase}
					data-phrase={l.phrase}
					data-rake={l.rake ?? ''}
					data-yake={l.yake ?? ''}
					onpointerenter={() => (active = l.phrase)}
					onpointerleave={() => (active = null)}
				>
					{#if l.y1 !== null && l.y2 !== null}
						<line x1={layout.L} y1={l.y1} x2={layout.R} y2={l.y2} />
					{/if}
					{#if l.y1 !== null}<circle class:alone={l.y2 === null} cx={layout.L} cy={l.y1} r="3.2" />{/if}
					{#if l.y2 !== null}<circle class:alone={l.y1 === null} cx={layout.R} cy={l.y2} r="3.2" />{/if}
				</g>
			{/each}

			<!-- Tick labels after the lines, with a halo, so a bundle of lines
			     cannot hide them. -->
			{#each layout.ticks as t (t)}
				<text class="tick" x={(layout.L + layout.R) / 2} y={layout.y(t) - 4}>{t}</text>
			{/each}

			{#each layout.left as p (p.phrase)}
				<g class="lbl g-{inYake.has(p.phrase) ? 'both' : 'rake'}" class:lit={active === p.phrase}>
					{#if p.ly !== p.py}<path class="leader" d="M{layout.L - 10},{p.ly}L{layout.L - 3},{p.py}" />{/if}
					<text x={layout.L - 12} y={p.ly} text-anchor="end">{p.phrase}<tspan class="rank" dx="6">{p.rank}</tspan></text>
				</g>
			{/each}
			{#each layout.right as p (p.phrase)}
				<g class="lbl g-{inRake.has(p.phrase) ? 'both' : 'yake'}" class:lit={active === p.phrase}>
					{#if p.ly !== p.py}<path class="leader" d="M{layout.R + 10},{p.ly}L{layout.R + 3},{p.py}" />{/if}
					<text x={layout.R + 12} y={p.ly}><tspan class="rank">{p.rank}</tspan><tspan dx="6">{p.phrase}</tspan></text>
				</g>
			{/each}
		</svg>
	</div>
	{#if hint}
		<p class="fig-hint" aria-hidden="true">Scroll sideways to see both rankings.</p>
	{/if}

	{#snippet legend()}
		<ul class="fig-legend" aria-label="Line colours">
			<li><span class="swatch g-rake" aria-hidden="true"></span>in RAKE's top {data.shown}</li>
			<li><span class="swatch g-yake" aria-hidden="true"></span>in YAKE's top {data.shown}</li>
			{#if layout.lines.some((l) => l.group === 'both')}
				<li><span class="swatch g-both" aria-hidden="true"></span>in both</li>
			{/if}
			{#if layout.lines.some((l) => l.y1 === null || l.y2 === null)}
				<li>
					<svg class="slope g-both" width="12" height="12" aria-hidden="true"><circle class="alone" cx="6" cy="6" r="4" /></svg>
					only one method produces the phrase
				</li>
			{/if}
		</ul>
	{/snippet}

	{#snippet note()}
		Both of matra's scores rank higher as more relevant: RAKE sums each word's co-occurrence degree
		over its frequency, and matra's YAKE reports the reciprocal of the published YAKE score, which
		runs lower as more relevant. The two scores share no scale, so the figure compares ranks, on a
		log scale, out of {data.total.rake} RAKE and {data.total.yake} YAKE candidates. Equal scores share
		a rank, and a tie at the cut of {data.shown} is taken in alphabetical order. A phrase a method does
		not produce at all is a hollow dot on the side that ranks it, with no line.
	{/snippet}
</FigureFrame>

<style>
	:global(.mx-figure[data-figure='keyphrases']) {
		/* One neutral ink: the line style carries the group. */
		--g-rake: var(--mark);
		--g-yake: var(--mark);
		--g-both: var(--text);
	}

	.g-rake {
		--g: var(--g-rake);
	}
	.g-yake {
		--g: var(--g-yake);
	}
	.g-both {
		--g: var(--g-both);
	}

	.tall {
		max-height: 17rem;
		overflow-y: auto;
	}

	.tall thead th {
		position: sticky;
		top: 0;
		background: var(--bg-raised);
	}

	td.phrase {
		color: var(--text);
		font-weight: 600;
	}

	.swatch {
		display: inline-block;
		width: 0.9em;
		height: 0.18em;
		margin-right: 0.45em;
		vertical-align: 0.25em;
		background: var(--g);
	}

	svg {
		font: 11px var(--font-mono);
	}

	.head {
		font: 650 11px var(--font-sans);
		letter-spacing: 0.06em;
		text-transform: uppercase;
		fill: var(--text-muted);
	}

	.grid {
		stroke: var(--border);
	}

	.axis {
		stroke: var(--border-strong);
	}

	.tick {
		font-size: 10px;
		fill: var(--text-muted);
		text-anchor: middle;
		paint-order: stroke;
		stroke: var(--bg-raised);
		stroke-width: 3px;
	}

	.slope line {
		stroke: var(--g);
		stroke-width: 1.4px;
		opacity: 0.8;
	}

	.slope circle {
		fill: var(--g);
		stroke: var(--g);
		stroke-width: 1.4px;
	}

	.slope circle.alone {
		fill: var(--bg-raised);
	}

	.lbl text {
		fill: var(--text);
		dominant-baseline: middle;
	}

	.lbl .rank {
		fill: var(--g);
		font-weight: 600;
	}

	.leader {
		fill: none;
		stroke: var(--g);
		stroke-width: 1px;
		opacity: 0.6;
	}

	/* The group as a line: solid for RAKE's top, dashed for YAKE's, heavy for both. */
	.slope.g-yake line {
		stroke-dasharray: 5 3;
	}

	.slope.g-both line {
		stroke-width: 2.4px;
	}

	.swatch.g-yake {
		background: none;
		border-top: 2px dashed var(--g);
		height: 0;
	}

	.swatch.g-both {
		height: 0.28em;
	}

	.slope {
		transition: opacity var(--duration-fast) linear;
	}

	.slope.dim {
		opacity: 0.25;
	}

	.slope.lit {
		--g: var(--spark);
	}

	.lbl.lit text {
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
		.slope {
			transition: none;
		}
	}
</style>
