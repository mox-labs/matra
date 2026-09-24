import { error } from '@sveltejs/kit';
import { examples, publishedFiles } from '$lib/server/examples';
import type { EntryGenerator, RequestHandler } from './$types';

/**
 * Each worked example's files, published so a reader can run the example
 * as the page shows it: the input under the file name the calls read, and
 * what the calls print, whole (`output.json`, `cli.json`).
 */
export const prerender = true;

export const entries: EntryGenerator = () =>
	[...examples.values()].flatMap((ex) =>
		Object.keys(publishedFiles(ex)).map((file) => ({ name: ex.name, file }))
	);

export const GET: RequestHandler = ({ params }) => {
	const ex = examples.get(params.name);
	const file = ex ? publishedFiles(ex)[params.file] : undefined;
	if (!file) error(404, 'No such example file');
	return new Response(file.body, { headers: { 'content-type': file.type } });
};
