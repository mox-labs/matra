import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, searchForWorkspaceRoot } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 3000,
		fs: {
			// site/content/roadmap.md is a symlink to the repository's
			// ROADMAP.md, which sits outside this project. Setting `allow`
			// replaces Vite's default, so the project root is named too.
			allow: [searchForWorkspaceRoot(process.cwd()), '../ROADMAP.md']
		}
	}
});
