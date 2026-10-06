<script lang="ts">
	/**
	 * The record frame: the furniture of a proposal or a plan, owned by the
	 * layout and drawn from what the build parsed (the record's header and
	 * its components), never from its Markdown body, so a record cannot set,
	 * restate or style its own state. site/README.md, "The collaborate
	 * stance".
	 *
	 * In order: the kind as a badge, the id and title, the status, the
	 * decisions settled out of all of them (Spark while any is open, Emergence
	 * once all are settled), the assumptions to confirm (Spark while any is),
	 * the pinned commit, and the pull request and tracking issue. A field
	 * that is absent says so in words.
	 *
	 * On a desktop it heads the column, and the sidebar's navigator keeps the
	 * decisions and assumptions in view. On a phone it is one row, sticky
	 * under the header (the id and the two tallies), whose disclosure opens a
	 * sheet holding the rest of the frame and the navigator (Read, Decide,
	 * Confirm). Without a script the
	 * sheet is a plain `<details>`; with one, following a link in it closes
	 * it. Nothing moves: the sheet appears and disappears.
	 */
	import { ASSUMED, DECISION_STATES, KIND_LABEL, dependencyOrder, statusGlyph } from '$lib/record-vocabulary';
	import type { RecordFrame } from '$lib/types';
	import RecordNav from './RecordNav.svelte';

	let { frame }: { frame: RecordFrame } = $props();

	const total = $derived(frame.decisions.length);
	const settled = $derived(frame.decisions.filter((d) => DECISION_STATES[d.state].settled).length);
	const allSettled = $derived(total > 0 && settled === total);
	const k = $derived(frame.assumptions.length);
	/** Where the decisions tally leads: the first open decision in dependency order, else the first. */
	const decideHref = $derived(`#${(dependencyOrder(frame.decisions).find((d) => d.state === 'open') ?? frame.decisions[0])?.id ?? ''}`);
	const awaitingAcceptance = $derived(allSettled && frame.status === 'proposed');

	/** Following a link in the sheet closes it, so the place it leads to is in view. */
	function closeOnFollow(details: HTMLDetailsElement) {
		const close = (e: MouseEvent) => {
			if ((e.target as Element | null)?.closest('a')) details.open = false;
		};
		details.addEventListener('click', close);
		return () => details.removeEventListener('click', close);
	}
</script>

{#snippet identity()}
	<span class="badge">{KIND_LABEL[frame.kind]}</span>
	<span class="rid">{frame.id}</span>
	<span class="title">{frame.title}</span>
{/snippet}

{#snippet status()}
	<span class="status"><span class="glyph" aria-hidden="true">{statusGlyph(frame.status)}</span> {frame.status}</span>
{/snippet}

{#snippet decisionsTally(short: boolean)}
	{#if total === 0}
		<span class="tally" data-tally="decisions">no decisions</span>
	{:else}
		<!-- In the phone's row the tallies are words, not links: the row is the
		     sheet's summary, and the sheet holds the links. -->
		<svelte:element this={short ? 'span' : 'a'} class="tally" data-tally="decisions" data-role={allSettled ? 'emergence' : 'spark'} href={short ? undefined : decideHref}
			><span class="glyph" aria-hidden="true">{allSettled ? DECISION_STATES.accept.glyph : DECISION_STATES.open.glyph}</span>
			{settled} of {total}{short ? '' : total === 1 ? ' decision' : ' decisions'} settled</svelte:element
		>
	{/if}
{/snippet}

{#snippet assumptionsTally(short: boolean)}
	{#if k === 0}
		<span class="tally" data-tally="assumptions">{short ? 'none to confirm' : 'no assumptions to confirm'}</span>
	{:else}
		<svelte:element this={short ? 'span' : 'a'} class="tally" data-tally="assumptions" data-role="spark" href={short ? undefined : '#assumptions-to-confirm'}
			><span class="glyph" aria-hidden="true">{ASSUMED.glyph}</span>
			{k}{short ? '' : k === 1 ? ' assumption' : ' assumptions'} to confirm</svelte:element
		>
	{/if}
{/snippet}

{#snippet provenance()}
	<span class="field"
		>pinned at {#if frame.pin}<a href={frame.pin.href}><code>{frame.pin.sha.slice(0, 7)}</code></a>{:else}no commit{/if}</span
	>
	<span class="field"
		>{#if frame.pr}<a href={frame.pr.href}>pull request {frame.pr.text}</a>{:else}no pull request{/if}</span
	>
	<span class="field"
		>{#if frame.tracking}<a href={frame.tracking.href}>tracking issue {frame.tracking.text}</a>{:else}tracking issue: {frame.trackingNote ?? 'none yet'}{/if}</span
	>
{/snippet}

<!-- The rest of the record's header, which the masthead table carries in
     print and in the .md twin: the frame owns it on screen. -->
{#snippet header()}
	{#each frame.header as f (f.label)}
		<span class="field" data-header={f.label}>{f.label.toLowerCase()}: {f.text}</span>
	{/each}
	<span class="field" data-header="Decides">decides: the owner, who alone sets the status to accepted</span>
{/snippet}

<!-- data-* carry the tallies for gate 9, which holds them to the page's own
     decisions and assumptions. -->
<div
	class="record-frame"
	data-frame="full"
	data-total={total}
	data-settled={settled}
	data-confirm={k}
	data-pagefind-ignore
	data-print="hide"
>
	<p class="row">{@render identity()} {@render status()}</p>
	<p class="row state">
		{@render decisionsTally(false)}
		{@render assumptionsTally(false)}
		{#if awaitingAcceptance}<span class="field">all decisions settled; awaiting acceptance</span>{/if}
		{@render provenance()}
		{@render header()}
	</p>
</div>

<details
	class="frame-compact"
	{@attach closeOnFollow}
	data-frame="compact"
	data-total={total}
	data-settled={settled}
	data-confirm={k}
	data-pagefind-ignore
	data-print="hide"
>
	<summary>
		<span class="rid">{frame.id}</span>
		{@render decisionsTally(true)}
		{@render assumptionsTally(true)}
		<span class="open-label" aria-hidden="true"></span>
	</summary>
	<div class="sheet-body">
		<p class="row">{@render identity()} {@render status()}</p>
		<p class="row state">
			{#if awaitingAcceptance}<span class="field">all decisions settled; awaiting acceptance</span>{/if}
			{@render provenance()}
			{@render header()}
		</p>
		<RecordNav {frame} variant="sheet" />
	</div>
</details>

<style>
	/* Aligned with the reading column below it (DocPage's .doc). The
	   column is measured in the reading face's ch, so the frame's box keeps
	   the page's font and only its rows are set in mono. */
	.record-frame,
	.frame-compact {
		width: min(var(--column), 100%);
		margin: 0 auto;
		padding: var(--space-2) var(--space-2) 0;
	}

	.row,
	.frame-compact > summary,
	.sheet-body {
		font: var(--type-sm) / 1.5 var(--font-mono);
		font-variant-numeric: normal;
	}

	@media (max-width: 76.99rem) {
		.record-frame,
		.frame-compact {
			width: min(calc(var(--measure) + 2 * var(--space-2)), 100%);
		}
	}

	.record-frame {
		padding-bottom: var(--space-1);
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-0-5) var(--space-2);
		min-height: 36px;
		margin: 0;
		padding: var(--space-0-5) var(--space-1);
		background: var(--bg-raised);
		border-inline: 1px solid var(--border);
		border-top: 1px solid var(--border);
	}

	.row.state {
		border-bottom: 1px solid var(--border);
	}

	/* The kind is a monochrome badge, as a panel frame's type badge is. */
	.badge {
		padding: 0 0.4em;
		border: 1px solid var(--border-strong);
		font: 600 var(--type-xs) / 1.6 var(--font-ui);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.rid {
		font-weight: 600;
		color: var(--text);
	}

	.title {
		font-family: var(--font-ui);
		color: var(--text);
	}

	.status {
		padding: 0 0.4em;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		color: var(--text);
		white-space: nowrap;
	}

	.tally,
	.field {
		display: inline-flex;
		align-items: center;
		gap: 0.3em;
		min-height: 24px;
		white-space: nowrap;
		color: var(--text-muted);
	}

	/* A header fact may be a sentence (a tracking issue's note): it wraps. */
	.field[data-header],
	.field:has(+ .field[data-header]) {
		white-space: normal;
	}

	a.tally {
		color: var(--text);
		text-decoration: none;
	}

	a.tally:hover {
		color: var(--spark);
	}

	/* The glyph carries the role's colour; the words carry the count. */
	.tally[data-role='spark'] .glyph {
		color: var(--spark);
	}

	.tally[data-role='emergence'] .glyph {
		color: var(--emergence);
	}

	.field a {
		display: inline-flex;
		align-items: center;
		min-height: 24px;
		color: var(--text);
	}

	.field a:hover {
		color: var(--spark);
	}

	.field code {
		color: inherit;
	}

	/* The phone's frame: one row, sticky under the header. */
	.frame-compact {
		display: none;
	}

	@media (max-width: 54rem) {
		.record-frame {
			display: none;
		}

		.frame-compact {
			display: block;
			position: sticky;
			top: var(--header-h);
			z-index: calc(var(--z-sticky) - 1);
			padding-top: 0;
			background: var(--bg);
		}

		.frame-compact > summary {
			display: flex;
			flex-wrap: wrap;
			align-items: center;
			gap: 0 var(--space-1);
			/* As tall as app.css's --frame-h says, so what a link leads to
			   lands below it. */
			min-height: var(--frame-h, 2.25rem);
			padding: var(--space-0-5) var(--space-1);
			background: var(--bg-raised);
			border: 1px solid var(--border);
			font-size: var(--type-xs);
			cursor: pointer;
			list-style: none;
		}

		.frame-compact > summary::-webkit-details-marker {
			display: none;
		}

		.open-label {
			margin-inline-start: auto;
			color: var(--text-muted);
			white-space: nowrap;
		}

		.open-label::after {
			content: '▾';
		}

		.frame-compact[open] .open-label::after {
			content: '▴';
		}

		/* The sheet: under the row, as tall as the window allows, scrolling
		   on its own. */
		.sheet-body {
			max-height: calc(100dvh - var(--header-h) - var(--frame-h, 2.25rem) - 1px);
			overflow-y: auto;
			overscroll-behavior: contain;
			padding: 0 0 var(--space-2);
			background: var(--bg);
			border-inline: 1px solid var(--border);
			border-bottom: 1px solid var(--border);
		}

		.sheet-body .row {
			border-inline: 0;
		}

		.sheet-body :global(.record-nav) {
			padding: var(--space-2) var(--space-1) 0;
		}
	}
</style>
