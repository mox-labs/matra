<script lang="ts">
	/**
	 * A worked example's input (EP-0012, M6): where it comes from, its
	 * licence, the file name the calls read it under, and the text. A short
	 * input is shown whole; a long one sits in a disclosure, open without
	 * scripts as with them, since `<details>` needs none.
	 */
	import { SITE_URL } from '$lib/site';
	import type { ExampleView } from '$lib/types';

	let { example }: { example: ExampleView } = $props();

	const input = $derived(example.input);
	const short = $derived(input.words <= 120);
	const markdown = $derived(input.file.endsWith('.md'));
	const paragraphs = $derived(input.text.split(/\n\s*\n/).filter((p) => p.trim() !== ''));
	const download = $derived(`${SITE_URL}/example-files/${example.name}/${input.file}`);
</script>

<div class="ex-input" data-example={example.name} data-part="input">
	<p class="meta">
		<a href={input.source.url}>{input.source.title}</a>, {input.source.author}. {input.source.licence}.
		{input.words.toLocaleString('en')} words.
	</p>
	<p class="save">
		The calls read it as <code>{input.file}</code> in the directory you run them from. Save it there:
	</p>
	<pre class="fetch"><code>curl -fLO {download}</code></pre>
	<p class="save">or <a href={input.url} download={input.file}>download {input.file}</a>.</p>
	{#snippet body()}
		{#if markdown}
			<!-- Markdown is shown as its source: the headings and the quote are
			     what the decomposer reads. -->
			<pre class="source"><code>{input.text}</code></pre>
		{:else}
			{#each paragraphs as p, i (i)}<p>{p}</p>{/each}
		{/if}
	{/snippet}
	{#if short}
		<blockquote class="text">{@render body()}</blockquote>
	{:else}
		<details class="text">
			<summary>The text, {paragraphs.length} paragraphs</summary>
			<!-- A region that scrolls must take focus (WCAG 2.1.1). -->
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<blockquote class="scroll" tabindex="0" aria-label="The input text">{@render body()}</blockquote>
		</details>
	{/if}
</div>

<style>
	.meta,
	.save {
		margin: 0.4rem 0;
		font-size: 0.9rem;
		color: var(--text-muted);
	}

	.fetch {
		margin: 0.4rem 0;
		font-size: 0.82rem;
		overflow-x: auto;
	}

	details.text {
		margin: 0.8rem 0 0;
	}

	details.text summary {
		cursor: pointer;
		font-size: 0.9rem;
		color: var(--text-muted);
	}

	blockquote {
		margin: 0.8rem 0 0;
		font-size: 0.92rem;
	}

	blockquote.scroll {
		max-height: 22rem;
		overflow-y: auto;
	}

	pre.source {
		margin: 0;
		white-space: pre-wrap;
		font-size: 0.82rem;
	}
</style>
