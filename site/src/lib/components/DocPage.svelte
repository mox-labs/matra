<script lang="ts">
	import { base } from '$app/paths';
	import { SITE_NAME } from '$lib/site';
	import type { Doc } from '$lib/types';
	import Giscus from './Giscus.svelte';
	import Toc from './Toc.svelte';

	type GiscusProps = { repo: string; repoId: string; categoryId: string };

	let {
		doc,
		giscus,
		home = false
	}: { doc: Doc; giscus: GiscusProps | null; home?: boolean } = $props();

	const markdownHref = $derived(`${base}${doc.route}.md`);

	/**
	 * A copy button on each code block. It is added after hydration because it
	 * needs a script to work; without one, the block is still selectable.
	 */
	function copyButtons(html: string) {
		return (node: HTMLElement) => {
			void html;
			for (const pre of node.querySelectorAll('pre')) {
				if (pre.querySelector(':scope > .copy')) continue;
				const button = document.createElement('button');
				button.type = 'button';
				button.className = 'copy';
				button.textContent = 'Copy';
				button.setAttribute('aria-label', 'Copy code to clipboard');
				button.addEventListener('click', async () => {
					const code = pre.querySelector('code')?.innerText ?? '';
					try {
						await navigator.clipboard.writeText(code.replace(/\n$/, ''));
						button.textContent = 'Copied';
						button.dataset.copied = '';
						setTimeout(() => {
							button.textContent = 'Copy';
							delete button.dataset.copied;
						}, 1500);
					} catch {
						button.textContent = 'Copy failed';
					}
				});
				pre.append(button);
			}
		};
	}
</script>

<svelte:head>
	<title>{home ? `${SITE_NAME} documentation` : `${doc.title} · ${SITE_NAME}`}</title>
	<meta name="description" content={doc.description} />
	<link rel="alternate" type="text/markdown" href={markdownHref} title="Markdown source" />
	{#if home}
		<link rel="canonical" href="{base}{doc.route}" />
	{/if}
</svelte:head>

<div class="doc-grid" class:home>
	<article class="doc">
		<div class="prose" data-pagefind-body={home ? undefined : ''}>
			<header class="doc-header">
				{#if doc.part}
					<p class="eyebrow" data-pagefind-ignore>{doc.part}</p>
				{/if}
				<h1 id={doc.titleId || undefined}>{@html doc.titleHtml}</h1>
			</header>

			{#if doc.toc.length > 1}
				<details class="toc-inline" data-print="hide" data-pagefind-ignore>
					<summary>On this page</summary>
					<Toc toc={doc.toc} />
				</details>
			{/if}

			<div class="body" {@attach copyButtons(doc.html)}>
				{@html doc.html}
			</div>
		</div>

		<footer class="doc-footer" data-print="hide">
			<a href={doc.editUrl}>Edit this page</a>
			<a href={markdownHref}>View as Markdown</a>
		</footer>

		{#if doc.prev || doc.next}
			<nav class="pager" aria-label="Previous and next page" data-print="hide">
				{#if doc.prev}
					<a class="prev" href="{base}{doc.prev.route}" rel="prev">
						<span class="dir">Previous</span>
						<span class="title">{doc.prev.title}</span>
					</a>
				{:else}
					<span></span>
				{/if}
				{#if doc.next}
					<a class="next" href="{base}{doc.next.route}" rel="next">
						<span class="dir">Next</span>
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

	{#if doc.toc.length > 1}
		<aside class="toc-rail" aria-label="On this page" data-print="hide">
			<p class="toc-title">On this page</p>
			<Toc toc={doc.toc} />
		</aside>
	{/if}
</div>

<style>
	.doc-grid {
		display: grid;
		grid-template-columns: minmax(0, var(--measure));
		justify-content: center;
		gap: var(--space-5);
		padding: var(--space-5) var(--gutter) var(--space-6);
	}

	@media (min-width: 75rem) {
		.doc-grid:has(.toc-rail) {
			grid-template-columns: minmax(0, var(--measure)) var(--toc-w);
		}
	}

	.doc {
		min-width: 0;
	}

	.doc-header {
		margin-bottom: var(--space-4);
	}

	.eyebrow {
		margin: 0 0 var(--space-2);
		font-size: 0.78rem;
		font-weight: 650;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--accent);
	}

	h1 {
		margin: 0;
		font-size: clamp(1.9rem, 1.4rem + 2vw, 2.5rem);
		line-height: 1.15;
		font-weight: 700;
		letter-spacing: -0.02em;
		scroll-margin-top: calc(var(--header-h) + var(--space-3));
	}

	.home h1 {
		font-size: clamp(2.4rem, 1.6rem + 3vw, 3.4rem);
		letter-spacing: -0.03em;
	}

	.toc-inline {
		margin: 0 0 var(--space-4);
		padding: 0.6rem 0.9rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--bg-raised);
	}

	.toc-inline summary {
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 600;
	}

	.toc-inline[open] summary {
		margin-bottom: var(--space-2);
	}

	@media (min-width: 75rem) {
		.toc-inline {
			display: none;
		}
	}

	.toc-rail {
		display: none;
	}

	@media (min-width: 75rem) {
		.toc-rail {
			display: block;
			position: sticky;
			top: calc(var(--header-h) + var(--space-5));
			align-self: start;
			max-height: calc(100vh - var(--header-h) - var(--space-6));
			overflow-y: auto;
		}
	}

	.toc-title {
		margin: 0 0 var(--space-2);
		font-size: 0.72rem;
		font-weight: 650;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.doc-footer {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2) var(--space-4);
		margin-top: var(--space-5);
		padding-top: var(--space-3);
		border-top: 1px solid var(--border);
		font-size: 0.9rem;
	}

	.doc-footer a {
		color: var(--text-muted);
	}

	.doc-footer a:hover {
		color: var(--accent);
	}

	.pager {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
		margin-top: var(--space-4);
	}

	.pager a {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		padding: 0.8rem 1rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		text-decoration: none;
		color: var(--text);
		background: var(--bg-raised);
	}

	.pager a:hover {
		border-color: var(--accent);
	}

	.pager .next {
		grid-column: 2;
		text-align: end;
	}

	.pager .dir {
		font-size: 0.75rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.pager .title {
		font-weight: 600;
		color: var(--accent);
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
