/**
 * Local comments: the record format, and how a page's lines fold into threads.
 *
 * Dev only. This module is imported by the comments UI, which the layout loads
 * only under `import.meta.env.DEV`, and by the dev server's middleware and the
 * `just comments` script. Nothing in the static build imports it, and
 * `scripts/check-no-dev-comments.ts` fails the build if anything does.
 *
 * A page's comments are `discussion/<route>.jsonl` at the repository root,
 * one JSON object per line, appended and never rewritten. A line is either a
 * comment (it has a `body`) or a status event (it has a `status`). A thread is
 * a root comment (`parent` null, with a `selector`), the replies whose
 * `thread` is the root's id, and the status events that resolve or reopen it.
 * The last status event wins; a thread with none is open.
 *
 * The selector is the W3C Web Annotation TextQuoteSelector (exact, prefix,
 * suffix, each in the page's rendered text with whitespace collapsed), plus
 * the id of the nearest heading before the quote, so a quote that occurs
 * twice can be told apart after the page around it changes.
 */

export const AUTHORS = ['owner', 'claude'] as const;
export type Author = (typeof AUTHORS)[number];

/** Limits, enforced where a line is written; a line over them is refused, not cut. */
export const LIMITS = {
	/** Characters in a comment's body. */
	body: 8000,
	/** Characters in the quoted text. */
	exact: 2000,
	/** Characters of context either side of it. */
	affix: 64,
	/** Characters in a page route. */
	route: 200,
	/** Bytes in one page's file; a page past it takes no more comments. */
	file: 4 * 1024 * 1024
} as const;

/** The context kept either side of a quote. */
export const AFFIX = 32;

export interface Selector {
	type: 'TextQuoteSelector';
	exact: string;
	prefix: string;
	suffix: string;
	/** The id of the nearest heading before the quote, or null above the first. */
	heading: string | null;
}

interface Base {
	id: string;
	/** The id of the thread's root comment; a root's thread is its own id. */
	thread: string;
	/** The comment this one answers, or null for a thread's root. */
	parent: string | null;
	author: Author;
	/** ISO 8601, UTC. */
	created: string;
	/** The page's route, as the site serves it: `/blueprints/proposals/0001-...`. */
	page: string;
}

export interface CommentEntry extends Base {
	/** On a thread's root only. */
	selector: Selector | null;
	body: string;
}

export interface StatusEntry extends Base {
	parent: string;
	status: 'resolved' | 'open';
}

export type Entry = CommentEntry | StatusEntry;

export interface Thread {
	id: string;
	page: string;
	selector: Selector;
	status: 'open' | 'resolved';
	messages: CommentEntry[];
}

export interface Problem {
	line: number;
	error: string;
}

const ID = /^[0-9a-f]{8}$/;
const ROUTE = /^\/[a-z0-9][a-z0-9._-]*(\/[a-z0-9][a-z0-9._-]*)*$/;
const HEADING = /^[A-Za-z0-9_-]{1,200}$/;
// Control characters other than tab and newline have no place in a comment.
// eslint-disable-next-line no-control-regex
const CONTROL = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/;

/** A page route is a path the site serves: lowercase segments, no `..`, no trailing slash. */
export function validRoute(route: unknown): route is string {
	return (
		typeof route === 'string' &&
		route.length <= LIMITS.route &&
		ROUTE.test(route) &&
		!route.split('/').some((s) => s === '.' || s === '..')
	);
}

function text(v: unknown, max: number, what: string, empty = true): string {
	if (typeof v !== 'string') throw new Error(`${what} must be a string`);
	if (!empty && v.trim() === '') throw new Error(`${what} is empty`);
	if (v.length > max) throw new Error(`${what} is longer than ${max} characters`);
	if (CONTROL.test(v)) throw new Error(`${what} carries a control character`);
	return v;
}

export function checkSelector(v: unknown): Selector {
	if (typeof v !== 'object' || v === null) throw new Error('selector must be an object');
	const s = v as Record<string, unknown>;
	if (s.type !== 'TextQuoteSelector') throw new Error('selector.type must be TextQuoteSelector');
	const heading = s.heading ?? null;
	if (heading !== null && (typeof heading !== 'string' || !HEADING.test(heading))) {
		throw new Error('selector.heading must be a heading id or null');
	}
	return {
		type: 'TextQuoteSelector',
		exact: text(s.exact, LIMITS.exact, 'selector.exact', false),
		prefix: text(s.prefix, LIMITS.affix, 'selector.prefix'),
		suffix: text(s.suffix, LIMITS.affix, 'selector.suffix'),
		heading
	};
}

export function checkBody(v: unknown): string {
	return text(v, LIMITS.body, 'body', false);
}

/** One line, checked field by field; anything else on it is refused. */
export function checkEntry(v: unknown): Entry {
	if (typeof v !== 'object' || v === null || Array.isArray(v)) throw new Error('not an object');
	const e = v as Record<string, unknown>;
	for (const k of ['id', 'thread'] as const) {
		if (typeof e[k] !== 'string' || !ID.test(e[k] as string)) throw new Error(`${k} must be 8 hex digits`);
	}
	if (e.parent !== null && (typeof e.parent !== 'string' || !ID.test(e.parent))) {
		throw new Error('parent must be 8 hex digits or null');
	}
	if (!AUTHORS.includes(e.author as Author)) throw new Error(`author must be one of ${AUTHORS.join(', ')}`);
	if (typeof e.created !== 'string' || Number.isNaN(Date.parse(e.created))) throw new Error('created must be a date');
	if (!validRoute(e.page)) throw new Error('page must be a site route');
	const base = {
		id: e.id as string,
		thread: e.thread as string,
		author: e.author as Author,
		created: e.created,
		page: e.page
	};
	if ('status' in e) {
		if (e.status !== 'resolved' && e.status !== 'open') throw new Error('status must be resolved or open');
		if (typeof e.parent !== 'string') throw new Error('a status event names its thread as parent');
		const known = ['id', 'thread', 'parent', 'author', 'created', 'page', 'status'];
		const extra = Object.keys(e).filter((k) => !known.includes(k));
		if (extra.length) throw new Error(`unknown field ${extra.join(', ')}`);
		return { ...base, parent: e.parent, status: e.status };
	}
	const known = ['id', 'thread', 'parent', 'author', 'created', 'page', 'selector', 'body'];
	const extra = Object.keys(e).filter((k) => !known.includes(k));
	if (extra.length) throw new Error(`unknown field ${extra.join(', ')}`);
	const root = e.parent === null;
	if (root && e.thread !== e.id) throw new Error("a thread's root carries its own id as thread");
	return {
		...base,
		parent: (e.parent as string | null) ?? null,
		selector: root ? checkSelector(e.selector) : null,
		body: checkBody(e.body)
	};
}

/** Parse a page's file. A line that does not parse is reported, never dropped silently. */
export function parseLines(source: string): { entries: Entry[]; problems: Problem[] } {
	const entries: Entry[] = [];
	const problems: Problem[] = [];
	source.split('\n').forEach((line, i) => {
		if (line.trim() === '') return;
		try {
			entries.push(checkEntry(JSON.parse(line)));
		} catch (err) {
			problems.push({ line: i + 1, error: (err as Error).message });
		}
	});
	return { entries, problems };
}

/** Fold a page's lines into threads, in the order they were opened. */
export function fold(entries: Entry[]): { threads: Thread[]; problems: Problem[] } {
	const threads = new Map<string, Thread>();
	const problems: Problem[] = [];
	entries.forEach((e, i) => {
		if ('status' in e) {
			const t = threads.get(e.thread);
			if (!t) problems.push({ line: i + 1, error: `status for unknown thread ${e.thread}` });
			else t.status = e.status;
			return;
		}
		if (e.parent === null) {
			if (threads.has(e.id)) problems.push({ line: i + 1, error: `thread ${e.id} opened twice` });
			else threads.set(e.id, { id: e.id, page: e.page, selector: e.selector!, status: 'open', messages: [e] });
			return;
		}
		const t = threads.get(e.thread);
		if (!t) problems.push({ line: i + 1, error: `reply to unknown thread ${e.thread}` });
		else t.messages.push(e);
	});
	return { threads: [...threads.values()], problems };
}
