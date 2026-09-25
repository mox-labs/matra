/**
 * The mark: matra's own parse of "Amplify radical nonconformity.", drawn.
 *
 * A horizontal headline bar is the shirorekha, the line Devanagari letters
 * hang from. Each token of the sentence hangs from it as a stroke (or, in the
 * full mark, as its word). The root, the one token matra says everything
 * depends on, hangs as a longer stroke in Emergence: a matra, the small vowel
 * sign that attaches to a letter and changes its sound without replacing it,
 * which is what matra does to text. Below, each dependency matra found hangs
 * as an arc from head to dependent, levelled so none collide.
 *
 * Nothing here is drawn by hand. The input is the committed figure data
 * site/src/lib/figures/motto/parse.json, which gate 8 regenerates and diffs,
 * so if matra's parse of the motto changes, the mark changes, and review sees
 * it. The geometry is on the 9-grid (the unit U is 9).
 *
 * Variants:
 *   glyph     bar, strokes and arcs; no words
 *   full      the words hang from the bar in the mono face, the root stroke
 *             after its word as the vowel sign sits after its consonant
 *   favicon   the reduced form for 16px: the bar, the root, and the root's
 *             dependents that are not punctuation, with their arcs
 *   mono      the glyph in one ink; hierarchy carried by opacity
 *   paper     the glyph in ink on paper
 */
import type { ParseFigureFile, ParseToken } from '$lib/types';

export const U = 9;

export type Variant = 'glyph' | 'full' | 'favicon' | 'mono' | 'paper';

export interface MarkArc {
	head: number;
	dependent: number;
	dep: string;
	level: number;
	/** Whether the arc leaves the root. */
	fromRoot: boolean;
	d: string;
}

export interface MarkStroke {
	id: number;
	x: number;
	y1: number;
	y2: number;
	root: boolean;
}

export interface MarkWord {
	id: number;
	text: string;
	x: number;
	y: number;
}

export interface MarkLayout {
	width: number;
	height: number;
	bar: { x1: number; x2: number; y: number };
	strokes: MarkStroke[];
	arcs: MarkArc[];
	words: MarkWord[];
	/** Stroke widths, in the layout's units. */
	weight: { bar: number; stroke: number; root: number; arc: number };
	/** The clear space the mark keeps on every side: the root stroke's length. */
	clearSpace: number;
	/** Font size of the words in the full mark. */
	wordSize?: number;
}

/** The motto's tokens, the first (only) sentence of the committed parse. */
export function mottoTokens(file: ParseFigureFile): ParseToken[] {
	const sentence = file.data.sentences[0];
	if (!sentence) throw new Error('the motto parse has no sentence; regenerate the figure data');
	return sentence.tokens;
}

/**
 * Levels, as the parse figure levels them: shorter spans first, each one
 * level above the highest already-placed arc whose span overlaps its own.
 */
function levelled(tokens: ParseToken[]): Omit<MarkArc, 'd'>[] {
	const root = tokens.find((t) => t.head === 0)?.id;
	const arcs = tokens
		.filter((t) => t.head !== 0)
		.map((t) => ({ head: t.head, dependent: t.id, dep: t.dep, fromRoot: t.head === root }))
		.sort(
			(p, q) =>
				Math.abs(p.head - p.dependent) - Math.abs(q.head - q.dependent) ||
				Math.min(p.head, p.dependent) - Math.min(q.head, q.dependent)
		);
	const placed: { lo: number; hi: number; level: number }[] = [];
	return arcs.map((a) => {
		const lo = Math.min(a.head, a.dependent);
		const hi = Math.max(a.head, a.dependent);
		let level = 1;
		for (const p of placed) if (p.lo < hi && lo < p.hi) level = Math.max(level, p.level + 1);
		placed.push({ lo, hi, level });
		return { ...a, level };
	});
}

/** A hanging arc between two x positions, `depth` below `y`. */
function hang(x1: number, x2: number, y: number, depth: number): string {
	// A cubic with both handles straight down: the curve reaches three
	// quarters of the handle length, so handles of 4/3 depth reach `depth`.
	const h = (depth * 4) / 3;
	return `M${x1},${y}C${x1},${y + h} ${x2},${y + h} ${x2},${y}`;
}

function glyphLayout(tokens: ParseToken[], keep?: Set<number>): MarkLayout {
	const shown = keep ? tokens.filter((t) => keep.has(t.id)) : tokens;
	const index = new Map(shown.map((t, i) => [t.id, i]));
	const step = 3 * U;
	const x = (id: number) => U * 2 + (index.get(id) ?? 0) * step;
	const barY = U;
	const foot = barY + 3 * U;
	const arcs = levelled(tokens)
		.filter((a) => index.has(a.head) && index.has(a.dependent))
		.map((a) => ({ ...a, d: hang(x(a.head), x(a.dependent), foot, a.level * U) }));
	const deepest = Math.max(0, ...arcs.map((a) => a.level));
	return {
		width: x(shown[shown.length - 1].id) + U * 2,
		height: foot + deepest * U + U,
		bar: { x1: U, x2: x(shown[shown.length - 1].id) + U, y: barY },
		strokes: shown.map((t) => ({
			id: t.id,
			x: x(t.id),
			y1: barY,
			y2: t.head === 0 ? foot : barY + 2 * U,
			root: t.head === 0
		})),
		arcs,
		words: [],
		weight: { bar: 2, stroke: 2, root: 2.5, arc: 1.5 },
		clearSpace: 3 * U
	};
}

/** The mono face's advance, as a fraction of its size (IBM Plex Mono). */
const MONO_ADVANCE = 0.6;
/** Its x-height, as a fraction of its size. */
const MONO_XHEIGHT = 0.516;

function fullLayout(tokens: ParseToken[]): MarkLayout {
	const size = 2 * U;
	const adv = size * MONO_ADVANCE;
	const barY = 2 * U;
	const baseline = barY + size * MONO_XHEIGHT;
	const foot = 4 * U + U;
	let cursor = 2 * U;
	const anchors = new Map<number, number>();
	const words: MarkWord[] = [];
	const strokes: MarkStroke[] = [];
	for (const [i, t] of tokens.entries()) {
		const punct = t.dep === 'punct';
		if (i > 0 && !punct) cursor += adv; // a space, but none before punctuation
		const w = [...t.text].length * adv;
		words.push({ id: t.id, text: t.text, x: cursor, y: baseline });
		if (t.head === 0) {
			// The vowel sign hangs just after its consonant.
			const sx = cursor + w + U / 2;
			anchors.set(t.id, sx);
			strokes.push({ id: t.id, x: sx, y1: barY, y2: foot, root: true });
			cursor = sx + U / 2;
		} else {
			anchors.set(t.id, cursor + w / 2);
			cursor += w;
		}
	}
	const arcs = levelled(tokens).map((a) => ({
		...a,
		d: hang(anchors.get(a.head)!, anchors.get(a.dependent)!, foot, a.level * U)
	}));
	const deepest = Math.max(0, ...arcs.map((a) => a.level));
	// Every non-root word's foot: a short tick from under its baseline to the arcs.
	for (const t of tokens) {
		if (t.head === 0) continue;
		strokes.push({ id: t.id, x: anchors.get(t.id)!, y1: foot - U / 2, y2: foot, root: false });
	}
	return {
		width: cursor + 2 * U,
		height: foot + deepest * U + U,
		bar: { x1: U, x2: cursor + U, y: barY },
		strokes,
		arcs,
		words,
		weight: { bar: 2, stroke: 1.5, root: 2.5, arc: 1.5 },
		clearSpace: 3 * U,
		wordSize: size
	};
}

/**
 * The favicon keeps what a 16px square can carry: the bar, the root, and the
 * root's dependents that are not punctuation. The rule reads the tree; for
 * the motto it keeps `Amplify` and its object `nonconformity`.
 */
function faviconLayout(tokens: ParseToken[]): MarkLayout {
	const root = tokens.find((t) => t.head === 0);
	if (!root) throw new Error('the motto parse has no root');
	const keep = new Set([root.id, ...tokens.filter((t) => t.head === root.id && t.dep !== 'punct').map((t) => t.id)]);
	const kept = tokens.filter((t) => keep.has(t.id));
	const n = kept.length;
	// A 16 x 16 square: the bar 2px from the top, strokes on whole pixels.
	const x = (i: number) => (n === 1 ? 8 : 4 + (i * 8) / (n - 1));
	const barY = 3;
	const foot = 9;
	const index = new Map(kept.map((t, i) => [t.id, i]));
	const arcs = levelled(tokens)
		.filter((a) => keep.has(a.head) && keep.has(a.dependent))
		.map((a) => ({ ...a, d: hang(x(index.get(a.head)!), x(index.get(a.dependent)!), foot, 3.5 * a.level) }));
	return {
		width: 16,
		height: 16,
		bar: { x1: 1.5, x2: 14.5, y: barY },
		strokes: kept.map((t, i) => ({
			id: t.id,
			x: x(i),
			y1: barY,
			y2: t.head === 0 ? foot : barY + 4,
			root: t.head === 0
		})),
		arcs,
		words: [],
		weight: { bar: 2, stroke: 1.5, root: 2, arc: 1.5 },
		clearSpace: 0
	};
}

export function layoutMark(tokens: ParseToken[], variant: Variant): MarkLayout {
	if (variant === 'full') return fullLayout(tokens);
	if (variant === 'favicon') return faviconLayout(tokens);
	return glyphLayout(tokens);
}

/** Inks for a standalone SVG file, where no page CSS reaches. */
export interface Palette {
	ink: string;
	root: string;
	/** Opacity of the non-root strokes and the arcs; 1 unless the mark is mono. */
	quiet: number;
	background?: string;
}

/**
 * The palettes, the brand tokens converted to hex (OKLCH to sRGB, the same
 * arithmetic as scripts/check-contrast.ts) so a file works where oklch() may
 * not: on the void, --dao-text oklch(92% 0 0) and --emergence-core
 * oklch(75% 0.20 145); on paper, --ink oklch(22% 0.01 240), the site's
 * paper-side Emergence oklch(47% 0.14 145) and --paper oklch(98% 0.002 240).
 */
export const PALETTES: Record<'void' | 'paper' | 'mono', Palette> = {
	void: { ink: '#e4e4e4', root: '#45cd55', quiet: 1 },
	paper: { ink: '#171b1f', root: '#106e20', quiet: 1, background: '#f7f9fa' },
	mono: { ink: 'currentColor', root: 'currentColor', quiet: 0.6 }
};

function esc(s: string): string {
	return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** The mark as a standalone SVG document. */
export function markSvg(layout: MarkLayout, palette: Palette, opts: { title?: string; scheme?: boolean } = {}): string {
	const { bar, strokes, arcs, words, weight } = layout;
	const title = opts.title ?? 'matra';
	const q = palette.quiet;
	// A favicon has no page around it: it follows the reader's scheme itself.
	const style = opts.scheme
		? `<style>.i{stroke:${PALETTES.paper.ink};fill:${PALETTES.paper.ink}}.r{stroke:${PALETTES.paper.root}}@media (prefers-color-scheme:dark){.i{stroke:${PALETTES.void.ink};fill:${PALETTES.void.ink}}.r{stroke:${PALETTES.void.root}}}</style>`
		: '';
	const ink = (cls: string) => (opts.scheme ? `class="${cls}"` : `stroke="${cls === 'r' ? palette.root : palette.ink}"`);
	const parts = [
		`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${layout.width} ${layout.height}" width="${layout.width}" height="${layout.height}" role="img" aria-label="${esc(title)}">`,
		`<title>${esc(title)}</title>`,
		style,
		palette.background ? `<rect width="100%" height="100%" fill="${palette.background}"/>` : '',
		`<g fill="none" stroke-linecap="square">`,
		`<line ${ink('i')} stroke-width="${weight.bar}" x1="${bar.x1}" x2="${bar.x2}" y1="${bar.y}" y2="${bar.y}"/>`,
		...strokes
			.filter((s) => !s.root)
			.map(
				(s) =>
					`<line ${ink('i')} stroke-width="${weight.stroke}" opacity="${q}" x1="${s.x}" x2="${s.x}" y1="${s.y1}" y2="${s.y2}"/>`
			),
		...arcs.map((a) => `<path ${ink('i')} stroke-width="${weight.arc}" opacity="${q === 1 ? 1 : 0.8}" d="${a.d}"/>`),
		...strokes
			.filter((s) => s.root)
			.map(
				(s) => `<line ${ink('r')} stroke-width="${weight.root}" x1="${s.x}" x2="${s.x}" y1="${s.y1}" y2="${s.y2}"/>`
			),
		`</g>`,
		...(words.length
			? [
					`<g ${opts.scheme ? 'class="i" stroke="none"' : `fill="${palette.ink}"`} font-family="'IBM Plex Mono', ui-monospace, monospace" font-size="${layout.wordSize}">`,
					...words.map((w) => `<text x="${w.x}" y="${w.y}">${esc(w.text)}</text>`),
					`</g>`
				]
			: []),
		`</svg>`
	];
	return parts.filter(Boolean).join('');
}
