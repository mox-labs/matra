<script lang="ts">
	/**
	 * A page body: runs of build-time HTML, and between them the figures the
	 * page names, each drawn by the component registered for its kind.
	 */
	import type { Segment } from '$lib/types';
	import ClustersFigure from './figures/ClustersFigure.svelte';
	import KeyphrasesFigure from './figures/KeyphrasesFigure.svelte';
	import MetricsFigure from './figures/MetricsFigure.svelte';
	import ParseFigure from './figures/ParseFigure.svelte';
	import PipelineFigure from './figures/PipelineFigure.svelte';
	import PrimitivesFigure from './figures/PrimitivesFigure.svelte';
	import TextrankFigure from './figures/TextrankFigure.svelte';

	let { segments }: { segments: Segment[] } = $props();
</script>

{#each segments as segment, i (i)}
	{#if segment.kind === 'html'}
		{@html segment.html}
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
