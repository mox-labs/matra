import { layoutMark } from '$lib/mark';
import { nav } from '$lib/server/content';
import { specimen } from '$lib/server/specimen';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = () => ({
	nav,
	// The mark in the header, computed once from matra's parse of the specimen,
	// and the parse itself for the home page's hero.
	glyph: layoutMark(specimen().tokens, 'glyph'),
	specimen: specimen().tokens
});
