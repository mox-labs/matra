/**
 * The mark's geometry keeps the rules that make it read as the mark.
 *
 * Gate 8 guards what the mark is drawn from (the committed parse of the
 * specimen); this guards how it is drawn, over every variant, from that same
 * data:
 *
 *   - the bar is a headline, never a strikethrough: in the full mark the
 *     tallest letter's top touches the bar's lower edge, so no word crosses
 *     it, and the words end above the arcs;
 *   - everything hangs: no stroke starts above the bar, and the root starts
 *     on it;
 *   - the favicon fits its 16px square, keeps at least two arcs, since
 *     one arc under a bar reads as a letter U, not as the mark, and leaves at
 *     least a pixel between its strokes, which otherwise fuse into a block;
 *   - the hero's bar sits inside the words' line box.
 *
 * Runs from `bun run check`, so in gate 4 of the docsite floor.
 *
 * Usage: bun scripts/check-mark.ts
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { ALEGREYA_BLACK, inkTop, PLEX_MONO, tallest } from '../src/lib/fonts';
import { layoutMark, specimenTokens, type Variant } from '../src/lib/mark';
import type { ParseFigureFile } from '../src/lib/types';

const SITE = resolve(import.meta.dir, '..');
const file = JSON.parse(readFileSync(resolve(SITE, 'src/lib/figures/specimen/parse.json'), 'utf8')) as ParseFigureFile;
const tokens = specimenTokens(file);
const text = tokens.map((t) => t.text).join('');
const EPS = 1e-9;

const failures: string[] = [];
const check = (ok: boolean, what: string) => {
	if (!ok) failures.push(what);
};

for (const variant of ['glyph', 'full', 'favicon', 'mono', 'paper'] as Variant[]) {
	const m = layoutMark(tokens, variant);
	const barBottom = m.bar.y + m.weight.bar / 2;
	for (const s of m.strokes) {
		check(s.y1 >= m.bar.y - EPS, `${variant}: stroke ${s.id} starts above the bar (${s.y1} < ${m.bar.y})`);
		if (s.root) check(Math.abs(s.y1 - m.bar.y) < EPS, `${variant}: the root does not start on the bar`);
	}
	check(m.strokes.some((s) => s.root), `${variant}: no root stroke`);

	if (variant === 'full') {
		const size = m.wordSize ?? 0;
		const top = tallest(PLEX_MONO, text);
		for (const w of m.words) {
			const wordTop = w.y - size * top;
			check(wordTop >= barBottom - EPS, `full: "${w.text}" rises through the bar (top ${wordTop.toFixed(2)}, bar ${barBottom})`);
		}
		const baseline = Math.max(...m.words.map((w) => w.y));
		// The words hang from the bar: the tallest letter's top is on it.
		check(
			Math.abs(baseline - size * top - barBottom) < 0.01,
			`full: the words do not hang from the bar (tallest top ${(baseline - size * top).toFixed(2)}, bar ${barBottom})`
		);
		const arcsStart = Math.min(...m.strokes.filter((s) => !s.root).map((s) => s.y1));
		check(baseline + size * PLEX_MONO.descent <= arcsStart + EPS, 'full: the words reach down into the arcs');
	}

	if (variant === 'favicon') {
		const half = Math.max(m.weight.bar, m.weight.stroke, m.weight.root) / 2;
		check(m.bar.x1 - half >= 0 && m.bar.x2 + half <= 16, 'favicon: the bar runs out of the 16px square');
		check(m.bar.y - half >= 0, 'favicon: the bar is cut at the top');
		for (const s of m.strokes) check(s.y2 + half <= 16, `favicon: stroke ${s.id} runs out of the square`);
		for (const a of m.arcs) {
			const ys = [...a.d.matchAll(/-?\d+(?:\.\d+)?,(-?\d+(?:\.\d+)?)/g)].map((x) => Number(x[1]));
			// A cubic reaches three quarters of its handles' depth below its ends.
			const depth = ys[0] + ((Math.max(...ys) - ys[0]) * 3) / 4;
			check(depth + m.weight.arc / 2 <= 16, `favicon: the ${a.dep} arc hangs out of the square`);
		}
		const xs = m.strokes.map((s) => s.x).sort((p, q) => p - q);
		for (let i = 1; i < xs.length; i++)
			check(xs[i] - xs[i - 1] - m.weight.stroke >= 1, `favicon: strokes at ${xs[i - 1]} and ${xs[i]} touch; at 16px they fuse into a block`);
		check(m.arcs.length >= 2, 'favicon: one arc or none; under a bar that reads as a letter U, not as the mark');
	}
}

const heroTop = inkTop(ALEGREYA_BLACK, tallest(ALEGREYA_BLACK, text));
check(heroTop >= 0 && heroTop < 1, `hero: the bar (${heroTop.toFixed(3)}em) falls outside the words' line box`);

if (failures.length) {
	console.error(`FAIL: the mark breaks ${failures.length} of its rules:`);
	for (const f of failures) console.error(`  ${f}`);
	process.exit(1);
}
console.log(`PASS: the mark hangs from its bar in every variant (words touch it, none crosses it; favicon keeps ${layoutMark(tokens, 'favicon').arcs.length} arcs)`);
