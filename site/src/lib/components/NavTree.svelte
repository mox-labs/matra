<script lang="ts">
	import { base } from '$app/paths';
	import type { NavItem, NavPart } from '$lib/types';

	let { nav, current }: { nav: NavPart[]; current: string | null } = $props();
</script>

{#snippet items(list: NavItem[])}
	<ul>
		{#each list as item (item.route)}
			<li>
				<a href="{base}{item.route}" aria-current={item.route === current ? 'page' : undefined}
					>{item.title}</a
				>
				{#if item.children.length > 0}
					{@render items(item.children)}
				{/if}
			</li>
		{/each}
	</ul>
{/snippet}

<div class="nav-tree">
	{#each nav as part, i (i)}
		<div class="part">
			{#if part.title}
				<p class="part-title">{part.title}</p>
			{/if}
			{@render items(part.items)}
		</div>
	{/each}
</div>

<style>
	.part + .part {
		margin-top: var(--space-4);
	}

	.part-title {
		margin: 0 0 var(--space-2);
		padding-inline-start: 0.6rem;
		font-size: 0.72rem;
		font-weight: 650;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	ul ul {
		padding-inline-start: var(--space-3);
	}

	a {
		display: block;
		padding: 0.3rem 0.6rem;
		border-radius: 6px;
		color: var(--text);
		text-decoration: none;
		font-size: 0.94rem;
		line-height: 1.4;
	}

	a:hover {
		background: var(--bg-subtle);
	}

	a[aria-current='page'] {
		background: var(--accent-soft);
		color: var(--accent);
		font-weight: 600;
	}
</style>
