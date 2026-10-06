import { fileURLToPath } from 'node:url';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, searchForWorkspaceRoot } from 'vite';
import { devComments } from './src/lib/dev/comments/plugin.ts';

/**
 * GitHub Pages serves the site under /matra; local dev and the docsite floor
 * serve it at /. BASE_PATH carries the difference, and
 * scripts/verify-base-path.ts fails the build when the emitted asset URLs do
 * not agree with it. The type is SvelteKit's for `paths.base`, which it
 * checks when the config loads: empty, or starting with a slash.
 */
const base = (process.env.BASE_PATH ?? '') as '' | `/${string}`;

export default defineConfig({
	plugins: [
		// SvelteKit's configuration. Since SvelteKit 3 it is passed here rather
		// than read from svelte.config.js, which is no longer supported.
		sveltekit({
			adapter: adapter({
				pages: 'build',
				assets: 'build',
				fallback: undefined,
				precompress: false,
				// Fail when any route could not be prerendered.
				strict: true
			}),
			paths: {
				base,
				// Absolute URLs throughout. Every page is prerendered for exactly
				// one base, and a link that means the same thing on every page is
				// easier to check than one that depends on where it sits.
				relative: false
			},
			prerender: {
				handleHttpError: ({ path, referrer, message }) => {
					// The API reference is rustdoc, assembled beside this site at
					// deploy time. It is not a route here, so the crawler cannot
					// reach it (it follows /api/ to a redirect to /api, then a 404).
					// Every other failure fails the build.
					if (path === `${base}/api` || path.startsWith(`${base}/api/`)) return;
					throw new Error(`${message} (from ${referrer ?? 'an entry'})`);
				},
				// A link to a heading that does not exist fails the build.
				handleMissingId: 'fail',
				handleEntryGeneratorMismatch: 'fail'
			},
			// No timed polling of _app/version.json. SvelteKit 3 polls hourly by
			// default to drive `updated` from $app/state, which this site never
			// reads; SvelteKit 2 did not poll. (It still checks once when a tab
			// regains focus, which no option turns off.)
			version: { pollInterval: 0 }
		}),
		// Local comments, for the dev server only (`apply: 'serve'`): the
		// static build never runs it. See site/README.md, Local comments.
		devComments({ repoRoot: fileURLToPath(new URL('..', import.meta.url)) })
	],
	server: {
		port: 3000,
		// Loopback only. The dev server writes comments into the repository,
		// so nothing on the network may reach it.
		host: 'localhost',
		fs: {
			// site/content/roadmap.md is a symlink to the repository's
			// ROADMAP.md, and the Blueprints and Lab areas read blueprints/ and lab/
			// in place; all sit outside this project. Setting `allow` replaces Vite's
			// default, so the project root is named too.
			allow: [searchForWorkspaceRoot(process.cwd()), '../ROADMAP.md', '../blueprints', '../lab']
		}
	}
});
