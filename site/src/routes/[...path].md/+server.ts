import { legacyForwards } from '#lib/server/blueprints.ts';
import { allPages, markdownOf } from '#lib/server/content.ts';
import type { EntryGenerator, RequestHandler } from './$types';

/**
 * Every page's Markdown source, served beside its HTML: `guides/cli.md` next
 * to `guides/cli.html`. An agent reads the page as it was authored, with no
 * layout to strip. At a legacy record's old address, the twin says where the
 * record is now.
 */
export const prerender = true;

export const entries: EntryGenerator = () => [
	...allPages.map((p) => ({ path: p.route.slice(1) })),
	...legacyForwards.map((f) => ({ path: f.from.slice(1) }))
];

export const GET: RequestHandler = ({ params }) => {
	const route = `/${params.path}`;
	const legacy = legacyForwards.find((f) => f.from === route);
	const body = legacy
		? `# ${legacy.id} has moved\n\nThis address published one of matra's design records before 2026-10-04. ` +
			`The record is kept unchanged in the repository, at [\`${legacy.file}\`](${legacy.url}), ` +
			`and is no longer rendered on the site.\n`
		: markdownOf(route);
	return new Response(body, { headers: { 'content-type': 'text/markdown; charset=utf-8' } });
};
