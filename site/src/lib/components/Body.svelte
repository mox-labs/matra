<script lang="ts">
	/**
	 * A page body: runs of build-time HTML, and between them the figures the
	 * page names, each drawn by its registered component.
	 */
	import type { Segment } from '$lib/types';
	import ParseFigure from './figures/ParseFigure.svelte';

	let { segments }: { segments: Segment[] } = $props();
</script>

{#each segments as segment, i (i)}
	{#if segment.kind === 'html'}
		{@html segment.html}
	{:else if segment.tag === 'figure-parse'}
		<ParseFigure
			id={segment.id}
			file={segment.file}
			sentence={segment.sentence}
			dataUrl={segment.dataUrl}
		/>
	{/if}
{/each}
