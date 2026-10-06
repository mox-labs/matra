import { resolve } from '$app/paths';

/**
 * The configured base path: `''` locally and `/matra` on GitHub Pages
 * (`paths.base` in vite.config.ts). It prefixes every absolute link the site
 * writes, and is passed to the Markdown renderer, which never imports
 * SvelteKit.
 *
 * SvelteKit 3 removed `base` from `$app/paths`. `resolve` prefixes a pathname
 * with the base and a slash, so resolving the empty pathname and dropping that
 * slash leaves the base. With `paths.relative` false, the value is the same
 * absolute base on the server and in the browser.
 */
export const base = resolve('').slice(0, -1);
