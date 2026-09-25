<script lang="ts">
	import { onMount } from 'svelte';
	import { chooseTheme, initTheme, theme } from '$lib/theme.svelte';
	import Icon from './Icon.svelte';

	onMount(initTheme);

	const next = $derived(theme.current === 'dark' ? 'light' : 'dark');
	const name = $derived(next === 'light' ? 'paper' : 'the void');
</script>

<!-- Both icons are rendered and CSS shows the one for the painted theme, so the
     prerendered button is right before any script has run. -->
<button
	type="button"
	class="icon-button theme-toggle"
	onclick={() => chooseTheme(next)}
	aria-label="Switch to {name}"
	title="Switch to {name}"
>
	<span class="when-light"><Icon name="moon" /></span>
	<span class="when-dark"><Icon name="sun" /></span>
</button>

<style>
	span {
		display: inline-flex;
	}

	/* The void is the default, so the button shows the way to paper (a sun)
	   unless the page is on paper. */
	.when-light,
	:global(html[data-theme='light']) .when-dark {
		display: none;
	}

	:global(html[data-theme='light']) .when-light {
		display: inline-flex;
	}

	/* Without scripts the button can do nothing, so it is not shown. */
	:global(html:not([data-js])) .theme-toggle {
		display: none;
	}
</style>
