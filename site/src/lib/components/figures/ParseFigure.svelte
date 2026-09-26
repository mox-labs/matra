<script lang="ts">
	/**
	 * The dependency parse of one input, a sentence at a time (EP-0012, M3).
	 *
	 * The token table leads and the arc diagram follows, both drawn from the
	 * same generated data. The table is the figure's text twin: the docsite
	 * floor checks every arc against it in the prerendered HTML.
	 *
	 * Relations are grouped the way Universal Dependencies groups them, and
	 * the group is drawn as a line, not a colour: core arguments heavy and
	 * solid, modifiers light and solid, function words dotted, the rest
	 * dashed. The root's stroke is Emergence, as the mark's is.
	 *
	 * Linked highlighting: pointing at or focusing a word, in the table or the
	 * diagram, keeps its arc from its head and its arcs to its dependents, and
	 * dims the rest, at once. The table's rows are one tab stop; the arrow
	 * keys walk them.
	 *
	 * Motion follows the research (./motion): nothing moves unless the reader
	 * picks another sentence, and then the two cross in one eased stage that
	 * a new pick reverses mid-flight. With reduced motion, at once.
	 */
	import { fade } from 'svelte/transition';
	import type { ParseFigureFile } from '$lib/types';
	import FigureFrame from './FigureFrame.svelte';
	import { FAMILY_LABELS, family, layoutParse, TYPE, type Family } from './parse-layout';
	import { roving, swap } from './motion';

	let {
		id,
		file,
		sentence,
		dataUrl
	}: { id: string; file: ParseFigureFile; sentence: number; dataUrl: string } = $props();

	const sentences = $derived(file.data.sentences);
	// The sentence shown: the page's choice until the reader makes one.
	let current = $derived(sentence);
	const shown = $derived(sentences[current - 1]);
	const layout = $derived(layoutParse(shown.tokens));
	const byId = $derived(new Map(shown.tokens.map((t) => [t.id, t])));
	const families = $derived(
		(['core', 'modifier', 'function', 'other'] as Family[]).filter((f) =>
			sentences.some((s) => s.tokens.some((t) => t.head !== 0 && family(t.dep) === f))
		)
	);

	/** Whether the diagram is wider than its container, so the scroll hint shows. */
	let overflowing = $state<boolean | null>(null);
	function measure(node: HTMLElement) {
		const update = () => (overflowing = node.scrollWidth > node.clientWidth + 1);
		const ro = new ResizeObserver(update);
		ro.observe(node);
		update();
		return () => ro.disconnect();
	}
	// Before any script runs, guess from the width of the column on a desktop.
	const hint = $derived(overflowing ?? layout.width > 882);

	/** The word the reader points at or focuses, and the words it connects to. */
	let active = $state<number | null>(null);
	const lit = $derived.by(() => {
		if (active === null) return null;
		const set = new Set<number>([active]);
		const t = byId.get(active);
		if (t && t.head !== 0) set.add(t.head);
		for (const d of shown.tokens) if (d.head === active) set.add(d.id);
		return set;
	});
	const arcLit = (a: { head: number; dep: number }) => active !== null && (a.dep === active || a.head === active);
</script>

<FigureFrame {id} kind="parse" title="Dependency parse" {file} {dataUrl}>
	{#snippet controls()}
		{#if sentences.length > 1}
			<div class="picker" role="group" aria-label="Sentence">
				<span class="picker-label" aria-hidden="true">Sentence</span>
				{#each sentences as s, i (i)}
					<button
						type="button"
						aria-pressed={current === i + 1}
						aria-label="Sentence {i + 1}: {s.text}"
						title={s.text}
						onclick={() => (current = i + 1)}>{i + 1}</button
					>
				{/each}
			</div>
		{/if}
	{/snippet}

	<div class="stage" data-sentence={current}>
		{#key current}
			<div class="state" class:focused={lit !== null} transition:fade={swap()}>
				<p class="sentence"><span class="n">{current}</span>{shown.text}</p>

				<!-- The data, one line above the arcs, in the form a reader already
				     reads: each word, its relation, and the word it depends on. The
				     keyboard lights a word's arcs from here, as the pointer does. -->
				<ol class="tokens" aria-label="Each word of sentence {current}, its relation and its head" {@attach roving}>
					{#each shown.tokens as t (t.id)}
						<li
							data-row={t.id}
							class:lit={lit?.has(t.id)}
							class:dim={lit !== null && !lit.has(t.id)}
							onpointerenter={() => (active = t.id)}
							onpointerleave={() => (active = null)}
							onfocus={() => (active = t.id)}
							onblur={() => (active = null)}
						>
							<span class="tw">{t.text}</span>
							{t.head === 0 ? 'root' : `${t.dep} of ${byId.get(t.head)?.text}`}
						</li>
					{/each}
				</ol>


				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div class="fig-scroll" tabindex="0" role="region" aria-label="Arc diagram of sentence {current}" {@attach measure}>
					<svg
						width={layout.width}
						height={layout.height}
						viewBox="0 0 {layout.width} {layout.height}"
						role="img"
						aria-label="Dependency arcs for sentence {current}: {shown.text}"
						data-arcs-for={id}
					>
						<title>Dependency arcs for: {shown.text}</title>
						{#if layout.root}
							<g class="arc fam-root" class:lit={active === layout.root.id} data-dep={layout.root.id} data-head="0">
								<line x1={layout.root.x} y1={layout.root.top} x2={layout.root.x} y2={layout.baseline - 2} />
								<path class="tip" d="M{layout.root.x - 3.5},{layout.baseline - 9}L{layout.root.x + 3.5},{layout.baseline - 9}L{layout.root.x},{layout.baseline - 2.5}Z" />
								<text class="rel" x={layout.root.x} y={layout.root.top - 6} font-size={TYPE.rel}>root</text>
							</g>
						{/if}
						{#each layout.arcs as a (a.dep)}
							<g class="arc fam-{a.family}" class:lit={arcLit(a)} data-dep={a.dep} data-head={a.head}>
								<path class="line" d={a.path} />
								<path class="tip" d={a.arrow} />
								<text class="rel" x={a.labelX} y={a.labelY} font-size={TYPE.rel}>{a.rel}</text>
							</g>
						{/each}
						{#each layout.tokens as t (t.id)}
							<!-- Pointer-only: the table's rows give the keyboard the same. -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<g
								class="tok"
								class:lit={lit?.has(t.id)}
								onpointerenter={() => (active = t.id)}
								onpointerleave={() => (active = null)}
							>
								<text class="word" data-id={t.id} x={t.x} y={layout.wordY} font-size={TYPE.word}>{t.text}</text>
								<text class="pos" x={t.x} y={layout.posY} font-size={TYPE.pos}>{t.pos}</text>
							</g>
						{/each}
					</svg>
				</div>
				{#if hint}
					<p class="fig-hint" aria-hidden="true">Scroll sideways to see the whole sentence.</p>
				{/if}
			</div>
		{/key}
	</div>

	{#snippet twin()}
		<!-- A region that scrolls must take focus, or it cannot be scrolled
		     from the keyboard (WCAG 2.1.1); the page's own tables do the same. -->
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<div class="fig-twin" tabindex="0" role="region" aria-label="Tokens of sentence {current}">
			<table data-twin-for={id}>
				<thead>
					<tr><th>#</th><th>Word</th><th>Lemma</th><th>POS</th><th>Head</th><th>Relation</th></tr>
				</thead>
				<tbody>
					{#each shown.tokens as t (t.id)}
						<tr
							data-row={t.id}
							class:lit={lit?.has(t.id)}
							class:dim={lit !== null && !lit.has(t.id)}
							onpointerenter={() => (active = t.id)}
							onpointerleave={() => (active = null)}
						>
							<td class="num">{t.id}</td>
							<td class="word">{t.text}</td>
							<td>{t.lemma}</td>
							<td class="pos">{t.pos}</td>
							<td class="num">{t.head}<span class="head-word">{t.head === 0 ? 'root' : byId.get(t.head)?.text}</span></td>
							<td><span class="swatch fam-{t.head === 0 ? 'root' : family(t.dep)}" aria-hidden="true"></span>{t.dep}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/snippet}

	{#snippet legend()}
		<ul class="fig-legend" aria-label="Arc lines">
			{#each families as f (f)}
				<li class="fam-{f}">
					<svg class="sample" width="22" height="10" viewBox="0 0 22 10" aria-hidden="true">
						<path class="line" d="M1,9C1,1 21,1 21,9" />
					</svg>{FAMILY_LABELS[f]}
				</li>
			{/each}
		</ul>
	{/snippet}

	{#snippet note()}
		Each arc runs from a word's head to the word, whose arrow it ends at, labelled with the
		relation. The line groups the relations the way Universal Dependencies does: the core
		arguments of a predicate, the modifiers, and the function words. Point at a word, here or in
		the table, to keep its arcs and dim the rest.
	{/snippet}
</FigureFrame>

<style>
	/* On the whole figure, since the legend renders in the frame's caption.
	   The families share one neutral ink (checked at 3:1 as --mark); the line
	   carries the family. The root is Emergence, as in the mark. */
	:global(.mx-figure[data-figure='parse']) {
		--fam-core: var(--mark);
		--fam-modifier: var(--mark);
		--fam-function: var(--mark);
		--fam-other: var(--mark-quiet);
		--fam-root: var(--emergence);
	}

	/* Old and new states occupy the same cell while one fades into the other. */
	.stage {
		display: grid;
	}

	.picker {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.picker-label {
		margin-right: 0.35rem;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	.picker button {
		min-width: 2rem;
		height: 2rem;
		padding: 0 0.5rem;
		font: 600 0.82rem var(--font-mono);
		color: var(--text-muted);
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
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

	.state {
		grid-area: 1 / 1;
		min-width: 0;
	}

	.sentence {
		margin: 0 0 0.8rem;
		font-size: 1.02rem;
		line-height: 1.5;
	}

	/* The compact data line: word, relation, head, set small and wrapping,
	   one line above the arcs. */
	.tokens {
		display: flex;
		flex-wrap: wrap;
		gap: 0.15rem 0.9rem;
		margin: 0 0 0.7rem;
		padding: 0;
		list-style: none;
		font: var(--type-xs) / 1.5 var(--font-mono);
		color: var(--text-muted);
	}

	.tokens li {
		margin: 0;
		transition: opacity var(--duration-fast) linear;
	}

	.tokens .tw {
		color: var(--text);
		font-weight: 600;
	}

	.tokens li.lit .tw {
		color: var(--spark);
	}

	.tokens li.dim {
		opacity: 0.4;
	}

	.tokens li:focus-visible {
		outline: 2px solid var(--spark);
		outline-offset: 2px;
	}

	.sentence .n {
		display: inline-block;
		min-width: 1.5em;
		margin-right: 0.4em;
		font: 600 0.8rem var(--font-mono);
		color: var(--text-muted);
	}

	.word {
		color: var(--text);
		font-weight: 600;
	}

	td.pos {
		color: var(--text-muted);
	}

	.head-word {
		margin-left: 0.5em;
		color: var(--text);
	}

	/* The family as a line sample, the same line the arc is drawn with. */
	.swatch {
		display: inline-block;
		width: 1.1em;
		height: 0;
		margin-right: 0.45em;
		vertical-align: 0.3em;
		border-bottom: 1.5px solid var(--swatch);
	}

	.swatch.fam-core {
		border-bottom-width: 2.5px;
	}

	.swatch.fam-function {
		border-bottom-style: dotted;
		border-bottom-width: 2px;
	}

	.swatch.fam-other {
		border-bottom-style: dashed;
	}

	tbody tr {
		transition: opacity var(--duration-fast) linear;
	}

	tbody tr:focus-visible {
		outline: 2px solid var(--spark);
		outline-offset: -2px;
	}

	tbody tr.dim {
		opacity: 0.4;
	}

	tbody tr.lit td.word {
		color: var(--spark);
	}

	.arc,
	.tok {
		transition: opacity var(--duration-fast) linear;
	}

	.focused .arc:not(.lit),
	.focused .tok:not(.lit) {
		opacity: 0.2;
	}

	.arc.lit {
		color: var(--spark);
	}

	.fam-root.lit {
		color: var(--emergence);
	}

	.tok.lit text.word {
		fill: var(--spark);
	}

	@media (prefers-reduced-motion: reduce) {
		tbody tr,
		.arc,
		.tok {
			transition: none;
		}
	}

	.fam-core {
		--swatch: var(--fam-core);
	}
	.fam-modifier {
		--swatch: var(--fam-modifier);
	}
	.fam-function {
		--swatch: var(--fam-function);
	}
	.fam-other {
		--swatch: var(--fam-other);
	}
	.fam-root {
		--swatch: var(--fam-root);
	}

	svg:not(.sample) {
		font-family: var(--font-mono);
	}

	.arc {
		color: var(--swatch);
	}

	.arc .line,
	.arc line {
		fill: none;
		stroke: currentColor;
		stroke-width: 1.4px;
	}

	.fam-core .line {
		stroke-width: 1.9px;
	}

	.fam-function .line {
		stroke-dasharray: 1.5 3;
		stroke-linecap: round;
	}

	.fam-other .line {
		stroke-dasharray: 4 3;
	}

	.arc .tip {
		fill: currentColor;
	}

	.rel {
		fill: currentColor;
		text-anchor: middle;
		dominant-baseline: middle;
		paint-order: stroke;
		stroke: var(--bg-raised);
		stroke-width: 4px;
		stroke-linejoin: round;
	}

	text.word {
		fill: var(--text);
		font-weight: 600;
		text-anchor: middle;
	}

	text.pos {
		fill: var(--text-muted);
		text-anchor: middle;
		letter-spacing: 0.04em;
	}

	.sample {
		color: var(--swatch);
		overflow: visible;
	}

	.sample .line {
		fill: none;
		stroke: currentColor;
		stroke-width: 1.6px;
	}

	.fam-core .sample .line {
		stroke-width: 2px;
	}

	.fam-function .sample .line {
		stroke-dasharray: 1.5 3;
		stroke-linecap: round;
	}

	.fam-other .sample .line {
		stroke-dasharray: 4 3;
	}
</style>
