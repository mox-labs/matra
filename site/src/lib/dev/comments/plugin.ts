/**
 * The local comments endpoint, `/__comments`, on the dev server only.
 *
 * A Vite plugin with `apply: 'serve'`: Vite never runs it for `vite build`, so
 * the static site has no endpoint to call, and the UI that calls it is loaded
 * only under `import.meta.env.DEV` (src/routes/+layout.svelte).
 * `scripts/check-no-dev-comments.ts` fails the build if either reaches it.
 *
 *   GET  /__comments?page=<route>   the page's lines, folded into threads
 *   POST /__comments                {op: "comment", page, selector, body}
 *                                   {op: "reply", page, thread, body}
 *                                   {op: "status", page, thread, status}
 *
 * Whoever posts here is the owner: the author is set here, never read from
 * the request. Claude writes through the same store from the command line
 * (`just comment-reply`), as "claude".
 *
 * The server listens on loopback only (vite.config.ts), and every request is
 * held to it: the Host must be a loopback name (a rebinding page cannot
 * borrow it), a POST must come from this server's own origin, as JSON (which a
 * page elsewhere cannot send without a preflight this server never answers),
 * and its body is capped. When a page's file changes, from either writer, the
 * browser is told over Vite's socket, so a reply appears without a reload.
 */
import { relative, sep } from 'node:path';
import type { IncomingMessage, ServerResponse } from 'node:http';
import type { Plugin, ViteDevServer } from 'vite';
import { CHANGED, ENDPOINT } from './constants.ts';
import { CommentError, Store } from './store.ts';

const MAX_REQUEST = 64 * 1024;
const LOOPBACK_HOST = /^(localhost|127\.0\.0\.1|\[::1\])(:\d+)?$/;
const LOOPBACK_ADDR = new Set(['127.0.0.1', '::1', '::ffff:127.0.0.1']);

export function devComments({ repoRoot }: { repoRoot: string }): Plugin {
	const store = new Store(repoRoot);
	return {
		name: 'matra-dev-comments',
		apply: 'serve',
		configureServer(server: ViteDevServer) {
			server.watcher.add(store.dir);
			const changed = (file: string) => {
				if (!file.startsWith(store.dir + sep) || !file.endsWith('.jsonl')) return;
				const page = `/${relative(store.dir, file).split(sep).join('/').replace(/\.jsonl$/, '')}`;
				server.ws.send({ type: 'custom', event: CHANGED, data: { page } });
			};
			server.watcher.on('add', changed);
			server.watcher.on('change', changed);

			server.middlewares.use(ENDPOINT, (req, res) => {
				handle(store, req, res).catch((err: unknown) => {
					const status = err instanceof CommentError ? err.status : 500;
					send(res, status, { error: (err as Error).message });
				});
			});
		}
	};
}

async function handle(store: Store, req: IncomingMessage, res: ServerResponse) {
	if (!LOOPBACK_ADDR.has(req.socket.remoteAddress ?? '')) throw new CommentError('loopback only', 403);
	const host = req.headers.host ?? '';
	if (!LOOPBACK_HOST.test(host)) throw new CommentError('loopback only', 403);
	const url = new URL(req.url ?? '/', `http://${host}`);

	if (req.method === 'GET') {
		const page = url.searchParams.get('page') ?? '';
		return send(res, 200, { page, ...store.threads(page) });
	}
	if (req.method !== 'POST') throw new CommentError('GET or POST', 405);

	if (req.headers.origin !== `http://${host}`) throw new CommentError('same origin only', 403);
	const site = req.headers['sec-fetch-site'];
	if (site !== undefined && site !== 'same-origin') throw new CommentError('same origin only', 403);
	if (!/^application\/json\b/.test(req.headers['content-type'] ?? '')) {
		throw new CommentError('send JSON', 415);
	}
	const body = await readBody(req);
	let msg: Record<string, unknown>;
	try {
		msg = JSON.parse(body);
	} catch {
		throw new CommentError('not JSON');
	}
	if (typeof msg !== 'object' || msg === null) throw new CommentError('not an object');
	const page = msg.page as string;
	const thread = typeof msg.thread === 'string' ? msg.thread : '';
	switch (msg.op) {
		case 'comment':
			store.comment(page, 'owner', msg.selector, msg.body);
			break;
		case 'reply':
			store.reply(page, 'owner', thread, msg.body);
			break;
		case 'status':
			store.setStatus(page, 'owner', thread, msg.status);
			break;
		default:
			throw new CommentError('op must be comment, reply or status');
	}
	return send(res, 200, { page, ...store.threads(page) });
}

function readBody(req: IncomingMessage): Promise<string> {
	return new Promise((ok, fail) => {
		const chunks: Buffer[] = [];
		let size = 0;
		req.on('data', (c: Buffer) => {
			size += c.length;
			if (size > MAX_REQUEST) {
				fail(new CommentError(`a request is at most ${MAX_REQUEST} bytes`, 413));
				req.destroy();
				return;
			}
			chunks.push(c);
		});
		req.on('end', () => ok(Buffer.concat(chunks).toString('utf8')));
		req.on('error', fail);
	});
}

function send(res: ServerResponse, status: number, value: unknown) {
	if (res.headersSent) return;
	res.statusCode = status;
	res.setHeader('content-type', 'application/json; charset=utf-8');
	res.setHeader('cache-control', 'no-store');
	res.end(JSON.stringify(value));
}
