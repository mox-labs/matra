<script lang="ts">
	import DocPage from '$lib/components/DocPage.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import LegacyRecord from '$lib/components/LegacyRecord.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	/** The first page in SUMMARY.md is the home page, at `/` and at its own path. */
	const home = $derived(data.doc?.route === data.nav[0]?.items[0]?.route);
</script>

{#if data.legacy}
	<LegacyRecord legacy={data.legacy} />
{:else if data.doc && home}
	<DocPage doc={data.doc} giscus={null} home>
		{#snippet hero()}
			<Hero tokens={data.specimen} titleId={data.doc!.titleId || 'matra'} />
		{/snippet}
	</DocPage>
{:else if data.doc}
	<DocPage doc={data.doc} giscus={data.giscus} />
{/if}
