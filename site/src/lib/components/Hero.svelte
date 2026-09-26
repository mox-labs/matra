<script lang="ts">
	/**
	 * The home page's hero: the mark at full scale, explaining itself.
	 *
	 * A sentence is set large in the reading face, as text, the way a type
	 * specimen sets one line to show a face. matra's parse of it
	 * is drawn over it from the same committed data the mark is drawn from:
	 * the headline bar on the tops of the letters, every word hanging from it
	 * as Devanagari letters hang from theirs, the
	 * root's stroke in Emergence just after its word, and each dependency as
	 * an arc hanging below.
	 *
	 * Placement needs no font measurement and no script. Each word spans two
	 * equal grid columns, so its centre is a grid line; an arc spans from its
	 * head's line to its dependent's (from the root's right edge, where its
	 * stroke hangs, when the head is the root), drawn as a hanging half
	 * ellipse. The bar's height is the font's own arithmetic ($lib/fonts), so
	 * it needs no measuring either. So the prerendered page is exact, at every
	 * width.
	 *
	 * Nothing moves on load. The reader lights a word by pointing at it or
	 * focusing it: its head, its dependents and the arcs between them stay,
	 * the rest dims (opacity only), and a click pins it. "Show the parse"
	 * reveals each word's part of speech and each arc's relation, and works
	 * without a script; its table twin is the parse as text.
	 */
	import { ALEGREYA_BLACK, inkTop, tallest } from '$lib/fonts';
	import { drawn } from '$lib/mark';
	import type { ParseToken } from '$lib/types';

	let { tokens, titleId = 'matra' }: { tokens: ParseToken[]; titleId?: string } = $props();

	const root = $derived(tokens.find((t) => t.head === 0));
	const n = $derived(tokens.length);
	const byId = $derived(new Map(tokens.map((t) => [t.id, t])));
	/** The sentence as written: a space before each word, none before punctuation. */
	const sentence = $derived(tokens.map((t, i) => (i > 0 && drawn(t) ? ' ' : '') + t.text).join(''));
	/** Where the tallest letter's top sits in a word's line box, in em. */
	const top = $derived(
		inkTop(ALEGREYA_BLACK, tallest(ALEGREYA_BLACK, tokens.map((t) => t.text).join(''))).toFixed(4)
	);

	/**
	 * Arcs, levelled as the mark levels them. As in the mark, punctuation is
	 * not drawn: the full stop stays in the sentence, as written, but no arc
	 * hangs to it.
	 */
	const arcs = $derived.by(() => {
		const ids = new Set(tokens.filter(drawn).map((t) => t.id));
		const list = tokens
			.filter((t) => t.head !== 0 && ids.has(t.id) && ids.has(t.head))
			.map((t) => ({ head: t.head, dep: t.id, rel: t.dep }))
			.sort(
				(p, q) =>
					Math.abs(p.head - p.dep) - Math.abs(q.head - q.dep) || Math.min(p.head, p.dep) - Math.min(q.head, q.dep)
			);
		const placed: { lo: number; hi: number; level: number }[] = [];
		return list.map((a) => {
			const lo = Math.min(a.head, a.dep);
			const hi = Math.max(a.head, a.dep);
			let level = 1;
			for (const p of placed) if (p.lo < hi && lo < p.hi) level = Math.max(level, p.level + 1);
			placed.push({ lo, hi, level });
			// Grid lines: word i spans columns 2i-1 and 2i, so its centre is line
			// 2i and its right edge line 2i+1.
			const anchor = (id: number) => (id === root?.id ? 2 * id + 1 : 2 * id);
			const [from, to] = [anchor(a.head), anchor(a.dep)].sort((x, y) => x - y);
			return { ...a, level, from, to };
		});
	});
	const deepest = $derived(Math.max(1, ...arcs.map((a) => a.level)));

	let hover = $state<number | null>(null);
	let pinned = $state<number | null>(null);
	const active = $derived(hover ?? pinned);
	/** The active word, its head and its dependents. */
	const lit = $derived.by(() => {
		if (active === null) return null;
		const t = byId.get(active);
		const set = new Set<number>([active]);
		if (t && t.head !== 0) set.add(t.head);
		for (const d of tokens) if (d.head === active && drawn(d)) set.add(d.id);
		return set;
	});
	const arcLit = (a: { head: number; dep: number }) => active !== null && (a.head === active || a.dep === active);
</script>

<section class="hero" aria-labelledby={titleId}>
	<!-- The page's title keeps the id the home page's title always had. -->
	<h1 id={titleId} class="visually-hidden">matra</h1>
	<p class="kicker">matra's parse of this sentence</p>

	<div
		class="specimen"
		class:dimmed={lit !== null}
		style="--n: {n}; --deep: {deepest}; --ink-top: {top}em"
		role="group"
		aria-label="{sentence} (with matra's dependency parse drawn over it)"
	>
		<div class="bar" aria-hidden="true"></div>
		{#each tokens as t (t.id)}
			<button
				type="button"
				class="word"
				class:root={t.head === 0}
				class:punct={!drawn(t)}
				class:lit={lit?.has(t.id)}
				style="grid-column: {2 * t.id - 1} / span 2"
				aria-pressed={pinned === t.id}
				aria-label="{t.text}: {t.pos}, {t.head === 0 ? 'the root' : `${t.dep} of ${byId.get(t.head)?.text}`}"
				onpointerenter={() => (hover = t.id)}
				onpointerleave={() => (hover = null)}
				onfocus={() => (hover = t.id)}
				onblur={() => (hover = null)}
				onclick={() => (pinned = pinned === t.id ? null : t.id)}
			>
				<span class="w">{t.text}</span>
				<span class="pos" aria-hidden="true">{t.pos}</span>
			</button>
		{/each}
		{#each arcs as a (`${a.head}-${a.dep}`)}
			<div
				class="arc"
				class:from-root={a.head === root?.id}
				class:lit={arcLit(a)}
				style="grid-column: {a.from} / {a.to}; --level: {a.level}"
				aria-hidden="true"
			>
				<span class="rel">{a.rel}</span>
			</div>
		{/each}
	</div>

	<details class="show-parse">
		<summary>show the parse</summary>
		<table>
			<thead><tr><th>#</th><th>Word</th><th>POS</th><th>Head</th><th>Relation</th></tr></thead>
			<tbody>
				{#each tokens as t (t.id)}
					<tr>
						<td>{t.id}</td>
						<td>{t.text}</td>
						<td>{t.pos}</td>
						<td>{t.head === 0 ? '0 (root)' : `${t.head} ${byId.get(t.head)?.text}`}</td>
						<td>{t.dep}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</details>
</section>

<style>
	.hero {
		container-type: inline-size;
		margin: var(--space-3) 0 var(--space-4);
	}

	.kicker {
		margin: 0 0 var(--space-2);
		font: var(--type-xs) var(--font-mono);
		letter-spacing: var(--tracking-wide);
		color: var(--text-muted);
	}

	/*
	 * The sentence. Alegreya at line-height 1: the tops of its tallest letters
	 * (l, f, d) sit --ink-top below the line box's top, set from the font's
	 * metrics in $lib/fonts. The headline bar's lower edge is drawn there, so
	 * the letters touch it from below and it never crosses them.
	 */
	.specimen {
		/* The sentence is about 11.0em wide at this weight, measured in the
		   browser with each word's padding: sized to its column with a little
		   to spare, never wider, never above the display size. A new sentence
		   needs this measured again. */
		--size: clamp(1rem, 8.8cqi, var(--type-4xl));
		--u: calc(var(--size) * 0.125);
		position: relative;
		display: grid;
		grid-template-columns: repeat(calc(var(--n) * 2), auto);
		/* Space for the labels is kept whether they show or not, so revealing
		   them moves nothing. */
		grid-template-rows: 1em calc(var(--deep) * var(--u) * 2.6 + var(--u) + 1.2rem);
		justify-content: start;
		font-size: var(--size);
		--above: 1.3rem;
		padding-top: var(--above);
		/* As wide as the words, so the bar ends where they do. */
		width: fit-content;
		max-width: 100%;
	}

	.bar {
		position: absolute;
		left: 0;
		right: 0;
		top: calc(var(--above) + var(--ink-top));
		height: 0;
		border-top: max(1.5px, 0.04em) solid var(--text);
		transform: translateY(-100%);
		pointer-events: none;
	}

	.word {
		all: unset;
		position: relative;
		grid-row: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		cursor: pointer;
		font-family: var(--font-read);
		font-weight: var(--weight-black);
		letter-spacing: var(--tracking-title);
		line-height: 1;
		color: var(--text);
		transition: opacity var(--duration-normal) var(--easing-smooth);
	}

	.word:not(.punct) {
		padding-inline: 0.12em;
	}

	.word:focus-visible {
		outline: 2px solid var(--spark);
		outline-offset: 4px;
	}

	.word[aria-pressed='true'] .w {
		text-decoration: underline 2px var(--spark);
		text-underline-offset: 0.12em;
	}

	/* The root's stroke: the vowel sign, hanging from the bar after its word,
	   down to where its arcs leave. */
	.word.root::after {
		content: '';
		position: absolute;
		right: -0.02em;
		top: var(--ink-top);
		height: calc(1em - var(--ink-top) + var(--u) * 0.6);
		border-right: max(2px, 0.055em) solid var(--emergence);
	}

	/* A word's part of speech stands above it, where the vowel signs above a
	   Devanagari headline stand. */
	.pos {
		position: absolute;
		bottom: calc(100% + 0.1rem);
		font: 400 var(--type-xs) / 1 var(--font-mono);
		letter-spacing: var(--tracking-wide);
		color: var(--text-muted);
		opacity: 0;
		transition: opacity var(--duration-normal) var(--easing-smooth);
	}

	/* Each arc hangs from its two anchors: a half ellipse below the words.
	   Positioned in its grid area, not placed in the flow: an arc in the flow
	   adds its own width to the columns it spans, and one spanning a single
	   column (the root's edge to a neighbour's centre) would squeeze that
	   column to its border and leave its word's other half the rest. */
	.arc {
		position: absolute;
		inset-inline: 0;
		top: 0;
		grid-row: 2;
		height: calc(var(--level) * var(--u) * 2.6);
		margin-top: calc(var(--u) * 0.6);
		border: max(1.5px, 0.03em) solid var(--mark);
		border-top: 0;
		border-radius: 0 0 50% 50% / 0 0 100% 100%;
		pointer-events: none;
		transition: opacity var(--duration-normal) var(--easing-smooth);
	}

	.rel {
		position: absolute;
		left: 50%;
		bottom: calc(-0.5em - 2px);
		transform: translateX(-50%);
		padding: 0 0.35em;
		font: 400 var(--type-xs) var(--font-mono);
		letter-spacing: var(--tracking-wide);
		color: var(--text-muted);
		background: var(--bg);
		white-space: nowrap;
		opacity: 0;
		transition: opacity var(--duration-normal) var(--easing-smooth);
	}

	/* Lighting: the active word, its head, its dependents and their arcs stay;
	   everything else dims. Opacity only. */
	.dimmed .word:not(.lit),
	.dimmed .arc:not(.lit) {
		opacity: 0.28;
	}

	.dimmed .arc.lit .rel,
	.dimmed .word.lit .pos {
		opacity: 1;
	}

	/* "Show the parse" reveals every label, with or without a script. */
	.hero:has(.show-parse[open]) :is(.pos, .rel) {
		opacity: 1;
	}

	.show-parse {
		margin-top: var(--space-2);
		font: var(--type-sm) var(--font-mono);
	}

	.show-parse summary {
		display: inline-block;
		cursor: pointer;
		padding: 0.15rem var(--space-1);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		color: var(--text);
		list-style: none;
	}

	.show-parse summary::-webkit-details-marker {
		display: none;
	}

	.show-parse[open] summary {
		border-color: var(--spark);
		color: var(--spark);
	}

	.show-parse table {
		margin-top: var(--space-2);
		border-collapse: collapse;
	}

	.show-parse :is(th, td) {
		padding: 0.35em 1.2em 0.35em 0;
		text-align: start;
		border-bottom: 1px solid var(--border);
	}

	.show-parse th {
		font: 600 var(--type-xs) var(--font-ui);
		letter-spacing: var(--tracking-widest);
		text-transform: uppercase;
		color: var(--text-muted);
	}
</style>
