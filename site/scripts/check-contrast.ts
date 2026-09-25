/**
 * Every colour pair the site sets text or a mark in meets WCAG 2.2 AA, in
 * both themes, measured rather than assumed.
 *
 * Reads the brand tokens (src/lib/brand/mox.tokens.css) and the site's own
 * variables (src/app.css), resolves `var()` and `light-dark()` for each
 * theme, converts OKLCH to sRGB, and computes the WCAG contrast ratio of each
 * pair listed in PAIRS. Text needs 4.5:1, or 3:1 when large (SC 1.4.3); a
 * mark a reader must see, such as an arc or a focus ring, needs 3:1 against
 * what it sits on (SC 1.4.11). A colour with alpha is composited over the
 * background it is checked against.
 *
 * Runs from `bun run check`, which gate 4 of the docsite floor runs, so a
 * token change that breaks a pair fails the build.
 *
 * Usage: bun scripts/check-contrast.ts
 */
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const SITE = resolve(import.meta.dir, '..');
const SOURCES = ['src/lib/brand/mox.tokens.css', 'src/app.css'];

type Theme = 'dark' | 'light';
type RGBA = [number, number, number, number];

/** [foreground, background, minimum ratio, what it is]. */
const PAIRS: [string, string, number, string][] = [
	['--text', '--bg', 4.5, 'body text'],
	['--text', '--bg-raised', 4.5, 'text on a raised surface'],
	['--text', '--code-bg', 4.5, 'text in a code block'],
	['--text-muted', '--bg', 4.5, 'muted text: captions, chrome, sidenotes'],
	['--text-muted', '--bg-raised', 4.5, 'muted text on a raised surface'],
	['--text-muted', '--code-bg', 4.5, 'muted text in a code block (comments)'],
	['--spark', '--bg', 4.5, 'links and interaction (Spark)'],
	['--spark', '--bg-raised', 4.5, 'links on a raised surface'],
	['--spark-strong', '--bg', 4.5, 'links on hover'],
	['--emergence', '--bg', 4.5, 'inline code and success (Emergence)'],
	['--emergence', '--code-bg', 4.5, 'strings in a code block'],
	['--emergence', '--bg-raised', 4.5, 'Emergence on a raised surface'],
	['--temperance', '--bg', 4.5, 'warnings and errors (Temperance)'],
	['--temperance', '--bg-raised', 4.5, 'Temperance on a raised surface'],
	['--syntax-keyword', '--code-bg', 4.5, 'keywords in a code block'],
	['--mark', '--bg', 3, 'figure marks: arcs, bars, rules'],
	['--mark', '--bg-raised', 3, 'figure marks on a figure card'],
	['--mark-quiet', '--bg-raised', 3, 'secondary figure marks'],
	['--spark', '--spark-wash', 4.5, 'Spark text on its own highlight'],
	['--text', '--spark-wash', 4.5, 'text on a Spark highlight'],
	['--text', '--emergence-wash', 4.5, 'text on an Emergence highlight'],
	['--border-strong', '--bg', 3, 'control borders'],
	['--spark', '--bg', 3, 'the focus ring']
];

// ---------------------------------------------------------------------------

function declarations(css: string): Map<string, string> {
	const out = new Map<string, string>();
	// Only plain `:root { ... }` blocks: media queries (reduced motion, high
	// contrast) and theme attribute blocks are not the default resolution.
	const noComments = css.replace(/\/\*[\s\S]*?\*\//g, '');
	const blocks = noComments.matchAll(/(^|[};])\s*:root\s*\{([^}]*)\}/g);
	for (const [, , body] of blocks) {
		for (const m of body.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/gi)) out.set(m[1], m[2].trim());
	}
	return out;
}

const vars = new Map<string, string>();
for (const file of SOURCES) {
	for (const [k, v] of declarations(readFileSync(join(SITE, file), 'utf8'))) vars.set(k, v);
}

/** Split on top-level commas. */
function args(s: string): string[] {
	const out: string[] = [];
	let depth = 0;
	let cur = '';
	for (const ch of s) {
		if (ch === '(') depth++;
		if (ch === ')') depth--;
		if (ch === ',' && depth === 0) {
			out.push(cur.trim());
			cur = '';
		} else cur += ch;
	}
	out.push(cur.trim());
	return out;
}

function resolveValue(value: string, theme: Theme, seen: string[] = []): string {
	let v = value.trim();
	for (let guard = 0; guard < 20; guard++) {
		const ld = /^light-dark\((.*)\)$/s.exec(v);
		if (ld) {
			const [light, dark] = args(ld[1]);
			v = theme === 'light' ? light : dark;
			continue;
		}
		const ref = /^var\((--[a-z0-9-]+)(?:\s*,\s*(.*))?\)$/is.exec(v);
		if (ref) {
			if (seen.includes(ref[1])) throw new Error(`cycle through ${ref[1]}`);
			const next = vars.get(ref[1]) ?? ref[2];
			if (next === undefined) throw new Error(`${ref[1]} is not defined`);
			seen = [...seen, ref[1]];
			v = next.trim();
			continue;
		}
		return v;
	}
	throw new Error(`cannot resolve ${value}`);
}

function oklchToRgba(v: string): RGBA {
	const m = /^oklch\(\s*([\d.]+)%\s+([\d.]+)\s+([\d.]+)\s*(?:\/\s*([\d.]+%?))?\s*\)$/i.exec(v);
	if (!m) throw new Error(`not an oklch() colour: ${v}`);
	const L = Number(m[1]) / 100;
	const C = Number(m[2]);
	const h = (Number(m[3]) * Math.PI) / 180;
	const alpha = m[4] === undefined ? 1 : m[4].endsWith('%') ? Number(m[4].slice(0, -1)) / 100 : Number(m[4]);
	const a = C * Math.cos(h);
	const b = C * Math.sin(h);
	const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
	const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
	const s_ = L - 0.0894841775 * a - 1.291485548 * b;
	const l = l_ ** 3;
	const mm = m_ ** 3;
	const s = s_ ** 3;
	const lin = [
		4.0767416621 * l - 3.3077115913 * mm + 0.2309699292 * s,
		-1.2684380046 * l + 2.6097574011 * mm - 0.3413193965 * s,
		-0.0041960863 * l - 0.7034186147 * mm + 1.707614701 * s
	].map((c) => Math.min(1, Math.max(0, c)));
	// Gamma-encode, the space browsers composite alpha in.
	const enc = lin.map((c) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055));
	return [enc[0], enc[1], enc[2], alpha];
}

function over(fg: RGBA, bg: RGBA): RGBA {
	const a = fg[3];
	return [fg[0] * a + bg[0] * (1 - a), fg[1] * a + bg[1] * (1 - a), fg[2] * a + bg[2] * (1 - a), 1];
}

function luminance([r, g, b]: RGBA): number {
	const lin = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
	return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function colour(name: string, theme: Theme): RGBA {
	const raw = vars.get(name);
	if (raw === undefined) throw new Error(`${name} is not defined in ${SOURCES.join(' or ')}`);
	return oklchToRgba(resolveValue(raw, theme, [name]));
}

const failures: string[] = [];
const rows: string[] = [];
for (const theme of ['dark', 'light'] as Theme[]) {
	for (const [fgName, bgName, min, what] of PAIRS) {
		try {
			const bg = colour(bgName, theme);
			const bgSolid = bg[3] < 1 ? over(bg, colour('--bg', theme)) : bg;
			const fg = over(colour(fgName, theme), bgSolid);
			const [hi, lo] = [luminance(fg), luminance(bgSolid)].sort((x, y) => y - x);
			const ratio = (hi + 0.05) / (lo + 0.05);
			const line = `${theme.padEnd(5)} ${ratio.toFixed(2).padStart(6)}:1 (needs ${min}) ${fgName} on ${bgName}: ${what}`;
			rows.push(line);
			if (ratio < min) failures.push(line);
		} catch (e) {
			failures.push(`${theme} ${fgName} on ${bgName}: ${(e as Error).message}`);
		}
	}
}

if (process.argv.includes('--table')) console.log(rows.join('\n'));
if (failures.length) {
	console.error(`FAIL (contrast): ${failures.length} pair(s) below WCAG AA:\n  ${failures.join('\n  ')}`);
	process.exit(1);
}
console.log(`PASS (contrast): ${PAIRS.length} pairs meet WCAG AA in both themes`);
