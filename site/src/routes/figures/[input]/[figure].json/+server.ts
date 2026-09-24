import { error } from '@sveltejs/kit';
import { figures } from '$lib/server/content';
import type { EntryGenerator, RequestHandler } from './$types';

/**
 * Each figure's data, published beside the pages that draw it, exactly as the
 * generator wrote it: `figures/<input>/<figure>.json`. A figure links to its
 * file, so a reader or an agent can take the numbers rather than the picture.
 */
export const prerender = true;

export const entries: EntryGenerator = () =>
	[...figures.keys()].map((key) => {
		const [input, figure] = key.split('/');
		return { input, figure };
	});

export const GET: RequestHandler = ({ params }) => {
	const file = figures.get(`${params.input}/${params.figure}`);
	if (!file) error(404, 'No such figure');
	return new Response(`${JSON.stringify(file, null, 2)}\n`, {
		headers: { 'content-type': 'application/json; charset=utf-8' }
	});
};
