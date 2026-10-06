<script lang="ts">
	/**
	 * The navigation, in mono: the apparatus, not the read. Each SUMMARY part
	 * carries a 3px rule by the role of what it holds ($lib/site PART_ROLES),
	 * named beside it so the colour is never the only signal. The current
	 * page is marked by shape (a rule and weight), and its sections are listed
	 * under it, so the page's map sits where the site's map is.
	 *
	 * The Blueprints part is the design records. It says so under its title,
	 * each record names its status beside it, and its two groups (the RFCs,
	 * the EPs) fold, opened on the group that holds the current page. Nothing
	 * else in the navigation carries a status. Under the current proposal or
	 * plan, its navigator (Read, Decide, Confirm) takes the place of the
	 * page's section list.
	 */
	import { base } from '$app/paths';
	import { PART_ROLES } from '$lib/site';
	import type { NavItem, NavPart, RecordFrame, TocEntry } from '$lib/types';
	import RecordNav from './RecordNav.svelte';

	let {
		nav,
		current,
		toc = [],
		frame
	}: { nav: NavPart[]; current: string | null; toc?: TocEntry[]; frame?: RecordFrame } = $props();

	const holds = (item: NavItem): boolean => item.children.some((c) => c.route === current || holds(c));
</script>

{#snippet items(list: NavItem[])}
	<ul>
		{#each list as item (item.route)}
			<li>
				{#if item.group}
					<details class="group" open={holds(item)}>
						<summary>{item.title} <span class="count">{item.children.length}</span></summary>
						{@render items(item.children)}
					</details>
				{:else}
				<a
					href="{base}{item.route}"
					aria-current={item.route === current ? 'page' : undefined}
					>{item.title}{#if item.status}<span class="status"><span class="visually-hidden">, status: </span>{item.status}</span>{/if}</a
				>
				{/if}
				{#if item.route === current && frame}
					<!-- On a proposal or a plan, its navigator: Read, Decide, Confirm. -->
					<div class="record-nav-slot">
						<RecordNav {frame} variant="sidebar" />
					</div>
				{:else if item.route === current && toc.length > 0}
					<ul class="sections" aria-label="On this page">
						{#each toc as entry (entry.id)}
							<li><a href="#{entry.id}">{entry.text}</a></li>
						{/each}
					</ul>
				{/if}
				{#if item.children.length > 0 && !item.group}
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
			{#if part.note}
				<p class="part-note">{part.note}</p>
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

	.part[data-role='record'] {
		border-inline-start-style: double;
		border-inline-start-color: var(--border-strong);
	}

	.part-note {
		margin: calc(-1 * var(--space-0-5)) 0 var(--space-0-5);
		font-size: var(--type-xs);
		color: var(--text-muted);
	}

	.group > summary {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		min-height: 24px;
		cursor: pointer;
		color: var(--text);
		line-height: 1.4;
	}

	.group > summary:hover {
		color: var(--spark);
	}

	.count {
		font-size: var(--type-xs);
		color: var(--text-muted);
	}

	/* A record's status, in words, under its title: the text is the signal. */
	.status {
		display: block;
		font-size: var(--type-xs);
		color: var(--text-muted);
		font-weight: 400;
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

	/* At least 24px tall, so the smaller section links still meet the WCAG
	   2.2 target size (2.5.8). */
	a {
		display: block;
		box-sizing: border-box;
		min-height: 24px;
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

	.record-nav-slot {
		margin: var(--space-1) 0 var(--space-2) var(--space-1);
	}

	/* The navigator's links keep their own size and padding. */
	.record-nav-slot :global(a) {
		margin-inline-start: 0;
		border-inline-start: 0;
		padding-inline-start: 0;
	}

	.sections a {
		color: var(--text-muted);
		font-size: var(--type-xs);
	}

	.sections a:hover {
		color: var(--spark);
	}
</style>
