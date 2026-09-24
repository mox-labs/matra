<script lang="ts">
	/**
	 * The structural primitives of one input's sentences (EP-0012, M4).
	 *
	 * Each primitive is a field matra fills on `Sentence` by reading its
	 * dependency tree. The table lists them, one row per value; below it each
	 * sentence is set with the words they were read from tinted and the words
	 * they attach to underlined, labelled above in the primitive's colour. The
	 * figure is static: nothing moves.
	 */
	import type { PrimitiveSentence, PrimitivesFigureFile } from '$lib/types';
	import FigureFrame from './FigureFrame.svelte';

	let { id, file, dataUrl }: { id: string; file: PrimitivesFigureFile; dataUrl: string } = $props();

	type Kind = 'negation' | 'modal' | 'reporting' | 'root_adverbial' | 'hearst_pair' | 'bare_assertion';
	const LABEL: Record<Kind, string> = {
		negation: 'negation',
		modal: 'modal',
		reporting: 'reporting',
		root_adverbial: 'root adverbial',
		hearst_pair: 'Hearst pair',
		bare_assertion: 'bare assertion'
	};
	const FIELD: Record<Kind, string> = {
		negation: 'negations',
		modal: 'modals',
		reporting: 'reportings',
		root_adverbial: 'root_adverbials',
		hearst_pair: 'hearst_pairs',
		bare_assertion: 'bare_assertion'
	};

	/** One primitive value: the word it is read from, and the word it attaches to. */
	interface Item {
		kind: Kind;
		cue: number;
		head: number | null;
		detail: string;
	}

	function items(s: PrimitiveSentence): Item[] {
		const word = (i: number | null) => (i === null ? '' : (s.tokens.find((t) => t.id === i)?.text ?? ''));
		const out: Item[] = [];
		for (const n of s.negations) out.push({ kind: 'negation', cue: n.cue_id, head: n.head_id, detail: '' });
		for (const m of s.modals) out.push({ kind: 'modal', cue: m.aux_id, head: m.head_id, detail: '' });
		for (const r of s.reportings)
			out.push({
				kind: 'reporting',
				cue: r.verb_id,
				head: r.ccomp_id,
				detail: r.subject_id === null ? 'no subject' : `source ${r.subject_id} ${word(r.subject_id)}`
			});
		for (const a of s.root_adverbials) out.push({ kind: 'root_adverbial', cue: a.adv_id, head: null, detail: '' });
		for (const h of s.hearst_pairs)
			out.push({
				kind: 'hearst_pair',
				cue: h.hyponym.head_id,
				head: h.hypernym.head_id,
				detail: `${h.pattern}, hyponym span ${h.hyponym.first_id} to ${h.hyponym.last_id}`
			});
		if (s.bare_assertion && s.root_id !== null)
			out.push({ kind: 'bare_assertion', cue: s.root_id, head: null, detail: 'root clause, no modal' });
		return out;
	}

	const sentences = $derived(
		file.data.sentences.map((s, i) => {
			const list = items(s);
			// Roles each word plays: read from (cue) or attached to (target).
			const roles = new Map<number, { kind: Kind; cue: boolean; label: string }[]>();
			// A word playing the same role twice (one hypernym for two hyponyms)
			// is labelled once.
			const add = (tok: number, kind: Kind, cue: boolean, label: string) => {
				const had = roles.get(tok) ?? [];
				if (!had.some((r) => r.kind === kind && r.label === label)) roles.set(tok, [...had, { kind, cue, label }]);
			};
			for (const it of list) {
				if (it.kind === 'bare_assertion') continue;
				const [c, t] =
					it.kind === 'hearst_pair' ? ['hyponym', 'hypernym'] : it.kind === 'reporting' ? ['reporting', 'reported'] : [LABEL[it.kind], it.kind === 'negation' ? 'negated' : 'governs'];
				add(it.cue, it.kind, true, c);
				if (it.head !== null) add(it.head, it.kind, false, t);
			}
			for (const r of s.reportings) if (r.subject_id !== null) add(r.subject_id, 'reporting', false, 'source');
			return { n: i + 1, s, list, roles, spans: spans(s) };
		})
	);

	const kinds = $derived(
		(Object.keys(LABEL) as Kind[]).filter((k) => sentences.some((x) => x.list.some((it) => it.kind === k)))
	);

	/** The sentence as written, split into the gaps between tokens and the tokens themselves. */
	function spans(s: PrimitiveSentence) {
		const out: { gap: string; id: number; text: string }[] = [];
		let pos = 0;
		for (const t of s.tokens) {
			const at = s.text.indexOf(t.text, pos);
			if (at === -1) {
				out.push({ gap: out.length ? ' ' : '', id: t.id, text: t.text });
				continue;
			}
			out.push({ gap: s.text.slice(pos, at), id: t.id, text: t.text });
			pos = at + t.text.length;
		}
		return { parts: out, tail: s.text.slice(pos) };
	}

	const word = (s: PrimitiveSentence, i: number | null) =>
		i === null ? '' : (s.tokens.find((t) => t.id === i)?.text ?? '');
	const prim = (list: Item[], tok: number) =>
		list
			.filter((it) => it.cue === tok && it.kind !== 'bare_assertion')
			.map((it) => `${it.kind} ${it.cue} ${it.head ?? '-'}`)
			.join('|') || undefined;
</script>

<FigureFrame {id} kind="primitives" title="Structural primitives" {file} {dataUrl}>
	<!-- A region that scrolls must take focus, or it cannot be scrolled from
	     the keyboard (WCAG 2.1.1). -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-twin" tabindex="0" role="region" aria-label="Structural primitives, by sentence">
		<table data-twin-for={id}>
			<thead>
				<tr><th>#</th><th>Primitive</th><th>Read from</th><th>Attaches to</th><th>Field and detail</th></tr>
			</thead>
			<tbody>
				{#each sentences as x (x.n)}
					{#each x.list as it, j (j)}
						<tr class:first={j === 0}>
							<td class="num">{x.n}</td>
							<td><span class="swatch k-{it.kind}" aria-hidden="true"></span>{LABEL[it.kind]}</td>
							<td>{it.cue}<span class="w">{word(x.s, it.cue)}</span></td>
							<td>{#if it.head === null}<span class="none">none</span>{:else}{it.head}<span class="w">{word(x.s, it.head)}</span>{/if}</td>
							<td class="detail"><code>{FIELD[it.kind]}</code>{it.detail ? `, ${it.detail}` : ''}</td>
						</tr>
					{:else}
						<tr class="first">
							<td class="num">{x.n}</td>
							<td colspan="4" class="none">no primitive</td>
						</tr>
					{/each}
				{/each}
			</tbody>
		</table>
	</div>

	<ol class="sentences" data-marks-for={id}>
		{#each sentences as x (x.n)}
			<li data-sentence={x.n}>
				<span class="n">{x.n}</span>
				<p class="line">
					{#each x.spans.parts as p (p.id)}{p.gap}{#if x.roles.has(p.id)}{@const r = x.roles.get(p.id) ?? []}<ruby
								class="k-{r[0].kind}"
								><span
									class="tok"
									class:cue={r.some((q) => q.cue)}
									class:target={r.every((q) => !q.cue)}
									data-id={p.id}
									data-prim={prim(x.list, p.id)}>{p.text}</span
								><rt>{r.map((q) => q.label).join(' · ')}</rt></ruby
							>{:else}<span class="tok" data-id={p.id}>{p.text}</span>{/if}{/each}{x.spans.tail}
				</p>
				{#if x.s.bare_assertion && x.s.root_id !== null}
					<span class="badge k-bare_assertion" data-prim="bare_assertion {x.s.root_id} -">bare assertion</span>
				{/if}
			</li>
		{/each}
	</ol>

	{#snippet legend()}
		<ul class="fig-legend" aria-label="Primitive colours">
			{#each kinds as k (k)}
				<li><span class="swatch k-{k}" aria-hidden="true"></span>{LABEL[k]}</li>
			{/each}
		</ul>
	{/snippet}

	{#snippet note()}
		Each primitive is a field matra fills on <code>Sentence</code> from its dependency tree: the tinted
		word is the one it is read from, the underlined word the one it attaches to. A bare assertion is a
		flag on the whole sentence: a finite indicative root clause with no modal over it. What any of it
		means for a text is the reader's call; matra reports the construction.
	{/snippet}
</FigureFrame>

<style>
	:global(.mx-figure[data-figure='primitives']) {
		--k-negation: light-dark(#b3401e, #f0915f);
		--k-modal: light-dark(#2b6a8c, #7fb9d8);
		--k-reporting: light-dark(#2f7a4f, #7cc79a);
		--k-root_adverbial: light-dark(#6d4f9e, #b69ae0);
		--k-hearst_pair: light-dark(#8a5a00, #e0b04a);
		--k-bare_assertion: light-dark(#5b564e, #a6a29a);
	}

	.k-negation {
		--k: var(--k-negation);
	}
	.k-modal {
		--k: var(--k-modal);
	}
	.k-reporting {
		--k: var(--k-reporting);
	}
	.k-root_adverbial {
		--k: var(--k-root_adverbial);
	}
	.k-hearst_pair {
		--k: var(--k-hearst_pair);
	}
	.k-bare_assertion {
		--k: var(--k-bare_assertion);
	}

	.swatch {
		display: inline-block;
		width: 0.6em;
		height: 0.6em;
		margin-right: 0.45em;
		border-radius: 2px;
		vertical-align: 0.05em;
		background: var(--k);
	}

	tr.first td {
		border-top: 1px solid var(--border-strong);
	}

	.w {
		margin-left: 0.5em;
		color: var(--text);
	}

	.none {
		color: var(--text-muted);
	}

	td.detail {
		color: var(--text-muted);
		white-space: normal;
		min-width: 12rem;
	}

	.sentences {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.sentences li {
		display: grid;
		grid-template-columns: 1.6em minmax(0, 1fr);
		align-items: baseline;
		gap: 0 0.4em;
		margin: 0;
		padding: 0.35rem 0 0.5rem;
		border-bottom: 1px solid var(--border);
	}

	.sentences li:last-child {
		border-bottom: 0;
	}

	.n {
		font: 600 0.8rem var(--font-mono);
		color: var(--text-muted);
	}

	.line {
		margin: 0;
		font-size: 1.02rem;
		/* Room above each line for the labels. */
		line-height: 2.7;
	}

	ruby {
		ruby-position: over;
		ruby-align: center;
		color: var(--k);
	}

	rt {
		font: 600 0.62rem var(--font-sans);
		letter-spacing: 0.03em;
		color: var(--k);
	}

	.tok.cue {
		padding: 0.1em 0.2em;
		border-radius: 4px;
		background: color-mix(in srgb, var(--k) 16%, transparent);
		color: var(--text);
		font-weight: 600;
	}

	.tok.target {
		color: var(--text);
		text-decoration: underline 2px var(--k);
		text-underline-offset: 0.25em;
	}

	.badge {
		grid-column: 2;
		justify-self: start;
		margin-top: 0.1rem;
		padding: 0.05rem 0.5rem;
		font-size: 0.72rem;
		font-weight: 600;
		letter-spacing: 0.03em;
		color: var(--k);
		border: 1px solid color-mix(in srgb, var(--k) 45%, transparent);
		border-radius: 999px;
	}
</style>
