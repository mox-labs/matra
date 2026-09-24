import { loadDoc, pages } from '$lib/server/content';
import type { PageServerLoad } from './$types';

/**
 * The home page is the first page in SUMMARY.md, as it was under mdBook. It
 * is also served at its own route, which is the canonical one.
 */
export const load: PageServerLoad = async () => ({ doc: await loadDoc(pages[0].route) });
