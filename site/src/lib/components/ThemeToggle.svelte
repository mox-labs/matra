<script lang="ts">
	import { onMount } from 'svelte';
	import { chooseTheme, initTheme, theme } from '$lib/theme.svelte';
	import Icon from './Icon.svelte';

	onMount(initTheme);

	const next = $derived(theme.current === 'dark' ? 'light' : 'dark');
</script>

<!-- Both icons are rendered and CSS shows the one for the painted theme, so the
     prerendered button is right before any script has run. -->
<button
	type="button"
	class="icon-button theme-toggle"
	onclick={() => chooseTheme(next)}
	aria-label="Switch to {next} theme"
	title="Switch to {next} theme"
>
	<span class="when-light"><Icon name="moon" /></span>
	<span class="when-dark"><Icon name="sun" /></span>
</button>

<style>
	span {
		display: inline-flex;
	}

	.when-dark,
	:global(html[data-theme='dark']) .when-light {
		display: none;
	}

	:global(html[data-theme='dark']) .when-dark {
		display: inline-flex;
	}

	/* Without scripts the button can do nothing, so it is not shown. */
	:global(html:not([data-js])) .theme-toggle {
		display: none;
	}
</style>
