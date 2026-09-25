<script lang="ts">
	/**
	 * The frame every page hangs from.
	 *
	 * The header is a shirorekha: one hairline rule across the page, with the
	 * mark and the wordmark hanging from it as Devanagari letters hang from
	 * their headline (the wordmark's x-height touches the rule; its `t` rises
	 * above it). The controls hang from the same rule. The footer's rule echoes
	 * it, and the maker's mark hangs from its end.
	 */
	import '../app.css';
	import { afterNavigate } from '$app/navigation';
	import { base } from '$app/paths';
	import { page } from '$app/state';
	import Icon from '$lib/components/Icon.svelte';
	import MakersMark from '$lib/components/MakersMark.svelte';
	import Mark from '$lib/components/Mark.svelte';
	import NavTree from '$lib/components/NavTree.svelte';
	import Search from '$lib/components/Search.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import { DISCUSSIONS_URL, ISSUES_URL, REPO_URL, SITE_NAME } from '$lib/site';
	import type { Doc } from '$lib/types';
	import type { LayoutProps } from './$types';

	let { data, children }: LayoutProps = $props();

	let navOpen = $state(false);
	afterNavigate(() => (navOpen = false));

	/** The route of the page being shown, without the base path; `/` is the first page. */
	const current = $derived.by(() => {
		const path = page.url.pathname.slice(base.length).replace(/\.html$/, '');
		return path === '' || path === '/' ? (data.nav[0]?.items[0]?.route ?? null) : path;
	});

	const toc = $derived(((page.data as { doc?: Doc }).doc?.toc ?? []).filter((t) => t.depth === 2));

	const links = [
		{ href: `${base}/api/`, label: 'api' },
		{ href: DISCUSSIONS_URL, label: 'discussions' },
		{ href: ISSUES_URL, label: 'issues' }
	];
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (navOpen = false)} />

<a class="skip" href="#main">Skip to content</a>

<header class="site-header" data-print="hide">
	<div class="rekha" aria-hidden="true"></div>
	<button
		type="button"
		class="icon-button menu-button"
		aria-expanded={navOpen}
		aria-controls="site-nav"
		aria-label={navOpen ? 'Close navigation' : 'Open navigation'}
		onclick={() => (navOpen = !navOpen)}
	>
		<Icon name={navOpen ? 'close' : 'menu'} />
	</button>

	<a class="lockup" href="{base}/" aria-label="{SITE_NAME}, home">
		<!-- The header's rule is the mark's bar: the glyph hangs from it. -->
		<span class="glyph"><Mark layout={data.glyph} height={36} decorative bar={false} /></span>
		<span class="wordmark" aria-hidden="true">{SITE_NAME}</span>
	</a>

	<div class="search-slot"><Search /></div>

	<nav class="header-links" aria-label="Project">
		{#each links as link (link.href)}
			<a href={link.href}>{link.label}</a>
		{/each}
		<a class="icon-button" href={REPO_URL} aria-label="matra on GitHub" title="matra on GitHub">
			<Icon name="github" />
		</a>
	</nav>

	<ThemeToggle />
</header>

<div class="shell">
	<aside class="sidebar" class:open={navOpen} id="site-nav" data-print="hide">
		<nav aria-label="Documentation">
			<NavTree nav={data.nav} {current} {toc} />
		</nav>
		<nav class="sidebar-links" aria-label="Project links">
			{#each links as link (link.href)}
				<a href={link.href}>{link.label}</a>
			{/each}
			<a href={REPO_URL}>github</a>
		</nav>
	</aside>

	<main id="main" tabindex="-1">
		{@render children()}
	</main>
</div>

<footer class="site-footer" data-print="hide">
	<div class="footer-rekha">
		<span class="carved"><MakersMark /></span>
	</div>
	<nav aria-label="Footer">
		<a href={REPO_URL}>github</a>
		<a href={ISSUES_URL}>issues</a>
		<a href={DISCUSSIONS_URL}>discussions</a>
		<a href="{base}/api/">api reference</a>
		<a href="{base}/toc">all pages</a>
		<a href="{base}/print">single page</a>
		<a href="{base}/llms.txt">llms.txt</a>
		<a href="{base}/mark">the mark</a>
	</nav>
	<p>matra is MIT licensed. Type: Alegreya and IBM Plex, both SIL OFL 1.1.</p>
</footer>

<style>
	.skip {
		position: absolute;
		left: var(--space-2);
		top: -3rem;
		z-index: var(--z-toast);
		padding: var(--space-1) var(--space-2);
		font: var(--type-sm) var(--font-mono);
		background: var(--bg-raised);
		border: 1px solid var(--border-strong);
	}

	.skip:focus {
		top: var(--space-1);
	}

	/* The header: 54px (6U). The rule is at 18px (2U); everything hangs from
	   it. The band above the rule is where the wordmark's `t` and the glyph's
	   top stand, as marks above a headline do. */
	.site-header {
		--rule: calc(2 * var(--space-1));
		position: sticky;
		top: 0;
		z-index: var(--z-sticky);
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		height: var(--header-h);
		padding: 0 var(--space-2);
		background: var(--bg);
	}

	/* One bar, so no second rule marks the header's lower edge: the page
	   fades into it over a unit instead, and scrolled text never looks cut. */
	.site-header::after {
		content: '';
		position: absolute;
		inset: 100% 0 auto;
		height: var(--space-1);
		background: linear-gradient(var(--bg), transparent);
		pointer-events: none;
	}

	.rekha {
		position: absolute;
		inset: calc(var(--rule) - 0.5px) 0 auto;
		height: 1px;
		background: var(--border-strong);
		pointer-events: none;
	}

	.lockup {
		position: relative;
		display: flex;
		align-items: flex-start;
		gap: 0;
		color: var(--text);
		text-decoration: none;
	}

	/* The glyph is drawn 72 units tall with its bar 9 units down; at 36px its
	   bar would be 4.5px from its top, so it is set 13.5px down and its
	   strokes start on the page's rule, which stands in for its bar. */
	.glyph {
		margin-top: calc(var(--rule) - 4.5px);
	}

	/* IBM Plex Mono at 18px, line-height 1: its x-height top is 6.46px below
	   the line box's top (ascent 1.025em, descent 0.275em, x-height 0.516em),
	   so the box starts 11.54px down and the letters hang from the rule. */
	.wordmark {
		margin-top: calc(var(--rule) - 6.46px);
		margin-inline-start: 0.1em;
		font: 600 18px / 1 var(--font-mono);
		letter-spacing: 0.02em;
	}

	.search-slot {
		margin-inline-start: auto;
		margin-top: var(--rule);
	}

	.header-links {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-top: var(--rule);
		height: calc(var(--header-h) - var(--rule));
		font: 400 var(--type-sm) var(--font-mono);
	}

	.header-links a:not(.icon-button) {
		color: var(--text-muted);
		text-decoration: none;
	}

	.header-links a:hover {
		color: var(--spark);
	}

	.site-header :global(.theme-toggle) {
		margin-top: var(--rule);
	}

	:global(.icon-button) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		padding: 0;
		color: var(--text-muted);
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		cursor: pointer;
	}

	:global(.icon-button:hover) {
		color: var(--spark);
		border-color: var(--border);
	}

	.menu-button {
		display: none;
	}

	.shell {
		display: grid;
		grid-template-columns: var(--sidebar-w) minmax(0, 1fr);
		min-height: calc(100vh - var(--header-h));
	}

	.sidebar {
		position: sticky;
		top: var(--header-h);
		align-self: start;
		height: calc(100vh - var(--header-h));
		overflow-y: auto;
		padding: var(--space-3) var(--space-2) var(--space-5);
		border-inline-end: 1px solid var(--border);
	}

	.sidebar-links {
		display: none;
	}

	main {
		min-width: 0;
		outline: none;
	}

	.site-footer {
		padding: 0 var(--space-2) var(--space-5);
		font: var(--type-sm) var(--font-mono);
		color: var(--text-muted);
	}

	/* The footer's rule echoes the header's, and the maker's mark hangs from
	   its end by its upper arm. */
	.footer-rekha {
		position: relative;
		height: calc(3 * var(--space-1) + var(--space-2));
		border-top: 1px solid var(--border-strong);
	}

	.carved {
		position: absolute;
		top: -1px;
		right: 0;
	}

	.site-footer nav {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1) var(--space-3);
	}

	.site-footer a {
		color: var(--text-muted);
	}

	.site-footer a:hover {
		color: var(--spark);
	}

	.site-footer p {
		margin: var(--space-2) 0 0;
	}

	/* Narrow screens: the sidebar becomes a panel under the header, opened by
	   the menu button. It appears and disappears without motion. */
	@media (max-width: 54rem) {
		.menu-button {
			display: inline-flex;
			margin-top: var(--rule);
		}

		.header-links {
			display: none;
		}

		.shell {
			grid-template-columns: minmax(0, 1fr);
		}

		.sidebar {
			display: none;
			position: fixed;
			inset: var(--header-h) 0 0 0;
			/* The desktop rule's align-self: start would size this to its
			   content and pin it to the top of the viewport, under the header. */
			align-self: auto;
			z-index: var(--z-overlay);
			height: auto;
			background: var(--bg);
			border: 0;
		}

		.sidebar.open {
			display: block;
		}

		.sidebar-links {
			display: flex;
			flex-wrap: wrap;
			gap: var(--space-1) var(--space-3);
			margin-top: var(--space-3);
			padding: var(--space-2) 0 0;
			border-top: 1px solid var(--border);
			font: var(--type-sm) var(--font-mono);
		}
	}

	/* Without scripts the menu button cannot open anything. The footer's
	   "all pages" link reaches every page instead. */
	:global(html:not([data-js])) .menu-button {
		display: none;
	}
</style>
