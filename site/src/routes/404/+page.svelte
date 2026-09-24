<script lang="ts">
	import { base } from '$app/paths';
	import NavTree from '$lib/components/NavTree.svelte';
	import { SITE_NAME } from '$lib/site';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>Page not found · {SITE_NAME}</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<div class="standalone prose">
	<h1>Page not found</h1>
	<p>
		Nothing is published at this address. The page may have moved; every page that exists is
		listed below, or start from the <a href="{base}/">introduction</a>.
	</p>
	<NavTree nav={data.nav} current={null} />
</div>

<style>
	.standalone {
		max-width: var(--measure);
		margin-inline: auto;
		padding: var(--space-5) var(--gutter);
	}
</style>
