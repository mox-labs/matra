/**
 * What gate 9 holds the Blueprints components to, in the prerendered HTML:
 * every component that carries data or structure says the same thing in
 * text as it draws. Used by check-figure-twins.ts over the build and by
 * check-legibility.ts over the fixture, so a component no record uses yet
 * (the Lab's experiment card) is held to the same rules.
 *
 *   sketch        every box and arrow drawn in every state is a row of its
 *                 twin table, and every row is drawn; each state's caption
 *                 says "Proposed" or "Shipped" as its drawing does (a
 *                 proposed state is drawn sketchy, a shipped one crisp); the
 *                 legend says "proposed"
 *   experiment    every run's dot is a row of the twin, with the same value
 *   record-index  every card is a row of the index's table with the same
 *                 status, and every row (the template aside) is a card
 *
 * And, outside the figures:
 *
 *   claims        every claim's basis is a word as well as a glyph, from the
 *                 closed set; a likelihood is from the closed set; every
 *                 chip to code opens GitHub at the commit the page's masthead
 *                 names, and its sheet quotes the lines
 *   assumptions   the list holds every assumed claim on the page, no more
 *   decisions     each states whether it is open or decided, in words, and
 *                 carries a recommendation and the case against it
 */
import { toString } from 'hast-util-to-string';
import type { Element, Root } from 'hast';
import { BASIS, LIKELIHOOD } from '../src/lib/server/markdown/legibility';

export function all(node: Root | Element, test: (e: Element) => boolean, out: Element[] = []): Element[] {
	for (const child of node.children) {
		if (child.type !== 'element') continue;
		if (test(child)) out.push(child);
		all(child, test, out);
	}
	return out;
}

export const hasClass = (e: Element, c: string) => Array.isArray(e.properties.className) && e.properties.className.includes(c);
export const prop = (e: Element, name: string) => {
	const v = e.properties[name];
	return v === undefined || v === null ? undefined : String(v);
};

/** The twin's body rows, each as its cells' visible text. */
export function rows(table: Element): string[][] {
	return all(table, (e) => e.tagName === 'tr')
		.map((tr) => all(tr, (e) => e.tagName === 'td').map((td) => toString(td).trim()))
		.filter((cells) => cells.length > 0);
}

type Compare = (fig: Element, table: Element, fail: (msg: string) => void) => number;

export const sketch: Compare = (fig, table, fail) => {
	const twin = new Set(rows(table).map((c) => `${c[0]} ${c[2]} ${c[3]} ${c[2] === 'box' ? c[4] : ''}`.trim()));
	const drawn = new Set<string>();
	const states = all(fig, (e) => hasClass(e, 'sk-state'));
	if (states.length === 0) fail('no state is drawn');
	for (const s of states) {
		const id = prop(s, 'dataState') ?? '';
		const kind = prop(s, 'dataKind');
		const caption = all(s, (e) => hasClass(e, 'sk-caption'))[0];
		const word = kind === 'proposed' ? 'Proposed' : 'Shipped';
		if (!caption || !toString(caption).startsWith(word)) fail(`state ${id} is ${kind}, and its caption does not begin "${word}"`);
		const paths = all(s, (e) => e.tagName === 'path').length;
		const crisp = all(s, (e) => ['rect', 'line', 'polygon'].includes(e.tagName) && !hasClass(e, 'sk-head')).length;
		if (kind === 'proposed' && (paths === 0 || crisp > 0)) fail(`state ${id} is proposed, so it is drawn sketchy, with no crisp shape`);
		if (kind === 'shipped' && paths > 0) fail(`state ${id} is shipped, so it is drawn crisp, with no sketched path`);
		for (const g of all(s, (e) => e.properties.dataNode !== undefined)) drawn.add(`${id} box ${prop(g, 'dataNode')} ${prop(g, 'dataLabel')}`);
		for (const g of all(s, (e) => e.properties.dataEdge !== undefined)) drawn.add(`${id} arrow ${prop(g, 'dataEdge')}`);
	}
	const legend = all(fig, (e) => hasClass(e, 'sk-legend'))[0];
	if (!legend || !/\bproposed\b/.test(toString(legend))) fail('the legend does not say what sketchy means, with the word "proposed"');
	if (twin.size === 0) fail('the twin table has no rows');
	for (const t of twin) if (!drawn.has(t)) fail(`"${t}" is in the table but not drawn`);
	for (const d of drawn) if (!twin.has(d)) fail(`"${d}" is drawn but not in the table`);
	return twin.size;
};

export const experiment: Compare = (fig, table, fail) => {
	const twin = rows(table).map((c) => `${c[0]} ${Number(c[1])}`);
	const drawn = all(fig, (e) => e.properties.dataRun !== undefined).map((d) => `${prop(d, 'dataRun')} ${Number(prop(d, 'dataValue'))}`);
	if (twin.length === 0) fail('the twin table has no runs');
	const t = new Set(twin);
	const d = new Set(drawn);
	for (const r of t) if (!d.has(r)) fail(`run "${r}" is in the table but has no dot`);
	for (const r of d) if (!t.has(r)) fail(`run "${r}" has a dot but no row`);
	return twin.length;
};

export const recordIndex: Compare = (fig, table, fail) => {
	const twin = new Map(
		rows(table)
			.filter((c) => c[2] !== 'not a record')
			.map((c) => [c[0], c[2]])
	);
	const cards = new Map(all(fig, (e) => hasClass(e, 'card')).map((c) => [prop(c, 'dataRecord') ?? '', prop(c, 'dataStatus') ?? '']));
	for (const c of all(fig, (e) => hasClass(e, 'card'))) {
		const shown = all(c, (e) => hasClass(e, 'card-status'))[0];
		if (!shown || toString(shown) !== prop(c, 'dataStatus')) fail(`card ${prop(c, 'dataRecord')} does not show its status in words`);
	}
	for (const [id, status] of twin) {
		if (!cards.has(id)) fail(`${id} is in the index but has no card`);
		else if (cards.get(id) !== status) fail(`${id} is "${status}" in the index, "${cards.get(id)}" on its card`);
	}
	for (const id of cards.keys()) if (!twin.has(id)) fail(`${id} has a card but no row in the index`);
	return cards.size;
};

export const COMPARE: Record<string, Compare> = { sketch, experiment, 'record-index': recordIndex };

/** The components outside figures, on one page. Returns how many it examined. */
export function checkPage(tree: Root, fail: (msg: string) => void): number {
	let n = 0;
	const pin = (() => {
		const a = all(tree, (e) => e.tagName === 'a' && /\/tree\/[0-9a-f]{40}$/.test(prop(e, 'href') ?? '') && all(tree, (m) => hasClass(m, 'masthead')).length > 0)[0];
		return a ? /([0-9a-f]{40})$/.exec(prop(a, 'href')!)![1] : null;
	})();
	const claims = all(tree, (e) => hasClass(e, 'claim'));
	for (const c of claims) {
		n += 1;
		const id = prop(c, 'id');
		const basis = prop(c, 'dataBasis') ?? '';
		const mark = all(c, (e) => hasClass(e, 'basis'))[0];
		if (!(basis in BASIS)) fail(`${id}: basis "${basis}" is not in the closed set`);
		else if (!mark || toString(mark).trim() !== `${BASIS[basis as keyof typeof BASIS].glyph} ${basis}`) {
			fail(`${id}: its mark does not say "${basis}" in words beside its glyph`);
		}
		const likelihood = prop(c, 'dataLikelihood');
		if (likelihood !== undefined) {
			const word = all(c, (e) => hasClass(e, 'likelihood'))[0];
			if (!(likelihood in LIKELIHOOD) || !word || toString(word) !== likelihood) fail(`${id}: likelihood "${likelihood}" is not shown as a word from the closed set`);
		}
	}
	for (const chip of all(tree, (e) => e.tagName === 'a' && prop(e, 'dataChip') === 'lines')) {
		n += 1;
		const href = prop(chip, 'href') ?? '';
		const sha = /\/blob\/([0-9a-f]{40})\//.exec(href)?.[1];
		if (!sha) fail(`chip ${toString(chip)} does not open a pinned commit: ${href}`);
		else if (pin && sha !== pin) fail(`chip ${toString(chip)} opens ${sha.slice(0, 7)}, the masthead names ${pin.slice(0, 7)}`);
		const sheet = all(tree, (e) => prop(e, 'id') === prop(chip, 'dataSheet'))[0];
		const quote = sheet ? all(sheet, (e) => hasClass(e, 'sheet-quote'))[0] : undefined;
		if (!quote || toString(quote).trim() === '') fail(`chip ${toString(chip)} has no sheet quoting its lines`);
	}
	const list = all(tree, (e) => e.properties.dataAssumptions !== undefined);
	if (list.length) {
		const assumed = new Set(claims.filter((c) => prop(c, 'dataBasis') === 'assumed').map((c) => prop(c, 'id')));
		const listed = new Set(list.flatMap((l) => all(l, (e) => e.tagName === 'li').map((li) => prop(li, 'dataClaim'))));
		for (const a of assumed) if (!listed.has(a)) fail(`${a} is assumed but missing from the assumptions list`);
		for (const a of listed) if (!assumed.has(a)) fail(`${a} is in the assumptions list but not assumed`);
		n += listed.size;
	}
	for (const d of all(tree, (e) => hasClass(e, 'decision'))) {
		n += 1;
		const state = prop(d, 'dataDecision');
		const word = all(d, (e) => hasClass(e, 'decision-state'))[0];
		if (!word || toString(word) !== state) fail(`${prop(d, 'id')}: whether it is open is not said in words`);
		if (!all(d, (e) => hasClass(e, 'recommendation')).length || !all(d, (e) => hasClass(e, 'against')).length) {
			fail(`${prop(d, 'id')}: a decision carries a recommendation and the case against it`);
		}
	}
	return n;
}
