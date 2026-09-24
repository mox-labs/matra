// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
	interface Window {
		/** Pagefind's default search UI, defined once pagefind-ui.js has loaded. */
		PagefindUI?: new (options: Record<string, unknown>) => unknown;
	}
}

export {};
