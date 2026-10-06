<script lang="ts">
	/**
	 * Local comments on a page, in the dev server only.
	 *
	 * The layout imports this component only under `import.meta.env.DEV`, so
	 * the static build holds none of it; scripts/check-no-dev-comments.ts
	 * fails the build if it does. It talks to `/__comments`, which only the dev
	 * server answers (./plugin.ts), and the comments themselves are files in
	 * the repository (`discussion/<route>.jsonl`), so any session can read
	 * them and Claude answers in them (`just comments`, `just comment-reply`).
	 *
	 *   select text     a "Comment" control appears under the selection
	 *   a thread        a marker in the margin, level with its quote; its
	 *                   popover shows the messages, a reply box and resolve
	 *   the panel      "Comments (n open)", fixed at the corner: every thread
	 *                   on the page, orphaned ones included, by keyboard too
	 *
	 * A quote is found again on every render (./anchor.ts). Bodies are text,
	 * set with Svelte's escaping, never as HTML. Nothing animates: a popover
	 * appears and goes. A marker lights its quote for a mouse that points at
	 * it, a keyboard that focuses it, and a finger that taps it (the figures'
	 * Linked); a click or tap also opens its thread.
	 */
	import { onMount, tick } from 'svelte';
	import { Linked } from '#lib/components/figures/linked.svelte.ts';
	import { anchor, describe, textIndex } from './anchor';
	import { CHANGED, ENDPOINT } from './constants';
	import type { Problem, Selector, Thread } from './model';

	let { route }: { route: string } = $props();


	let threads = $state<Thread[]>([]);
	let problems = $state<Problem[]>([]);
	/** Each thread's quote on the page as it is now, or null when it no longer occurs. */
	let ranges = $state.raw<Map<string, Range | null>>(new Map());
	let markers = $state<{ id: string; top: number; left: number }[]>([]);
	/** The live selection, if it is text of this page. */
	let selection = $state.raw<{ range: Range; top: number; left: number } | null>(null);
	let composing = $state<{ selector: Selector; top: number; left: number } | null>(null);
	let draft = $state('');
	let reply = $state('');
	let open = $state<string | null>(null);
	let panelOpen = $state(false);
	let showResolved = $state(false);
	let error = $state('');
	let narrow = $state(false);
	let busy = $state(false);

	const linked = new Linked<string>();
	let opener: HTMLElement | null = null;
	let popover = $state<HTMLElement | null>(null);

	const visible = $derived(threads.filter((t) => showResolved || t.status === 'open'));
	const openCount = $derived(threads.filter((t) => t.status === 'open').length);
	const current = $derived(threads.find((t) => t.id === open) ?? null);
	const orphaned = (t: Thread) => ranges.has(t.id) && ranges.get(t.id) === null;

	function root(): Element | null {
		return document.querySelector('article.doc .prose');
	}

	async function load() {
		try {
			const res = await fetch(`${ENDPOINT}?page=${encodeURIComponent(route)}`);
			const data = await res.json();
			if (!res.ok) throw new Error(data.error ?? res.statusText);
			threads = data.threads;
			problems = data.problems;
			error = '';
		} catch (err) {
			error = `comments could not be read: ${(err as Error).message}`;
		}
		await tick();
		reanchor();
	}

	async function post(message: Record<string, unknown>): Promise<boolean> {
		busy = true;
		try {
			const res = await fetch(ENDPOINT, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ page: route, ...message })
			});
			const data = await res.json();
			if (!res.ok) throw new Error(data.error ?? res.statusText);
			threads = data.threads;
			problems = data.problems;
			error = '';
			await tick();
			reanchor();
			return true;
		} catch (err) {
			error = `not saved: ${(err as Error).message}`;
			return false;
		} finally {
			busy = false;
		}
	}

	function reanchor() {
		const r = root();
		if (!r) return;
		const idx = textIndex(r);
		ranges = new Map(threads.map((t) => [t.id, anchor(r, t.selector, idx)]));
		place();
	}

	/** Where each marker sits: level with its quote, just right of the prose, inside the window. */
	function place() {
		const r = root();
		if (!r) return;
		const right = r.getBoundingClientRect().right;
		const max = document.documentElement.clientWidth - 36;
		const left = Math.max(4, Math.min(right + 8, max)) + window.scrollX;
		const placed: { id: string; top: number; left: number }[] = [];
		for (const t of visible) {
			const range = ranges.get(t.id);
			if (!range) continue;
			const rect = range.getClientRects()[0] ?? range.getBoundingClientRect();
			placed.push({ id: t.id, top: rect.top + window.scrollY - 2, left });
		}
		placed.sort((a, b) => a.top - b.top);
		// Two quotes on one line keep both markers: the second steps down.
		for (let i = 1; i < placed.length; i++) {
			placed[i].top = Math.max(placed[i].top, placed[i - 1].top + 32);
		}
		markers = placed;
		paint();
	}

	/** The quotes, lit with the CSS Custom Highlight API: nothing is added to the page's DOM. */
	function paint() {
		if (typeof CSS === 'undefined' || !('highlights' in CSS)) return;
		const all: Range[] = [];
		const active: Range[] = [];
		const lit = linked.active ?? open;
		for (const t of visible) {
			const range = ranges.get(t.id);
			if (!range) continue;
			(t.id === lit ? active : all).push(range);
		}
		CSS.highlights.set('dev-comment', new Highlight(...all));
		CSS.highlights.set('dev-comment-active', new Highlight(...active));
	}

	$effect(() => {
		void linked.active;
		void open;
		void showResolved;
		place();
	});

	function onSelection() {
		const sel = getSelection();
		const r = root();
		if (!sel || sel.isCollapsed || sel.rangeCount === 0 || !r) {
			selection = null;
			return;
		}
		const range = sel.getRangeAt(0);
		if (!r.contains(range.commonAncestorContainer)) {
			selection = null;
			return;
		}
		const rects = range.getClientRects();
		const last = rects[rects.length - 1] ?? range.getBoundingClientRect();
		const width = document.documentElement.clientWidth;
		selection = {
			range: range.cloneRange(),
			top: last.bottom + window.scrollY + 8,
			left: Math.max(8, Math.min(last.right - 48, width - 120)) + window.scrollX
		};
	}

	function startComment() {
		const r = root();
		if (!selection || !r) return;
		const selector = describe(r, selection.range);
		if (!selector) {
			error = 'the selection holds no text of the page';
			return;
		}
		opener = document.activeElement as HTMLElement | null;
		composing = { selector, top: selection.top, left: selection.left };
		draft = '';
		open = null;
		tick().then(() => popover?.querySelector('textarea')?.focus());
	}

	async function submitComment() {
		if (!composing || draft.trim() === '') return;
		const before = new Set(threads.map((t) => t.id));
		if (await post({ op: 'comment', selector: composing.selector, body: draft })) {
			composing = null;
			draft = '';
			getSelection()?.removeAllRanges();
			const made = threads.find((t) => !before.has(t.id));
			if (made) show(made.id);
		}
	}

	function show(id: string, from?: EventTarget | null) {
		if (from instanceof HTMLElement) opener = from;
		composing = null;
		open = id;
		reply = '';
		const range = ranges.get(id);
		if (range) {
			const rect = range.getBoundingClientRect();
			if (rect.top < 64 || rect.bottom > window.innerHeight - 64) {
				// Instant: the site moves nothing the reader did not move.
				window.scrollTo({ top: rect.top + window.scrollY - window.innerHeight / 3, behavior: 'instant' });
			}
		}
		tick().then(() => popover?.focus());
	}

	function close() {
		open = null;
		composing = null;
		opener?.focus();
		opener = null;
	}

	async function sendReply() {
		if (!current || reply.trim() === '') return;
		if (await post({ op: 'reply', thread: current.id, body: reply })) reply = '';
	}

	async function setStatus(status: 'resolved' | 'open') {
		if (current) await post({ op: 'status', thread: current.id, status });
	}

	/** Where a popover sits: under its marker on a wide window, a sheet along the bottom on a narrow one. */
	const popoverAt = $derived.by(() => {
		if (narrow) return null;
		if (composing) return { top: composing.top, left: composing.left };
		const m = markers.find((x) => x.id === open);
		if (!m) return null;
		// Under the quote's line, ending at the marker, so the quote stays in view.
		return { top: m.top + 36, left: Math.max(8, m.left + 28 - 352) };
	});

	const excerpt = (s: string, n: number) => (s.length > n ? `${s.slice(0, n - 1)}…` : s);
	const when = (iso: string) =>
		new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });

	function keys(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			if (open || composing) close();
			else if (panelOpen) panelOpen = false;
		}
	}

	onMount(() => {
		const style = document.createElement('style');
		style.dataset.devComments = '';
		style.textContent =
			'::highlight(dev-comment){background-color:var(--spark-wash)}' +
			'::highlight(dev-comment-active){background-color:var(--spark-wash);text-decoration:underline 2px var(--spark)}';
		document.head.append(style);

		const media = matchMedia('(max-width: 40rem)');
		const sync = () => {
			narrow = media.matches;
			place();
		};
		sync();
		media.addEventListener('change', sync);

		let frame = 0;
		const later = (fn: () => void) => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(fn);
		};
		const selectionChanged = () => later(onSelection);
		document.addEventListener('selectionchange', selectionChanged);

		// The page re-rendered (a source edit, a navigation): find every quote again.
		let timer: ReturnType<typeof setTimeout>;
		const main = document.querySelector('main') ?? document.body;
		const mutations = new MutationObserver(() => {
			clearTimeout(timer);
			timer = setTimeout(reanchor, 100);
		});
		mutations.observe(main, { childList: true, subtree: true, characterData: true });
		// The layout moved (fonts, a disclosure opened, the window resized).
		const sizes = new ResizeObserver(() => later(place));
		sizes.observe(document.body);

		const changed = (data: { page: string }) => {
			if (data.page === route) load();
		};
		import.meta.hot?.on(CHANGED, changed);

		return () => {
			style.remove();
			media.removeEventListener('change', sync);
			document.removeEventListener('selectionchange', selectionChanged);
			mutations.disconnect();
			sizes.disconnect();
			clearTimeout(timer);
			cancelAnimationFrame(frame);
			import.meta.hot?.off(CHANGED, changed);
			if (typeof CSS !== 'undefined' && 'highlights' in CSS) {
				CSS.highlights.delete('dev-comment');
				CSS.highlights.delete('dev-comment-active');
			}
		};
	});

	// A new page: its own comments.
	$effect(() => {
		void route;
		open = null;
		composing = null;
		threads = [];
		ranges = new Map();
		load();
	});
</script>

<svelte:window onkeydown={keys} />

<div class="dev-comments" data-dev-comments data-print="hide" data-pagefind-ignore>
	{#if selection && !composing}
		<button
			type="button"
			class="control dc-comment-here"
			style="top: {selection.top}px; left: {selection.left}px"
			onpointerdown={(e) => e.preventDefault()}
			onclick={startComment}>Comment</button
		>
	{/if}

	{#each markers as m (m.id)}
		{@const t = threads.find((x) => x.id === m.id)}
		{#if t}
			<button
				type="button"
				class="dc-marker"
				class:resolved={t.status === 'resolved'}
				class:lit={(linked.active ?? open) === t.id}
				style="top: {m.top}px; left: {m.left}px"
				aria-label="Thread on “{excerpt(t.selector.exact, 40)}”, {t.messages.length} {t.messages.length === 1
					? 'message'
					: 'messages'}, {t.status}"
				aria-expanded={open === t.id}
				{...linked.on(t.id)}
				onclick={(e) => (open === t.id ? close() : show(t.id, e.currentTarget))}
				>{t.messages.length}</button
			>
		{/if}
	{/each}

	{#if composing || current}
		<div
			class="dc-popover"
			class:dc-sheet={!popoverAt}
			style={popoverAt ? `top: ${popoverAt.top}px; left: ${popoverAt.left}px` : undefined}
			role="dialog"
			aria-label={composing ? 'New comment' : 'Comment thread'}
			tabindex="-1"
			bind:this={popover}
		>
			<div class="head">
				<span>{composing ? 'new comment' : `thread, ${current?.status}`}</span>
				<button type="button" class="control" onclick={close}>close</button>
			</div>
			{#if composing}
				<blockquote>{excerpt(composing.selector.exact, 240)}</blockquote>
				<form
					onsubmit={(e) => {
						e.preventDefault();
						submitComment();
					}}
				>
					<label for="dc-draft">Your comment</label>
					<textarea
						id="dc-draft"
						bind:value={draft}
						rows="4"
						onkeydown={(e) => (e.metaKey || e.ctrlKey) && e.key === 'Enter' && submitComment()}
					></textarea>
					<div class="actions">
						<button type="submit" class="control primary" disabled={busy || draft.trim() === ''}>Post</button>
						<button type="button" class="control" onclick={close}>Cancel</button>
					</div>
				</form>
			{:else if current}
				<blockquote>{excerpt(current.selector.exact, 240)}</blockquote>
				{#if orphaned(current)}
					<p class="note">This quote no longer occurs on the page. The thread is kept.</p>
				{/if}
				<ol class="messages">
					{#each current.messages as m (m.id)}
						<li>
							<p class="who"><strong>{m.author}</strong> <time datetime={m.created}>{when(m.created)}</time></p>
							<p class="body">{m.body}</p>
						</li>
					{/each}
				</ol>
				<form
					onsubmit={(e) => {
						e.preventDefault();
						sendReply();
					}}
				>
					<label for="dc-reply">Reply</label>
					<textarea
						id="dc-reply"
						bind:value={reply}
						rows="3"
						onkeydown={(e) => (e.metaKey || e.ctrlKey) && e.key === 'Enter' && sendReply()}
					></textarea>
					<div class="actions">
						<button type="submit" class="control primary" disabled={busy || reply.trim() === ''}>Reply</button>
						{#if current.status === 'open'}
							<button type="button" class="control" disabled={busy} onclick={() => setStatus('resolved')}
								>Resolve</button
							>
						{:else}
							<button type="button" class="control" disabled={busy} onclick={() => setStatus('open')}>Reopen</button>
						{/if}
					</div>
				</form>
			{/if}
			{#if error}<p class="error" role="alert">{error}</p>{/if}
		</div>
	{/if}

	{#if panelOpen}
		<section class="dc-panel" aria-labelledby="dc-panel-title">
			<div class="head">
				<h2 id="dc-panel-title">Comments on this page</h2>
				<button type="button" class="control" onclick={() => (panelOpen = false)}>close</button>
			</div>
			<p class="note">Local, in the dev server only: each is a line in <code>discussion{route}.jsonl</code>.</p>
			<div class="actions">
				<button type="button" class="control" disabled={!selection} onclick={startComment}
					>Comment on the selection</button
				>
				<label class="check"><input type="checkbox" bind:checked={showResolved} /> show resolved</label>
			</div>
			{#if error && !(composing || current)}<p class="error" role="alert">{error}</p>{/if}
			{#if problems.length}
				<p class="error">
					{problems.length}
					{problems.length === 1 ? 'line' : 'lines'} of the file could not be read: {problems
						.map((p) => `line ${p.line}, ${p.error}`)
						.join('; ')}
				</p>
			{/if}
			{#if visible.length === 0}
				<p class="note">No {showResolved ? '' : 'open '}threads. Select text on the page to start one.</p>
			{:else}
				<ul>
					{#each visible as t (t.id)}
						{@const last = t.messages[t.messages.length - 1]}
						<li>
							<button
								type="button"
								class="dc-row"
								class:lit={(linked.active ?? open) === t.id}
								aria-current={open === t.id ? 'true' : undefined}
								{...linked.on(t.id)}
								onclick={(e) => show(t.id, e.currentTarget)}
							>
								<span class="quote">“{excerpt(t.selector.exact, 90)}”</span>
								<span class="last"><strong>{last.author}:</strong> {excerpt(last.body, 120)}</span>
								<span class="tags"
									>{t.status}{#if orphaned(t)}, orphaned: the quote no longer occurs{/if}, {t.messages.length}
									{t.messages.length === 1 ? 'message' : 'messages'}</span
								>
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}

	<button type="button" class="control dc-toggle" aria-expanded={panelOpen} onclick={() => (panelOpen = !panelOpen)}
		>Comments ({openCount} open)</button
	>
</div>

<style>
	.dev-comments {
		font: var(--type-sm) / 1.5 var(--font-ui);
		color: var(--text);
	}

	.control {
		min-height: 2rem;
		min-width: 2rem;
		padding: 0 var(--space-1);
		font: 500 var(--type-sm) / 1 var(--font-ui);
		color: var(--text);
		background: var(--bg-raised);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		cursor: pointer;
	}

	.control:hover:not(:disabled) {
		color: var(--spark);
		border-color: var(--spark);
	}

	.control:disabled {
		color: var(--text-muted);
		cursor: default;
	}

	.control.primary {
		border-color: var(--spark);
	}

	.control:focus-visible,
	.dc-marker:focus-visible,
	.dc-row:focus-visible,
	.dc-popover:focus-visible,
	textarea:focus-visible {
		outline: 2px solid var(--spark);
		outline-offset: 2px;
	}

	.dc-comment-here {
		position: absolute;
		z-index: var(--z-popover);
	}

	/* A marker: the thread's message count, in the margin level with its quote.
	   At least 28px square, past the 24px target minimum, for a finger. */
	.dc-marker {
		position: absolute;
		z-index: var(--z-raised);
		width: 1.75rem;
		height: 1.75rem;
		padding: 0;
		font: 600 var(--type-xs) / 1 var(--font-mono);
		color: var(--spark);
		background: var(--bg-raised);
		border: 1px solid var(--spark);
		border-radius: var(--radius-control);
		cursor: pointer;
	}

	.dc-marker.resolved {
		color: var(--text-muted);
		border-color: var(--border-strong);
		border-style: dashed;
	}

	.dc-marker.lit,
	.dc-marker:hover {
		background: var(--spark-wash);
	}

	.dc-popover {
		position: absolute;
		z-index: var(--z-popover);
		width: min(22rem, calc(100vw - 2 * var(--space-2)));
		max-height: min(32rem, 80vh);
		overflow-y: auto;
		padding: var(--space-2);
		background: var(--bg-raised);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
	}

	/* On a phone the popover is a sheet along the bottom, the full width less
	   the gutters, so it never runs off the side. */
	.dc-popover.dc-sheet {
		position: fixed;
		inset: auto var(--space-2) var(--space-2);
		width: auto;
		max-height: 60vh;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		margin-bottom: var(--space-1);
		font: var(--type-xs) / 1.4 var(--font-mono);
		color: var(--text-muted);
	}

	.head h2 {
		margin: 0;
		font: 600 var(--type-sm) / 1.4 var(--font-ui);
		color: var(--text);
	}

	blockquote {
		margin: 0 0 var(--space-1);
		padding-inline-start: var(--space-1);
		border-inline-start: 2px solid var(--spark);
		font-family: var(--font-read);
		color: var(--text-muted);
	}

	.messages {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.messages li + li {
		margin-top: var(--space-1);
		padding-top: var(--space-1);
		border-top: 1px solid var(--border);
	}

	.who {
		margin: 0;
		font: var(--type-xs) / 1.4 var(--font-mono);
		color: var(--text-muted);
	}

	.who strong {
		color: var(--text);
	}

	/* Plain text, as written: line breaks kept, nothing read as markup. */
	.body {
		margin: var(--space-0-5) 0 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	form {
		margin-top: var(--space-1);
	}

	label {
		display: block;
		font: var(--type-xs) / 1.6 var(--font-mono);
		color: var(--text-muted);
	}

	textarea {
		box-sizing: border-box;
		width: 100%;
		padding: var(--space-1);
		font: var(--type-sm) / 1.5 var(--font-ui);
		color: var(--text);
		background: var(--bg);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		resize: vertical;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1);
		margin-top: var(--space-1);
	}

	.check {
		display: inline-flex;
		align-items: center;
		gap: var(--space-0-5);
		min-height: 2rem;
	}

	.note {
		margin: 0 0 var(--space-1);
		font-size: var(--type-xs);
		color: var(--text-muted);
	}

	.error {
		margin: var(--space-1) 0 0;
		font-size: var(--type-xs);
		color: var(--temperance);
	}

	.dc-toggle {
		position: fixed;
		right: var(--space-2);
		bottom: var(--space-2);
		z-index: var(--z-overlay);
		min-height: 2.75rem;
		font-family: var(--font-mono);
	}

	.dc-panel {
		position: fixed;
		right: var(--space-2);
		bottom: calc(var(--space-2) + 3.25rem);
		z-index: var(--z-overlay);
		width: min(24rem, calc(100vw - 2 * var(--space-2)));
		max-height: min(36rem, calc(100vh - var(--header-h) - 5rem));
		overflow-y: auto;
		padding: var(--space-2);
		background: var(--bg-raised);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
	}

	.dc-panel ul {
		margin: var(--space-1) 0 0;
		padding: 0;
		list-style: none;
	}

	.dc-panel li + li {
		margin-top: var(--space-1);
	}

	.dc-row {
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
		width: 100%;
		padding: var(--space-1);
		text-align: start;
		font: inherit;
		color: var(--text);
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		cursor: pointer;
	}

	.dc-row.lit,
	.dc-row:hover {
		border-color: var(--spark);
	}

	.dc-row[aria-current='true'] {
		background: var(--spark-wash);
	}

	.quote {
		font-family: var(--font-read);
	}

	.last {
		overflow-wrap: anywhere;
		color: var(--text-muted);
	}

	.tags {
		font: var(--type-xs) / 1.4 var(--font-mono);
		color: var(--text-muted);
	}
</style>
