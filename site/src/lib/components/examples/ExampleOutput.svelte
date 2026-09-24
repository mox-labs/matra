<script lang="ts">
	/**
	 * What a worked example's calls print (EP-0012, M6): the JSON the Rust and
	 * Python calls print, which is also the `result` the CLI prints. Trimmed
	 * where long, saying how, with the whole output linked.
	 */
	import type { ExampleView } from '$lib/types';

	let { example }: { example: ExampleView } = $props();
	const output = $derived(example.output);
</script>

<div class="ex-output" data-example={example.name} data-part="output">
	{#if output.trimmed}
		<p class="note">Trimmed: {@html output.trimmed}.</p>
	{/if}
	{@html output.html}
	<p class="note">
		The whole output: <a href={output.url}>output.json</a>, and what the CLI prints,
		<a href={output.cliUrl}>cli.json</a>. The examples gate runs all three calls and compares what
		each prints with these files.
	</p>
</div>

<style>
	.note {
		margin: 0.4rem 0;
		font-size: 0.9rem;
		color: var(--text-muted);
	}
</style>
