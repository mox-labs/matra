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

/**
 * An exclusive lock: a file created with O_EXCL. Waits up to two seconds; a
 * lock older than ten seconds belongs to a writer that died, and is taken over.
 */
function lock(path: string): () => void {
	const deadline = Date.now() + 2000;
	for (;;) {
		try {
			closeSync(openSync(path, 'wx'));
			return () => {
				try {
					unlinkSync(path);
				} catch {
					// Already gone: a writer that judged it stale took it over.
				}
			};
		} catch (err) {
			if ((err as NodeJS.ErrnoException).code !== 'EEXIST') throw err;
			try {
				if (Date.now() - statSync(path).mtimeMs > 10_000) unlinkSync(path);
			} catch {
				// Released between the two calls; try again.
			}
			if (Date.now() > deadline) throw new CommentError(`${path} is locked by another writer`, 503);
			Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 20);
		}
	}
}

/** The repository root, from site/ (where the dev server and the scripts run). */
export function repoRootFrom(siteDir: string): string {
	return resolve(siteDir, '..');
}
