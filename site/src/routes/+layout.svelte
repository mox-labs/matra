<script lang="ts">
	import '../app.css';
	import { afterNavigate } from '$app/navigation';
	import { base } from '$app/paths';
	import { page } from '$app/state';
	import Icon from '$lib/components/Icon.svelte';
	import NavTree from '$lib/components/NavTree.svelte';
	import Search from '$lib/components/Search.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import { DISCUSSIONS_URL, ISSUES_URL, REPO_URL, SITE_NAME } from '$lib/site';
	import type { LayoutProps } from './$types';

	let { data, children }: LayoutProps = $props();

	let navOpen = $state(false);
	afterNavigate(() => (navOpen = false));

	/** The route of the page being shown, without the base path; `/` is the first page. */
	const current = $derived.by(() => {
		const path = page.url.pathname.slice(base.length).replace(/\.html$/, '');
		return path === '' || path === '/' ? (data.nav[0]?.items[0]?.route ?? null) : path;
	});

	const links = [
		{ href: `${base}/api/`, label: 'API' },
		{ href: DISCUSSIONS_URL, label: 'Discussions' },
		{ href: ISSUES_URL, label: 'Issues' }
	];
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (navOpen = false)} />

<a class="skip" href="#main">Skip to content</a>

<header class="site-header" data-print="hide">
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

	<a class="wordmark" href="{base}/">{SITE_NAME}</a>

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
			<NavTree nav={data.nav} {current} />
		</nav>
		<nav class="sidebar-links" aria-label="Project links">
			{#each links as link (link.href)}
				<a href={link.href}>{link.label}</a>
			{/each}
			<a href={REPO_URL}>GitHub</a>
		</nav>
	</aside>

	<main id="main" tabindex="-1">
		{@render children()}
	</main>
</div>

<footer class="site-footer" data-print="hide">
	<nav aria-label="Footer">
		<a href={REPO_URL}>GitHub</a>
		<a href={ISSUES_URL}>Issues</a>
		<a href={DISCUSSIONS_URL}>Discussions</a>
		<a href="{base}/api/">API reference</a>
		<a href="{base}/toc">All pages</a>
		<a href="{base}/print">Single page</a>
		<a href="{base}/llms.txt">llms.txt</a>
	</nav>
	<p>matra is MIT licensed.</p>
</footer>

<style>
	.skip {
		position: absolute;
		left: var(--space-3);
		top: -3rem;
		z-index: 100;
		padding: var(--space-2) var(--space-3);
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
	}

	.skip:focus {
		top: var(--space-2);
	}

	.site-header {
		position: sticky;
		top: 0;
		z-index: 50;
		display: flex;
		align-items: center;
		gap: var(--space-3);
		height: var(--header-h);
		padding: 0 var(--gutter);
		background: var(--bg);
		border-bottom: 1px solid var(--border);
	}

	.wordmark {
		font-weight: 750;
		font-size: 1.25rem;
		letter-spacing: -0.02em;
		color: var(--text);
		text-decoration: none;
	}

	.wordmark::before {
		content: '';
		display: inline-block;
		width: 0.55em;
		height: 0.55em;
		margin-inline-end: 0.4em;
		border-radius: 2px;
		background: var(--accent);
		vertical-align: 0.05em;
	}

	.search-slot {
		margin-inline-start: auto;
	}

	.header-links {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		font-size: 0.92rem;
	}

	.header-links a:not(.icon-button) {
		color: var(--text-muted);
		text-decoration: none;
	}

	.header-links a:hover {
		color: var(--text);
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
		border-radius: 8px;
		cursor: pointer;
	}

	:global(.icon-button:hover) {
		color: var(--text);
		background: var(--bg-subtle);
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
		padding: var(--space-4) var(--space-3) var(--space-5);
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
		padding: var(--space-4) var(--gutter) var(--space-5);
		border-top: 1px solid var(--border);
		font-size: 0.88rem;
		color: var(--text-muted);
		text-align: center;
	}

	.site-footer nav {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: var(--space-2) var(--space-4);
	}

	.site-footer a {
		color: var(--text-muted);
	}

	.site-footer a:hover {
		color: var(--accent);
	}

	.site-footer p {
		margin: var(--space-3) 0 0;
	}

	/* Narrow screens: the sidebar becomes a panel under the header, opened by
	   the menu button. It appears and disappears without motion. */
	@media (max-width: 54rem) {
		.menu-button {
			display: inline-flex;
			margin-inline-start: -0.5rem;
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
			z-index: 40;
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
			gap: var(--space-2) var(--space-4);
			margin-top: var(--space-4);
			padding: var(--space-3) 0.6rem 0;
			border-top: 1px solid var(--border);
			font-size: 0.94rem;
		}
	}

	/* Without scripts the menu button cannot open anything. The footer's
	   "All pages" link reaches every page instead. */
	:global(html:not([data-js])) .menu-button {
		display: none;
	}
</style>
