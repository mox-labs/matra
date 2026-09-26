/**
 * The worked examples (EP-0012, M6), read from site/examples/ at build time.
 *
 * Each example directory holds one task called three ways and what the calls
 * print:
 *
 *   example.json   which input (a name in site/inputs/), the file name the
 *                  calls read it from, and how the page trims the output
 *   main.rs        the Rust call, a whole program
 *   example.py     the Python call
 *   cli.sh         the command line, one line
 *   output.json    what the Rust call prints; the Python call prints the same
 *   cli.json       what the CLI prints
 *
 * The page shows these files as they are. scripts/check-examples.ts runs all
 * three calls and fails when what they print is not what is committed, so the
 * page cannot show a call that does not work or an output matra no longer
 * gives.
 */
import { SITE_URL } from '$lib/site';
import type { ExampleView, FigureFile } from '$lib/types';

const FILES = import.meta.glob('/examples/*/*', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

const INPUTS = import.meta.glob('/inputs/*.txt', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

interface OutputSpec {
	/** Dot path into the output to show, e.g. `sections.0.paragraphs`. */
	path?: string;
	/** Keys whose value is collapsed to a count. */
	omit?: string[];
	/** Lists show this many items, then a count of the rest. */
	keep?: number;
}

/**
 * From nothing to the call, per language: the command that installs matra,
 * and for a program, the file to save it as and the command that runs it.
 * Optional; the quick start and the tutorial carry one. The examples gate
 * runs the calls, not these lines: they are the installation page's commands.
 */
interface StartSpec {
	install: string;
	save?: string;
	run?: string;
}

interface Spec {
	input: string;
	file: string;
	/** One shell line that writes the input as `file`; the examples gate runs it. */
	save?: string;
	output?: OutputSpec;
	start?: Record<'rust' | 'python' | 'cli', StartSpec>;
}

export interface ExampleSource {
	name: string;
	spec: Spec;
	input: string;
	files: Record<'main.rs' | 'example.py' | 'cli.sh' | 'output.json' | 'cli.json', string>;
}

const PARTS = ['main.rs', 'example.py', 'cli.sh', 'output.json', 'cli.json'] as const;

function load(): Map<string, ExampleSource> {
	const byName = new Map<string, Record<string, string>>();
	for (const [path, text] of Object.entries(FILES)) {
		const [, , name, file] = path.split('/');
		if (!byName.has(name)) byName.set(name, {});
		byName.get(name)![file] = text;
	}
	const out = new Map<string, ExampleSource>();
	for (const [name, files] of byName) {
		const where = `site/examples/${name}`;
		if (!files['example.json']) throw new Error(`${where} has no example.json`);
		const spec = JSON.parse(files['example.json']) as Spec;
		for (const part of PARTS) {
			if (files[part] === undefined) {
				throw new Error(`${where} has no ${part}${part.endsWith('.json') ? '; run just docs-examples' : ''}`);
			}
		}
		const input = INPUTS[`/inputs/${spec.input}.txt`];
		if (input === undefined) throw new Error(`${where}: no input site/inputs/${spec.input}.txt`);
		out.set(name, {
			name,
			spec,
			input,
			files: Object.fromEntries(PARTS.map((p) => [p, files[p]])) as ExampleSource['files']
		});
	}
	return out;
}

export const examples: ReadonlyMap<string, ExampleSource> = load();

/** The files published for an example, by name: its input, and what the calls print. */
export function publishedFiles(ex: ExampleSource): Record<string, { body: string; type: string }> {
	return {
		[ex.spec.file]: {
			body: ex.input,
			type: ex.spec.file.endsWith('.md') ? 'text/markdown; charset=utf-8' : 'text/plain; charset=utf-8'
		},
		'output.json': { body: ex.files['output.json'], type: 'application/json; charset=utf-8' },
		'cli.json': { body: ex.files['cli.json'], type: 'application/json; charset=utf-8' }
	};
}

type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

const LONG_STRING = 72;

/**
 * JSON as `JSON.stringify(v, null, 2)` writes it, trimmed for a page: lists
 * cut to `keep` items with a comment counting the rest, `omit` keys collapsed
 * to a count, long strings shortened. Comments make it JSONC; the page says it
 * is trimmed and links the whole file.
 */
function trimmedText(value: Json, spec: OutputSpec, flags: { strings: boolean }): string {
	const omit = new Set(spec.omit ?? []);
	const keep = spec.keep ?? Infinity;
	const pad = (n: number) => '  '.repeat(n);
	const scalar = (v: Json): string => {
		if (typeof v === 'string' && v.length > LONG_STRING) {
			flags.strings = true;
			return JSON.stringify(`${v.slice(0, LONG_STRING - 1)}…`);
		}
		return JSON.stringify(v);
	};
	const walk = (v: Json, depth: number, key: string | null): string => {
		if (Array.isArray(v)) {
			if (v.length === 0) return '[]';
			if (key !== null && omit.has(key)) return `[ /* ${v.length} ${v.length === 1 ? 'item' : 'items'} */ ]`;
			const shown = v.slice(0, keep).map((x) => `${pad(depth + 1)}${walk(x, depth + 1, null)}`);
			const rest = v.length - Math.min(v.length, keep);
			const lines = shown.join(',\n') + (rest > 0 ? `\n${pad(depth + 1)}// … ${rest} more` : '');
			return `[\n${lines}\n${pad(depth)}]`;
		}
		if (v !== null && typeof v === 'object') {
			const entries = Object.entries(v);
			if (entries.length === 0) return '{}';
			const lines = entries.map(([k, x]) => `${pad(depth + 1)}${JSON.stringify(k)}: ${walk(x, depth + 1, k)}`);
			return `{\n${lines.join(',\n')}\n${pad(depth)}}`;
		}
		return scalar(v);
	};
	return walk(value, 0, null);
}

function select(value: Json, path: string | undefined, where: string): { value: Json; label: string } {
	if (!path) return { value, label: '' };
	let v = value;
	let label = '';
	for (const step of path.split('.')) {
		if (Array.isArray(v) && /^\d+$/.test(step) && Number(step) < v.length) {
			v = v[Number(step)];
			label += `[${step}]`;
		} else if (v !== null && typeof v === 'object' && !Array.isArray(v) && step in v) {
			v = v[step];
			label += `.${step}`;
		} else {
			throw new Error(`${where}: output path ${path} does not exist in output.json at "${step}"`);
		}
	}
	return { value: v, label };
}

type Highlight = (code: string, lang: string) => string;

/**
 * The output as the page shows it, and how it was trimmed: a note in
 * Markdown (code in backticks), or null when the output is whole.
 */
function shownOutput(ex: ExampleSource): { text: string; note: string | null } {
	const output = JSON.parse(ex.files['output.json']) as Json;
	const spec = ex.spec.output ?? {};
	const { value, label } = select(output, spec.path, `site/examples/${ex.name}`);
	const flags = { strings: false };
	const text = trimmedText(value, spec, flags);
	const notes: string[] = [];
	if (label) notes.push(`only \`result${label}\``);
	if (spec.omit?.length) notes.push(`${spec.omit.map((k) => `\`${k}\``).join(' and ')} shown as a count`);
	if (text.includes('// … ')) notes.push(`lists cut to the first ${spec.keep}`);
	if (flags.strings) notes.push('long strings shortened');
	return { text, note: notes.length ? notes.join(', ') : null };
}

const codeHtml = (md: string) => md.replace(/`([^`]+)`/g, '<code>$1</code>');

function sourceOf(ex: ExampleSource, figures: ReadonlyMap<string, FigureFile>): Record<string, string> {
	const figure = [...figures.values()].find((f) => f.input === ex.spec.input);
	if (!figure) {
		throw new Error(
			`site/examples/${ex.name}: input ${ex.spec.input} has no figure data, so no source to show; run just docs-figures`
		);
	}
	return figure.source;
}

function envelopeText(ex: ExampleSource): string {
	const cli = JSON.parse(ex.files['cli.json']) as Record<string, Json>;
	const lines = Object.entries(cli).map(
		([k, v]) => `  ${JSON.stringify(k)}: ${k === 'result' ? '/* the output below */' : JSON.stringify(v)}`
	);
	return `{\n${lines.join(',\n')}\n}`;
}

/**
 * One part of an example as Markdown, for the page's `.md` twin, so an agent
 * reading the source gets the input, the calls and the output rather than a
 * tag naming them. Links are absolute: the twin is read outside the site.
 */
export function exampleMarkdown(
	ex: ExampleSource,
	part: 'input' | 'call' | 'output',
	figures: ReadonlyMap<string, FigureFile>
): string {
	const files = `${SITE_URL}/example-files/${ex.name}`;
	if (part === 'input') {
		const src = sourceOf(ex, figures);
		return [
			`[${src.title}](${src.url}), ${src.author}. ${src.licence}.`,
			'',
			`The calls read it as \`${ex.spec.file}\` in the directory they run from:`,
			'',
			'```bash',
			`curl -fLO ${files}/${ex.spec.file}`,
			'```'
		].join('\n');
	}
	if (part === 'call') {
		const start = (lang: 'rust' | 'python' | 'cli') => {
			const s = ex.spec.start?.[lang];
			if (!s) return [];
			return [
				'Install:',
				'',
				'```bash',
				s.install,
				'```',
				'',
				...(ex.spec.save ? ['Save the text:', '', '```bash', ex.spec.save, '```', ''] : []),
				...(s.save ? [`Save the program below as \`${s.save}\`${s.run ? ` and run it with \`${s.run}\`` : ''}:`, ''] : [])
			];
		};
		return [
			'Rust:',
			'',
			...start('rust'),
			'```rust',
			ex.files['main.rs'].trimEnd(),
			'```',
			'',
			'Python:',
			'',
			...start('python'),
			'```python',
			ex.files['example.py'].trimEnd(),
			'```',
			'',
			'The command line, which prints the output below inside an envelope:',
			'',
			...start('cli'),
			'```bash',
			ex.files['cli.sh'].trim(),
			'```',
			'',
			'```jsonc',
			envelopeText(ex),
			'```'
		].join('\n');
	}
	const shown = shownOutput(ex);
	return [
		...(shown.note ? [`Trimmed: ${shown.note}.`, ''] : []),
		shown.note ? '```jsonc' : '```json',
		shown.text,
		'```',
		'',
		`The whole output: ${files}/output.json, and what the CLI prints: ${files}/cli.json.`
	].join('\n');
}

/** Everything the page shows for one example, highlighted and trimmed. */
export function exampleView(ex: ExampleSource, figures: ReadonlyMap<string, FigureFile>, base: string, hl: Highlight): ExampleView {
	const shown = shownOutput(ex);
	const command = ex.files['cli.sh'].trim();
	const files = `${base}/example-files/${ex.name}`;

	return {
		name: ex.name,
		input: {
			name: ex.spec.input,
			file: ex.spec.file,
			text: ex.input,
			words: ex.input.split(/\s+/).filter(Boolean).length,
			source: sourceOf(ex, figures),
			url: `${files}/${ex.spec.file}`
		},
		calls: [
			{ lang: 'rust', label: 'Rust', file: 'main.rs', html: hl(ex.files['main.rs'].trimEnd(), 'rust') },
			{ lang: 'python', label: 'Python', file: 'example.py', html: hl(ex.files['example.py'].trimEnd(), 'python') },
			{ lang: 'cli', label: 'CLI', file: 'cli.sh', html: hl(`$ ${command}`, 'shellsession') }
		].map((call) => {
			const s = ex.spec.start?.[call.lang as 'rust' | 'python' | 'cli'];
			if (!s) return call;
			const shell = (cmds: string) =>
				hl(
					cmds
						.split('\n')
						.map((c) => `$ ${c}`)
						.join('\n'),
					'shellsession'
				);
			return {
				...call,
				start: {
					installHtml: shell(s.install),
					inputHtml: ex.spec.save ? shell(ex.spec.save) : undefined,
					save: s.save,
					runHtml: s.run ? shell(s.run) : undefined
				}
			};
		}) as ExampleView['calls'],
		cliEnvelopeHtml: hl(envelopeText(ex), 'jsonc'),
		output: {
			html: hl(shown.text, shown.note ? 'jsonc' : 'json'),
			trimmed: shown.note ? codeHtml(shown.note) : null,
			url: `${files}/output.json`,
			cliUrl: `${files}/cli.json`
		}
	};
}
