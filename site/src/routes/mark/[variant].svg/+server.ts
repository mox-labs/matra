import { error } from '@sveltejs/kit';
import { layoutMark, markSvg, PALETTES, type Palette, type Variant } from '$lib/mark';
import { motto } from '$lib/server/motto';
import type { EntryGenerator, RequestHandler } from './$types';

/**
 * Each variant of the mark as a file, for use off the site: generated from
 * the committed parse of the motto, like every figure.
 */
export const prerender = true;

const FILES: Record<string, { variant: Variant; palette: Palette }> = {
	full: { variant: 'full', palette: PALETTES.void },
	glyph: { variant: 'glyph', palette: PALETTES.void },
	favicon: { variant: 'favicon', palette: PALETTES.void },
	mono: { variant: 'mono', palette: PALETTES.mono },
	paper: { variant: 'paper', palette: PALETTES.paper },
	'full-paper': { variant: 'full', palette: PALETTES.paper }
};

export const entries: EntryGenerator = () => Object.keys(FILES).map((variant) => ({ variant }));

export const GET: RequestHandler = ({ params }) => {
	const file = Object.hasOwn(FILES, params.variant) ? FILES[params.variant] : undefined;
	if (!file) error(404, 'No such variant of the mark');
	const svg = markSvg(layoutMark(motto().tokens, file.variant), file.palette, {
		title: 'matra: its parse of "Amplify radical nonconformity."'
	});
	return new Response(svg, { headers: { 'content-type': 'image/svg+xml' } });
};
