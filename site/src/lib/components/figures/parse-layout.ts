/**
 * Layout for the dependency-parse figure: where each word sits, how high each
 * arc rises, where its label goes. Pure arithmetic over one sentence's tokens,
 * run at build time for the prerendered page and again in the browser when the
 * reader picks another sentence. Svelte draws the result; nothing here touches
 * the DOM.
 *
 * Words and labels are set in the monospace face, so their widths are known
 * from their length alone and the layout needs no font measurement: the
 * prerendered SVG is placed exactly as the browser will draw it.
 *
 * Arcs never collide. Each arc is given a level, one above the highest arc
 * whose span overlaps its own among those already placed, placing shorter arcs
 * first. Nested arcs therefore sit inside the arcs that contain them, arcs
 * that cross (a non-projective parse) rise to different heights, and no two
 * labels share a height where their arcs overlap. Words are spaced so every
 * label fits inside its arc.
 */
import { scaleLinear } from 'd3-scale';
import type { ParseToken } from '$lib/types';

/** The type scale, in px. */
export const TYPE = { word: 14, pos: 10, rel: 10.5 } as const;

/** Advance width of the monospace face, as a fraction of its size. */
const ADVANCE = 0.6;
const WORD_GAP = 18;
const LABEL_PAD = 14;
const FOOT_OFFSET = 5;
const LEVEL_BASE = 30;
const LEVEL_STEP = 21;
const PAD_X = 16;
const ROOT_CLEARANCE = 26;

export function textWidth(text: string, size: number): number {
	return [...text].length * size * ADVANCE;
}

/**
 * Universal Dependencies groups relations by what they attach: core
 * arguments of a predicate, modifiers, and function words. The figure colours
 * arcs by that grouping, because it is what separates a clause's skeleton
 * from its trimmings; relations outside the three are drawn quietly.
 * https://universaldependencies.org/u/dep/
 */
export type Family = 'core' | 'modifier' | 'function' | 'other';

const FAMILIES: Record<string, Family> = {
	nsubj: 'core',
	obj: 'core',
	iobj: 'core',
	csubj: 'core',
	ccomp: 'core',
	xcomp: 'core',
	obl: 'modifier',
	vocative: 'modifier',
	expl: 'modifier',
	dislocated: 'modifier',
	advcl: 'modifier',
	advmod: 'modifier',
	discourse: 'modifier',
	nmod: 'modifier',
	appos: 'modifier',
	nummod: 'modifier',
	acl: 'modifier',
	amod: 'modifier',
	aux: 'function',
	cop: 'function',
	mark: 'function',
	det: 'function',
	clf: 'function',
	case: 'function',
	cc: 'function'
};

/** The family of a relation, read from its universal part (`nsubj:pass` is `nsubj`). */
export function family(dep: string): Family {
	return FAMILIES[dep.split(':')[0]] ?? 'other';
}

export const FAMILY_LABELS: Record<Family, string> = {
	core: 'Core argument',
	modifier: 'Modifier',
	function: 'Function word',
	other: 'Other'
};

export interface LaidToken {
	id: number;
	text: string;
	pos: string;
	x: number;
}

export interface LaidArc {
	/** The dependent's id: the arrow points at it. */
	dep: number;
	/** The head's id. */
	head: number;
	rel: string;
	family: Family;
	level: number;
	path: string;
	arrow: string;
	labelX: number;
	labelY: number;
}

export interface ParseLayout {
	width: number;
	height: number;
	/** Where the arcs meet the words. */
	baseline: number;
	wordY: number;
	posY: number;
	tokens: LaidToken[];
	arcs: LaidArc[];
	root: { id: number; x: number; top: number } | null;
}

export function layoutParse(tokens: ParseToken[]): ParseLayout {
	const n = tokens.length;
	const index = new Map(tokens.map((t, i) => [t.id, i]));
	const widths = tokens.map((t) => Math.max(textWidth(t.text, TYPE.word), textWidth(t.pos, TYPE.pos)));

	// Spacing between neighbouring word centres, widened until every label
	// fits inside its arc. Shortest arcs first: widening for them also widens
	// every arc that contains them.
	const gaps = widths.slice(1).map((w, i) => (widths[i] + w) / 2 + WORD_GAP);
	const spans = tokens
		.filter((t) => t.head !== 0 && index.has(t.head))
		.map((t) => {
			const a = index.get(t.id)!;
			const b = index.get(t.head)!;
			return { token: t, lo: Math.min(a, b), hi: Math.max(a, b) };
		})
		.sort((p, q) => p.hi - p.lo - (q.hi - q.lo) || p.lo - q.lo);
	for (const s of spans) {
		const need = textWidth(s.token.dep, TYPE.rel) + LABEL_PAD + 2 * FOOT_OFFSET;
		const have = gaps.slice(s.lo, s.hi).reduce((sum, g) => sum + g, 0);
		if (have < need) {
			const add = (need - have) / (s.hi - s.lo);
			for (let i = s.lo; i < s.hi; i++) gaps[i] += add;
		}
	}
	const xs = [PAD_X + widths[0] / 2];
	for (const g of gaps) xs.push(xs[xs.length - 1] + g);
	const width = n === 0 ? 0 : Math.ceil(xs[n - 1] + widths[n - 1] / 2 + PAD_X);

	// Levels: one above the highest already-placed arc whose span overlaps.
	const levels: number[] = [];
	const placed: { lo: number; hi: number; level: number }[] = [];
	for (const s of spans) {
		let level = 1;
		for (const p of placed) {
			if (p.lo < s.hi && s.lo < p.hi) level = Math.max(level, p.level + 1);
		}
		placed.push({ lo: s.lo, hi: s.hi, level });
		levels.push(level);
	}
	const maxLevel = Math.max(1, ...levels);
	const height = scaleLinear()
		.domain([1, 2])
		.range([LEVEL_BASE, LEVEL_BASE + LEVEL_STEP]);
	const tallest = height(maxLevel);
	const baseline = Math.ceil(tallest + ROOT_CLEARANCE + 18);

	const arcs: LaidArc[] = spans.map((s, i) => {
		const t = s.token;
		const d = index.get(t.id)!;
		const h = index.get(t.head)!;
		const x1 = xs[d];
		// The head end steps aside toward the dependent, so the arrows into a
		// word and the arcs out of it do not meet at one point.
		const x2 = xs[h] + (d < h ? -FOOT_OFFSET : FOOT_OFFSET);
		const rise = height(levels[i]);
		// A cubic whose control points sit at rise / 0.75 peaks at exactly
		// `rise` halfway across, where the label goes.
		const k = rise / 0.75;
		const y0 = baseline - 2;
		return {
			dep: t.id,
			head: t.head,
			rel: t.dep,
			family: family(t.dep),
			level: levels[i],
			path: `M${x1},${y0}C${x1},${y0 - k} ${x2},${y0 - k} ${x2},${y0}`,
			arrow: `M${x1 - 3.5},${y0 - 7}L${x1 + 3.5},${y0 - 7}L${x1},${y0 - 0.5}Z`,
			labelX: (x1 + x2) / 2,
			labelY: y0 - rise
		};
	});

	const rootToken = tokens.find((t) => t.head === 0);
	return {
		width,
		height: baseline + 44,
		baseline,
		wordY: baseline + 18,
		posY: baseline + 34,
		tokens: tokens.map((t, i) => ({ id: t.id, text: t.text, pos: t.pos, x: xs[i] })),
		arcs,
		root: rootToken
			? { id: rootToken.id, x: xs[index.get(rootToken.id)!], top: baseline - tallest - ROOT_CLEARANCE }
			: null
	};
}
