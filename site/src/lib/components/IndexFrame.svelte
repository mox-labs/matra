<script lang="ts">
	/**
	 * The Blueprints index's frame: how much awaits the owner across every
	 * proposal, counted by the build from the records' components, leading to
	 * the queue that lists each item (the `awaiting` component). Drawn by the
	 * layout, never from the index's Markdown.
	 */
	import { ASSUMED, DECISION_STATES } from '#lib/record-vocabulary.ts';
	import type { Awaiting } from '#lib/types.ts';

	let { awaiting }: { awaiting: Awaiting } = $props();
	const nothing = $derived(awaiting.open + awaiting.assumptions === 0);
</script>

<div
	class="index-frame"
	data-frame="index"
	data-open={awaiting.open}
	data-confirm={awaiting.assumptions}
	data-pagefind-ignore
	data-print="hide"
>
	<p class="row">
		<span class="badge">Blueprints</span>
		{#if nothing}
			<span class="tally">nothing awaits you</span>
		{:else}
			<a class="tally" href={awaiting.href}
				><span class="label">awaiting you:</span>
				<span class="mark" data-role="spark"
					><span class="glyph" aria-hidden="true">{DECISION_STATES.open.glyph}</span>
					{awaiting.open} open {awaiting.open === 1 ? 'decision' : 'decisions'} across {awaiting.proposals}
					{awaiting.proposals === 1 ? 'proposal' : 'proposals'}</span
				>
				<span class="mark" data-role="spark"
					><span class="glyph" aria-hidden="true">{ASSUMED.glyph}</span>
					{awaiting.assumptions}
					{awaiting.assumptions === 1 ? 'assumption' : 'assumptions'} to confirm</span
				></a
			>
		{/if}
	</p>
</div>

<style>
	/* The column is measured in the reading face's ch, so the box keeps the
	   page's font and only the row is set in mono. */
	.index-frame {
		width: min(var(--column), 100%);
		margin: 0 auto;
		padding: var(--space-2) var(--space-2) 0;
	}

	.row {
		font: var(--type-sm) / 1.5 var(--font-mono);
		font-variant-numeric: normal;
	}

	@media (max-width: 76.99rem) {
		.index-frame {
			width: min(calc(var(--measure) + 2 * var(--space-2)), 100%);
		}
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
		border: 1px solid var(--border);
	}

	.badge {
		padding: 0 0.4em;
		border: 1px solid var(--border-strong);
		font: 600 var(--type-xs) / 1.6 var(--font-ui);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.tally {
		display: inline-flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0 var(--space-2);
		min-height: 24px;
		color: var(--text);
		text-decoration: none;
	}

	a.tally:hover {
		color: var(--spark);
	}

	.label {
		color: var(--text-muted);
	}

	.mark[data-role='spark'] .glyph {
		color: var(--spark);
	}
</style>
