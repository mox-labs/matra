/**
 * The local comments store under contention, with a stale lock planted.
 *
 * The owner (through the dev server) and Claude (through `just
 * comment-reply`) are two processes writing one page's file. Each of 50
 * rounds plants a lock file a minute old, as a killed writer would leave it,
 * then starts two writer processes at the same instant. Both must take the
 * stale lock over without both proceeding at once: afterwards the file holds
 * every line written, each exactly once, each parsing whole, and no lock,
 * guard or temporary file is left. A takeover that lets two writers in loses
 * a line (each renames its own copy of the file into place) and fails here.
 *
 * Runs in `bun run check`, so in gate 4 of the docsite floor.
 *
 * Usage: bun scripts/test-comments-store.ts
 */
import { mkdtempSync, readdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { parseLines } from '../src/lib/dev/comments/model.ts';
import { Store } from '../src/lib/dev/comments/store.ts';

const ROUNDS = 50;
const route = '/blueprints/rfcs/0019-rfc-and-ep-process';
const root = mkdtempSync(join(tmpdir(), 'matra-comments-'));
const worker = join(import.meta.dir, 'comments-append-worker.ts');
const fail = (msg: string): never => {
	console.error(`FAIL (comments store): ${msg}`);
	rmSync(root, { recursive: true, force: true });
	process.exit(1);
};

try {
	const store = new Store(root);
	const thread = store.comment(
		route,
		'owner',
		{ type: 'TextQuoteSelector', exact: 'a quote', prefix: '', suffix: '', heading: null },
		'the root'
	).id;
	const file = store.fileOf(route);
	const lock = `${file}.lock`;
	const old = new Date(Date.now() - 60_000);

	const expected: string[] = [];
	for (let round = 0; round < ROUNDS; round++) {
		writeFileSync(lock, 'dead.0');
		utimesSync(lock, old, old);
		const start = Date.now() + 250;
		const bodies = [`round ${round} writer a`, `round ${round} writer b`];
		expected.push(...bodies);
		// Writer a acts on the stale lock at once; writer b judges it stale at
		// the same instant but acts 20ms later, while a holds its fresh lock.
		const runs = bodies.map((body, i) =>
			Bun.spawn(['bun', worker, root, route, thread, String(start), body, i === 0 ? '0' : '20'], { stderr: 'pipe' })
		);
		const codes = await Promise.all(runs.map((r) => r.exited));
		for (const [i, code] of codes.entries()) {
			if (code !== 0) fail(`round ${round}: a writer exited ${code}: ${await new Response(runs[i].stderr).text()}`);
		}
	}

	const { entries, problems } = parseLines(readFileSync(file, 'utf8'));
	if (problems.length) fail(`lines that do not parse: ${JSON.stringify(problems.slice(0, 3))}`);
	const bodies = entries.flatMap((e) => ('body' in e && e.parent !== null ? [e.body] : []));
	const missing = expected.filter((b) => !bodies.includes(b));
	if (missing.length) fail(`${missing.length} lines lost, e.g. "${missing[0]}"`);
	if (bodies.length !== expected.length) fail(`${bodies.length} replies for ${expected.length} writes`);
	const leftover = readdirSync(dirname(file)).filter((f) => !f.endsWith('.jsonl'));
	if (leftover.length) fail(`left behind: ${leftover.join(', ')}`);
	console.log(`PASS (comments store): ${ROUNDS} rounds of two writers over a stale lock, ${expected.length} lines, none lost or torn`);
} finally {
	rmSync(root, { recursive: true, force: true });
}
