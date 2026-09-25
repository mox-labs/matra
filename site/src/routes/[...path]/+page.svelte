<script lang="ts">
	import DocPage from '$lib/components/DocPage.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	/** The first page in SUMMARY.md is the home page, at `/` and at its own path. */
	const home = $derived(data.doc.route === data.nav[0]?.items[0]?.route);
</script>

{#if home}
	<DocPage doc={data.doc} giscus={null} home>
		{#snippet hero()}
			<Hero tokens={data.motto} titleId={data.doc.titleId || 'matra'} />
		{/snippet}
	</DocPage>
{:else}
	<DocPage doc={data.doc} giscus={data.giscus} />
{/if}
