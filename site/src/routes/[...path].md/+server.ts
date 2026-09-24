import { markdownOf, pages } from '$lib/server/content';
import type { EntryGenerator, RequestHandler } from './$types';

/**
 * Every page's Markdown source, served beside its HTML: `guides/cli.md` next
 * to `guides/cli.html`. An agent reads the page as it was authored, with no
 * layout to strip.
 */
export const prerender = true;

export const entries: EntryGenerator = () => pages.map((p) => ({ path: p.route.slice(1) }));

export const GET: RequestHandler = ({ params }) =>
	new Response(markdownOf(`/${params.path}`), {
		headers: { 'content-type': 'text/markdown; charset=utf-8' }
	});
