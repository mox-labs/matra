import { giscusConfig } from '#lib/server/config.ts';
import { legacyForwards } from '#lib/server/blueprints.ts';
import { allPages, loadDoc } from '#lib/server/content.ts';
import type { EntryGenerator, PageServerLoad } from './$types';

/**
 * One route per page the site renders (the SUMMARY.md pages, the design
 * records and the Lab), and one per address a legacy record was published at,
 * which now says where the record is.
 */
export const entries: EntryGenerator = () => [
	...allPages.map((p) => ({ path: p.route.slice(1) })),
	...legacyForwards.map((f) => ({ path: f.from.slice(1) }))
];

export const load: PageServerLoad = async ({ params }) => {
	const route = `/${params.path}`;
	const legacy = legacyForwards.find((f) => f.from === route) ?? null;
	if (legacy) return { legacy, doc: null, giscus: null };
	return { legacy: null, doc: await loadDoc(route), giscus: giscusConfig() };
};
