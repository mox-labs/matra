/**
 * The legibility components: how a design record makes Claude's
 * understanding something the owner can read, check and answer.
 *
 * Every component is a registered tag (registry.ts) written in the record's
 * Markdown and turned into plain HTML here, at build time. Nothing in this
 * file reaches the browser; the two behaviours that need a script (a chip's
 * sheet, a sketch's today-and-proposed toggle) are progressive enhancements
 * in $lib/legibility.client.ts, and every component reads in full without
 * them. The syntax, the purpose of each and the evidence behind it are in
 * site/README.md, "The Blueprints components"; the research they follow is
 * cited there.
 *
 *   claim           a load-bearing claim, its basis (observed, inferred or
 *                   assumed: a glyph and a word), a likelihood on a
 *                   prediction only, and chips to its grounds
 *   assumptions     every assumed claim on the page, collected
 *   decision        options, the recommendation set apart, the strongest
 *                   case against it, and the owner's decision slot
 *   pragmatics      the ask, what Claude will do, what it needs, what it
 *                   will not do, and what silence means
 *   changed         what changed in Claude's understanding: was, now, why
 *   sketch-figure   structure drawn sketchy when proposed, crisp when
 *                   shipped, with Rough.js at a fixed seed
 *   experiment      a Lab experiment card
 *   record-index    the Blueprints index as cards, the table their twin
 *
 * Provenance is pinned. A record's header names the commit its citations
 * were read at (`- Pinned at:` and a full SHA). Every link from the record to
 * a repository file with a line fragment (`../../src/lib.rs#L250`) then
 * opens that file at that commit, not at `main`, and must carry, as its
 * Markdown title, text the cited lines contain: the build reads the lines at
 * the commit and fails if they do not. Moving the commit forward re-checks
 * every citation.
 *
 * This module imports nothing from SvelteKit, so the gate scripts can run it
 * on fixtures (scripts/check-legibility.ts).
 */
import { execFileSync } from 'node:child_process';
import { posix } from 'node:path';
import { unified, type Plugin } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import remarkRehype from 'remark-rehype';
import rehypeRaw from 'rehype-raw';
import { toHtml } from 'hast-util-to-html';
import { toString } from 'hast-util-to-string';
import { SKIP, visit } from 'unist-util-visit';
import { scaleLinear } from 'd3-scale';
import rough from 'roughjs';
import type { RoughGenerator } from 'roughjs/bin/generator';
import type { Element, ElementContent, Properties, Root, RootContent } from 'hast';
import type { Root as MdRoot } from 'mdast';
import { REGISTRY } from './registry';

/* ------------------------------------------------------------------------ */
/* Vocabularies                                                              */
/* ------------------------------------------------------------------------ */

/**
 * What a claim stands on. A word and a glyph, never colour alone: the glyph
 * fills as the grounds firm up (hollow: none; half: reasoned; full: seen).
 */
export const BASIS = {
	observed: { glyph: '●', meaning: 'Claude read it or ran it; a chip points at the evidence.' },
	inferred: { glyph: '◐', meaning: 'reasoned from observed claims; the reasoning is written out.' },
	assumed: { glyph: '○', meaning: 'no grounds yet; the owner should confirm or strike it.' }
} as const;
export type Basis = keyof typeof BASIS;

/**
 * The closed likelihood vocabulary, for predictions only: the bands of US
 * Intelligence Community Directive 203, which the dossier-grammar research
 * derives from Kent's "Words of Estimative Probability". The ranges are what
 * each word means, stated once (blueprints/README.md); a record never puts a
 * number on its own confidence.
 */
export const LIKELIHOOD: Readonly<Record<string, string>> = {
	'almost no chance': '1 to 5%',
	'very unlikely': '5 to 20%',
	unlikely: '20 to 45%',
	'roughly even chance': '45 to 55%',
	likely: '55 to 80%',
	'very likely': '80 to 95%',
	'almost certain': '95 to 99%'
};

/** The owner's possible answers to a proposal or a decision, enumerated so silence cannot pass for one. */
export const RESPONSES = {
	accept: 'accept',
	'accept-with-reservation': 'accept with a reservation',
	object: 'object',
	redirect: 'redirect'
} as const;

/** The states a sketch figure may draw, and how. */
const STATE_KINDS = { shipped: 'Shipped', proposed: 'Proposed' } as const;

/** Tags written alone on a line, self-closing, outside any paragraph. */
export const SELF_CLOSING = new Set(['assumptions', 'record-index', 'outcomes']);

/* ------------------------------------------------------------------------ */
/* Context                                                                   */
/* ------------------------------------------------------------------------ */

/** What the Blueprints index shows of each record, read by blueprints.ts. */
export interface RecordCard {
	id: string;
	title: string;
	status: string;
	route: string;
	summary: string;
	readAt: string | null;
	decisions: { id: string; title: string; open: boolean }[];
	assumptions: number;
}

export interface LegibilityContext {
	/** The page, for error messages. */
	file: string;
	/** Its path from the repository root; relative links resolve from it. */
	repoFile: string;
	/** The commit the record's citations were read at, from its header, or null. */
	pin: string | null;
	/** The repository on GitHub. */
	repoUrl: string;
	/** The repository root on disk, where git runs. */
	repoRoot: string;
	/** The base path, for links to other records. */
	base: string;
	/** Every record, for the index's cards. */
	records?: readonly RecordCard[];
}

/** The commit a record names in its header (`- Pinned at: <sha>`), or null. */
export function readPin(markdown: string, file: string): string | null {
	const line = /^- Pinned at:\s*(.*)$/m.exec(markdown)?.[1];
	if (line === undefined) return null;
	const sha = /\b([0-9a-f]{40})\b/.exec(line)?.[1];
	if (!sha) throw new Error(`${file}: "- Pinned at:" names no full 40-character commit SHA`);
	return sha;
}

/* ------------------------------------------------------------------------ */
/* Reading the repository at a commit                                        */
/* ------------------------------------------------------------------------ */

const fileCache = new Map<string, string[] | null>();
const commitCache = new Map<string, string | null>();

function git(root: string, args: string[]): string {
	return execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: 64 << 20 });
}

/**
 * Whether the commit exists here and is on the history being built, so the
 * link GitHub serves for it outlives the branch it was read on. Returns the
 * reason it is not, or null.
 */
function commitProblem(root: string, sha: string): string | null {
	if (commitCache.has(sha)) return commitCache.get(sha)!;
	let problem: string | null = null;
	try {
		git(root, ['cat-file', '-e', `${sha}^{commit}`]);
		try {
			git(root, ['merge-base', '--is-ancestor', sha, 'HEAD']);
		} catch {
			problem = `${sha} is not an ancestor of HEAD; pin a commit on main`;
		}
	} catch {
		problem = `${sha} is not in this clone (a shallow checkout? fetch with full history)`;
	}
	commitCache.set(sha, problem);
	return problem;
}

function linesAt(root: string, sha: string, path: string): string[] | null {
	const key = `${sha}:${path}`;
	if (!fileCache.has(key)) {
		try {
			fileCache.set(key, git(root, ['show', key]).split('\n'));
		} catch {
			fileCache.set(key, null);
		}
	}
	return fileCache.get(key)!;
}

const squash = (s: string) => s.replace(/\s+/g, ' ').trim();

/* ------------------------------------------------------------------------ */
/* hast helpers                                                              */
/* ------------------------------------------------------------------------ */

function h(tagName: string, properties: Record<string, unknown> = {}, children: (ElementContent | string)[] = []): Element {
	return {
		type: 'element',
		tagName,
		properties: properties as Properties,
		children: children.map((c) => (typeof c === 'string' ? { type: 'text', value: c } : c))
	};
}

const isEl = (n: unknown): n is Element => (n as Element)?.type === 'element';
const attr = (e: Element, name: string): string | undefined => {
	const v = e.properties?.[name];
	return v === undefined || v === null ? undefined : Array.isArray(v) ? v.join(' ') : String(v);
};
/** Element children, whitespace text dropped. */
const kids = (e: Element | Root) =>
	e.children.filter((c) => !(c.type === 'text' && c.value.trim() === '')) as (ElementContent | RootContent)[];

function slug(s: string): string {
	return s
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-|-$/g, '');
}

class Problems {
	list: string[] = [];
	constructor(private file: string) {}
	add(node: { position?: { start: { line: number } } } | undefined, msg: string) {
		const line = node?.position?.start.line;
		this.list.push(`  ${this.file}${line ? `:${line}` : ''}: ${msg}`);
	}
	throwIfAny() {
		if (this.list.length > 0) throw new Error(`Blueprints components that cannot render:\n${this.list.join('\n')}`);
	}
}

/**
 * Every component tag against its registry entry: the attributes it may
 * carry, those it must, and the component it belongs inside. One pass, before
 * anything is drawn, so every mistake on a page is reported at once.
 */
function checkComponents(tree: Root, p: Problems) {
	visit(tree, 'element', (e, _i, parent) => {
		const entry = REGISTRY[e.tagName];
		if (entry?.kind !== 'component') return;
		const allowed = Object.keys(entry.attributes);
		for (const k of Object.keys(e.properties ?? {})) {
			if (!allowed.includes(k)) p.add(e, `<${e.tagName}> has no attribute "${k}" (it takes ${allowed.join(', ') || 'none'})`);
		}
		for (const [k, need] of Object.entries(entry.attributes)) {
			if (need === 'required' && attr(e, k) === undefined) p.add(e, `<${e.tagName}> needs ${k}="..."`);
		}
		const up = parent && isEl(parent) ? parent.tagName : null;
		if (entry.within && up !== entry.within) p.add(e, `<${e.tagName}> belongs directly inside <${entry.within}>`);
		if (!entry.within && up && REGISTRY[up]?.kind === 'component' && e.tagName !== 'claim') {
			p.add(e, `<${e.tagName}> stands on its own, not inside <${up}>`);
		}
	});
}

/* ------------------------------------------------------------------------ */
/* Markdown: self-closing tags                                               */
/* ------------------------------------------------------------------------ */

const SELF_TAG = /^<([a-z][a-z-]*)((?:\s+[a-z][a-z-]*="[^"<>]*")*)\s*\/>$/;
const SELF_ATTR = /([a-z][a-z-]*)="([^"<>]*)"/g;

/**
 * `<assumptions />` and its kind, on a line of their own, become elements
 * before raw HTML is parsed: an HTML parser reads a self-closing custom tag
 * as left open and would swallow what follows.
 */
export const selfClosingTags: Plugin<[], MdRoot> = () => (tree) => {
	tree.children = tree.children.map((node) => {
		if (node.type !== 'html') return node;
		const m = SELF_TAG.exec(node.value.trim());
		if (!m || !SELF_CLOSING.has(m[1])) return node;
		const hProperties: Record<string, string> = {};
		for (const [, name, value] of m[2].matchAll(SELF_ATTR)) hProperties[name] = value;
		return { type: 'legibilityTag', data: { hName: m[1], hProperties }, children: [], position: node.position } as unknown as typeof node;
	});
};

/* ------------------------------------------------------------------------ */
/* The transform                                                             */
/* ------------------------------------------------------------------------ */

export const legibility: Plugin<[LegibilityContext], Root> = (ctx) => (tree) => {
	const p = new Problems(ctx.file);
	checkComponents(tree, p);
	const record = ctx.repoFile.startsWith('blueprints/') || ctx.repoFile.startsWith('lab/');
	if (ctx.pin) {
		const problem = commitProblem(ctx.repoRoot, ctx.pin);
		if (problem) p.add(undefined, `"- Pinned at:" ${problem}`);
	}
	if (/^blueprints\/(proposals|plans)\//.test(ctx.repoFile)) masthead(tree, ctx);
	if (record) pinLines(tree, ctx, p);
	const claims = transformClaims(tree, ctx, p);
	reflowClaimTables(tree);
	visit(tree, 'element', (node, index, parent) => {
		if (!parent || index === undefined) return;
		let out: Element | null = null;
		switch (node.tagName) {
			case 'assumptions':
				out = assumptionsList(node, claims, p);
				break;
			case 'decision':
				out = decision(node, p);
				break;
			case 'pragmatics':
				out = pragmatics(node, p);
				break;
			case 'changed':
				out = changed(node, p);
				break;
			case 'sketch-figure':
				out = sketch(node, ctx, p);
				break;
			case 'experiment':
				out = experiment(node, ctx, p);
				break;
			case 'record-index':
				out = recordIndex(node, parent as Root | Element, index, ctx, p);
				break;
			default:
				return;
		}
		if (out) parent.children[index] = out;
		return [SKIP, index + 1];
	});
	p.throwIfAny();
};

/* ------------------------------------------------------------------------ */
/* Masthead                                                                  */
/* ------------------------------------------------------------------------ */

/** The header's lines, in the order a reader looks for them. Status is under the title already. */
const MASTHEAD_ORDER = ['Start Date', 'Pinned at', 'Proposal PR', 'Plan PR', 'Tracking issue', 'Implements', 'Feature Name'];
const MASTHEAD_LABEL: Record<string, string> = { 'Start Date': 'Started', 'Feature Name': 'Feature name' };

/**
 * A record's header, the list under its title, becomes its masthead: the
 * dates, the commit its citations were read at, who decides, and the pull
 * request and tracking issue. Each line stays as written in the Markdown.
 */
function masthead(tree: Root, ctx: LegibilityContext) {
	const at = tree.children.findIndex((n) => isEl(n) && n.tagName === 'h1');
	if (at === -1) return;
	let j = at + 1;
	while (j < tree.children.length && !isEl(tree.children[j])) j++;
	const ul = tree.children[j];
	if (!isEl(ul) || ul.tagName !== 'ul') return;
	const rows = new Map<string, (ElementContent | string)[]>();
	for (const li of ul.children.filter(isEl)) {
		const first = li.children[0];
		if (first?.type !== 'text') continue;
		const colon = first.value.indexOf(':');
		if (colon === -1) continue;
		const key = first.value.slice(0, colon).trim();
		const rest = [{ type: 'text', value: first.value.slice(colon + 1).replace(/^\s+/, '') } as ElementContent, ...li.children.slice(1)];
		rows.set(key, rest);
	}
	if (ctx.pin) {
		rows.set('Pinned at', [
			h('a', { href: `${ctx.repoUrl}/tree/${ctx.pin}` }, [h('code', {}, [ctx.pin.slice(0, 7)])]),
			', the commit every code citation below opens at'
		]);
	}
	const keys = [...MASTHEAD_ORDER.filter((k) => rows.has(k)), ...[...rows.keys()].filter((k) => !MASTHEAD_ORDER.includes(k) && k !== 'Status')];
	const pair = (label: string, value: (ElementContent | string)[]) => h('div', { className: ['mh-row'] }, [h('dt', {}, [label]), h('dd', {}, value)]);
	tree.children[j] = h('dl', { className: ['masthead'], dataPagefindIgnore: '' }, [
		...keys.slice(0, 2).map((k) => pair(MASTHEAD_LABEL[k] ?? k, rows.get(k)!)),
		pair('Decides', ['the owner, who alone sets the status to ', h('code', {}, ['accepted'])]),
		...keys.slice(2).map((k) => pair(MASTHEAD_LABEL[k] ?? k, rows.get(k)!))
	]);
}

/* ------------------------------------------------------------------------ */
/* Pinned citations                                                          */
/* ------------------------------------------------------------------------ */

const LINES = /^L(\d+)(?:-L(\d+))?$/;

/**
 * In a record whose header names a commit, every relative link to a
 * repository file with a line fragment opens the file at that commit, and
 * the lines must exist there. A title names text the cited lines must hold,
 * and the build checks it; a chip (a link in a claim, or the cause in
 * `changed`) must carry one. The link keeps the quoted lines in
 * `data-quote`, for the chip's sheet. A record with no commit in its header
 * keeps its links on `main`, as before pinning.
 */
function pinLines(tree: Root, ctx: LegibilityContext, p: Problems) {
	visit(tree, 'element', (a) => {
		if (a.tagName !== 'a') return;
		const href = attr(a, 'href');
		if (!href || /^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith('#')) return;
		const hash = href.indexOf('#');
		if (hash === -1) return;
		const m = LINES.exec(href.slice(hash + 1));
		if (!m) return;
		const path = posix.normalize(posix.join(posix.dirname(ctx.repoFile), href.slice(0, hash)));
		if (path.startsWith('../')) {
			p.add(a, `${href} leaves the repository`);
			return;
		}
		// A record with no commit in its header keeps its links on main.
		if (!ctx.pin) return;
		const from = Number(m[1]);
		const to = m[2] ? Number(m[2]) : from;
		const has = attr(a, 'title');
		const lines = linesAt(ctx.repoRoot, ctx.pin, path);
		if (!lines) {
			p.add(a, `${path} does not exist at ${ctx.pin.slice(0, 12)}`);
			return;
		}
		if (to < from || to > lines.length) {
			p.add(a, `${path}#L${from}-L${to} is past the file's ${lines.length} lines at ${ctx.pin.slice(0, 12)}`);
			return;
		}
		const quoted = lines.slice(from - 1, to);
		if (has && !squash(quoted.join('\n')).includes(squash(has))) {
			p.add(a, `${path} lines ${from} to ${to} at ${ctx.pin.slice(0, 12)} do not contain "${has}"`);
		}
		a.properties.href = `${ctx.repoUrl}/blob/${ctx.pin}/${path}#L${from}${to !== from ? `-L${to}` : ''}`;
		delete a.properties.title;
		a.properties.dataPinned = ctx.pin;
		a.properties.dataPath = path;
		a.properties.dataFrom = String(from);
		a.properties.dataTo = String(to);
		a.properties.dataQuote = quoted.join('\n');
		if (has) a.properties.dataChecked = '';
	});
}

/* ------------------------------------------------------------------------ */
/* Claims                                                                    */
/* ------------------------------------------------------------------------ */

interface ClaimInfo {
	id: string;
	basis: Basis;
	text: string;
	section: string;
}

/** A chip: a pinned line link, or a link to a pull request or an issue. */
function chipKind(a: Element): 'lines' | 'pr' | 'issue' | 'record' | 'link' {
	if (a.properties.dataPinned) return 'lines';
	const href = attr(a, 'href') ?? '';
	if (/\/pull\/\d+/.test(href)) return 'pr';
	if (/\/issues\/\d+/.test(href)) return 'issue';
	if (!/^[a-z][a-z0-9+.-]*:/i.test(href)) return 'record';
	return 'link';
}

/**
 * A table whose cells carry claims is read row by row (a task, and what
 * each surface does with it), and its chips make it too wide for a phone. On
 * a narrow screen each row becomes a block with each cell labelled by its
 * column (legibility.css, `table.reflow`); wider, it is the table as written.
 */
function reflowClaimTables(tree: Root) {
	visit(tree, 'element', (table) => {
		if (table.tagName !== 'table') return;
		let has = false;
		visit(table, 'element', (e) => {
			if (e.properties?.className && Array.isArray(e.properties.className) && e.properties.className.includes('claim')) has = true;
		});
		if (!has) return;
		const heads: string[] = [];
		visit(table, 'element', (th) => {
			if (th.tagName === 'th') heads.push(squash(toString(th)));
		});
		visit(table, 'element', (tr) => {
			if (tr.tagName !== 'tr') return;
			tr.children.filter(isEl).filter((c) => c.tagName === 'td').forEach((td, i) => (td.properties.dataLabel = heads[i] ?? ''));
			return SKIP;
		});
		table.properties.className = ['reflow'];
		return SKIP;
	});
}

/** Makes a link a chip; a pinned line link also gets its sheet, pushed onto `sheets`. */
function chipify(a: Element, sheetId: string, sheets: Element[], p: Problems) {
	const kind = chipKind(a);
	if (kind === 'lines' && a.properties.dataChecked === undefined) {
		p.add(a, `${String(a.properties.dataPath)}: a chip names, as its title, text on the lines it cites, [label](path#L1 "text"), so a moved line fails the build`);
	}
	a.properties.className = ['chip', `chip-${kind}`];
	a.properties.dataChip = kind;
	if (kind === 'lines') {
		a.properties.dataSheet = sheetId;
		sheets.push(sheet(sheetId, a, toString(a)));
	}
}

function transformClaims(tree: Root, ctx: LegibilityContext, p: Problems): ClaimInfo[] {
	const claims: ClaimInfo[] = [];
	let section = '';
	let n = 0;
	const top = tree.children;
	for (let t = 0; t < top.length; t++) {
		const block = top[t];
		if (isEl(block) && /^h[23]$/.test(block.tagName)) section = toString(block).trim();
		if (!isEl(block)) continue;
		const sheets: Element[] = [];
		visit(block, 'element', (claim, index, parent) => {
			if (claim.tagName !== 'claim' || !parent || index === undefined) return;
			n += 1;
			const id = `claim-${n}`;
			const basis = attr(claim, 'basis') as Basis;
			if (basis && !(basis in BASIS)) p.add(claim, `basis="${basis}" is not one of ${Object.keys(BASIS).join(', ')}`);
			const likelihood = attr(claim, 'likelihood');
			if (likelihood !== undefined) {
				if (!(likelihood in LIKELIHOOD)) {
					p.add(claim, `likelihood="${likelihood}" is not in the closed vocabulary (${Object.keys(LIKELIHOOD).join(', ')})`);
				}
				if (basis === 'observed') p.add(claim, 'an observed claim is not a prediction, so it takes no likelihood');
			}
			for (const c of claim.children) {
				if (isEl(c) && /^(p|div|ul|ol|table|pre|blockquote|h\d)$/.test(c.tagName)) {
					p.add(claim, '<claim> wraps a phrase or a sentence inside a paragraph, not a block');
				}
			}
			// Chips: the links in the claim that point at its grounds move
			// from the prose to the claim's mark.
			const chips: Element[] = [];
			const body: ElementContent[] = [];
			for (const c of claim.children) {
				if (isEl(c) && c.tagName === 'a') {
					chipify(c, `${id}-src-${chips.length + 1}`, sheets, p);
					chips.push(c);
				} else body.push(c);
			}
			// Trailing whitespace and punctuation left where a chip was.
			while (body.length && body.at(-1)!.type === 'text' && (body.at(-1) as { value: string }).value.trim() === '') body.pop();
			if (basis === 'observed' && chips.length === 0) {
				p.add(claim, 'an observed claim points at its evidence: put a link to it inside the <claim>');
			}
			const text = squash(toString(h('span', {}, body)));
			claims.push({ id, basis, text, section });
			const b = BASIS[basis] ?? BASIS.assumed;
			const mark = h('span', { className: ['claim-mark'], dataPagefindIgnore: '' }, [
				h('span', { className: ['basis'], dataBasis: basis }, [
					h('span', { className: ['basis-glyph'], ariaHidden: 'true' }, [b.glyph]),
					` ${basis}`
				]),
				...(likelihood
					? [
							' ',
							// The word links its meaning, the band, stated once in the index.
							h('a', { className: ['likelihood'], dataLikelihood: likelihood, href: `${ctx.base}/blueprints/index#likelihood`, dataResolved: '', ariaLabel: `${likelihood}, ${LIKELIHOOD[likelihood]}` }, [likelihood])
						]
					: []),
				...chips.flatMap((c) => [' ', c])
			]);
			parent.children[index] = h(
				'span',
				{ className: ['claim'], id, dataBasis: basis, ...(likelihood ? { dataLikelihood: likelihood } : {}) },
				[h('span', { className: ['claim-text'] }, body), ' ', mark]
			);
			return SKIP;
		});
		if (sheets.length) {
			top.splice(t + 1, 0, ...sheets);
			t += sheets.length;
		}
	}
	return claims;
}

/**
 * A chip's sheet: the quoted lines, what kind of check this is, and the
 * link to them at the commit. A popover, so it opens with no script where a
 * script calls it, and the chip stays a plain link where none runs.
 */
function sheet(id: string, a: Element, label: string): Element {
	const from = Number(a.properties.dataFrom);
	const quote = String(a.properties.dataQuote ?? '').split('\n');
	const path = String(a.properties.dataPath);
	const short = String(a.properties.dataPinned).slice(0, 7);
	const width = String(from + quote.length - 1).length;
	return h('div', { id, className: ['sheet'], popover: 'auto', role: 'dialog', ariaLabel: `Source: ${label}`, dataPagefindIgnore: '' }, [
		h('div', { className: ['sheet-head'] }, [
			h('p', { className: ['sheet-title'] }, [h('code', {}, [path]), `${quote.length > 1 ? ' lines' : ' line'} ${from}${quote.length > 1 ? ` to ${from + quote.length - 1}` : ''}, at `, h('code', {}, [short])]),
			h('button', { type: 'button', className: ['sheet-close'], popovertarget: id, popovertargetaction: 'hide' }, ['close'])
		]),
		h('div', { className: ['sheet-quote'], tabIndex: 0, role: 'region', ariaLabel: 'The quoted lines' }, [
			h(
				'span',
				{ className: ['q-code'] },
				quote.map((line, i) => h('span', { className: ['q-line'] }, [h('span', { className: ['q-n'], ariaHidden: 'true' }, [String(from + i).padStart(width, ' ')]), ` ${line}\n`]))
			)
		]),
		h('p', { className: ['sheet-note'] }, [
			'Claude read these lines at this commit; that is a reading, not a test. ',
			h('a', { href: String(a.properties.href) }, [`Open them on GitHub at ${short}`]),
			'.'
		])
	]);
}

/* ------------------------------------------------------------------------ */
/* Assumptions                                                               */
/* ------------------------------------------------------------------------ */

function assumptionsList(node: Element, claims: ClaimInfo[], p: Problems): Element {
	const assumed = claims.filter((c) => c.basis === 'assumed');
	if (assumed.length === 0) {
		return h('p', { className: ['assumptions-none'], dataAssumptions: '0' }, ['No claim here is assumed: each has grounds, marked beside it.']);
	}
	return h('div', { className: ['assumptions'], dataAssumptions: String(assumed.length) }, [
		h('p', { className: ['assumptions-lead'] }, [
			h('span', { className: ['basis-glyph'], ariaHidden: 'true' }, [BASIS.assumed.glyph]),
			` ${assumed.length} ${assumed.length === 1 ? 'claim stands' : 'claims stand'} on no grounds yet. Confirm or strike each with a comment on it.`
		]),
		h(
			'ol',
			{},
			assumed.map((c) =>
				h('li', { dataClaim: c.id }, [h('span', { className: ['assumption-text'] }, [c.text]), ' ', h('a', { href: `#${c.id}`, className: ['assumption-where'] }, [`where: ${c.section || 'above'}`])])
			)
		)
	]);
}

/* ------------------------------------------------------------------------ */
/* Decision                                                                  */
/* ------------------------------------------------------------------------ */

function decision(node: Element, p: Problems): Element {
	const id = `decision-${slug(attr(node, 'id') ?? '')}`;
	const title = attr(node, 'title') ?? '';
	const choices: Element[] = [];
	let recommendation: Element | undefined;
	let against: Element | undefined;
	let ruling: Element | undefined;
	for (const c of kids(node)) {
		if (!isEl(c)) {
			p.add(node, '<decision> holds only <choice>, <recommendation>, <against> and <ruling>');
			continue;
		}
		if (c.tagName === 'choice') choices.push(c);
		else if (c.tagName === 'recommendation' && !recommendation) recommendation = c;
		else if (c.tagName === 'against' && !against) against = c;
		else if (c.tagName === 'ruling' && !ruling) ruling = c;
		else p.add(c, `<decision> holds <choice> elements and one each of <recommendation>, <against> and <ruling>, not <${c.tagName}> here`);
	}
	if (choices.length < 2) p.add(node, 'a decision offers at least two <choice> elements');
	if (!recommendation) p.add(node, 'a decision carries Claude\'s <recommendation>');
	if (!against) p.add(node, 'a decision carries <against>, the strongest case against the recommendation');
	const keys = new Map<string, string>();
	const choiceEls = choices.map((c) => {
		const key = attr(c, 'key') ?? '?';
		keys.set(key, attr(c, 'title') ?? '');
		const recommended = recommendation && attr(recommendation, 'choice') === key;
		return h('details', { className: ['choice'], dataChoice: key, ...(recommended ? { dataRecommended: '' } : {}) }, [
			h('summary', {}, [h('span', { className: ['choice-key'] }, [key.toUpperCase()]), ' ', attr(c, 'title') ?? '', recommended ? h('span', { className: ['choice-rec'] }, [' recommended']) : '']),
			h('div', { className: ['choice-body'] }, c.children)
		]);
	});
	let recEl: Element | null = null;
	if (recommendation) {
		const key = attr(recommendation, 'choice') ?? '';
		if (!keys.has(key)) p.add(recommendation, `choice="${key}" names no <choice key="${key}">`);
		const basis = (attr(recommendation, 'basis') ?? 'inferred') as Basis;
		if (!(basis in BASIS)) p.add(recommendation, `basis="${basis}" is not one of ${Object.keys(BASIS).join(', ')}`);
		recEl = h('div', { className: ['recommendation'], dataChoice: key }, [
			h('p', { className: ['decision-role'] }, [
				'Claude recommends ',
				h('strong', {}, [`${key.toUpperCase()}, ${keys.get(key) ?? ''}`]),
				'. ',
				h('span', { className: ['basis'], dataBasis: basis }, [h('span', { className: ['basis-glyph'], ariaHidden: 'true' }, [BASIS[basis]?.glyph ?? '']), ` ${basis}`]),
				', a judgment'
			]),
			...recommendation.children
		]);
	}
	const againstEl = against
		? h('div', { className: ['against'] }, [h('p', { className: ['decision-role'] }, ['The strongest case against it']), ...against.children])
		: null;
	let rulingEl: Element;
	if (ruling) {
		const response = attr(ruling, 'response') ?? '';
		if (!(response in RESPONSES)) p.add(ruling, `response="${response}" is not one of ${Object.keys(RESPONSES).join(', ')}`);
		if (!/^\d{4}-\d{2}-\d{2}$/.test(attr(ruling, 'date') ?? '')) p.add(ruling, 'date="YYYY-MM-DD"');
		rulingEl = h('div', { className: ['ruling'], dataResponse: response }, [
			h('p', { className: ['decision-role'] }, [`The owner's decision, ${attr(ruling, 'date')}: `, h('strong', {}, [RESPONSES[response as keyof typeof RESPONSES] ?? response])]),
			...ruling.children
		]);
	} else {
		rulingEl = h('div', { className: ['ruling'], dataResponse: 'open' }, [
			h('p', { className: ['decision-role'] }, [
				"The owner's decision: ",
				h('strong', {}, ['open']),
				'. Answer with accept, accept with a reservation, object or redirect, in a comment on this block or on the pull request. Silence is not assent.'
			])
		]);
	}
	const open = !ruling;
	return h('section', { className: ['decision'], id, dataDecision: open ? 'open' : 'decided', ariaLabelledby: `${id}-title` }, [
		h('p', { className: ['decision-kicker'] }, [`Decision`, ' · ', h('span', { className: ['decision-state'] }, [open ? 'open' : 'decided'])]),
		h('h3', { id: `${id}-title`, className: ['decision-title'] }, [title]),
		h('div', { className: ['choices'], dataCount: String(choiceEls.length) }, choiceEls),
		...(recEl ? [recEl] : []),
		...(againstEl ? [againstEl] : []),
		rulingEl
	]);
}

/* ------------------------------------------------------------------------ */
/* Pragmatics                                                                */
/* ------------------------------------------------------------------------ */

const PRAGMATICS: [string, string][] = [
	['ask', 'The ask'],
	['will', 'If you accept, Claude will'],
	['needs', 'Claude needs from you'],
	['wont', 'Claude will not'],
	['silence', 'If nobody answers']
];

function pragmatics(node: Element, p: Problems): Element {
	const parts = new Map<string, Element>();
	for (const c of kids(node)) {
		if (!isEl(c) || !PRAGMATICS.some(([t]) => t === c.tagName) || parts.has(c.tagName)) {
			p.add(isEl(c) ? c : node, '<pragmatics> holds one each of <ask>, <will>, <needs>, <wont> and <silence>');
			continue;
		}
		parts.set(c.tagName, c);
	}
	for (const [t] of PRAGMATICS) if (!parts.has(t)) p.add(node, `<pragmatics> is missing <${t}>`);
	return h('aside', { className: ['pragmatics'], ariaLabel: 'What this record asks, and what happens next' }, [
		...PRAGMATICS.map(([t, label]) =>
			h('div', { className: ['prag', `prag-${t}`], dataPart: t }, [
				h('p', { className: ['prag-label'] }, [label]),
				...(t === 'silence' ? [h('p', { className: ['prag-silence'] }, [h('strong', {}, ['Silence is not assent.'])])] : []),
				...(parts.get(t)?.children ?? [])
			])
		),
		h('p', { className: ['prag-responses'] }, [
			'Your answer is one of: ',
			...Object.values(RESPONSES).flatMap((r, i, all) => [h('strong', {}, [r]), i < all.length - 2 ? ', ' : i === all.length - 2 ? ' or ' : '']),
			'. The status log records it.'
		])
	]);
}

/* ------------------------------------------------------------------------ */
/* What changed in my understanding                                          */
/* ------------------------------------------------------------------------ */

function changed(node: Element, p: Problems): Element {
	const pairs: [Element, Element][] = [];
	let was: Element | null = null;
	for (const c of kids(node)) {
		if (isEl(c) && c.tagName === 'was' && !was) was = c;
		else if (isEl(c) && c.tagName === 'now' && was) {
			pairs.push([was, c]);
			was = null;
		} else p.add(isEl(c) ? c : node, '<changed> holds <was> and <now> in pairs, each <was> followed by its <now>');
	}
	if (was) p.add(was, 'this <was> has no <now> after it');
	if (pairs.length === 0) p.add(node, '<changed> holds at least one <was> and <now> pair');
	for (const [, now] of pairs) {
		let cause = false;
		visit(now, 'element', (e) => {
			if (e.tagName === 'a') cause = true;
		});
		if (!cause) p.add(now, 'each <now> links what changed it: the lines, the comment or the pull request');
	}
	// What changed it is a chip, as a claim's grounds are, with its sheet.
	const sheets: Element[] = [];
	pairs.forEach(([, now], i) => {
		let k = 0;
		visit(now, 'element', (a) => {
			if (a.tagName !== 'a') return;
			k += 1;
			chipify(a, `changed-${i + 1}-src-${k}`, sheets, p);
		});
	});
	// Struck through and named "was", so the change never rests on the line alone.
	const struck = (children: ElementContent[]) =>
		children.map((c) => (isEl(c) && c.tagName === 'p' ? h('p', {}, [h('del', {}, c.children)]) : c));
	return h('section', { className: ['changed'], ariaLabelledby: 'what-changed' }, [
		...sheets,
		h('p', { className: ['changed-head'], id: 'what-changed' }, [
			h('strong', {}, ['What changed in my understanding']),
			`, ${attr(node, 'date')}, since ${attr(node, 'since')}`
		]),
		h(
			'ol',
			{ className: ['changed-pairs'] },
			pairs.map(([w, n]) =>
				h('li', {}, [
					h('div', { className: ['was'] }, [h('span', { className: ['changed-word'] }, ['was']), h('div', { className: ['changed-body'] }, struck(w.children))]),
					h('div', { className: ['now'] }, [h('span', { className: ['changed-word'] }, ['now']), h('div', { className: ['changed-body'] }, n.children)])
				])
			)
		)
	]);
}

/* ------------------------------------------------------------------------ */
/* Sketch figure                                                             */
/* ------------------------------------------------------------------------ */

interface SketchNode {
	id: string;
	shape: 'box' | 'hex';
	label: string[];
	x: number;
	y: number;
	w: number;
	h: number;
	quiet: boolean;
}
interface SketchEdge {
	from: string;
	to: string;
	dashed: boolean;
}
interface SketchState {
	id: string;
	kind: keyof typeof STATE_KINDS;
	label: string;
	nodes: SketchNode[];
	edges: SketchEdge[];
}

const TOKEN = /"((?:[^"\\]|\\.)*)"|(\S+)/g;

/**
 * The declared structure: one statement a line.
 *
 *   width 400                                   the drawing's width in units
 *   state today shipped "At 4fcfb4a"           a state, shipped or proposed
 *   box  py "Python binding" 10 20 120 36      x y width height
 *   hex  port "Application port" 150 90 140 90
 *   quiet box ...                               drawn lighter: context
 *   edge py port                                an arrow from py to port
 *   edge py port dashed                         a dependency to be removed
 *
 * `\n` in a label breaks the line.
 */
export function parseSketch(text: string, where: string): { width: number; states: SketchState[] } {
	const states: SketchState[] = [];
	let width = 400;
	const fail = (n: number, msg: string): never => {
		throw new Error(`${where}: sketch line ${n + 1}: ${msg}`);
	};
	text.split('\n').forEach((raw, n) => {
		const line = raw.trim();
		if (line === '' || line.startsWith('#')) return;
		const t = [...line.matchAll(TOKEN)].map((m) => (m[1] !== undefined ? m[1].replace(/\\"/g, '"') : m[2]));
		let quiet = false;
		if (t[0] === 'quiet') {
			quiet = true;
			t.shift();
		}
		const state = states.at(-1);
		switch (t[0]) {
			case 'width':
				width = Number(t[1]);
				if (!(width > 0)) fail(n, 'width takes a positive number');
				break;
			case 'state':
				if (t.length !== 4 || !(t[2] in STATE_KINDS)) fail(n, 'state <id> shipped|proposed "<label>"');
				states.push({ id: t[1], kind: t[2] as keyof typeof STATE_KINDS, label: t[3], nodes: [], edges: [] });
				break;
			case 'box':
			case 'hex': {
				if (!state) fail(n, 'a node comes after its state');
				const nums = t.slice(3).map(Number);
				if (t.length !== 7 || nums.some((v) => !Number.isFinite(v))) fail(n, `${t[0]} <id> "<label>" x y width height`);
				if (state!.nodes.some((s) => s.id === t[1])) fail(n, `${t[1]} is declared twice in ${state!.id}`);
				state!.nodes.push({ id: t[1], shape: t[0], label: t[2].split('\\n'), x: nums[0], y: nums[1], w: nums[2], h: nums[3], quiet });
				break;
			}
			case 'edge': {
				if (!state) fail(n, 'an edge comes after its state');
				if (t.length < 3 || t.length > 4 || (t[3] !== undefined && t[3] !== 'dashed')) fail(n, 'edge <from> <to> [dashed]');
				for (const id of [t[1], t[2]]) if (!state!.nodes.some((s) => s.id === id)) fail(n, `no node ${id} in ${state!.id} (declare nodes before edges)`);
				state!.edges.push({ from: t[1], to: t[2], dashed: t[3] === 'dashed' });
				break;
			}
			default:
				fail(n, `"${t[0]}" is not a statement (width, state, box, hex, quiet, edge)`);
		}
	});
	if (states.length === 0) throw new Error(`${where}: a sketch declares at least one state`);
	return { width, states };
}

const round = (d: string) => d.replace(/-?\d+\.\d+/g, (v) => String(Math.round(Number(v) * 10) / 10));

function hexPoints(n: SketchNode): [number, number][] {
	const k = Math.min(n.h / 2, n.w / 4);
	return [
		[n.x + k, n.y],
		[n.x + n.w - k, n.y],
		[n.x + n.w, n.y + n.h / 2],
		[n.x + n.w - k, n.y + n.h],
		[n.x + k, n.y + n.h],
		[n.x, n.y + n.h / 2]
	];
}

/** Where the segment from a node's centre towards (tx, ty) leaves its box. */
function exit(n: SketchNode, tx: number, ty: number): [number, number] {
	const cx = n.x + n.w / 2;
	const cy = n.y + n.h / 2;
	const dx = tx - cx;
	const dy = ty - cy;
	if (dx === 0 && dy === 0) return [cx, cy];
	const sx = dx === 0 ? Infinity : n.w / 2 / Math.abs(dx);
	const sy = dy === 0 ? Infinity : n.h / 2 / Math.abs(dy);
	const s = Math.min(sx, sy);
	return [cx + dx * s, cy + dy * s];
}

/** Draws one state: crisp when shipped, through Rough.js at the figure's seed when proposed. */
function drawState(state: SketchState, width: number, seed: number): { svg: Element; height: number } {
	const sketchy = state.kind === 'proposed';
	const gen = rough.generator();
	let k = 0;
	const opts = () => ({ seed: seed * 1000 + ++k, roughness: 1.1, bowing: 0.8, strokeWidth: 1.4 });
	const sketchPath = (d: ReturnType<RoughGenerator['line']>, cls: string) =>
		gen.toPaths(d).map((pi) => h('path', { d: round(pi.d), className: [cls] }));
	const height = Math.ceil(Math.max(...state.nodes.map((n) => n.y + n.h)) + 8);
	const byId = new Map(state.nodes.map((n) => [n.id, n]));
	const edges = state.edges.map((e) => {
		const a = byId.get(e.from)!;
		const b = byId.get(e.to)!;
		const [x1, y1] = exit(a, b.x + b.w / 2, b.y + b.h / 2);
		const [x2, y2] = exit(b, a.x + a.w / 2, a.y + a.h / 2);
		const len = Math.hypot(x2 - x1, y2 - y1) || 1;
		const ux = (x2 - x1) / len;
		const uy = (y2 - y1) / len;
		const head: [number, number][] = [
			[x2, y2],
			[x2 - 7 * ux + 3.5 * uy, y2 - 7 * uy - 3.5 * ux],
			[x2 - 7 * ux - 3.5 * uy, y2 - 7 * uy + 3.5 * ux]
		];
		const cls = e.dashed ? 'sk-edge sk-dashed' : 'sk-edge';
		const marks: Element[] = sketchy
			? [...sketchPath(gen.line(x1, y1, x2 - 5 * ux, y2 - 5 * uy, { ...opts(), ...(e.dashed ? { strokeLineDash: [5, 4] } : {}) }), cls), h('polygon', { points: head.map((q) => q.map((v) => v.toFixed(1)).join(',')).join(' '), className: ['sk-head'] })]
			: [
					h('line', { x1: x1.toFixed(1), y1: y1.toFixed(1), x2: (x2 - 5 * ux).toFixed(1), y2: (y2 - 5 * uy).toFixed(1), className: cls.split(' ') }),
					h('polygon', { points: head.map((q) => q.map((v) => v.toFixed(1)).join(',')).join(' '), className: ['sk-head'] })
				];
		return h('g', { dataEdge: `${e.from}>${e.to}`, ...(e.dashed ? { dataDashed: '' } : {}) }, marks);
	});
	const nodes = state.nodes.map((n) => {
		const cls = n.quiet ? 'sk-shape sk-quiet' : 'sk-shape';
		const shape: Element[] =
			n.shape === 'hex'
				? sketchy
					? sketchPath(gen.polygon(hexPoints(n), opts()), cls)
					: [h('polygon', { points: hexPoints(n).map((q) => q.join(',')).join(' '), className: cls.split(' ') })]
				: sketchy
					? sketchPath(gen.rectangle(n.x, n.y, n.w, n.h, opts()), cls)
					: [h('rect', { x: n.x, y: n.y, width: n.w, height: n.h, className: cls.split(' ') })];
		const lh = 13;
		const top = n.y + n.h / 2 - ((n.label.length - 1) * lh) / 2;
		const label = h(
			'text',
			{ className: n.quiet ? ['sk-label', 'sk-quiet-label'] : ['sk-label'], x: n.x + n.w / 2, y: top, dominantBaseline: 'middle', textAnchor: 'middle' },
			n.label.map((l, i) => h('tspan', { x: n.x + n.w / 2, dy: i === 0 ? '0' : String(lh) }, [l]))
		);
		return h('g', { dataNode: n.id, dataLabel: n.label.join(' ') }, [...shape, label]);
	});
	const svg = h(
		'svg',
		{ viewBox: `0 0 ${width} ${height}`, role: 'img', ariaLabel: `${STATE_KINDS[state.kind]}: ${state.label}. The table under "show the data" lists every box and arrow.`, className: ['sk-svg'] },
		[...edges, ...nodes]
	);
	return { svg, height };
}

function sketch(node: Element, ctx: LegibilityContext, p: Problems): Element | null {
	const seed = Number(attr(node, 'seed'));
	if (!Number.isInteger(seed) || seed < 1) {
		p.add(node, 'seed="N", a positive integer: Rough.js treats 0 as "no seed" and draws differently each build');
		return null;
	}
	const pre = kids(node).filter(isEl);
	if (pre.length !== 1 || pre[0].tagName !== 'pre') {
		p.add(node, '<sketch-figure> holds one fenced block: the structure, one statement a line');
		return null;
	}
	let parsed: ReturnType<typeof parseSketch>;
	try {
		parsed = parseSketch(toString(pre[0]), `${ctx.file}:${node.position?.start.line ?? '?'}`);
	} catch (e) {
		p.add(node, (e as Error).message);
		return null;
	}
	const id = `sketch-${slug(attr(node, 'id') ?? '')}`;
	const layers = parsed.states.map((s, i) => {
		const { svg } = drawState(s, parsed.width, seed + i);
		const caption = h('p', { className: ['sk-caption'] }, [h('strong', {}, [STATE_KINDS[s.kind]]), `: ${s.label}`]);
		return h('div', { className: ['sk-state'], dataState: s.id, dataKind: s.kind, ...(i === 0 ? { dataCurrent: '' } : {}) }, [caption, svg]);
	});
	const rows: Element[] = parsed.states.flatMap((s) => [
		...s.nodes.map((n) => h('tr', {}, [h('td', {}, [s.id]), h('td', {}, [STATE_KINDS[s.kind].toLowerCase()]), h('td', {}, ['box']), h('td', {}, [n.id]), h('td', {}, [n.label.join(' ')])])),
		...s.edges.map((e) => h('tr', {}, [h('td', {}, [s.id]), h('td', {}, [STATE_KINDS[s.kind].toLowerCase()]), h('td', {}, ['arrow']), h('td', {}, [`${e.from}>${e.to}`]), h('td', {}, [e.dashed ? `${e.from} to ${e.to}, dashed: removed` : `${e.from} to ${e.to}`])]))
	]);
	const twin = h('table', { dataTwinFor: id }, [
		h('thead', {}, [h('tr', {}, ['state', 'drawn', 'element', 'id', 'label'].map((c) => h('th', {}, [c])))]),
		h('tbody', {}, rows)
	]);
	const gen = rough.generator();
	const sample = gen.toPaths(gen.line(2, 7, 40, 7, { seed, roughness: 1.1, bowing: 0.8 })).map((pi) => h('path', { d: round(pi.d), className: ['sk-edge'] }));
	const toggle =
		parsed.states.length > 1
			? [
					h(
						'div',
						{ className: ['sk-toggle'], role: 'group', ariaLabel: 'Which state to show', hidden: true, dataPagefindIgnore: '' },
						parsed.states.map((s, i) => h('button', { type: 'button', dataShow: s.id, ariaPressed: i === 0 ? 'true' : 'false' }, [s.id]))
					)
				]
			: [];
	return h('figure', { className: ['sketch'], id, dataFigure: 'sketch', dataSeed: String(seed), ariaLabelledby: `${id}-title`, dataPagefindIgnore: '' }, [
		h('p', { className: ['sk-title'], id: `${id}-title` }, [attr(node, 'title') ?? '']),
		...toggle,
		h('div', { className: ['sk-states'] }, layers),
		h('details', { className: ['fig-data'] }, [h('summary', {}, ['show the data']), twin]),
		h('figcaption', { className: ['sk-legend'] }, [
			h('span', { className: ['sk-key'] }, [h('svg', { viewBox: '0 0 42 14', className: ['sk-swatch'], ariaHidden: 'true' }, sample), ' sketchy: ', h('strong', {}, ['proposed']), ', not decided']),
			h('span', { className: ['sk-key'] }, [
				h('svg', { viewBox: '0 0 42 14', className: ['sk-swatch'], ariaHidden: 'true' }, [h('line', { x1: 2, y1: 7, x2: 40, y2: 7, className: ['sk-edge'] })]),
				' crisp: ',
				h('strong', {}, ['shipped']),
				...(ctx.pin ? [' at ', h('code', {}, [ctx.pin.slice(0, 7)])] : [])
			]),
			...(parsed.states.some((s) => s.edges.some((e) => e.dashed))
				? [h('span', { className: ['sk-key'] }, [h('svg', { viewBox: '0 0 42 14', className: ['sk-swatch'], ariaHidden: 'true' }, [h('line', { x1: 2, y1: 7, x2: 40, y2: 7, className: ['sk-edge', 'sk-dashed'] })]), ' dashed: a dependency the proposal removes'])]
				: [])
		])
	]);
}

/* ------------------------------------------------------------------------ */
/* Lab experiment card                                                       */
/* ------------------------------------------------------------------------ */

const EXPERIMENT_PARTS = ['hypothesis', 'method', 'outcomes', 'result', 'provenance', 'limits'];

function experiment(node: Element, ctx: LegibilityContext, p: Problems): Element | null {
	const parts = new Map<string, Element>();
	for (const c of kids(node)) {
		if (!isEl(c) || !EXPERIMENT_PARTS.includes(c.tagName) || parts.has(c.tagName)) {
			p.add(isEl(c) ? c : node, `<experiment> holds one each of ${EXPERIMENT_PARTS.map((t) => `<${t}>`).join(', ')}`);
			continue;
		}
		parts.set(c.tagName, c);
	}
	for (const t of EXPERIMENT_PARTS) if (!parts.has(t)) p.add(node, `<experiment> is missing <${t}>`);
	if (EXPERIMENT_PARTS.some((t) => !parts.has(t))) return null;
	const hyp = parts.get('hypothesis')!;
	if (!/^\d{4}-\d{2}-\d{2}$/.test(attr(hyp, 'recorded') ?? '')) p.add(hyp, 'recorded="YYYY-MM-DD", the day the hypothesis was written, before the run');
	const commit = attr(hyp, 'commit') ?? '';
	if (!/^[0-9a-f]{40}$/.test(commit)) p.add(hyp, 'commit="<full SHA>", the commit the hypothesis was recorded at');
	else {
		const problem = commitProblem(ctx.repoRoot, commit);
		if (problem) p.add(hyp, problem);
	}
	const out = parts.get('outcomes')!;
	const values = (attr(out, 'values') ?? '').trim().split(/\s+/).map(Number);
	if (values.length === 0 || values.some((v) => !Number.isFinite(v))) {
		p.add(out, 'values="..." lists every run\'s outcome as numbers, separated by spaces; never a bare mean');
		return null;
	}
	const lo = attr(out, 'min') !== undefined ? Number(attr(out, 'min')) : Math.min(...values);
	const hi = attr(out, 'max') !== undefined ? Number(attr(out, 'max')) : Math.max(...values);
	const id = `exp-${slug(attr(node, 'id') ?? '')}`;
	const W = 400;
	const x = scaleLinear().domain(lo === hi ? [lo - 1, hi + 1] : [lo, hi]).range([16, W - 16]).nice();
	// A dot a run; runs that land on the same value stack upwards.
	const seen = new Map<string, number>();
	const dots = values.map((v, i) => {
		const key = x(v).toFixed(0);
		const stack = seen.get(key) ?? 0;
		seen.set(key, stack + 1);
		return h('circle', { cx: x(v).toFixed(1), cy: String(44 - stack * 9), r: '4', className: ['exp-dot'], dataRun: String(i + 1), dataValue: String(v) });
	});
	const ticks = x.ticks(5).map((t) =>
		h('g', {}, [h('line', { x1: x(t).toFixed(1), x2: x(t).toFixed(1), y1: '52', y2: '56', className: ['exp-tick'] }), h('text', { x: x(t).toFixed(1), y: '68', textAnchor: 'middle', className: ['exp-tick-label'] }, [String(t)])])
	);
	const unit = attr(out, 'unit') ?? '';
	const runs = h('table', { dataTwinFor: id }, [
		h('thead', {}, [h('tr', {}, [h('th', {}, ['run']), h('th', {}, [unit])])]),
		h('tbody', {}, values.map((v, i) => h('tr', {}, [h('td', {}, [String(i + 1)]), h('td', {}, [String(v)])])))
	]);
	const section = (cls: string, label: string, body: ElementContent[]) => h('div', { className: ['exp-part', cls] }, [h('p', { className: ['exp-label'] }, [label]), ...body]);
	return h('article', { className: ['experiment'], id, dataFigure: 'experiment', ariaLabelledby: `${id}-title` }, [
		h('p', { className: ['exp-kicker'] }, ['Experiment']),
		h('h3', { className: ['exp-title'], id: `${id}-title` }, [attr(node, 'title') ?? '']),
		section('exp-hypothesis', `Hypothesis and prediction, recorded ${attr(hyp, 'recorded')} at ${commit.slice(0, 7)}, before the run`, hyp.children),
		section('exp-result', 'Result', parts.get('result')!.children),
		h('figure', { className: ['exp-outcomes'] }, [
			h('svg', { viewBox: `0 0 ${W} 74`, role: 'img', ariaLabel: `${attr(out, 'label')}: ${values.length} runs, one dot each, in ${unit}. The table under "show the data" lists them.` }, [
				h('line', { x1: '16', x2: String(W - 16), y1: '52', y2: '52', className: ['exp-axis'] }),
				...ticks,
				...dots
			]),
			h('figcaption', {}, [`${attr(out, 'label')}: one dot a run, ${values.length} runs, in ${unit}.`])
		]),
		h('details', { className: ['fig-data'] }, [h('summary', {}, ['show the data']), runs]),
		h('details', { className: ['exp-method'] }, [h('summary', {}, ['method']), ...parts.get('method')!.children]),
		section('exp-limits', 'What this does not show', parts.get('limits')!.children),
		h('div', { className: ['exp-provenance'] }, parts.get('provenance')!.children)
	]);
}

/* ------------------------------------------------------------------------ */
/* The Blueprints index as cards                                             */
/* ------------------------------------------------------------------------ */

function recordIndex(node: Element, parent: Root | Element, index: number, ctx: LegibilityContext, p: Problems): Element | null {
	const kind = attr(node, 'kind');
	if (kind !== 'proposals' && kind !== 'plans') {
		p.add(node, 'kind="proposals" or kind="plans"');
		return null;
	}
	// The table that follows is the index as written; it becomes the cards' twin.
	let j = index + 1;
	while (j < parent.children.length && parent.children[j].type === 'text' && (parent.children[j] as { value: string }).value.trim() === '') j++;
	const table = parent.children[j];
	if (!isEl(table) || table.tagName !== 'table') {
		p.add(node, `<record-index kind="${kind}" /> is followed directly by the index's table, which becomes its twin`);
		return null;
	}
	parent.children.splice(index + 1, j - index);
	const prefix = kind === 'proposals' ? 'EPR-' : 'EPL-';
	const records = (ctx.records ?? []).filter((r) => r.id.startsWith(prefix));
	const id = `index-${kind}`;
	table.properties.dataTwinFor = id;
	const cards = records.map((r) => {
		const open = r.decisions.filter((d) => d.open);
		return h('li', { className: ['card'], dataRecord: r.id, dataStatus: r.status }, [
			h('p', { className: ['card-top'] }, [h('span', { className: ['card-id'] }, [r.id]), ' ', h('span', { className: ['card-status'] }, [r.status])]),
			h('p', { className: ['card-title'] }, [h('a', { href: `${ctx.base}${r.route}`, dataResolved: '' }, [r.title])]),
			...(r.summary ? [h('p', { className: ['card-summary'] }, [r.summary])] : []),
			h('div', { className: ['card-open'] }, [
				h('p', { className: ['card-count'], dataOpen: String(open.length) }, [
					open.length === 0 ? 'No open decisions' : `${open.length} open ${open.length === 1 ? 'decision' : 'decisions'}`,
					r.assumptions ? `, ${r.assumptions} ${r.assumptions === 1 ? 'assumption' : 'assumptions'} to confirm` : ''
				]),
				...(open.length ? [h('ol', { className: ['card-decisions'] }, open.map((d) => h('li', {}, [h('a', { href: `${ctx.base}${r.route}#decision-${d.id}`, dataResolved: '' }, [d.title])])))] : [])
			]),
			...(r.readAt ? [h('p', { className: ['card-read'] }, ['pinned at ', h('code', {}, [r.readAt.slice(0, 7)])])] : [])
		]);
	});
	return h('div', { className: ['record-index'], id, dataFigure: 'record-index', dataKind: kind }, [
		records.length ? h('ol', { className: ['cards'] }, cards) : h('p', { className: ['cards-none'] }, [`No ${kind} yet.`]),
		h('details', { className: ['fig-data'] }, [h('summary', {}, ['the index as a table']), table])
	]);
}

/* ------------------------------------------------------------------------ */
/* For the gates: render a fragment as a record would be                     */
/* ------------------------------------------------------------------------ */

/**
 * Markdown to HTML through the same component pass a record goes through,
 * without the site's other passes: what scripts/check-legibility.ts and the
 * responsive gate render their fixture with.
 */
export async function renderFragment(markdown: string, ctx: LegibilityContext): Promise<string> {
	const processor = unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(selfClosingTags)
		.use(remarkRehype, { allowDangerousHtml: true })
		.use(rehypeRaw)
		.use(legibility, ctx);
	const tree = (await processor.run(processor.parse(markdown))) as Root;
	// Tables scroll in their own box, as render.ts's wrapTables makes them on a page.
	visit(tree, 'element', (node, index, parent) => {
		if (node.tagName !== 'table' || !parent || index === undefined) return;
		parent.children[index] = h('div', { className: ['table-scroll'], tabIndex: 0 }, [node]);
		return SKIP;
	});
	return toHtml(tree);
}
