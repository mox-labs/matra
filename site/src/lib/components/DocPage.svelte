<script lang="ts">
	import type { Snippet } from 'svelte';
	import { base } from '$app/paths';
	import { SITE_NAME } from '$lib/site';
	import type { Doc, Segment } from '$lib/types';
	import Body from './Body.svelte';
	import Giscus from './Giscus.svelte';
	import Toc from './Toc.svelte';

	type GiscusProps = { repo: string; repoId: string; categoryId: string };

	let {
		doc,
		giscus,
		home = false,
		hero
	}: { doc: Doc; giscus: GiscusProps | null; home?: boolean; hero?: Snippet } = $props();

	const markdownHref = $derived(`${base}${doc.route}.md`);

	/**
	 * A copy button on each code block. It is added after hydration because it
	 * needs a script to work; without one, the block is still selectable.
	 */
	function copyButtons(segments: Segment[]) {
		return (node: HTMLElement) => {
			void segments;
			for (const pre of node.querySelectorAll('pre')) {
				if (pre.querySelector(':scope > .copy')) continue;
				const button = document.createElement('button');
				button.type = 'button';
				button.className = 'copy';
				button.textContent = 'copy';
				button.setAttribute('aria-label', 'Copy code to clipboard');
				button.addEventListener('click', async () => {
					const code = pre.querySelector('code')?.innerText ?? '';
					try {
						await navigator.clipboard.writeText(code.replace(/\n$/, ''));
						button.textContent = 'copied';
						button.dataset.copied = '';
						setTimeout(() => {
							button.textContent = 'copy';
							delete button.dataset.copied;
						}, 1500);
					} catch {
						button.textContent = 'copy failed';
					}
				});
				pre.append(button);
			}
		};
	}

	const m = $derived(doc.measured);
</script>

<svelte:head>
	<title>{home ? `${SITE_NAME}: text in, structure out` : `${doc.title} · ${SITE_NAME}`}</title>
	<meta name="description" content={doc.description} />
	<link rel="alternate" type="text/markdown" href={markdownHref} title="Markdown source" />
	{#if home}
		<link rel="canonical" href="{base}{doc.route}" />
	{/if}
</svelte:head>

<article class="doc" class:home>
	<div class="prose" data-pagefind-body={home ? undefined : ''}>
		{#if hero}
			{@render hero()}
		{:else}
			<header class="doc-header">
				{#if doc.crumbs.length}
					<nav class="crumbs" aria-label="Breadcrumb" data-pagefind-ignore>
						<a href="{base}/">{SITE_NAME}</a>
						{#each doc.crumbs as crumb, i (i)}
							<span class="sep" aria-hidden="true">/</span>
							{#if crumb.route}
								<a href="{base}{crumb.route}">{crumb.title}</a>
							{:else}
								<span>{crumb.title}</span>
							{/if}
						{/each}
					</nav>
				{/if}
				<h1 id={doc.titleId || undefined}>{@html doc.titleHtml}</h1>
			</header>
		{/if}

		{#if doc.toc.length > 1}
			<details class="toc-inline" data-print="hide" data-pagefind-ignore>
				<summary>on this page</summary>
				<Toc toc={doc.toc} />
			</details>
		{/if}

		<div class="body" {@attach copyButtons(doc.segments)}>
			<Body segments={doc.segments} />
		</div>
	</div>

	<!-- Where the margin's numbers come from: at the end, as provenance is,
	     so it never stands between the title and the reading. -->
	{#if m}
		<p class="measured" data-pagefind-ignore data-print="hide">
			{#if m.mapped}
				<span>This page, measured by matra {m.generator.matra} with {m.generator.udpipe_model.name}:
					{m.measured} {m.measured === 1 ? 'paragraph' : 'paragraphs'}, in the margin.</span>
				<a href={m.dataUrl}>the data</a>
			{:else}
				<span>matra measured this page ({m.generator.matra}), but could not match its paragraphs to the page
					reliably, so the margin shows nothing rather than a number beside the wrong paragraph.</span>
				<a href={m.dataUrl}>the data</a>
			{/if}
		</p>
	{/if}

	<footer class="doc-footer" data-print="hide">
		<a href={doc.editUrl}>edit this page</a>
		<a href={markdownHref}>view as markdown</a>
	</footer>

	{#if doc.prev || doc.next}
		<nav class="pager" aria-label="Previous and next page" data-print="hide">
			{#if doc.prev}
				<a class="prev" href="{base}{doc.prev.route}" rel="prev">
					<span class="dir">previous</span>
					<span class="title">{doc.prev.title}</span>
				</a>
			{:else}
				<span></span>
			{/if}
			{#if doc.next}
				<a class="next" href="{base}{doc.next.route}" rel="next">
					<span class="dir">next</span>
					<span class="title">{doc.next.title}</span>
				</a>
			{/if}
		</nav>
	{/if}

	{#if giscus}
		{#key doc.route}
			<Giscus {...giscus} />
		{/key}
	{/if}
</article>

<style>
	/* One column: the reading measure plus the margin, the prose held to the
	   measure and the wide things (figures, code, tables) given all of it. */
	.doc {
		width: min(var(--column), 100%);
		margin: 0 auto;
		padding: var(--space-4) var(--space-2) var(--space-6);
		min-width: 0;
	}

	@media (max-width: 76.99rem) {
		.doc {
			width: min(calc(var(--measure) + 2 * var(--space-2)), 100%);
		}
	}

	.doc-header {
		max-width: var(--measure);
		margin-bottom: var(--space-2);
	}

	/* The home page: the line under the motto leads; the three ways on are
	   square cards, the whole card the link's target. */
	.home :global(.body > p:first-of-type) {
		font-size: var(--type-lg);
		line-height: var(--leading-snug);
	}

	.home :global(h2#where-to-go-next + ul) {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 15rem), 1fr));
		gap: var(--space-2);
		max-width: none;
		padding: 0;
		list-style: none;
	}

	.home :global(h2#where-to-go-next + ul > li) {
		position: relative;
		margin: 0;
		padding: var(--space-2);
		border: 1px solid var(--border);
		border-top: var(--border-accent) solid var(--border-strong);
		font-size: var(--type-sm);
		line-height: var(--leading-normal);
		color: var(--text-muted);
	}

	/* Each card carries the accent of the part it leads to, the same roles
	   the sidebar's parts carry ($lib/site PART_ROLES): installing is the
	   reader's own hands (Spark), the examples are matra's output
	   (Emergence), the concepts are neutral. */
	.home :global(h2#where-to-go-next + ul > li:has(a[href*='/tutorials/'])) {
		border-top-color: var(--spark);
	}

	.home :global(h2#where-to-go-next + ul > li:has(a[href*='/examples/'])) {
		border-top-color: var(--emergence);
	}

	.home :global(h2#where-to-go-next + ul > li:hover) {
		border-color: var(--border-strong);
	}

	/* The card's title is the link; the colon that ends it in the Markdown
	   (and for a screen reader) is set at zero size, since a title needs none. */
	.home :global(h2#where-to-go-next + ul > li > strong) {
		display: block;
		margin-bottom: var(--space-0-5);
		font-family: var(--font-mono);
		font-size: 0;
	}

	.home :global(h2#where-to-go-next + ul > li > strong a) {
		font-size: var(--type-base);
	}

	.home :global(h2#where-to-go-next + ul > li > strong a) {
		text-decoration: none;
	}

	.home :global(h2#where-to-go-next + ul > li > strong a::after) {
		content: '';
		position: absolute;
		inset: 0;
	}

	.home :global(h2#where-to-go-next + ul > li > strong a:focus-visible) {
		outline: none;
	}

	.home :global(h2#where-to-go-next + ul > li:has(a:focus-visible)) {
		outline: 2px solid var(--spark);
		outline-offset: 2px;
	}

	.crumbs {
		display: flex;
		flex-wrap: wrap;
		gap: 0 var(--space-1);
		margin: 0 0 var(--space-1);
		font: var(--type-xs) / 1.6 var(--font-mono);
		letter-spacing: var(--tracking-wide);
		color: var(--text-muted);
	}

	.crumbs a {
		color: var(--text-muted);
		text-decoration: none;
	}

	.crumbs a:hover {
		color: var(--spark);
	}

	.crumbs .sep {
		color: var(--border-strong);
	}

	h1 {
		margin: 0;
		font-family: var(--font-display);
		font-size: clamp(var(--type-xl), 1.2rem + 2.6vw, var(--type-2xl));
		line-height: var(--leading-tight);
		font-weight: var(--weight-black);
		letter-spacing: var(--tracking-title);
		scroll-margin-top: calc(var(--header-h) + var(--space-3));
	}

	/* One line where the column allows: it may use the margin's width too. */
	.measured {
		clear: both;
		margin: var(--space-5) 0 0;
		font: var(--type-xs) / 1.6 var(--font-mono);
		color: var(--text-muted);
	}

	.measured a {
		margin-inline-start: var(--space-1);
		color: var(--text-muted);
		white-space: nowrap;
	}

	.measured a:hover {
		color: var(--spark);
	}

	.measured + .doc-footer {
		margin-top: var(--space-1);
	}

	.toc-inline {
		max-width: var(--measure);
		margin: 0 0 var(--space-3);
		font-family: var(--font-mono);
		font-size: var(--type-sm);
	}

	.toc-inline summary {
		cursor: pointer;
		color: var(--text-muted);
	}

	.toc-inline[open] summary {
		margin-bottom: var(--space-1);
	}

	/* On a desktop the page's sections are in the sidebar, under the page. */
	@media (min-width: 54.01rem) {
		.toc-inline {
			display: none;
		}
	}

	.doc-footer {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1) var(--space-3);
		max-width: var(--measure);
		margin-top: var(--space-5);
		padding-top: var(--space-2);
		border-top: 1px solid var(--border);
		font: var(--type-sm) var(--font-mono);
		clear: both;
	}

	.doc-footer a {
		color: var(--text-muted);
	}

	.doc-footer a:hover {
		color: var(--spark);
	}

	.pager {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-2);
		max-width: var(--measure);
		margin-top: var(--space-3);
	}

	.pager a {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		padding: var(--space-1) var(--space-2);
		border: 1px solid var(--border);
		text-decoration: none;
		color: var(--text);
	}

	.pager a:hover {
		border-color: var(--spark);
	}

	.pager .next {
		grid-column: 2;
		text-align: end;
	}

	.pager .dir {
		font: var(--type-xs) var(--font-mono);
		letter-spacing: var(--tracking-wide);
		color: var(--text-muted);
	}

	.pager .title {
		font-weight: var(--weight-bold);
		color: var(--spark);
	}

	@media (max-width: 30rem) {
		.pager {
			grid-template-columns: 1fr;
		}

		.pager .next {
			grid-column: 1;
		}
	}
</style>
