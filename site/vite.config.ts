import { fileURLToPath } from 'node:url';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, searchForWorkspaceRoot } from 'vite';
import { devComments } from './src/lib/dev/comments/plugin.ts';

export default defineConfig({
	plugins: [
		sveltekit(),
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
			// ROADMAP.md, and the Blueprints part reads blueprints/ in place;
			// both sit outside this project. Setting `allow` replaces Vite's
			// default, so the project root is named too.
			allow: [searchForWorkspaceRoot(process.cwd()), '../ROADMAP.md', '../blueprints']
		}
	}
});
