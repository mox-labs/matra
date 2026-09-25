import { layoutMark } from '$lib/mark';
import { nav } from '$lib/server/content';
import { motto } from '$lib/server/motto';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = () => ({
	nav,
	// The mark in the header, computed once from matra's parse of the motto,
	// and the parse itself for the home page's hero.
	glyph: layoutMark(motto().tokens, 'glyph'),
	motto: motto().tokens
});
