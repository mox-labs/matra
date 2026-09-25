import { layoutMark, markSvg, PALETTES } from '$lib/mark';
import { motto } from '$lib/server/motto';
import type { RequestHandler } from './$types';

/**
 * The favicon: the mark's reduced form, generated from matra's parse of the
 * motto at build time. It follows the reader's colour scheme by itself,
 * since no page CSS reaches a favicon.
 */
export const prerender = true;

export const GET: RequestHandler = () =>
	new Response(markSvg(layoutMark(motto().tokens, 'favicon'), PALETTES.void, { title: 'matra', scheme: true }), {
		headers: { 'content-type': 'image/svg+xml' }
	});
