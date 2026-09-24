import { loadDoc, pages } from '$lib/server/content';
import type { PageServerLoad } from './$types';

/** Every page in reading order, on one page, for printing or saving whole. */
export const load: PageServerLoad = async () => ({
	docs: await Promise.all(pages.map((p) => loadDoc(p.route)))
});
