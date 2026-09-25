import { error } from '@sveltejs/kit';
import { measures } from '$lib/server/content';
import type { EntryGenerator, RequestHandler } from './$types';

/**
 * matra's measures of each docs page, as the generator wrote them:
 * `figures/pages/<page>/measures.json`. A page's "measured by matra" line
 * links here, so the numbers in its margin can be checked.
 */
export const prerender = true;

export const entries: EntryGenerator = () =>
	[...measures.keys()].map((page) => ({ path: page.replace(/\.md$/, '') }));

export const GET: RequestHandler = ({ params }) => {
	const file = measures.get(`${params.path}.md`);
	if (!file) error(404, 'No measures for that page');
	return new Response(`${JSON.stringify(file, null, 2)}\n`, {
		headers: { 'content-type': 'application/json; charset=utf-8' }
	});
};
