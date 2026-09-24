<script lang="ts">
	/**
	 * A page body: runs of build-time HTML, and between them the figures and
	 * worked-example parts the page names, each drawn by its component.
	 */
	import type { Segment } from '$lib/types';
	import ClustersFigure from './figures/ClustersFigure.svelte';
	import KeyphrasesFigure from './figures/KeyphrasesFigure.svelte';
	import MetricsFigure from './figures/MetricsFigure.svelte';
	import ParseFigure from './figures/ParseFigure.svelte';
	import PipelineFigure from './figures/PipelineFigure.svelte';
	import PrimitivesFigure from './figures/PrimitivesFigure.svelte';
	import TextrankFigure from './figures/TextrankFigure.svelte';
	import ExampleCall from './examples/ExampleCall.svelte';
	import ExampleInput from './examples/ExampleInput.svelte';
	import ExampleOutput from './examples/ExampleOutput.svelte';

	let { segments }: { segments: Segment[] } = $props();
</script>

{#each segments as segment, i (i)}
	{#if segment.kind === 'html'}
		{@html segment.html}
	{:else if segment.kind === 'example'}
		{#if segment.part === 'input'}
			<ExampleInput example={segment.example} />
		{:else if segment.part === 'call'}
			<ExampleCall example={segment.example} />
		{:else}
			<ExampleOutput example={segment.example} />
		{/if}
	{:else if segment.figure === 'parse'}
		<ParseFigure id={segment.id} file={segment.file} sentence={segment.sentence} dataUrl={segment.dataUrl} />
	{:else if segment.figure === 'primitives'}
		<PrimitivesFigure id={segment.id} file={segment.file} dataUrl={segment.dataUrl} />
	{:else if segment.figure === 'metrics'}
		<MetricsFigure id={segment.id} file={segment.file} dataUrl={segment.dataUrl} />
	{:else if segment.figure === 'keyphrases'}
		<KeyphrasesFigure id={segment.id} file={segment.file} dataUrl={segment.dataUrl} />
	{:else if segment.figure === 'textrank'}
		<TextrankFigure id={segment.id} file={segment.file} dataUrl={segment.dataUrl} />
	{:else if segment.figure === 'clusters'}
		<ClustersFigure id={segment.id} file={segment.file} threshold={segment.threshold} dataUrl={segment.dataUrl} />
	{:else if segment.figure === 'pipeline'}
		<PipelineFigure id={segment.id} file={segment.file} dataUrl={segment.dataUrl} />
	{/if}
{/each}
