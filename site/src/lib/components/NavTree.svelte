<script lang="ts">
	/**
	 * The navigation, in mono: the apparatus, not the read. Each SUMMARY part
	 * carries a 3px rule by the role of what it holds ($lib/site PART_ROLES),
	 * named beside it so the colour is never the only signal. The current
	 * page is marked by shape (a rule and weight), and its sections are listed
	 * under it, so the page's map sits where the site's map is.
	 */
	import { base } from '$app/paths';
	import { PART_ROLES } from '$lib/site';
	import type { NavItem, NavPart, TocEntry } from '$lib/types';

	let { nav, current, toc = [] }: { nav: NavPart[]; current: string | null; toc?: TocEntry[] } = $props();
</script>

{#snippet items(list: NavItem[])}
	<ul>
		{#each list as item (item.route)}
			<li>
				<a href="{base}{item.route}" aria-current={item.route === current ? 'page' : undefined}
					>{item.title}</a
				>
				{#if item.route === current && toc.length > 0}
					<ul class="sections" aria-label="On this page">
						{#each toc as entry (entry.id)}
							<li><a href="#{entry.id}">{entry.text}</a></li>
						{/each}
					</ul>
				{/if}
				{#if item.children.length > 0}
					{@render items(item.children)}
				{/if}
			</li>
		{/each}
	</ul>
{/snippet}

<div class="nav-tree">
	{#each nav as part, i (i)}
		<div class="part" data-role={part.title ? (PART_ROLES[part.title] ?? 'neutral') : 'none'}>
			{#if part.title}
				<p class="part-title">{part.title}</p>
			{/if}
			{@render items(part.items)}
		</div>
	{/each}
</div>

<style>
	.nav-tree {
		font-family: var(--font-mono);
		font-size: var(--type-sm);
	}

	.part {
		padding-inline-start: var(--space-1);
		border-inline-start: var(--border-accent) solid transparent;
	}

	.part + .part {
		margin-top: var(--space-3);
	}

	.part[data-role='neutral'] {
		border-inline-start-color: var(--border);
	}

	.part[data-role='spark'] {
		border-inline-start-color: var(--spark);
	}

	.part[data-role='emergence'] {
		border-inline-start-color: var(--emergence);
	}

	.part[data-role='planned'] {
		border-inline-start-style: dashed;
		border-inline-start-color: var(--border-strong);
	}

	.part-title {
		margin: 0 0 var(--space-0-5);
		font-family: var(--font-ui);
		font-size: var(--type-xs);
		font-weight: 600;
		letter-spacing: var(--tracking-widest);
		text-transform: uppercase;
		color: var(--text-muted);
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	ul ul {
		padding-inline-start: var(--space-2);
	}

	a {
		display: block;
		padding: 0.2rem 0 0.2rem var(--space-1);
		margin-inline-start: calc(-1 * var(--space-1));
		border-inline-start: 2px solid transparent;
		color: var(--text);
		text-decoration: none;
		line-height: 1.4;
	}

	a:hover {
		color: var(--spark);
	}

	a[aria-current='page'] {
		border-inline-start-color: var(--text);
		font-weight: 600;
	}

	.sections a {
		color: var(--text-muted);
		font-size: var(--type-xs);
	}

	.sections a:hover {
		color: var(--spark);
	}
</style>
