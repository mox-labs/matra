/**
 * Every worked example's calls still print what its page shows (EP-0012, M6).
 *
 * An example in site/examples/<name>/ is one input and one task, called three
 * ways: `main.rs` (Rust), `example.py` (Python) and `cli.sh` (the command
 * line). This runs each of the three in a fresh directory holding only the
 * input, under the file name the calls read, and compares what each printed
 * with what is committed beside the example:
 *
 *   output.json   what the Rust call printed; the Python call must print the
 *                 same value, and so must the `result` of the CLI's
 *   cli.json      what the CLI printed, its envelope included
 *
 * The comparison is of values, not bytes, with two allowances that are
 * matra's documented behaviour rather than slack: numbers agree to ten
 * significant digits, since a sum over a hash map can differ in its last bit
 * from run to run, and items with equal scores may come back in either
 * order. A tie at an example's cut, where which phrase makes the list
 * changes, is not allowed for; an example must not ask for one.
 *
 * With --write, the Rust and CLI output are written as the new committed
 * files (`just docs-examples`), and the Python call is still compared.
 *
 * The calls run with an empty configuration file, so a contributor's own
 * matra config cannot change what they print, and with the caller's model
 * directory, so a cached model is used rather than fetched.
 *
 * Environment:
 *   EXAMPLES_RUNNER    the built `docsite_examples` binary (required)
 *   MATRA_BIN          the built `matra` binary (required); the CLI call
 *                      runs with its directory first on PATH
 *   EXAMPLES_PYTHON    a Python that can `import matra` (default: python3)
 *   EXAMPLES_REQUIRED  when 1, a Python without matra fails the check
 *                      instead of skipping the Python calls (CI sets it)
 *
 * Usage: bun scripts/check-examples.ts [--write]
 */
import { spawnSync } from 'node:child_process';
import {
	copyFileSync,
	existsSync,
	mkdtempSync,
	readdirSync,
	readFileSync,
	realpathSync,
	statSync,
	writeFileSync
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';

const SITE = resolve(import.meta.dir, '..');
const EXAMPLES = join(SITE, 'examples');
const INPUTS = join(SITE, 'inputs');
const write = process.argv.includes('--write');

if (!process.env.EXAMPLES_RUNNER || !process.env.MATRA_BIN) {
	console.error('check-examples: set EXAMPLES_RUNNER and MATRA_BIN to the built binaries');
	process.exit(2);
}
// The calls run from a scratch directory, so a relative path would miss.
const runner = resolve(process.env.EXAMPLES_RUNNER);
const matraBin = resolve(process.env.MATRA_BIN);
if (basename(matraBin) !== 'matra') {
	console.error(`check-examples: MATRA_BIN must name a binary called matra, got ${matraBin}`);
	process.exit(2);
}
const python = process.env.EXAMPLES_PYTHON || 'python3';
const required = process.env.EXAMPLES_REQUIRED === '1';

// A matra config file of the caller's would change what the calls print.
const scratch = mkdtempSync(join(tmpdir(), 'matra-examples-'));
const emptyConfig = join(scratch, 'config.toml');
writeFileSync(emptyConfig, '');
const env: Record<string, string> = { ...process.env, MATRA_CONFIG_FILE: emptyConfig } as Record<
	string,
	string
>;
delete env.EXAMPLES_RUNNER;

const pythonHasMatra =
	spawnSync(python, ['-c', 'import matra'], { env, encoding: 'utf8' }).status === 0;
if (!pythonHasMatra && required) {
	console.error(
		`FAIL (examples): ${python} cannot import matra, and EXAMPLES_REQUIRED=1. ` +
			'Install it (maturin develop) or set EXAMPLES_PYTHON.'
	);
	process.exit(1);
}

type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

interface Example {
	name: string;
	input: string;
	file: string;
	dir: string;
}

function readExample(name: string): Example {
	const dir = join(EXAMPLES, name);
	const spec = JSON.parse(readFileSync(join(dir, 'example.json'), 'utf8')) as {
		input?: unknown;
		file?: unknown;
	};
	if (typeof spec.input !== 'string' || typeof spec.file !== 'string') {
		throw new Error(`site/examples/${name}/example.json needs input and file, as strings`);
	}
	if (!existsSync(join(INPUTS, `${spec.input}.txt`))) {
		throw new Error(`site/examples/${name}: no input site/inputs/${spec.input}.txt`);
	}
	if (!/^[a-z0-9-]+\.(txt|md)$/.test(spec.file)) {
		throw new Error(`site/examples/${name}: file "${spec.file}" must be a plain .txt or .md name`);
	}
	for (const part of ['main.rs', 'example.py', 'cli.sh']) {
		if (!existsSync(join(dir, part))) throw new Error(`site/examples/${name}: no ${part}`);
	}
	return { name, input: spec.input, file: spec.file, dir };
}

interface Run {
	stdout: string;
	failure: string | null;
}

function run(cmd: string, args: string[], cwd: string, extraPath?: string): Run {
	const res = spawnSync(cmd, args, {
		cwd,
		env: extraPath ? { ...env, PATH: `${extraPath}:${env.PATH ?? ''}` } : env,
		encoding: 'utf8',
		maxBuffer: 64 * 1024 * 1024
	});
	if (res.error) return { stdout: '', failure: String(res.error) };
	if (res.status !== 0) {
		return {
			stdout: res.stdout,
			failure: `exit ${res.status}: ${res.stderr.trim().split('\n').slice(-3).join(' / ')}`
		};
	}
	return { stdout: res.stdout, failure: null };
}

function parse(text: string, what: string): Json {
	try {
		return JSON.parse(text) as Json;
	} catch (e) {
		throw new Error(`${what} is not JSON: ${(e as Error).message}`);
	}
}

/** Ten significant digits: last-bit noise goes, a real change does not. */
function round(n: number): number {
	return Number.isInteger(n) ? n : Number(n.toPrecision(10));
}

/** Numbers rounded, and runs of equal-score items put in one order. */
function canonical(v: Json): Json {
	if (typeof v === 'number') return round(v);
	if (Array.isArray(v)) {
		const items = v.map(canonical);
		const scored = items.every(
			(x) => x !== null && typeof x === 'object' && !Array.isArray(x) && typeof x.score === 'number'
		);
		if (!scored) return items;
		const out: Json[] = [];
		let i = 0;
		while (i < items.length) {
			const score = (items[i] as { score: number }).score;
			let j = i;
			while (j < items.length && (items[j] as { score: number }).score === score) j++;
			out.push(...items.slice(i, j).sort((a, b) => (JSON.stringify(a) < JSON.stringify(b) ? -1 : 1)));
			i = j;
		}
		return out;
	}
	if (v !== null && typeof v === 'object') {
		return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, canonical(x)]));
	}
	return v;
}

/** Where two values first differ, as a path, or null when they agree. */
function firstDifference(a: Json, b: Json, path = '$'): string | null {
	if (Array.isArray(a) && Array.isArray(b)) {
		for (let i = 0; i < Math.max(a.length, b.length); i++) {
			if (i >= a.length || i >= b.length) return `${path} has ${b.length} items, expected ${a.length}`;
			const d = firstDifference(a[i], b[i], `${path}[${i}]`);
			if (d) return d;
		}
		return null;
	}
	if (a && b && typeof a === 'object' && typeof b === 'object' && !Array.isArray(a) && !Array.isArray(b)) {
		const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
		for (const k of keys) {
			if (!(k in a)) return `${path}.${k} is not expected`;
			if (!(k in b)) return `${path}.${k} is missing`;
			const d = firstDifference(a[k], b[k], `${path}.${k}`);
			if (d) return d;
		}
		return null;
	}
	if (a === b) return null;
	return `${path} is ${JSON.stringify(b)}, expected ${JSON.stringify(a)}`;
}

const failures: string[] = [];
const names = readdirSync(EXAMPLES)
	.filter((n) => statSync(join(EXAMPLES, n)).isDirectory())
	.sort();
if (names.length === 0) {
	console.error('FAIL (examples): no examples in site/examples/: a check that ran nothing has not passed');
	process.exit(1);
}

let calls = 0;
for (const name of names) {
	let ex: Example;
	try {
		ex = readExample(name);
	} catch (e) {
		failures.push(`  ${(e as Error).message}`);
		continue;
	}
	const work = mkdtempSync(join(scratch, `${name}-`));
	copyFileSync(join(INPUTS, `${ex.input}.txt`), join(work, ex.file));
	const fail = (msg: string) => failures.push(`  ${name}: ${msg}`);

	const rust = run(runner, [name], work);
	calls++;
	if (rust.failure) {
		fail(`the Rust call failed (${rust.failure})`);
		continue;
	}
	const cliCommand = readFileSync(join(ex.dir, 'cli.sh'), 'utf8').trim();
	if (cliCommand.includes('\n') || !cliCommand.startsWith('matra ')) {
		fail('cli.sh must be one line that runs matra');
		continue;
	}
	const cli = run('sh', ['-c', cliCommand], work, dirname(realpathSync(matraBin)));
	calls++;
	if (cli.failure) {
		fail(`the CLI call failed (${cli.failure})`);
		continue;
	}

	const outputPath = join(ex.dir, 'output.json');
	const cliPath = join(ex.dir, 'cli.json');
	if (write) {
		writeFileSync(outputPath, rust.stdout);
		writeFileSync(cliPath, cli.stdout);
	}
	if (!existsSync(outputPath) || !existsSync(cliPath)) {
		fail('no committed output.json or cli.json; run just docs-examples');
		continue;
	}
	try {
		const committed = canonical(parse(readFileSync(outputPath, 'utf8'), 'output.json'));
		const committedCli = canonical(parse(readFileSync(cliPath, 'utf8'), 'cli.json'));

		const d = firstDifference(committed, canonical(parse(rust.stdout, 'the Rust output')));
		if (d) fail(`the Rust call's output differs from output.json: ${d}`);

		const cliNow = canonical(parse(cli.stdout, 'the CLI output'));
		const dc = firstDifference(committedCli, cliNow);
		if (dc) fail(`the CLI call's output differs from cli.json: ${dc}`);
		const envelope = cliNow as { input?: Json; result?: Json };
		if (envelope.input !== ex.file) fail(`the CLI's input is ${JSON.stringify(envelope.input)}, not ${ex.file}`);
		const dr = firstDifference(committed, envelope.result ?? null, '$.result');
		if (dr) fail(`the CLI's result differs from output.json: ${dr}`);

		if (pythonHasMatra) {
			const py = run(python, [join(ex.dir, 'example.py')], work);
			calls++;
			if (py.failure) fail(`the Python call failed (${py.failure})`);
			else {
				const dp = firstDifference(committed, canonical(parse(py.stdout, 'the Python output')));
				if (dp) fail(`the Python call's output differs from output.json: ${dp}`);
			}
		}
	} catch (e) {
		fail((e as Error).message);
	}
}

const skipped = pythonHasMatra ? '' : `; Python calls skipped (${python} cannot import matra)`;
if (failures.length > 0) {
	console.error(`FAIL (examples): ${failures.length} problem(s) in ${names.length} examples:`);
	console.error(failures.join('\n'));
	process.exit(1);
}
console.log(
	`${write ? 'WROTE' : 'PASS'} (examples): ${names.length} examples, ${calls} calls` +
		`${write ? '' : ' agree with the committed output'}${skipped}`
);
