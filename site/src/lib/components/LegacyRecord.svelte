<script lang="ts">
	/**
	 * The page at an address a legacy record was published at before
	 * 2026-10-04. The record is no longer rendered on the site; this says so
	 * and links it on GitHub, where it is kept unchanged.
	 */
	import { base } from '#lib/paths.ts';
	import { SITE_NAME } from '#lib/site.ts';
	import type { LegacyForward } from '#lib/server/blueprints.ts';

	let { legacy }: { legacy: LegacyForward } = $props();
</script>

<svelte:head>
	<title>{legacy.id}, moved · {SITE_NAME}</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<article class="legacy prose">
	<h1>{legacy.id} has moved</h1>
	<p>
		This address published one of matra's design records before 2026-10-04. The record is
		kept unchanged in the repository, at
		<a href={legacy.url}><code>{legacy.file}</code></a>, and is no longer rendered on this
		site.
	</p>
	<p>
		The process that replaced it, and the records written since, are in
		<a href="{base}/blueprints/index">Blueprints</a>.
	</p>
</article>

<style>
	.legacy {
		max-width: var(--measure);
		margin-inline: auto;
		padding: var(--space-4) var(--space-2) var(--space-6);
	}

	h1 {
		margin: 0 0 var(--space-2);
		font-family: var(--font-display);
		font-size: var(--type-xl);
		line-height: var(--leading-tight);
	}
</style>
