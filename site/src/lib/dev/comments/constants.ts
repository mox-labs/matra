/**
 * The names the dev server and the comments UI share: the endpoint's path,
 * and the event the server sends when a page's comments change. Client-safe
 * (no Node imports), so plugin.ts and DevComments.svelte both import it.
 * scripts/check-no-dev-comments.ts reads the build for the first.
 */
export const ENDPOINT = '/__comments';
export const CHANGED = 'matra:comments';
