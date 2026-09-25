<script lang="ts">
	/**
	 * Search, by Pagefind. The index is built from the prerendered HTML after
	 * the site builds (`pagefind --site build`), so it covers exactly what a
	 * reader sees. Its script and index load the first time search is opened,
	 * not with the page.
	 */
	import { afterNavigate } from '$app/navigation';
	import { base } from '$app/paths';
	import Icon from './Icon.svelte';

	let dialog: HTMLDialogElement | undefined = $state();
	let container: HTMLDivElement | undefined = $state();
	let status: 'idle' | 'loading' | 'ready' | 'unavailable' = $state('idle');

	function attach(tag: 'script' | 'link', url: string): Promise<void> {
		return new Promise((resolve, reject) => {
			const el = document.createElement(tag);
			if (el instanceof HTMLScriptElement) el.src = url;
			if (el instanceof HTMLLinkElement) {
				el.rel = 'stylesheet';
				el.href = url;
			}
			el.onload = () => resolve();
			el.onerror = () => reject(new Error(`could not load ${url}`));
			document.head.append(el);
		});
	}

	async function load() {
		if (status !== 'idle' || !container) return;
		status = 'loading';
		try {
			await attach('link', `${base}/pagefind/pagefind-ui.css`);
			await attach('script', `${base}/pagefind/pagefind-ui.js`);
			if (!window.PagefindUI) throw new Error('PagefindUI missing');
			new window.PagefindUI({
				element: container,
				baseUrl: `${base}/`,
				showSubResults: true,
				showImages: false,
				autofocus: true,
				translations: { placeholder: 'Search the documentation' }
			});
			status = 'ready';
		} catch {
			status = 'unavailable';
		}
	}

	function open() {
		if (!dialog || dialog.open) return;
		dialog.showModal();
		void load().then(() => container?.querySelector('input')?.focus());
	}

	function onKey(e: KeyboardEvent) {
		const target = e.target as HTMLElement | null;
		const typing =
			target?.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target?.tagName ?? '');
		if ((e.key === '/' && !typing) || (e.key === 'k' && (e.metaKey || e.ctrlKey))) {
			e.preventDefault();
			open();
		}
	}

	afterNavigate(() => dialog?.close());
</script>

<svelte:window onkeydown={onKey} />

<button type="button" class="search-trigger" onclick={open} aria-haspopup="dialog">
	<Icon name="search" size={16} />
	<span class="label">search</span>
	<kbd aria-hidden="true">/</kbd>
</button>

<!-- A click on the backdrop lands on the dialog itself; one on the panel does
     not. Escape closes it natively. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
	bind:this={dialog}
	class="search-dialog"
	aria-label="Search the documentation"
	onclick={(e) => e.target === dialog && dialog?.close()}
>
	<div class="panel">
		<div class="panel-head">
			<span class="panel-title">Search</span>
			<button type="button" class="icon-button" onclick={() => dialog?.close()} aria-label="Close search">
				<Icon name="close" />
			</button>
		</div>
		<div bind:this={container} class="results"></div>
		{#if status === 'loading'}
			<p class="note">Loading the index.</p>
		{:else if status === 'unavailable'}
			<p class="note">
				The search index is built with the site, so the development server has none. Run
				<code>just docs-build</code> and preview the build to search.
			</p>
		{/if}
	</div>
</dialog>

<style>
	.search-trigger {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 14rem;
		height: 2.25rem;
		padding: 0 0.6rem 0 0.75rem;
		font: var(--type-sm) var(--font-mono);
		color: var(--text-muted);
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		cursor: pointer;
	}

	.search-trigger:hover {
		border-color: var(--border-strong);
	}

	.label {
		flex: 1;
		text-align: start;
	}

	kbd {
		padding: 0 0.4rem;
		font-size: 0.78rem;
		line-height: 1.4rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--bg-subtle);
	}

	@media (max-width: 40rem) {
		.search-trigger {
			min-width: 0;
			width: 2.25rem;
			justify-content: center;
			padding: 0;
		}

		.label,
		kbd {
			display: none;
		}
	}

	/* Pagefind needs scripts; without them the button would do nothing. */
	:global(html:not([data-js])) .search-trigger {
		display: none;
	}

	.search-dialog {
		width: min(42rem, calc(100vw - 2rem));
		max-height: min(80vh, 44rem);
		margin: 8vh auto auto;
		padding: 0;
		color: var(--text);
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 0;
		box-shadow: 0 20px 50px rgb(0 0 0 / 0.25);
	}

	.search-dialog::backdrop {
		background: rgb(0 0 0 / 0.4);
	}

	.panel {
		padding: var(--space-3);
		--pagefind-ui-scale: 0.85;
		--pagefind-ui-primary: var(--accent);
		--pagefind-ui-text: var(--text);
		--pagefind-ui-background: var(--bg-raised);
		--pagefind-ui-border: var(--border);
		--pagefind-ui-tag: var(--bg-subtle);
		--pagefind-ui-border-width: 1px;
		--pagefind-ui-border-radius: var(--radius-control);
		--pagefind-ui-font: var(--font-sans);
	}

	.panel-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-2);
	}

	.panel-title {
		font-weight: 650;
	}

	.note {
		color: var(--text-muted);
		font-size: 0.9rem;
	}
</style>
