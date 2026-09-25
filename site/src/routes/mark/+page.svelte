<script lang="ts">
	/**
	 * The mark sheet: every variant, its rules, and the parse it is drawn from.
	 * A page outside SUMMARY.md, reached from the footer; the site README says
	 * why the mark is this and how it is made.
	 */
	import { base } from '$app/paths';
	import Mark from '$lib/components/Mark.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	const { layouts, rules, tokens, generator } = $derived(data);
	const byId = $derived(new Map(tokens.map((t) => [t.id, t])));
</script>

<svelte:head>
	<title>The mark · matra</title>
	<meta name="description" content="matra's mark is matra's parse of Amplify radical nonconformity., drawn." />
</svelte:head>

<article class="sheet">
	<nav class="crumbs" aria-label="Breadcrumb"><a href="{base}/">matra</a> <span aria-hidden="true">/</span> the mark</nav>
	<h1>The mark</h1>
	<p class="lede">
		matra's parse of <q>Amplify radical nonconformity.</q>, drawn. Every token hangs from a headline bar, as Devanagari
		letters hang from theirs. The root hangs longest, in Emergence: a mātrā, the small vowel sign that attaches to a
		letter and changes its sound without replacing it. The arcs below are the dependencies matra found.
	</p>

	<h2>The parse it is drawn from</h2>
	<div class="table-scroll">
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
	</div>
	<p class="note">
		Produced by matra {generator.matra} with {generator.udpipe_model.name}. The data is
		<a href="{base}/figures/motto/parse.json">motto/parse.json</a>; gate 8 regenerates it and fails on any change, so
		the mark cannot drift from what matra says.
	</p>

	<h2>Variants</h2>
	<div class="grid">
		{#each [{ name: 'full', label: 'Full, with the words', file: 'full' }, { name: 'glyph', label: 'Glyph, bar and arcs', file: 'glyph' }] as v (v.name)}
			<figure class="tile void">
				<div class="stage"><Mark layout={layouts[v.name as 'full' | 'glyph']} height={v.name === 'full' ? 90 : 108} /></div>
				<figcaption>{v.label} · <a href="{base}/mark/{v.file}.svg">{v.file}.svg</a></figcaption>
			</figure>
		{/each}
		<figure class="tile void">
			<div class="stage favicons">
				<Mark layout={layouts.favicon} height={16} />
				<Mark layout={layouts.favicon} height={32} />
				<Mark layout={layouts.favicon} height={64} />
			</div>
			<figcaption>Favicon, 16, 32 and 64px · <a href="{base}/favicon.svg">favicon.svg</a></figcaption>
		</figure>
		<figure class="tile void">
			<div class="stage"><Mark layout={layouts.glyph} height={108} mono /></div>
			<figcaption>Mono: one ink, hierarchy by opacity · <a href="{base}/mark/mono.svg">mono.svg</a></figcaption>
		</figure>
		<figure class="tile paper">
			<div class="stage"><Mark layout={layouts.glyph} height={108} /></div>
			<figcaption>Paper · <a href="{base}/mark/paper.svg">paper.svg</a></figcaption>
		</figure>
		<figure class="tile paper">
			<div class="stage"><Mark layout={layouts.full} height={90} /></div>
			<figcaption>Full, on paper · <a href="{base}/mark/full-paper.svg">full-paper.svg</a></figcaption>
		</figure>
	</div>

	<h2>Rules</h2>
	<dl class="rules">
		<dt>Unit</dt>
		<dd>{rules.unit}, the grid's. Every coordinate of the glyph is a multiple of it or of half of it.</dd>
		<dt>Clear space</dt>
		<dd>
			The root stroke's length ({rules.clearSpace} units at full size) on every side. Nothing else enters it.
		</dd>
		<dt>Minimum size</dt>
		<dd>
			The glyph no smaller than {rules.glyphMinHeight}px tall; below that, the favicon form. The full mark no narrower
			than {rules.fullMinWidth}px, where its words reach the 11px floor.
		</dd>
		<dt>Colour</dt>
		<dd>
			Ink, and the root in Emergence: the mark's one vivid moment. In one ink (mono), hierarchy is carried by opacity:
			bar and root whole, arcs at 80%, strokes at 60%.
		</dd>
		<dt>Change</dt>
		<dd>Only by matra. If its parse of the motto changes, the mark changes with it, and review sees it.</dd>
	</dl>
</article>

<style>
	.sheet {
		width: min(var(--column), 100%);
		margin: 0 auto;
		padding: var(--space-4) var(--space-2) var(--space-6);
	}

	.crumbs {
		margin: 0 0 var(--space-1);
		font: var(--type-xs) var(--font-mono);
		color: var(--text-muted);
	}

	.crumbs a {
		color: var(--text-muted);
	}

	h1 {
		margin: 0 0 var(--space-2);
		font-size: var(--type-2xl);
		font-weight: var(--weight-black);
		letter-spacing: var(--tracking-title);
	}

	h2 {
		margin: var(--space-5) 0 var(--space-2);
		font-size: var(--type-xl);
	}

	.lede {
		max-width: var(--measure);
	}

	.table-scroll {
		overflow-x: auto;
	}

	table {
		border-collapse: collapse;
		font: var(--type-sm) var(--font-mono);
	}

	th,
	td {
		padding: 0.4em 1.2em 0.4em 0;
		text-align: start;
		border-bottom: 1px solid var(--border);
	}

	th {
		font: 600 var(--type-xs) var(--font-ui);
		letter-spacing: var(--tracking-widest);
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.note {
		max-width: var(--measure);
		font: var(--type-xs) / 1.6 var(--font-mono);
		color: var(--text-muted);
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 18rem), 1fr));
		gap: var(--space-2);
	}

	/* A column, so a tile shorter than its row grows its stage, not a strip
	   of its ground under the caption. */
	.tile {
		display: flex;
		flex-direction: column;
		margin: 0;
		border: 1px solid var(--border);
	}

	/* Each tile shows the mark on its own ground, whatever the page's theme. */
	.tile.void {
		color-scheme: dark;
		background: var(--depth-void);
		color: var(--dao-text);
	}

	.tile.paper {
		color-scheme: light;
		background: var(--paper);
		color: var(--ink);
	}

	.stage {
		flex: 1;
		display: flex;
		align-items: center;
		/* `safe`: a mark wider than its tile starts at the left edge and
		   scrolls, rather than being centred past an edge no scroll reaches. */
		justify-content: safe center;
		gap: var(--space-3);
		min-height: 11rem;
		padding: var(--space-4) var(--space-2);
		overflow-x: auto;
	}

	/* A mark scales down to its tile, keeping its proportions; the rules
	   above say how small each form may go, and the tile is wider than that
	   at 360px. */
	.stage :global(svg) {
		max-width: 100%;
		height: auto;
	}

	.tile figcaption {
		padding: var(--space-1) var(--space-2);
		border-top: 1px solid var(--border);
		font: var(--type-xs) var(--font-mono);
		color: var(--text-muted);
		background: var(--bg);
	}

	.rules {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: var(--space-1) var(--space-3);
		max-width: var(--measure);
	}

	.rules dt {
		font: 600 var(--type-sm) var(--font-mono);
	}

	.rules dd {
		margin: 0;
	}

	@media (max-width: 30rem) {
		.rules {
			grid-template-columns: 1fr;
		}

		.rules dd {
			margin-bottom: var(--space-1);
		}

		/* The parse table fits a phone's column whole, so no column hides
		   behind a scroll the reader has no cue for. */
		table {
			font-size: var(--type-xs);
		}

		th,
		td {
			padding-inline-end: 0.6em;
		}
	}
</style>
