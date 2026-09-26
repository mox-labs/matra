<script lang="ts">
	import DocPage from '$lib/components/DocPage.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<DocPage doc={data.doc} giscus={null} home>
	{#snippet hero()}
		<Hero tokens={data.specimen} titleId={data.doc.titleId || 'matra'} />
	{/snippet}
</DocPage>
