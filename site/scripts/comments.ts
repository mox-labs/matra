/**
 * The local comments from the command line: Claude's side of them.
 *
 *   list [--all]                     every open thread on every page (with
 *                                    --all, resolved ones too): the page, the
 *                                    thread, the quoted text, the last message
 *   reply <page> <thread> <body>     append a reply as "claude"
 *
 * `just comments` and `just comment-reply` run these. They read and write the
 * same `discussion/<route>.jsonl` files as the dev server, through the same
 * store (src/lib/dev/comments/store.ts): validated, locked, written whole and
 * renamed into place. A page is its route (`/blueprints/proposals/0001-...`); the
 * forms `blueprints/proposals/0001-...`, `.html` and the discussion file's own
 * path are read as the same page.
 *
 * Whether a thread still anchors is decided in the browser, against the page
 * as rendered; this lists every thread and does not guess.
 *
 * Usage: bun scripts/comments.ts list [--all]
 *        bun scripts/comments.ts reply <page> <thread> <body>
 */
import { existsSync, readdirSync, statSync } from 'node:fs';
import { join, relative, resolve, sep } from 'node:path';
import { CommentError, Store } from '../src/lib/dev/comments/store.ts';

const store = new Store(resolve(import.meta.dir, '..', '..'));

function routeOf(arg: string): string {
	let p = arg.trim();
	const inDir = resolve(p);
	if (inDir.startsWith(store.dir + sep)) p = relative(store.dir, inDir).split(sep).join('/');
	p = p.replace(/^discussion\//, '').replace(/\.(jsonl|html|md)$/, '').replace(/\/$/, '');
	return p.startsWith('/') ? p : `/${p}`;
}

function pages(dir: string): string[] {
	if (!existsSync(dir)) return [];
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return pages(path);
		return name.endsWith('.jsonl') ? [`/${relative(store.dir, path).split(sep).join('/').replace(/\.jsonl$/, '')}`] : [];
	});
}

const flat = (s: string, n: number) => {
	const one = s.replace(/\s+/g, ' ').trim();
	return one.length > n ? `${one.slice(0, n - 1)}…` : one;
};

function list(all: boolean): number {
	let shown = 0;
	let troubled = 0;
	for (const page of pages(store.dir).sort()) {
		const { threads, problems } = store.threads(page);
		for (const p of problems) {
			troubled += 1;
			console.log(`${page}  line ${p.line} could not be read: ${p.error}`);
		}
		for (const t of threads) {
			if (!all && t.status !== 'open') continue;
			shown += 1;
			const last = t.messages[t.messages.length - 1];
			console.log(`${page}  thread ${t.id}  ${t.status}  ${t.messages.length} message${t.messages.length === 1 ? '' : 's'}`);
			console.log(`  quote: "${flat(t.selector.exact, 160)}"${t.selector.heading ? `  (under #${t.selector.heading})` : ''}`);
			console.log(`  last:  ${last.author}, ${last.created}: ${flat(last.body, 400)}`);
		}
	}
	if (shown === 0) console.log(all ? 'no threads' : 'no open threads');
	return troubled > 0 ? 1 : 0;
}

const [cmd, ...args] = process.argv.slice(2);
try {
	if (cmd === 'list') {
		process.exit(list(args.includes('--all')));
	} else if (cmd === 'reply' && args.length === 3) {
		const [page, thread, body] = args;
		const route = routeOf(page);
		const e = store.reply(route, 'claude', thread, body);
		console.log(`replied in ${route}, thread ${thread}: ${e.id}`);
	} else {
		console.error('usage: bun scripts/comments.ts list [--all]');
		console.error('       bun scripts/comments.ts reply <page> <thread> <body>');
		process.exit(2);
	}
} catch (err) {
	console.error(`comments: ${(err as Error).message}`);
	process.exit(err instanceof CommentError ? 1 : 2);
}
