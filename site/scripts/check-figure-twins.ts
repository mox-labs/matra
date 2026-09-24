/**
 * Every figure has a text twin carrying the same data (EP-0012).
 *
 * A reader who cannot see the picture, and an agent that reads HTML rather
 * than pixels, get the figure's content from its twin. This reads the
 * prerendered HTML, as a browser with scripts off would, and for every
 * element marked `data-figure` finds its twin (`table[data-twin-for]`) and
 * compares the two. The twin is read from its visible cell text, not from
 * attributes, because the text is what a reader gets; the figure is read from
 * the data attributes on the marks it draws.
 *
 *   parse        every table row has a word with the same id and text, and
 *                an arc into it from the same head with the same relation;
 *                no word or arc the table lacks
 *   primitives   the same set of (sentence, primitive, read-from word,
 *                attaches-to word) in the table as marked in the sentences
 *   metrics      every paragraph's three measures agree to the table's
 *                precision, a gap exactly where the table says none, and
 *                the reference line matches the document value listed
 *   keyphrases   every phrase has the same RAKE and YAKE rank, or the same
 *                absence, in the table as in the slopegraph
 *   textrank     every sentence's score and summary membership agree
 *   clusters     at the threshold drawn, the same clusters and the same
 *                edges with the same scores as the table's row for it
 *   pipeline     every paragraph drawn at the annotate and compose stages
 *                has the table's sentence and token counts, no measures
 *                after annotate, and the table's measures after compose
 *
 * A figure with no twin fails, a kind this script cannot compare fails, and
 * so does a build with no figures at all: a check that examined nothing has
 * not passed.
 *
 * Usage: bun scripts/check-figure-twins.ts <build-dir>
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fromHtml } from 'hast-util-from-html';
import { toString } from 'hast-util-to-string';
import type { Element, Root } from 'hast';

const dir = process.argv[2];
if (!dir) {
	console.error('usage: bun scripts/check-figure-twins.ts <build-dir>');
	process.exit(2);
}

function pages(d: string): string[] {
	return readdirSync(d).flatMap((name) => {
		const path = join(d, name);
		if (statSync(path).isDirectory()) return name === '_app' || name === 'pagefind' ? [] : pages(path);
		return name.endsWith('.html') ? [path] : [];
	});
}

function all(node: Root | Element, test: (e: Element) => boolean, out: Element[] = []): Element[] {
	for (const child of node.children) {
		if (child.type !== 'element') continue;
		if (test(child)) out.push(child);
		all(child, test, out);
	}
	return out;
}

const hasClass = (e: Element, c: string) =>
	Array.isArray(e.properties.className) && e.properties.className.includes(c);
const prop = (e: Element, name: string) => {
	const v = e.properties[name];
	return v === undefined || v === null ? undefined : String(v);
};

/** The twin's body rows, each as its cells' visible text. */
function rows(table: Element): string[][] {
	return all(table, (e) => e.tagName === 'tr')
		.map((tr) => all(tr, (e) => e.tagName === 'td').map((td) => toString(td).trim()))
		.filter((cells) => cells.length > 0);
}

type Compare = (fig: Element, table: Element, fail: (msg: string) => void) => number;

const parse: Compare = (fig, table, fail) => {
	const id = prop(fig, 'id');
	const svg = all(fig, (e) => e.tagName === 'svg' && prop(e, 'dataArcsFor') === id)[0];
	if (!svg) {
		fail('no diagram (an svg with data-arcs-for)');
		return 0;
	}
	const twin = rows(table).map((c) => ({
		id: Number.parseInt(c[0], 10),
		text: c[1],
		head: Number.parseInt(c[4], 10),
		dep: c[5]
	}));
	const words = new Map(
		all(svg, (e) => e.tagName === 'text' && e.properties.dataId !== undefined).map((t) => [Number(prop(t, 'dataId')), toString(t)])
	);
	const arcs = new Map(
		all(svg, (e) => e.tagName === 'g' && e.properties.dataDep !== undefined).map((g) => {
			const label = all(g, (e) => e.tagName === 'text' && hasClass(e, 'rel'))[0];
			return [Number(prop(g, 'dataDep')), { head: Number(prop(g, 'dataHead')), dep: label ? toString(label) : '' }];
		})
	);
	if (twin.length === 0) fail('the twin table has no rows');
	for (const row of twin) {
		if (words.get(row.id) !== row.text) {
			fail(`token ${row.id} is "${row.text}" in the table, "${words.get(row.id) ?? 'absent'}" in the diagram`);
		}
		const arc = arcs.get(row.id);
		if (!arc) fail(`token ${row.id} "${row.text}" has no arc in the diagram`);
		else if (arc.head !== row.head || arc.dep !== row.dep) {
			fail(`token ${row.id} "${row.text}" is ${row.head}/${row.dep} in the table, ${arc.head}/${arc.dep} in the diagram`);
		}
	}
	const listed = new Set(twin.map((r) => r.id));
	for (const w of words.keys()) if (!listed.has(w)) fail(`word ${w} is drawn but not in the twin`);
	for (const a of arcs.keys()) if (!listed.has(a)) fail(`an arc into ${a} is drawn but not in the twin`);
	return twin.length;
};

const primitives: Compare = (fig, table, fail) => {
	const twin = new Set<string>();
	for (const c of rows(table)) {
		if (c.length < 4) continue; // "no primitive"
		const kind = c[1].toLowerCase().replace(/ /g, '_');
		const cue = Number.parseInt(c[2], 10);
		const head = c[3] === 'none' ? '-' : String(Number.parseInt(c[3], 10));
		twin.add(`${c[0]} ${kind} ${cue} ${head}`);
	}
	const drawn = new Set<string>();
	for (const li of all(fig, (e) => e.properties.dataSentence !== undefined && e.tagName === 'li')) {
		const n = prop(li, 'dataSentence');
		for (const mark of all(li, (e) => e.properties.dataPrim !== undefined)) {
			for (const item of (prop(mark, 'dataPrim') ?? '').split('|')) drawn.add(`${n} ${item}`);
		}
	}
	if (twin.size === 0) fail('the twin table lists no primitive');
	for (const t of twin) if (!drawn.has(t)) fail(`"${t}" is in the table but not marked`);
	for (const d of drawn) if (!twin.has(d)) fail(`"${d}" is marked but not in the table`);
	return twin.size;
};

const metrics: Compare = (fig, table, fail) => {
	const COLUMNS = ['readability_grade', 'lexical_density', 'compression_ratio'];
	const twin = new Map<string, number | null>();
	for (const c of rows(table)) {
		COLUMNS.forEach((m, k) => {
			const cell = c[3 + k];
			twin.set(`${c[0]} ${m}`, cell === 'none' ? null : Number.parseFloat(cell));
		});
	}
	const drawn = new Map<string, number | null>();
	for (const mark of all(fig, (e) => e.properties.dataMetric !== undefined && e.properties.dataParagraph !== undefined)) {
		const v = prop(mark, 'dataValue') ?? '';
		drawn.set(`${prop(mark, 'dataParagraph')} ${prop(mark, 'dataMetric')}`, v === '' ? null : Number.parseFloat(v));
	}
	if (twin.size === 0) fail('the twin table has no rows');
	for (const [key, t] of twin) {
		if (!drawn.has(key)) {
			fail(`${key} is in the table but not drawn`);
			continue;
		}
		const d = drawn.get(key) ?? null;
		// The table shows two decimals; the figure carries the data's four.
		if ((t === null) !== (d === null) || (t !== null && d !== null && Math.abs(t - d) > 0.005 + 1e-9)) {
			fail(`${key} is ${t ?? 'none'} in the table, ${d ?? 'none'} in the figure`);
		}
	}
	for (const key of drawn.keys()) if (!twin.has(key)) fail(`${key} is drawn but not in the table`);
	for (const ref of all(fig, (e) => e.properties.dataReference !== undefined)) {
		const docValue = all(fig, (e) => prop(e, 'dataDoc') === 'mean_readability')[0];
		const r = Number.parseFloat(prop(ref, 'dataValue') ?? '');
		const listed = docValue ? Number.parseFloat(toString(docValue)) : Number.NaN;
		if (!(Math.abs(r - listed) <= 0.005 + 1e-9)) fail(`the reference line is ${r}, the listed document value ${listed}`);
	}
	return twin.size;
};

const keyphrases: Compare = (fig, table, fail) => {
	const rank = (s: string) => (s === 'not ranked' || s === '' ? 'none' : String(Number.parseInt(s, 10)));
	const twin = new Map(rows(table).map((c) => [c[0], `${rank(c[1])}/${rank(c[3])}`]));
	const drawn = new Map(
		all(fig, (e) => e.properties.dataPhrase !== undefined).map((g) => [
			prop(g, 'dataPhrase') ?? '',
			`${rank(prop(g, 'dataRake') ?? '')}/${rank(prop(g, 'dataYake') ?? '')}`
		])
	);
	if (twin.size === 0) fail('the twin table has no rows');
	for (const [phrase, t] of twin) {
		const d = drawn.get(phrase);
		if (d === undefined) fail(`"${phrase}" is in the table but not drawn`);
		else if (d !== t) fail(`"${phrase}" ranks ${t} (RAKE/YAKE) in the table, ${d} in the figure`);
	}
	for (const phrase of drawn.keys()) if (!twin.has(phrase)) fail(`"${phrase}" is drawn but not in the table`);
	return twin.size;
};

const close = (a: number, b: number, places: number) => Math.abs(a - b) <= 0.5 * 10 ** -places + 1e-9;

const textrank: Compare = (fig, table, fail) => {
	const twin = new Map(
		rows(table).map((c) => [Number.parseInt(c[0], 10), { score: Number.parseFloat(c[2]), summary: c[3] === 'yes' }])
	);
	const drawn = new Map(
		all(fig, (e) => e.properties.dataPosition !== undefined).map((b) => [
			Number(prop(b, 'dataPosition')),
			{ score: Number.parseFloat(prop(b, 'dataScore') ?? ''), summary: prop(b, 'dataSummary') === 'yes' }
		])
	);
	if (twin.size === 0) fail('the twin table has no rows');
	for (const [pos, t] of twin) {
		const d = drawn.get(pos);
		if (!d) fail(`sentence ${pos} is in the table but has no bar`);
		else if (!close(t.score, d.score, 4) || t.summary !== d.summary) {
			fail(`sentence ${pos} is ${t.score}${t.summary ? ' (summary)' : ''} in the table, ${d.score}${d.summary ? ' (summary)' : ''} in the figure`);
		}
	}
	for (const pos of drawn.keys()) if (!twin.has(pos)) fail(`sentence ${pos} has a bar but no row`);
	return twin.size;
};

const clusters: Compare = (fig, table, fail) => {
	const svg = all(fig, (e) => e.properties.dataThreshold !== undefined)[0];
	if (!svg) {
		fail('no diagram carrying data-threshold');
		return 0;
	}
	const at = prop(svg, 'dataThreshold');
	const row = rows(table).find((c) => c[0] === at);
	if (!row) {
		fail(`the twin has no row for the threshold drawn, ${at}`);
		return 0;
	}
	const twinEdges = new Map(
		[...row[2].matchAll(/(\d+)-(\d+) \((\d\.\d+)\)/g)].map((m) => [`${m[1]}-${m[2]}`, Number.parseFloat(m[3])])
	);
	const twinClusters = new Set(
		[...row[1].matchAll(/\{([^}]*)\}/g)].map((m) =>
			m[1]
				.split(',')
				.map((x) => Number.parseInt(x, 10))
				.sort((a, b) => a - b)
				.join(',')
		)
	);
	const drawnEdges = new Map(
		all(svg, (e) => e.properties.dataA !== undefined).map((g) => [
			`${prop(g, 'dataA')}-${prop(g, 'dataB')}`,
			Number.parseFloat(prop(g, 'dataScore') ?? '')
		])
	);
	const groups = new Map<string, number[]>();
	for (const m of all(svg, (e) => e.properties.dataRow !== undefined)) {
		const k = prop(m, 'dataCluster') ?? '';
		if (k !== '') groups.set(k, [...(groups.get(k) ?? []), Number(prop(m, 'dataRow'))]);
	}
	const drawnClusters = new Set([...groups.values()].map((g) => g.sort((a, b) => a - b).join(',')));
	for (const [e, s] of twinEdges) {
		const d = drawnEdges.get(e);
		if (d === undefined) fail(`at ${at}, edge ${e} is in the table but not drawn`);
		else if (!close(s, d, 4)) fail(`at ${at}, edge ${e} is ${s} in the table, ${d} in the figure`);
	}
	for (const e of drawnEdges.keys()) if (!twinEdges.has(e)) fail(`at ${at}, edge ${e} is drawn but not in the table`);
	for (const c of twinClusters) if (!drawnClusters.has(c)) fail(`at ${at}, cluster {${c}} is in the table but not drawn`);
	for (const c of drawnClusters) if (!twinClusters.has(c)) fail(`at ${at}, cluster {${c}} is drawn but not in the table`);
	return twinEdges.size + twinClusters.size;
};

const pipeline: Compare = (fig, table, fail) => {
	const num = (s: string) => (s === 'none' || s === '' ? null : Number.parseFloat(s));
	const twin = new Map(
		rows(table).map((c) => [
			c[0],
			{ sentences: c[2], tokens: c[3], metrics: [num(c[4]), num(c[5]), num(c[6])] }
		])
	);
	let compared = 0;
	const stages = all(fig, (e) => e.properties.dataStage !== undefined);
	for (const stage of ['annotated', 'composed']) {
		if (!stages.some((s) => prop(s, 'dataStage') === stage)) fail(`the ${stage} stage is not in the prerendered figure`);
	}
	for (const s of stages) {
		const stage = prop(s, 'dataStage');
		for (const p of all(s, (e) => e.properties.dataParagraph !== undefined)) {
			const n = prop(p, 'dataParagraph') ?? '';
			const t = twin.get(n);
			if (!t) {
				fail(`${stage} paragraph ${n} is drawn but not in the table`);
				continue;
			}
			compared += 1;
			if (prop(p, 'dataSentences') !== t.sentences || prop(p, 'dataTokens') !== t.tokens) {
				fail(`${stage} paragraph ${n} has ${prop(p, 'dataSentences')}/${prop(p, 'dataTokens')} sentences/tokens, the table ${t.sentences}/${t.tokens}`);
			}
			const drawn = ['dataGrade', 'dataDensity', 'dataCompression'].map((k) => num(prop(p, k) ?? ''));
			drawn.forEach((d, k) => {
				const expected = stage === 'annotated' ? null : t.metrics[k];
				if ((d === null) !== (expected === null) || (d !== null && expected !== null && !close(d, expected, 2))) {
					fail(`${stage} paragraph ${n} measure ${k + 1} is ${d ?? 'none'} in the figure, ${expected ?? 'none'} expected from the table`);
				}
			});
		}
	}
	if (twin.size === 0) fail('the twin table has no rows');
	return compared;
};

const COMPARE: Record<string, Compare> = {
	parse,
	primitives,
	metrics,
	keyphrases,
	textrank,
	clusters,
	pipeline
};

const failures: string[] = [];
const byKind = new Map<string, number>();
let figures = 0;
let records = 0;
const withFigures = new Set<string>();

for (const file of pages(dir)) {
	const page = relative(dir, file);
	const tree = fromHtml(readFileSync(file, 'utf8'));
	for (const fig of all(tree, (e) => e.properties.dataFigure !== undefined)) {
		figures += 1;
		withFigures.add(page);
		const kind = prop(fig, 'dataFigure') ?? '';
		byKind.set(kind, (byKind.get(kind) ?? 0) + 1);
		const id = prop(fig, 'id') ?? '';
		const where = `${page}#${id} (${kind})`;
		const fail = (msg: string) => failures.push(`${where}: ${msg}`);
		const table = all(fig, (e) => e.tagName === 'table' && prop(e, 'dataTwinFor') === id)[0];
		if (!table) {
			fail(`no text twin (a table with data-twin-for="${id}")`);
			continue;
		}
		const compare = COMPARE[kind];
		if (!compare) {
			fail('no comparison for this kind of figure: teach site/scripts/check-figure-twins.ts');
			continue;
		}
		records += compare(fig, table, fail);
	}
}

const kinds = [...byKind].map(([k, n]) => `${n} ${k}`).join(', ');
console.log(
	`figure twins: ${figures} figures (${kinds || 'none'}) on ${withFigures.size} pages, ${records} records compared, ${failures.length} mismatches`
);
if (figures === 0) {
	console.log('FAIL (figure twins): no figures found in the build; a check that examined nothing has not passed');
	process.exit(1);
}
if (failures.length > 0) {
	console.log('FAIL (figure twins):');
	for (const f of failures) console.log(`  ${f}`);
	process.exit(1);
}
console.log('PASS (figure twins): every figure has a text twin carrying the same data');
