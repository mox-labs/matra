<script lang="ts">
	import type { TocEntry } from '$lib/types';

	let { toc }: { toc: TocEntry[] } = $props();
</script>

<ul class="toc">
	{#each toc as entry (entry.id)}
		<li class:sub={entry.depth === 3}><a href="#{entry.id}">{entry.text}</a></li>
	{/each}
</ul>

<style>
	.toc {
		list-style: none;
		margin: 0;
		padding: 0;
		font-size: 0.86rem;
		line-height: 1.4;
	}

	li {
		margin: 0;
	}

	a {
		display: block;
		padding: 0.25rem 0 0.25rem 0.75rem;
		color: var(--text-muted);
		text-decoration: none;
		border-inline-start: 1px solid var(--border);
	}

	.sub a {
		padding-inline-start: 1.5rem;
	}

	a:hover {
		color: var(--text);
		border-inline-start-color: var(--accent);
	}
</style>
