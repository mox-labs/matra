/**
 * Every figure has a text twin carrying the same data (EP-0012, M3).
 *
 * A reader who cannot see the picture, and an agent that reads HTML rather
 * than pixels, get the figure's content from its twin. This reads the
 * prerendered HTML, as a browser with scripts off would, and for every
 * element marked `data-figure` finds its twin and compares the two.
 *
 * For the parse figure the twin is the token table (`data-twin-for`), and the
 * figure is the arc diagram (`data-arcs-for`). Every table row must have a
 * word with the same id and text in the diagram, and an arc into that word
 * from the same head, labelled with the same relation; the diagram may have
 * no word or arc the table lacks. The table is read from its visible cell
 * text, not from attributes, because the text is what a reader gets.
 *
 * A figure with no twin fails, and so does a build with no figures at all: a
 * check that examined nothing has not passed.
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

const failures: string[] = [];
let figures = 0;
let tokens = 0;
const withFigures = new Set<string>();

for (const file of pages(dir)) {
	const page = relative(dir, file);
	const tree = fromHtml(readFileSync(file, 'utf8'));
	for (const fig of all(tree, (e) => e.properties.dataFigure !== undefined)) {
		figures += 1;
		withFigures.add(page);
		const id = String(fig.properties.id ?? '');
		const where = `${page}#${id} (${String(fig.properties.dataFigure)})`;
		const table = all(fig, (e) => e.tagName === 'table' && e.properties.dataTwinFor === id)[0];
		const svg = all(fig, (e) => e.tagName === 'svg' && e.properties.dataArcsFor === id)[0];
		if (!table) {
			failures.push(`${where}: no text twin (a table with data-twin-for="${id}")`);
			continue;
		}
		if (!svg) {
			failures.push(`${where}: no diagram (an svg with data-arcs-for="${id}")`);
			continue;
		}

		const rows = all(table, (e) => e.tagName === 'tr').filter(
			(tr) => all(tr, (e) => e.tagName === 'td').length > 0
		);
		const twin = rows.map((tr) => {
			const cells = all(tr, (e) => e.tagName === 'td').map((td) => toString(td).trim());
			return {
				id: Number.parseInt(cells[0], 10),
				text: cells[1],
				head: Number.parseInt(cells[4], 10),
				dep: cells[5]
			};
		});
		const words = new Map(
			all(svg, (e) => e.tagName === 'text' && e.properties.dataId !== undefined).map((t) => [
				Number(t.properties.dataId),
				toString(t)
			])
		);
		const arcs = new Map(
			all(svg, (e) => e.tagName === 'g' && e.properties.dataDep !== undefined).map((g) => {
				const label = all(g, (e) => e.tagName === 'text' && hasClass(e, 'rel'))[0];
				return [Number(g.properties.dataDep), { head: Number(g.properties.dataHead), dep: label ? toString(label) : '' }];
			})
		);

		if (twin.length === 0) failures.push(`${where}: the twin table has no rows`);
		for (const row of twin) {
			tokens += 1;
			if (words.get(row.id) !== row.text) {
				failures.push(`${where}: token ${row.id} is "${row.text}" in the table, "${words.get(row.id) ?? 'absent'}" in the diagram`);
			}
			const arc = arcs.get(row.id);
			if (!arc) {
				failures.push(`${where}: token ${row.id} "${row.text}" has no arc in the diagram`);
			} else if (arc.head !== row.head || arc.dep !== row.dep) {
				failures.push(
					`${where}: token ${row.id} "${row.text}" is ${row.head}/${row.dep} in the table, ${arc.head}/${arc.dep} in the diagram`
				);
			}
		}
		const listed = new Set(twin.map((r) => r.id));
		for (const w of words.keys()) if (!listed.has(w)) failures.push(`${where}: word ${w} is drawn but not in the twin`);
		for (const a of arcs.keys()) if (!listed.has(a)) failures.push(`${where}: an arc into ${a} is drawn but not in the twin`);
	}
}

console.log(`figure twins: ${figures} figures on ${withFigures.size} pages, ${tokens} tokens compared, ${failures.length} mismatches`);
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
