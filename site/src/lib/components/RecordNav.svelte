<script lang="ts">
	/**
	 * The record navigator: a proposal or a plan in three groups, each a way
	 * the owner moves through it (site/README.md, "The collaborate stance").
	 *
	 *   Read     the template's sections, labelled by their role in the
	 *            dossier anatomy (orientation, case, queue, references), and
	 *            the margin, where comments land
	 *   Decide   every decision in dependency order, its state and how
	 *            readily it can be undone, as glyphs and words, and the
	 *            decisions it waits for
	 *   Confirm  every assumption, with the decisions that rest on it
	 *
	 * Drawn from the record's parsed components (the frame's data), never from
	 * its prose. It sits in the sidebar, in the frame's sheet and inline in
	 * the page on a phone; the three are the same list.
	 */
	import { ASSUMED, DECISION_STATES, REVERSIBLE, dependencyOrder, firstWords } from '$lib/record-vocabulary';
	import type { RecordFrame } from '$lib/types';

	let { frame, variant }: { frame: RecordFrame; variant: 'sidebar' | 'sheet' | 'inline' } = $props();

	const ordered = $derived(dependencyOrder(frame.decisions));
	const numberOf = (id: string) => frame.decisions.find((d) => d.id === id)?.n ?? 0;
	const settled = $derived(frame.decisions.filter((d) => DECISION_STATES[d.state].settled).length);
</script>

<div class="record-nav" data-record-nav={variant}>
	<section class="group" aria-label="Read">
		<p class="group-head">Read</p>
		<ul class="read">
			{#each frame.sections as s, i (s.id)}
				{#if s.role && s.role !== frame.sections[i - 1]?.role}
					<li class="role" aria-hidden="true">{s.role}</li>
				{/if}
				<li data-nav-section={s.id}><a href="#{s.id}">{s.text}</a></li>
			{/each}
			<li class="role" aria-hidden="true">margin</li>
			<li data-nav-margin>
				{#if frame.pr}
					<a href={frame.pr.href}>comments, on pull request {frame.pr.text}</a>
				{:else}
					<span class="none">comments, on the pull request</span>
				{/if}
			</li>
		</ul>
	</section>

	<section class="group" aria-label="Decide">
		<p class="group-head">Decide <span class="count">{settled} of {frame.decisions.length} settled</span></p>
		{#if ordered.length}
			<ol class="decide">
				{#each ordered as d (d.id)}
					{@const s = DECISION_STATES[d.state]}
					<li data-nav-decision={d.id} data-state={d.state} data-depends={d.depends.join(' ')}>
						<a href="#{d.id}">{d.title}</a>
						<span class="meta">
							<span class="state-mark" data-role={s.role}
								><span class="state-glyph" aria-hidden="true">{s.glyph}</span> <span class="state-word">{s.word}</span></span
							>
							· <span class="reversible" data-reversible={d.reversible}
								><span class="rev-glyph" aria-hidden="true">{REVERSIBLE[d.reversible].glyph}</span> {REVERSIBLE[d.reversible].word}</span
							>
							{#if d.depends.length}
								· <span class="after">after {d.depends.map(numberOf).join(', ')}</span>
							{/if}
						</span>
					</li>
				{/each}
			</ol>
		{:else}
			<p class="none">No decisions.</p>
		{/if}
	</section>

	<section class="group" aria-label="Confirm">
		<p class="group-head">Confirm <span class="count">{frame.assumptions.length} to confirm</span></p>
		{#if frame.assumptions.length}
			<ol class="confirm">
				{#each frame.assumptions as a (a.id)}
					<li data-nav-assumption={a.id} data-bears-on={a.decisions.join(' ')}>
						<a href="#{a.id}"
							><span class="state-mark" data-role="spark"
								><span class="state-glyph" aria-hidden="true">{ASSUMED.glyph}</span> <span class="state-word">{ASSUMED.word}</span></span
							>
							{firstWords(a.text)}</a
						>
						<span class="meta">
							{#if a.decisions.length}
								bears on
								{#each a.decisions as id, k (id)}{k > 0 ? ', ' : ''}<a class="bears-on" href="#{id}">decision {numberOf(id)}</a>{/each}
							{:else}
								bears on no decision
							{/if}
						</span>
					</li>
				{/each}
			</ol>
		{:else}
			<p class="none">No assumptions to confirm.</p>
		{/if}
	</section>
</div>

<style>
	.record-nav {
		font: var(--type-sm) / 1.4 var(--font-mono);
	}

	.group + .group {
		margin-top: var(--space-2);
	}

	.group-head {
		display: flex;
		justify-content: space-between;
		gap: var(--space-1);
		margin: 0 0 var(--space-0-5);
		font: 600 var(--type-xs) / 1.5 var(--font-ui);
		letter-spacing: var(--tracking-widest);
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.count {
		font-family: var(--font-mono);
		font-weight: 400;
		letter-spacing: 0;
		text-transform: none;
	}

	ul,
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	li + li {
		margin-top: 0.15rem;
	}

	.role {
		margin-top: var(--space-1);
		font-size: var(--type-xs);
		color: var(--text-muted);
	}

	.role:first-child {
		margin-top: 0;
	}

	/* At least 24px tall, so a link meets the WCAG 2.2 target size (2.5.8). */
	a {
		display: block;
		box-sizing: border-box;
		min-height: 24px;
		padding: 0.15rem 0;
		color: var(--text);
		text-decoration: none;
		overflow-wrap: anywhere;
	}

	a:hover {
		color: var(--spark);
	}

	.read a {
		color: var(--text-muted);
		font-size: var(--type-xs);
	}

	.read a:hover {
		color: var(--spark);
	}

	.meta {
		display: block;
		font-size: var(--type-xs);
		color: var(--text-muted);
	}

	.meta a.bears-on {
		display: inline-flex;
		align-items: center;
		color: var(--text);
		text-decoration: underline;
		text-decoration-color: var(--border-strong);
	}

	.state-mark {
		white-space: nowrap;
	}

	/* The glyph carries the role's colour; the word beside it the meaning. */
	.state-mark[data-role='spark'] .state-glyph {
		color: var(--spark);
	}

	.state-mark[data-role='emergence'] .state-glyph {
		color: var(--emergence);
	}

	.state-word,
	.reversible {
		color: var(--text);
	}

	.reversible {
		white-space: nowrap;
	}

	.none {
		margin: 0;
		font-size: var(--type-xs);
		color: var(--text-muted);
	}

	.decide > li,
	.confirm > li {
		padding-block: 0.1rem;
	}
</style>
