<script lang="ts">
	/**
	 * The dependency parse of one input, a sentence at a time (EP-0012, M3).
	 *
	 * The token table leads and the arc diagram follows, both drawn from the
	 * same generated data. The table is the figure's text twin: the docsite
	 * floor checks every arc against it in the prerendered HTML.
	 *
	 * Motion follows the EP's rule: nothing moves unless the reader picks
	 * another sentence, and then there are two stages, the old state fading
	 * out and the new one in, about 0.6s in all, easing in and out. With
	 * reduced motion requested the new state appears at once.
	 */
	import { fade } from 'svelte/transition';
	import { cubicInOut } from 'svelte/easing';
	import type { ParseFigureFile } from '$lib/types';
	import { FAMILY_LABELS, family, layoutParse, TYPE, type Family } from './parse-layout';

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
	const source = $derived(file.source);
	const model = $derived(file.generator.udpipe_model);

	function reduced(): boolean {
		return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	}
	const leave = () => ({ duration: reduced() ? 0 : 220, easing: cubicInOut });
	const enter = () => ({ duration: reduced() ? 0 : 360, delay: reduced() ? 0 : 220, easing: cubicInOut });

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
	const hint = $derived(overflowing ?? layout.width > 712);
</script>

<figure class="parse-figure" {id} data-figure="parse" data-input={file.input} data-sentence={current} data-pagefind-ignore aria-labelledby="{id}-title">
	<div class="head">
		<p class="eyebrow" id="{id}-title">Dependency parse</p>
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
	</div>

	<div class="stage">
		{#key current}
			<div class="state" in:fade={enter()} out:fade={leave()}>
				<p class="sentence"><span class="n">{current}</span>{shown.text}</p>

				<!-- A region that scrolls must take focus, or it cannot be scrolled
				     from the keyboard (WCAG 2.1.1); the page's own tables do the same. -->
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div class="twin" tabindex="0" role="region" aria-label="Tokens of sentence {current}">
					<table data-twin-for={id}>
						<thead>
							<tr><th>#</th><th>Word</th><th>Lemma</th><th>POS</th><th>Head</th><th>Relation</th></tr>
						</thead>
						<tbody>
							{#each shown.tokens as t (t.id)}
								<tr>
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

				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div class="scroll" tabindex="0" role="region" aria-label="Arc diagram of sentence {current}" {@attach measure}>
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
							<g class="arc fam-root" data-dep={layout.root.id} data-head="0">
								<line x1={layout.root.x} y1={layout.root.top} x2={layout.root.x} y2={layout.baseline - 2} />
								<path class="tip" d="M{layout.root.x - 3.5},{layout.baseline - 9}L{layout.root.x + 3.5},{layout.baseline - 9}L{layout.root.x},{layout.baseline - 2.5}Z" />
								<text class="rel" x={layout.root.x} y={layout.root.top - 6} font-size={TYPE.rel}>root</text>
							</g>
						{/if}
						{#each layout.arcs as a (a.dep)}
							<g class="arc fam-{a.family}" data-dep={a.dep} data-head={a.head}>
								<path class="line" d={a.path} />
								<path class="tip" d={a.arrow} />
								<text class="rel" x={a.labelX} y={a.labelY} font-size={TYPE.rel}>{a.rel}</text>
							</g>
						{/each}
						{#each layout.tokens as t (t.id)}
							<text class="word" data-id={t.id} x={t.x} y={layout.wordY} font-size={TYPE.word}>{t.text}</text>
							<text class="pos" x={t.x} y={layout.posY} font-size={TYPE.pos}>{t.pos}</text>
						{/each}
					</svg>
				</div>
				{#if hint}
					<p class="hint" aria-hidden="true">Scroll sideways to see the whole sentence.</p>
				{/if}
			</div>
		{/key}
	</div>

	<figcaption>
		<ul class="legend" aria-label="Arc colours">
			{#each families as f (f)}
				<li class="fam-{f}">
					<svg class="sample" width="22" height="10" viewBox="0 0 22 10" aria-hidden="true">
						<path class="line" d="M1,9C1,1 21,1 21,9" />
					</svg>{FAMILY_LABELS[f]}
				</li>
			{/each}
		</ul>
		<p>
			Each arc runs from a word's head to the word, whose arrow it ends at, labelled with the
			relation. Colour groups the relations the way Universal Dependencies does: the core
			arguments of a predicate, the modifiers, and the function words.
		</p>
		<p class="provenance">
			Input: <a href={source.url}>{source.title}</a>, {source.author}. {source.licence}.
			Parsed by matra {file.generator.matra} with the UDPipe model {model.name}
			(SHA-256 <code>{model.sha256.slice(0, 12)}</code>). Data:
			<a href={dataUrl}>{file.input}/parse.json</a>.
		</p>
	</figcaption>
</figure>

<style>
	.parse-figure {
		--fam-core: light-dark(#b3401e, #f0915f);
		--fam-modifier: light-dark(#2b6a8c, #7fb9d8);
		--fam-function: light-dark(#6f695f, #a6a29a);
		/* Every colour here holds 4.5:1 against the figure's background in its
		   theme, because the relation labels are drawn in it at 10.5px. */
		--fam-other: light-dark(#7a746a, #8a867f);
		--fam-root: var(--text);
		margin: 2.2em 0;
		padding: 1rem 1.1rem 0.9rem;
		max-width: none;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 12px;
	}

	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem 1rem;
		margin-bottom: 0.6rem;
	}

	.eyebrow {
		margin: 0;
		font-size: 0.72rem;
		font-weight: 650;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
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

	/* Old and new states occupy the same cell while one fades into the other. */
	.stage {
		display: grid;
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

	.sentence .n {
		display: inline-block;
		min-width: 1.5em;
		margin-right: 0.4em;
		font: 600 0.8rem var(--font-mono);
		color: var(--text-muted);
	}

	.twin {
		overflow-x: auto;
		margin-bottom: 0.9rem;
	}

	.twin table {
		width: 100%;
		margin: 0;
		font-size: 0.84rem;
		border-collapse: collapse;
	}

	.twin :is(th, td) {
		padding: 0.28em 0.7em 0.28em 0;
		white-space: nowrap;
		border-bottom: 1px solid var(--border);
	}

	.twin th {
		font-size: 0.72rem;
		font-weight: 650;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--text-muted);
		border-bottom-width: 1px;
	}

	.twin td {
		font-family: var(--font-mono);
		font-size: 0.8rem;
	}

	.twin .num {
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}

	.twin .word {
		color: var(--text);
		font-weight: 600;
	}

	.twin .pos {
		color: var(--text-muted);
	}

	.head-word {
		margin-left: 0.5em;
		color: var(--text);
	}

	.swatch {
		display: inline-block;
		width: 0.6em;
		height: 0.6em;
		margin-right: 0.45em;
		border-radius: 2px;
		vertical-align: 0.05em;
		background: var(--swatch);
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

	/* The diagram keeps its drawn size and scrolls inside its own box, with
	   the scrolling-shadow fade the page's other wide diagrams use. */
	.scroll {
		overflow-x: auto;
		overscroll-behavior-x: contain;
		background:
			linear-gradient(to right, var(--bg-raised) 30%, transparent) left / 2.5rem 100% no-repeat local,
			linear-gradient(to left, var(--bg-raised) 30%, transparent) right / 2.5rem 100% no-repeat local,
			linear-gradient(to right, light-dark(rgb(0 0 0 / 0.14), rgb(0 0 0 / 0.55)), transparent)
				left / 1rem 100% no-repeat scroll,
			linear-gradient(to left, light-dark(rgb(0 0 0 / 0.14), rgb(0 0 0 / 0.55)), transparent)
				right / 1rem 100% no-repeat scroll;
		background-color: var(--bg-raised);
	}

	.scroll svg {
		display: block;
		max-width: none;
		margin: 0 auto;
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

	.fam-other .line {
		stroke-dasharray: 3 3;
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

	.word {
		fill: var(--text);
		font-weight: 600;
		text-anchor: middle;
	}

	.pos {
		fill: var(--text-muted);
		text-anchor: middle;
		letter-spacing: 0.04em;
	}

	.hint {
		margin: 0.35rem 0 0;
		font-size: 0.78rem;
		color: var(--text-muted);
		text-align: center;
	}

	.parse-figure figcaption {
		margin: 0.9rem 0 0;
		padding-top: 0.75rem;
		max-width: none;
		text-align: left;
		font-size: 0.84rem;
		line-height: 1.5;
		color: var(--text-muted);
		border-top: 1px solid var(--border);
	}

	.parse-figure figcaption p {
		margin: 0.4rem 0 0;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem 1.1rem;
		margin: 0;
		padding: 0;
		list-style: none;
		color: var(--text);
	}

	.legend li {
		display: inline-flex;
		align-items: center;
		gap: 0.4em;
		margin: 0;
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

	.fam-other .sample .line {
		stroke-dasharray: 3 3;
	}

	.provenance code {
		font-size: 0.92em;
	}
</style>
