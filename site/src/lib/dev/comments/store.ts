/**
 * Local comments on disk: `discussion/<route>.jsonl` at the repository root.
 *
 * Dev only, and Node only: the dev server's middleware (./plugin.ts) and the
 * `just comments` script (site/scripts/comments.ts) write through here, and
 * nothing the site builds imports it.
 *
 * Every write is the whole file, replaced: the current lines and the new one
 * are written to a temporary file beside it, flushed, and renamed over it, so a
 * reader sees the file before the line or after it, never part of a line. A
 * lock file taken with O_EXCL serializes the two writers (the dev server and
 * the script), so neither replaces the other's line. A path is resolved and
 * held inside `discussion/` before anything is read or written, symlinks
 * included.
 */
import { randomBytes } from 'node:crypto';
import {
	closeSync,
	existsSync,
	fsyncSync,
	lstatSync,
	mkdirSync,
	openSync,
	readFileSync,
	realpathSync,
	renameSync,
	statSync,
	unlinkSync,
	writeSync
} from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
import {
	checkBody,
	checkEntry,
	checkSelector,
	fold,
	LIMITS,
	parseLines,
	validRoute,
	type Author,
	type CommentEntry,
	type Entry,
	type Problem,
	type StatusEntry,
	type Thread
} from './model.ts';

export class CommentError extends Error {
	constructor(
		message: string,
		readonly status = 400
	) {
		super(message);
	}
}

export class Store {
	readonly dir: string;

	constructor(readonly repoRoot: string) {
		this.dir = resolve(repoRoot, 'discussion');
	}

	/** The file a route's comments live in, held inside discussion/. */
	fileOf(route: string): string {
		if (!validRoute(route)) throw new CommentError(`not a page route: ${JSON.stringify(route)}`);
		const file = resolve(this.dir, `${route.slice(1)}.jsonl`);
		if (!file.startsWith(this.dir + sep)) throw new CommentError('the path leaves discussion/');
		// A symlink anywhere on the way could lead out of discussion/: the
		// deepest part of the path that exists must resolve inside it.
		let existing = file;
		while (!existsSync(existing) && existing !== this.dir) existing = dirname(existing);
		if (existsSync(existing)) {
			const real = realpathSync(existing);
			const realDir = existsSync(this.dir) ? realpathSync(this.dir) : this.dir;
			if (real !== realDir && !real.startsWith(realDir + sep)) {
				throw new CommentError('the path resolves outside discussion/');
			}
			if (existing === file && !lstatSync(file).isFile()) throw new CommentError('not a regular file');
		}
		return file;
	}

	read(route: string): { entries: Entry[]; problems: Problem[] } {
		const file = this.fileOf(route);
		if (!existsSync(file)) return { entries: [], problems: [] };
		if (statSync(file).size > LIMITS.file) throw new CommentError(`${relative(this.repoRoot, file)} is over ${LIMITS.file} bytes`, 413);
		return parseLines(readFileSync(file, 'utf8'));
	}

	threads(route: string): { threads: Thread[]; problems: Problem[] } {
		const { entries, problems } = this.read(route);
		const folded = fold(entries);
		return { threads: folded.threads, problems: [...problems, ...folded.problems] };
	}

	/** Open a thread on a quote. */
	comment(route: string, author: Author, selector: unknown, body: unknown): CommentEntry {
		const id = this.newId(route);
		return this.append(route, {
			id,
			thread: id,
			parent: null,
			author,
			created: new Date().toISOString(),
			page: route,
			selector: checkSelector(selector),
			body: checkBody(body)
		}) as CommentEntry;
	}

	/** Answer a thread, after its last message unless `parent` names another. */
	reply(route: string, author: Author, thread: string, body: unknown, parent?: string): CommentEntry {
		const t = this.thread(route, thread);
		const to = parent ?? t.messages[t.messages.length - 1].id;
		if (!t.messages.some((m) => m.id === to)) throw new CommentError(`no message ${to} in thread ${thread}`);
		return this.append(route, {
			id: this.newId(route),
			thread,
			parent: to,
			author,
			created: new Date().toISOString(),
			page: route,
			selector: null,
			body: checkBody(body)
		}) as CommentEntry;
	}

	/** Resolve or reopen a thread: a line of its own, so the history stays. */
	setStatus(route: string, author: Author, thread: string, status: unknown): StatusEntry {
		if (status !== 'resolved' && status !== 'open') throw new CommentError('status must be resolved or open');
		this.thread(route, thread);
		return this.append(route, {
			id: this.newId(route),
			thread,
			parent: thread,
			author,
			created: new Date().toISOString(),
			page: route,
			status
		}) as StatusEntry;
	}

	private thread(route: string, id: string): Thread {
		const t = this.threads(route).threads.find((x) => x.id === id);
		if (!t) throw new CommentError(`no thread ${JSON.stringify(id)} on ${route}`, 404);
		return t;
	}

	private newId(route: string): string {
		const taken = new Set(this.read(route).entries.map((e) => e.id));
		for (;;) {
			const id = randomBytes(4).toString('hex');
			if (!taken.has(id)) return id;
		}
	}

	private append(route: string, entry: Entry): Entry {
		const checked = checkEntry(JSON.parse(JSON.stringify(entry)));
		const line = `${JSON.stringify(checked)}\n`;
		const file = this.fileOf(route);
		mkdirSync(dirname(file), { recursive: true });
		this.fileOf(route); // again, now that the directories exist
		const release = lock(`${file}.lock`);
		try {
			let current = existsSync(file) ? readFileSync(file, 'utf8') : '';
			if (Buffer.byteLength(current) + Buffer.byteLength(line) > LIMITS.file) {
				throw new CommentError(`${relative(this.repoRoot, file)} would pass ${LIMITS.file} bytes`, 413);
			}
			// A file edited by hand may have lost its last newline; the new line
			// must not join it.
			if (current !== '' && !current.endsWith('\n')) current += '\n';
			testPause('hold');
			const tmp = `${file}.${process.pid}.${randomBytes(4).toString('hex')}.tmp`;
			const fd = openSync(tmp, 'wx', 0o644);
			try {
				writeSync(fd, current + line);
				fsyncSync(fd);
			} finally {
				closeSync(fd);
			}
			try {
				renameSync(tmp, file);
			} catch (err) {
				unlinkSync(tmp);
				throw err;
			}
		} finally {
			release();
		}
		return checked;
	}
}

/** A lock older than this belongs to a writer that died: a write holds it for milliseconds. */
const STALE_MS = 10_000;
const WAIT_MS = 2000;

/**
 * Test only: scripts/test-comments-store.ts sets these to widen the two
 * windows a broken lock loses lines in, so its test fails on a broken
 * takeover every time rather than by chance: `judged`, between judging a lock
 * stale and acting on it, and `hold`, between reading the file and replacing
 * it. Unset, they do nothing.
 */
function testPause(at: 'judged' | 'hold') {
	const ms = Number(process.env[`MATRA_COMMENTS_TEST_${at.toUpperCase()}_MS`] ?? 0);
	if (ms > 0) sleep(Math.min(ms, 1000));
}

const sleep = (ms: number) => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
const ageOf = (path: string): number | null => {
	try {
		return Date.now() - statSync(path).mtimeMs;
	} catch {
		return null;
	}
};

/**
 * An exclusive lock: a file created with O_EXCL, holding its owner's token
 * (pid and a random nonce). Waits up to two seconds.
 *
 * Taking over a stale lock is where a naive lock breaks: two waiters both see
 * the stale file, the first removes it and creates its own, and the second
 * then removes the first's fresh lock, so two writers proceed. So a takeover
 * is itself serialized, by a second O_EXCL file (`<lock>.takeover`): only its
 * holder may remove the lock, and it removes it only after looking again,
 * under that guard, and finding it still stale. A waiter that saw the old
 * stale lock and gets the guard later finds the new holder's fresh lock and
 * leaves it alone. Ordinary release removes only a lock that still holds its
 * own token. The guard is held for one stat and one unlink, so it is never
 * taken over: one older than STALE_MS means a process died inside that
 * window, and the error names the file to delete.
 *
 * This was chosen over the alternatives: renaming the stale lock aside still
 * lets a late waiter rename a fresh one, and comparing a token before
 * unlinking leaves a window between the read and the unlink; dropping
 * staleness altogether would leave the owner's comments refused after any
 * killed dev server until someone deleted the file by hand.
 */
function lock(path: string): () => void {
	const token = `${process.pid}.${randomBytes(8).toString('hex')}`;
	const guard = `${path}.takeover`;
	const deadline = Date.now() + WAIT_MS;
	for (;;) {
		try {
			const fd = openSync(path, 'wx', 0o644);
			try {
				writeSync(fd, token);
			} finally {
				closeSync(fd);
			}
			return () => {
				try {
					if (readFileSync(path, 'utf8') === token) unlinkSync(path);
				} catch {
					// Already gone.
				}
			};
		} catch (err) {
			if ((err as NodeJS.ErrnoException).code !== 'EEXIST') throw err;
		}
		const age = ageOf(path);
		if (age !== null && age > STALE_MS) {
			testPause('judged');
			takeOver(path, guard);
		}
		if (Date.now() > deadline) throw new CommentError(`${path} is locked by another writer`, 503);
		sleep(5 + Math.floor(Math.random() * 15));
	}
}

/** Remove the lock at `path` if, under the takeover guard, it is still stale. */
function takeOver(path: string, guard: string) {
	try {
		closeSync(openSync(guard, 'wx'));
	} catch (err) {
		if ((err as NodeJS.ErrnoException).code !== 'EEXIST') throw err;
		const age = ageOf(guard);
		if (age !== null && age > STALE_MS) {
			throw new CommentError(`${guard} was left by a writer that died; delete it if no writer is running`, 503);
		}
		return; // Another waiter is taking over; wait for its outcome.
	}
	try {
		const age = ageOf(path);
		if (age !== null && age > STALE_MS) unlinkSync(path);
	} finally {
		unlinkSync(guard);
	}
}

/** The repository root, from site/ (where the dev server and the scripts run). */
export function repoRootFrom(siteDir: string): string {
	return resolve(siteDir, '..');
}
