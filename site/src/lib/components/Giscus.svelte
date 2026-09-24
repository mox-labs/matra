<script lang="ts">
	/**
	 * Comments on a page, stored as GitHub Discussions through giscus
	 * (giscus.app). Rendered only when the build sets both ids; see
	 * site/README.md for switching it on.
	 *
	 * The mapping is `pathname`. giscus strips the extension before matching,
	 * so `/matra/guides/cli` and `/matra/guides/cli.html` share one discussion.
	 * `strict` matches on a hash of that path rather than a fuzzy title search.
	 *
	 * Nothing loads from giscus.app until the section is near the viewport,
	 * and the theme follows the site's toggle through giscus's setConfig
	 * message.
	 */
	import { theme } from '$lib/theme.svelte';

	let { repo, repoId, categoryId }: { repo: string; repoId: string; categoryId: string } =
		$props();

	const ORIGIN = 'https://giscus.app';
	let host: HTMLDivElement | undefined = $state();
	let injected = $state(false);

	$effect(() => {
		if (!host || injected) return;
		const target = host;
		const observer = new IntersectionObserver(
			(entries) => {
				if (!entries.some((e) => e.isIntersecting)) return;
				observer.disconnect();
				const script = document.createElement('script');
				script.src = `${ORIGIN}/client.js`;
				script.async = true;
				script.crossOrigin = 'anonymous';
				Object.assign(script.dataset, {
					repo,
					repoId,
					categoryId,
					mapping: 'pathname',
					strict: '1',
					reactionsEnabled: '1',
					emitMetadata: '0',
					inputPosition: 'top',
					theme: theme.current,
					lang: 'en',
					loading: 'lazy'
				});
				target.append(script);
				injected = true;
			},
			{ rootMargin: '600px 0px' }
		);
		observer.observe(target);
		return () => observer.disconnect();
	});

	$effect(() => {
		const next = theme.current;
		const frame = host?.querySelector<HTMLIFrameElement>('iframe.giscus-frame');
		frame?.contentWindow?.postMessage({ giscus: { setConfig: { theme: next } } }, ORIGIN);
	});
</script>

<section class="comments" aria-labelledby="comments-heading" data-print="hide" data-pagefind-ignore>
	<h2 id="comments-heading">Comments</h2>
	<p class="note">
		Comments are GitHub Discussions on the matra repository. Sign in with GitHub to join in.
	</p>
	<div class="giscus" bind:this={host}></div>
</section>

<style>
	.comments {
		margin-top: var(--space-5);
		padding-top: var(--space-4);
		border-top: 1px solid var(--border);
	}

	h2 {
		margin: 0 0 var(--space-2);
		font-size: 1.2rem;
	}

	.note {
		margin: 0 0 var(--space-3);
		color: var(--text-muted);
		font-size: 0.92rem;
	}
</style>
