import adapter from '@sveltejs/adapter-static';

/**
 * GitHub Pages serves the site under /matra; local dev and the docsite floor
 * serve it at /. BASE_PATH carries the difference, and
 * scripts/verify-base-path.ts fails the build when the emitted asset URLs do
 * not agree with it.
 */
const base = process.env.BASE_PATH ?? '';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
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
		}
	}
};

export default config;
