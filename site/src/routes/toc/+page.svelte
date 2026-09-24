<script lang="ts">
	import NavTree from '$lib/components/NavTree.svelte';
	import { SITE_NAME } from '$lib/site';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>Contents · {SITE_NAME}</title>
</svelte:head>

<div class="standalone prose">
	<h1>Contents</h1>
	<NavTree nav={data.nav} current={null} />
</div>

<style>
	.standalone {
		max-width: var(--measure);
		margin-inline: auto;
		padding: var(--space-5) var(--gutter);
	}
</style>
