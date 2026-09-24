import { giscusConfig } from '$lib/server/config';
import { loadDoc, pages } from '$lib/server/content';
import type { EntryGenerator, PageServerLoad } from './$types';

/** One route per SUMMARY.md entry. Nothing renders that is not listed there. */
export const entries: EntryGenerator = () => pages.map((p) => ({ path: p.route.slice(1) }));

export const load: PageServerLoad = async ({ params }) => ({
	doc: await loadDoc(`/${params.path}`),
	giscus: giscusConfig()
});
