<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import DocPage from '$lib/components/DocPage.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	/**
	 * Until EP-0012's M6 the front page was the introduction, so its section
	 * ids are published URLs (site/anchors.txt). The ids stay on this page,
	 * and a link to one is sent on to the same section of the introduction.
	 * Without scripts the link lands at the top of this page, whose
	 * navigation opens with the introduction.
	 */
	const MOVED = ['four-tiers-of-output', 'end-to-end'];

	onMount(() => {
		const id = decodeURIComponent(location.hash.slice(1));
		if (MOVED.includes(id)) location.replace(`${base}/introduction#${id}`);
	});
</script>

{#each MOVED as id (id)}<span {id} hidden></span>{/each}
<DocPage doc={data.doc} giscus={null} home>
	{#snippet hero()}
		<Hero tokens={data.motto} titleId={data.doc.titleId || 'matra'} />
	{/snippet}
</DocPage>
