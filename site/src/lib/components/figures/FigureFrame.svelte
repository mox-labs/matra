<script lang="ts">
	/**
	 * What every figure shares: a titled card, an optional control in the
	 * header, the body, a legend, a note on how to read it, and the provenance
	 * line. The provenance names the input and its licence, the matra version
	 * and model that produced the data, and links the data itself.
	 */
	import type { Snippet } from 'svelte';
	import type { FigureFile } from '$lib/types';

	let {
		id,
		kind,
		title,
		file,
		dataUrl,
		controls,
		legend,
		note,
		children
	}: {
		id: string;
		/** The registered figure kind, `data-figure` for the twin test. */
		kind: string;
		title: string;
		file: FigureFile;
		dataUrl: string;
		controls?: Snippet;
		legend?: Snippet;
		note?: Snippet;
		children: Snippet;
	} = $props();

	const source = $derived(file.source);
	const model = $derived(file.generator.udpipe_model);
</script>

<figure class="mx-figure" {id} data-figure={kind} data-input={file.input} data-pagefind-ignore aria-labelledby="{id}-title">
	<div class="head">
		<p class="eyebrow" id="{id}-title">{title}</p>
		{#if controls}{@render controls()}{/if}
	</div>

	{@render children()}

	<figcaption>
		{#if legend}{@render legend()}{/if}
		{#if note}<p>{@render note()}</p>{/if}
		<p class="provenance">
			Input: <a href={source.url}>{source.title}</a>, {source.author}. {source.licence}.
			Produced by matra {file.generator.matra} with the UDPipe model {model.name}
			(SHA-256 <code>{model.sha256.slice(0, 12)}</code>){#if file.generator.embedding_model}{' '}and
				the embedding model {file.generator.embedding_model.name} (SHA-256
				<code>{file.generator.embedding_model.sha256.slice(0, 12)}</code>){/if}. Data:
			<a href={dataUrl}>{file.input}/{file.figure}.json</a>.
		</p>
	</figcaption>
</figure>

<style>
	.mx-figure {
		margin: 2.2em 0;
		padding: 1rem 1.1rem 0.9rem;
		max-width: none;
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 12px;
	}

	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem 1rem;
		margin-bottom: 0.6rem;
	}

	.eyebrow {
		margin: 0;
		font-size: 0.72rem;
		font-weight: 650;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.mx-figure figcaption {
		margin: 0.9rem 0 0;
		padding-top: 0.75rem;
		max-width: none;
		text-align: left;
		font-size: 0.84rem;
		line-height: 1.5;
		color: var(--text-muted);
		border-top: 1px solid var(--border);
	}

	.mx-figure figcaption p {
		margin: 0.4rem 0 0;
	}

	.provenance code {
		font-size: 0.92em;
	}

	/* Shared by every figure's parts, which render inside this card. */
	.mx-figure :global(.fig-scroll) {
		overflow-x: auto;
		overscroll-behavior-x: contain;
		background:
			linear-gradient(to right, var(--bg-raised) 30%, transparent) left / 2.5rem 100% no-repeat local,
			linear-gradient(to left, var(--bg-raised) 30%, transparent) right / 2.5rem 100% no-repeat local,
			linear-gradient(to right, light-dark(rgb(0 0 0 / 0.14), rgb(0 0 0 / 0.55)), transparent)
				left / 1rem 100% no-repeat scroll,
			linear-gradient(to left, light-dark(rgb(0 0 0 / 0.14), rgb(0 0 0 / 0.55)), transparent)
				right / 1rem 100% no-repeat scroll;
		background-color: var(--bg-raised);
	}

	.mx-figure :global(.fig-scroll > svg) {
		display: block;
		max-width: none;
		margin: 0 auto;
	}

	.mx-figure :global(.fig-hint) {
		margin: 0.35rem 0 0;
		font-size: 0.78rem;
		color: var(--text-muted);
		text-align: center;
	}

	.mx-figure :global(.fig-twin) {
		overflow-x: auto;
		margin-bottom: 0.9rem;
	}

	.mx-figure :global(.fig-twin table) {
		width: 100%;
		margin: 0;
		font-size: 0.84rem;
		border-collapse: collapse;
	}

	.mx-figure :global(.fig-twin :is(th, td)) {
		padding: 0.28em 0.7em 0.28em 0;
		white-space: nowrap;
		border-bottom: 1px solid var(--border);
	}

	.mx-figure :global(.fig-twin th) {
		font-size: 0.72rem;
		font-weight: 650;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--text-muted);
		border-bottom-width: 1px;
	}

	.mx-figure :global(.fig-twin td) {
		font-family: var(--font-mono);
		font-size: 0.8rem;
	}

	.mx-figure :global(.fig-twin .num) {
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}

	.mx-figure :global(.fig-legend) {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem 1.1rem;
		margin: 0;
		padding: 0;
		list-style: none;
		color: var(--text);
	}

	.mx-figure :global(.fig-legend li) {
		display: inline-flex;
		align-items: center;
		gap: 0.4em;
		margin: 0;
	}
</style>
