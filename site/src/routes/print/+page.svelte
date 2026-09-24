<script lang="ts">
	import Body from '$lib/components/Body.svelte';
	import { SITE_NAME } from '$lib/site';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>All pages · {SITE_NAME}</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<div class="print-page">
	<p class="print-note">Every page of this documentation, in reading order.</p>
	{#each data.docs as doc (doc.route)}
		<section class="prose print-section" aria-label={doc.title}>
			<h1>{@html doc.titleHtml}</h1>
			<Body segments={doc.segments} />
		</section>
	{/each}
</div>

<style>
	.print-page {
		max-width: var(--measure);
		margin-inline: auto;
		padding: var(--space-5) var(--gutter);
	}

	.print-note {
		color: var(--text-muted);
	}

	.print-section + .print-section {
		margin-top: var(--space-6);
		padding-top: var(--space-5);
		border-top: 1px solid var(--border);
		break-before: page;
	}
</style>
