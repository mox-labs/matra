import { layoutMark, U } from '$lib/mark';
import { specimen } from '$lib/server/specimen';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = () => {
	const { tokens, file } = specimen();
	const full = layoutMark(tokens, 'full');
	const glyph = layoutMark(tokens, 'glyph');
	return {
		tokens,
		generator: file.generator,
		layouts: {
			full,
			glyph,
			favicon: layoutMark(tokens, 'favicon')
		},
		rules: {
			unit: U,
			clearSpace: glyph.clearSpace,
			// The full mark's words may not fall below the 11px micro-label floor.
			fullMinWidth: Math.ceil((full.width * 11) / (full.wordSize ?? 18)),
			// Below 24px the glyph's strokes merge; the favicon form takes over.
			glyphMinHeight: 24
		}
	};
};
