/**
 * Vertical metrics of the self-hosted faces, in em, read from the files in
 * static/fonts/ with fontTools: `ascent` and `descent` from the hhea table
 * (each face sets USE_TYPO_METRICS, and its typo values equal these), the
 * letter tops from the glyphs' own bounds.
 *
 * They are what lets a headline bar sit on the tops of the letters, the way a
 * Devanagari headline does, with no script measuring text: in a line box of
 * a known line-height, the baseline and every letter's top are arithmetic.
 * If a font file is replaced, re-read them:
 *
 *   the command is in site/README.md, under "The mark"
 */

export interface FaceMetrics {
	/** hhea ascent: the top of the content area above the baseline. */
	ascent: number;
	/** hhea descent, as a positive distance below the baseline. */
	descent: number;
	/** The top of the tallest lowercase letter (b d f h k l, or i's dot). */
	ascender: number;
	/** The top of the capitals and of `t` where it reaches it. */
	capHeight: number;
}

/** Alegreya ExtraBold (800), the motto's face in the home page's hero. l, f and d reach 0.742; A 0.655. */
export const ALEGREYA_BLACK: FaceMetrics = { ascent: 1.016, descent: 0.345, ascender: 0.742, capHeight: 0.637 };

/**
 * IBM Plex Mono, 400 and 600: the mark's words and the wordmark. i's dot
 * reaches 0.75 (0.748 at 400); b d f h k l reach 0.74; capitals and t 0.698.
 */
export const PLEX_MONO: FaceMetrics & { advance: number } = {
	ascent: 1.025,
	descent: 0.275,
	ascender: 0.75,
	capHeight: 0.698,
	advance: 0.6
};

/**
 * How far below the top of a line box of line-height `lh` (in em) a letter
 * whose top is `top` above the baseline begins: the half-leading, plus the
 * ascent, less the letter's height.
 */
export function inkTop(face: FaceMetrics, top: number, lh = 1): number {
	return (lh - (face.ascent + face.descent)) / 2 + face.ascent - top;
}

/** The tallest letter's top in `text`, for a face with these metrics. */
export function tallest(face: FaceMetrics, text: string): number {
	if (/[bdfhkli]/.test(text)) return face.ascender;
	if (/[A-Z0-9t]/.test(text)) return face.capHeight;
	throw new Error(`no ascender or capital in "${text}": the bar would float above it`);
}
