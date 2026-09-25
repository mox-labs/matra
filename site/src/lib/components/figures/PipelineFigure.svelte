<script lang="ts">
	/**
	 * One document through the pipeline (EP-0012, M5): ingested, annotated,
	 * composed, each the output of the public call, stepped by the reader.
	 *
	 * Ingest hands the text over as a `RawDocument`. `Engine::annotate`
	 * decomposes it into sections and paragraphs and parses each paragraph
	 * that is not a quote. `Engine::compose` fills the measures. The stepper
	 * moves between those three states; each change is the two-stage fade, and
	 * under reduced motion it is immediate. Without scripts every stage is
	 * shown, one after another. The twin table holds every paragraph at both
	 * later stages.
	 */
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import type { PipelineFigureFile, PipelineParagraph, PipelineStage } from '$lib/types';
	import FigureFrame from './FigureFrame.svelte';
	import { roving, swap } from './motion';

	let { id, file, dataUrl }: { id: string; file: PipelineFigureFile; dataUrl: string } = $props();

	const STAGES = [
		{ key: 'raw', name: 'Ingest', call: 'Ingest::text', adds: 'the text and its format' },
		{ key: 'annotated', name: 'Annotate', call: 'Engine::annotate', adds: 'sections, paragraphs, sentences, tokens' },
		{ key: 'composed', name: 'Compose', call: 'Engine::compose', adds: 'the measures' }
	] as const;

	const data = $derived(file.data);
	let step = $state(0);
	// Before scripts run, and without them, every stage shows at once.
	let js = $state(false);
	onMount(() => (js = true));

	const paragraphsOf = (s: PipelineStage) => s.sections.flatMap((sec) => sec.paragraphs);
	const composedByIndex = $derived(new Map(paragraphsOf(data.composed).map((p) => [p.index, p])));
	const fmt = (v: number | null) => (v === null ? 'none' : v.toFixed(2));
	/** The paragraph the reader points at, in the table or the tree. */
	let active = $state<number | null>(null);
</script>

{#snippet tree(stage: PipelineStage, key: 'annotated' | 'composed')}
	<ul class="tree" data-stage={key}>
		{#each stage.sections as sec, s (s)}
			<li class="section">
				<span class="label">section</span>
				<span class="heading">{'#'.repeat(sec.level)} {sec.heading ?? '(no heading)'}</span>
				<ul>
					{#each sec.paragraphs as p (p.index)}
						<!-- Pointer-only: the table's rows give the keyboard the same. -->
						<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
						<li
							class="para"
							class:quote={p.in_blockquote}
							class:lit={active === p.index}
							onpointerenter={() => (active = p.index)}
							onpointerleave={() => (active = null)}
							data-paragraph={p.index}
							data-sentences={p.sentences}
							data-tokens={p.tokens}
							data-grade={p.readability_grade ?? ''}
							data-density={p.lexical_density ?? ''}
							data-compression={p.compression_ratio ?? ''}
						>
							<span class="label">¶{p.index}</span>
							<span class="opening">{p.opening}…</span>
							<span class="facts">
								{#if p.in_blockquote}
									<span class="fact muted">quote, not parsed</span>
								{:else}
									<span class="fact">{p.sentences} sentences</span>
									<span class="fact">{p.tokens} tokens</span>
								{/if}
								{#if key === 'composed'}
									<span class="fact m">grade {fmt(p.readability_grade)}</span>
									<span class="fact m">density {fmt(p.lexical_density)}</span>
									<span class="fact m">compression {fmt(p.compression_ratio)}</span>
								{:else if !p.in_blockquote}
									<span class="fact muted">measures not computed yet</span>
								{/if}
							</span>
						</li>
					{/each}
				</ul>
			</li>
		{/each}
	</ul>
	{#if key === 'composed'}
		<p class="doc">
			Document: vocabulary TTR {fmt(stage.document.vocabulary_ttr)}, nominalization ratio
			{fmt(stage.document.nominalization_ratio)}, passive ratio {fmt(stage.document.passive_ratio)}
		</p>
	{/if}
{/snippet}

{#snippet panel(i: number)}
	<section class="panel" aria-label="Stage {i + 1}: {STAGES[i].name}">
		<p class="panel-head">
			<span class="step-n">{i + 1}</span>
			<strong>{STAGES[i].name}</strong> <code>{STAGES[i].call}</code> adds {STAGES[i].adds}.
		</p>
		{#if i === 0}
			<p class="raw-meta">
				<code>RawDocument</code>: format <strong>{data.raw.format}</strong>, {data.raw.bytes} bytes
			</p>
			<pre class="raw">{data.raw.text}</pre>
		{:else if i === 1}
			{@render tree(data.annotated, 'annotated')}
		{:else}
			{@render tree(data.composed, 'composed')}
		{/if}
	</section>
{/snippet}

<FigureFrame {id} kind="pipeline" title="One document through the pipeline" {file} {dataUrl}>
	{#snippet controls()}
		<div class="stepper-controls" role="group" aria-label="Pipeline stage">
			<button type="button" onclick={() => (step = Math.max(0, step - 1))} disabled={step === 0} aria-label="Previous stage">‹</button>
			<button type="button" onclick={() => (step = Math.min(2, step + 1))} disabled={step === 2} aria-label="Next stage">›</button>
		</div>
	{/snippet}

	<!-- A region that scrolls must take focus, or it cannot be scrolled from
	     the keyboard (WCAG 2.1.1). -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="fig-twin" tabindex="0" role="region" aria-label="Every paragraph after annotate and after compose">
		<table data-twin-for={id}>
			<thead>
				<tr>
					<th>¶</th><th>Quote</th><th>Sentences</th><th>Tokens</th><th>Grade</th><th>Density</th><th>Compression</th>
				</tr>
			</thead>
			<tbody {@attach roving}>
				{#each paragraphsOf(data.annotated) as p (p.index)}
					{@const c = composedByIndex.get(p.index) as PipelineParagraph}
					<tr
						data-row={p.index}
						class:lit={active === p.index}
						onpointerenter={() => (active = p.index)}
						onpointerleave={() => (active = null)}
						onfocus={() => (active = p.index)}
						onblur={() => (active = null)}
					>
						<td class="num">{p.index}</td>
						<td>{p.in_blockquote ? 'yes' : ''}</td>
						<td>{p.sentences}</td>
						<td>{p.tokens}</td>
						<td>{fmt(c.readability_grade)}</td>
						<td>{fmt(c.lexical_density)}</td>
						<td>{fmt(c.compression_ratio)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<ol class="rail" aria-label="Stages">
		{#each STAGES as s, i (s.key)}
			<li>
				<button
					type="button"
					class:done={js && i < step}
					aria-current={js && i === step ? 'step' : undefined}
					onclick={() => (step = i)}
				>
					<span class="step-n">{i + 1}</span>{s.name}
				</button>
			</li>
		{/each}
	</ol>

	{#if js}
		<div class="stage">
			{#key step}
				<div class="state" transition:fade={swap()}>{@render panel(step)}</div>
			{/key}
		</div>
	{:else}
		{#each STAGES as _, i (i)}{@render panel(i)}{/each}
	{/if}

	{#snippet note()}
		Each stage is the output of the public call named, run by matra on the text shown. The quote
		is kept as a paragraph but never parsed, so it has no sentences and no measures. A measure whose
		applicability conditions a paragraph does not meet stays <code>None</code> after compose, shown as
		none.
	{/snippet}
</FigureFrame>

<style>
	.stepper-controls {
		display: flex;
		gap: 0.25rem;
	}

	.stepper-controls button {
		width: 2rem;
		height: 2rem;
		font: 700 1rem var(--font-sans);
		color: var(--text-muted);
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		cursor: pointer;
	}

	.stepper-controls button:disabled {
		opacity: 0.4;
		cursor: default;
	}

	:global(html:not([data-js])) .stepper-controls {
		display: none;
	}

	.rail {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
		margin: 0 0 0.8rem;
		padding: 0;
		list-style: none;
	}

	.rail li {
		display: flex;
		align-items: center;
		margin: 0;
	}

	.rail li + li::before {
		content: '→';
		margin-right: 0.35rem;
		color: var(--text-muted);
	}

	.rail button {
		display: inline-flex;
		align-items: center;
		gap: 0.45em;
		padding: 0.3rem 0.7rem;
		font: 600 0.86rem var(--font-sans);
		color: var(--text-muted);
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		cursor: pointer;
	}

	.rail button.done {
		color: var(--text);
	}

	.rail button[aria-current='step'] {
		color: var(--accent);
		background: var(--accent-soft);
		border-color: var(--accent);
	}

	.step-n {
		display: inline-grid;
		place-items: center;
		width: 1.35rem;
		height: 1.35rem;
		margin-right: 0.3rem;
		font: 700 0.72rem var(--font-mono);
		border-radius: 0;
		border: 1px solid currentColor;
	}

	.stage {
		display: grid;
	}

	.state {
		grid-area: 1 / 1;
		min-width: 0;
	}

	.panel + .panel {
		margin-top: 1rem;
		padding-top: 0.8rem;
		border-top: 1px solid var(--border);
	}

	.panel-head {
		margin: 0 0 0.6rem;
		font-size: 0.92rem;
	}

	.raw-meta {
		margin: 0 0 0.4rem;
		font-size: 0.86rem;
		color: var(--text-muted);
	}

	pre.raw {
		margin: 0;
		padding: 0.7rem 0.9rem;
		max-height: 16rem;
		overflow: auto;
		font-size: 0.8rem;
		line-height: 1.5;
		white-space: pre-wrap;
		background: var(--code-bg);
		border: 1px solid var(--border);
		border-radius: 0;
	}

	.tree,
	.tree ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.tree ul {
		margin-left: 0.9rem;
		padding-left: 0.8rem;
		border-left: 1px solid var(--border-strong);
	}

	.tree li {
		margin: 0.35rem 0;
	}

	.label {
		margin-right: 0.5em;
		font: 700 0.72rem var(--font-mono);
		color: var(--accent);
	}

	.heading {
		font-weight: 650;
	}

	.para {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 0 0.3em;
	}

	.opening {
		color: var(--text);
		font-size: 0.9rem;
	}

	.para.quote .opening {
		font-style: italic;
		color: var(--text-muted);
	}

	.facts {
		grid-column: 2;
		display: flex;
		flex-wrap: wrap;
		gap: 0.2rem 0.4rem;
		margin-top: 0.15rem;
	}

	.fact {
		padding: 0.02rem 0.45rem;
		font: 0.74rem var(--font-mono);
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
	}

	/* What compose adds: the measures, the pipeline's finished result, in
	   Emergence, each named. */
	.fact.m {
		color: var(--emergence);
		border-color: var(--emergence);
	}

	tbody tr:focus-visible {
		outline: 2px solid var(--spark);
		outline-offset: -2px;
	}

	tbody tr.lit td,
	.para.lit .label {
		color: var(--spark);
	}

	.para.lit {
		box-shadow: -9px 0 0 -7px var(--spark);
	}

	.fact.muted {
		color: var(--text-muted);
		border-style: dashed;
	}

	.doc {
		margin: 0.6rem 0 0;
		font-size: 0.84rem;
		color: var(--text-muted);
	}
</style>
