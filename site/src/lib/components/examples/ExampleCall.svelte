<script lang="ts">
	/**
	 * A worked example's call in Rust, Python and the CLI (EP-0012, M6), as
	 * tabs over one example. Each panel is the file the examples gate runs,
	 * highlighted at build time. Without scripts there are no tabs: the three
	 * calls stand one after another, each labelled. Switching a tab moves
	 * nothing; the panel is simply replaced.
	 *
	 * An example with a start (the quick start and the tutorial) numbers the
	 * steps from nothing to the call: install, save the text if the page gives
	 * a line for it, run. It opens on the command line, the shortest of the
	 * three routes; the tabs stay in the same order everywhere.
	 */
	import { onMount } from 'svelte';
	import type { ExampleView } from '$lib/types';

	let { example }: { example: ExampleView } = $props();

	let js = $state(false);
	onMount(() => (js = true));
	// svelte-ignore state_referenced_locally
	let selected = $state(example.calls[0].start ? example.calls.findIndex((c) => c.lang === 'cli') : 0);
	const tabs: HTMLButtonElement[] = $state([]);
	const id = $derived(`ex-${example.name}-call`);

	function onkeydown(e: KeyboardEvent) {
		const n = example.calls.length;
		const to =
			e.key === 'ArrowRight'
				? (selected + 1) % n
				: e.key === 'ArrowLeft'
					? (selected - 1 + n) % n
					: e.key === 'Home'
						? 0
						: e.key === 'End'
							? n - 1
							: null;
		if (to === null) return;
		e.preventDefault();
		selected = to;
		tabs[to]?.focus();
	}
</script>

<div class="ex-call" class:js data-example={example.name} data-part="call">
	{#if js}
		<div class="tablist" role="tablist" aria-label="The call, in each language">
			{#each example.calls as call, i (call.lang)}
				<button
					bind:this={tabs[i]}
					role="tab"
					id="{id}-tab-{call.lang}"
					aria-selected={selected === i}
					aria-controls="{id}-panel-{call.lang}"
					tabindex={selected === i ? 0 : -1}
					onclick={() => (selected = i)}
					{onkeydown}>{call.label}</button
				>
			{/each}
		</div>
	{/if}
	{#each example.calls as call, i (call.lang)}
		<div
			class="panel"
			id="{id}-panel-{call.lang}"
			data-lang={call.lang}
			role={js ? 'tabpanel' : undefined}
			aria-labelledby={js ? `${id}-tab-${call.lang}` : undefined}
			hidden={js && selected !== i}
		>
			{#if !js}<p class="label">{call.label}</p>{/if}
			{#if call.start}
				<p class="step"><span class="n">1</span>Install</p>
				{@html call.start.installHtml}
				{#if call.start.inputHtml}
					<p class="step"><span class="n">2</span>Save the text</p>
					{@html call.start.inputHtml}
				{/if}
				<p class="step">
					<span class="n">{call.start.inputHtml ? 3 : 2}</span>Run{#if call.start.save}: save this as <code>{call.start.save}</code>{#if call.start.runHtml},
							then run it{/if}{/if}
				</p>
			{/if}
			{@html call.html}
			{#if call.start?.runHtml}{@html call.start.runHtml}{/if}
			{#if call.lang === 'cli'}
				<p class="prints">It prints the output below, inside an envelope that names the command and the input:</p>
				{@html example.cliEnvelopeHtml}
			{/if}
		</div>
	{/each}
</div>

<style>
	.tablist {
		display: flex;
		gap: 0.25rem;
		border-bottom: 1px solid var(--border);
		margin-bottom: 0.6rem;
	}

	.tablist button {
		font: 600 var(--type-sm) var(--font-mono);
		color: var(--text-muted);
		background: none;
		border: 0;
		border-bottom: 2px solid transparent;
		padding: 0.45rem 0.8rem;
		margin-bottom: -1px;
		cursor: pointer;
	}

	.tablist button[aria-selected='true'] {
		color: var(--text);
		border-bottom-color: var(--accent);
	}

	.tablist button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.label {
		margin: 1rem 0 0.3rem;
		font-size: 0.72rem;
		font-weight: 650;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-muted);
	}

	.step {
		display: flex;
		align-items: baseline;
		gap: 0.5em;
		margin: 0.9rem 0 0.35rem;
		font-size: 0.9rem;
		color: var(--text-muted);
	}

	.step:first-of-type {
		margin-top: 0;
	}

	.step .n {
		font: 600 var(--type-xs) var(--font-mono);
		color: var(--text);
	}

	.prints {
		margin: 0.6rem 0 0.3rem;
		font-size: 0.9rem;
		color: var(--text-muted);
	}
</style>
