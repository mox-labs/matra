/**
 * Markdown to HTML, at build time.
 *
 * Every page is rendered here during prerendering, and the HTML is written
 * into the static page. Nothing about a page's content depends on a script
 * running in the reader's browser, and neither the parser nor the syntax
 * highlighter is shipped to it.
 *
 *   remark-parse, remark-gfm      Markdown and GitHub's extensions to it
 *   figureTags                    a registered figure tag on a line of its
 *                                 own becomes an element, before raw HTML is
 *                                 parsed (an HTML parser would read a
 *                                 self-closing custom tag as left open)
 *   remark-rehype, rehype-raw     to HTML, parsing any raw HTML in the page
 *   checkTags                     every element Markdown did not produce is
 *                                 in the tag registry, or the build fails
 *   rewriteLinks                  `../guides/cli.md#x` to the route it is
 *                                 served at; a link to no page fails the build
 *   rehype-slug                   heading ids, the same ids mdBook produced
 *   collectHeadings               the table of contents, and heading anchors
 *   shiki                         syntax highlighting, as CSS variables the
 *                                 site defines by role (app.css), so one
 *                                 theme serves the void and paper
 *   wrapTables                    tables scroll on their own on a phone
 *   wrapDiagrams                  so do diagrams, at a size their labels
 *                                 stay legible at, with a visible hint
 *   liftTitle                     the `# Title`, set apart by the layout
 *   measureMargins                beside each prose paragraph, matra's
 *                                 measures of it, when every paragraph can
 *                                 be matched to what matra measured
 *   toSegments                    the body as runs of HTML, with each figure
 *                                 between them, validated against its data
 */
import { posix } from 'node:path';
import { unified, type Plugin } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import remarkRehype from 'remark-rehype';
import rehypeRaw from 'rehype-raw';
import rehypeSlug from 'rehype-slug';
import rehypeShikiFromHighlighter from '@shikijs/rehype/core';
import { createCssVariablesTheme, createHighlighter } from 'shiki';
import { SKIP, visit } from 'unist-util-visit';
import { toString } from 'hast-util-to-string';
import { toHtml } from 'hast-util-to-html';
import type { Element, ElementContent, Root, RootContent } from 'hast';
import type { Root as MdRoot } from 'mdast';
import type {
	ClustersFigureFile,
	FigureFile,
	MeasuredLine,
	PageMeasures,
	KeyphrasesFigureFile,
	PipelineFigureFile,
	TextrankFigureFile,
	MetricsFigureFile,
	ParseFigureFile,
	PrimitivesFigureFile,
	Segment,
	TocEntry
} from '$lib/types';
import { MARKDOWN_ELEMENTS, REGISTRY } from './registry';
import { exampleView, type ExampleSource } from '../examples';

/**
 * The grammars the pages use. A fence in a language not listed here fails the
 * build rather than rendering unhighlighted, so adding a language is a
 * decision someone makes here. `text` and an unlabelled fence need no grammar.
 * `console` is shiki's alias for shellsession.
 */
const LANGUAGES = ['rust', 'python', 'bash', 'shellsession', 'json', 'jsonc'];
/**
 * The syntax theme. Each token kind is a CSS variable (`--syntax-token-*`),
 * and app.css gives each a role: literals Emergence, comments and
 * punctuation muted, the rest ink. Nothing here names a colour.
 */
const THEME = createCssVariablesTheme({ name: 'matra', variablePrefix: '--syntax-', fontStyle: true });

let highlighter: ReturnType<typeof createHighlighter> | undefined;
function getHighlighter() {
	highlighter ??= createHighlighter({ themes: [THEME], langs: LANGUAGES });
	return highlighter;
}

export interface Rendered {
	/** The page title as plain text, for `<title>` and navigation. */
	title: string;
	/** The page title as HTML, keeping inline code and emphasis. */
	titleHtml: string;
	/** The id the title's heading carried, kept so `page.html#id` links still land. */
	titleId: string;
	description: string;
	/** The page body, without its title: HTML runs, and figures between them. */
	segments: Segment[];
	toc: TocEntry[];
	measured: MeasuredLine | null;
}

export interface RenderContext {
	/** The page's path under site/content/, for error messages and link resolution. */
	file: string;
	/** Every page in SUMMARY.md, by file, mapped to the route it is served at. */
	routes: ReadonlyMap<string, string>;
	/** The configured base path, `''` locally and `/matra` on GitHub Pages. */
	base: string;
	/** Every figure data file, keyed `<input>/<figure>`. */
	figures: ReadonlyMap<string, FigureFile>;
	/** Every worked example in site/examples/, by name. */
	examples: ReadonlyMap<string, ExampleSource>;
	/** matra's measures of this page, when it has them. */
	measures?: PageMeasures;
}

export async function render(markdown: string, ctx: RenderContext): Promise<Rendered> {
	const toc: TocEntry[] = [];
	const meta = { title: '', description: '' };
	const heading = { html: '', id: '' };

	const processor = unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(figureTags)
		.use(remarkRehype, { allowDangerousHtml: true })
		.use(rehypeRaw)
		.use(stripComments)
		.use(checkTags, ctx)
		.use(rewriteLinks, ctx)
		.use(rehypeSlug)
		.use(collectHeadings, { toc, meta })
		.use(normalizeFenceLanguage, ctx)
		.use(rehypeShikiFromHighlighter, await getHighlighter(), {
			theme: THEME,
			defaultLanguage: 'text'
		})
		.use(wrapTables)
		.use(wrapDiagrams)
		.use(liftTitle, { file: ctx.file, heading });

	const tree = (await processor.run(processor.parse(markdown))) as Root;
	const measured = measureMargins(tree, ctx);
	const hl = await getHighlighter();
	const highlight = (code: string, lang: string) =>
		hl.codeToHtml(code, { lang, theme: THEME });
	return {
		title: meta.title,
		titleHtml: heading.html,
		titleId: heading.id,
		description: meta.description,
		segments: toSegments(tree, ctx, highlight),
		toc,
		measured
	};
}

/**
 * The self-measuring margin. matra measured this page's Markdown, paragraph
 * by paragraph (examples/docsite_figures.rs, through Ingest::text); here each
 * prose paragraph of the rendered page is given a note of those measures,
 * set in the margin beside it.
 *
 * Only top-level paragraphs are prose in the sense matra's Markdown reader
 * uses: list items, table rows, code and quotes are not. Each is matched to
 * the next measured paragraph with the same text, compared as letters and
 * digits only, so Markdown syntax on one side and rendering on the other do
 * not matter. Measured paragraphs with no rendered twin (a list, an HTML
 * block, a figure tag) are passed over. If any rendered paragraph finds no
 * match, the order cannot be trusted, and the page shows no notes at all:
 * no number is better than a number beside the wrong paragraph.
 */
function measureMargins(tree: Root, ctx: RenderContext): MeasuredLine | null {
	const m = ctx.measures;
	if (!m) return null;
	const dataUrl = `${ctx.base}/figures/pages/${ctx.file.replace(/\.md$/, '')}/measures.json`;
	const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, '');
	// Links keep their text (which may be a code span); then code spans are
	// text as written (`Box<dyn NlpProvider>` is not a tag), and outside them
	// tags and entities go.
	const fromMarkdown = (s: string) =>
		norm(
			s
				.replace(/!?\[((?:[^\]`]|`[^`]*`)*)\]\([^)]*\)/g, '$1')
				.split(/(`+)([\s\S]*?)\1/)
				.map((part, i) =>
					i % 3 === 2
						? part
						: i % 3 === 1
							? ''
							: part
									.replace(/<(https?:[^>\s]+)>/g, '$1')
									.replace(/<[^>]+>/g, ' ')
									.replace(/&[a-z]+;|&#\d+;/g, ' ')
				)
				.join('')
		);
	const measuredTexts = m.paragraphs.map((p) => fromMarkdown(p.text));

	const pairs: { index: number; node: Element; at: number }[] = [];
	let next = 0;
	for (const [index, node] of tree.children.entries()) {
		if (node.type !== 'element' || node.tagName !== 'p') continue;
		const text = norm(toString(node));
		if (text === '') continue;
		let found = -1;
		for (let k = next; k < measuredTexts.length; k++) {
			if (measuredTexts[k] === text) {
				found = k;
				break;
			}
		}
		if (found === -1) {
			const opening = toString(node).replace(/\s+/g, ' ').slice(0, 60);
			// Said at build time too, so an unmeasured page is never silent.
			console.warn(`measures: ${ctx.file}: no margin notes; no measured paragraph matches "${opening}"`);
			return {
				mapped: false,
				measured: 0,
				generator: m.generator,
				dataUrl,
				reason: `no measured paragraph matches "${opening}"`
			};
		}
		pairs.push({ index, node, at: found });
		next = found + 1;
	}

	const fmt = (v: number | null, digits: number) => (v === null ? null : v.toFixed(digits));
	// Insert from the end, so earlier indices stay valid.
	for (const { index, at } of pairs.reverse()) {
		const p = m.paragraphs[at];
		const rows: [string, string, string | null][] = [
			['grade', 'grade', fmt(p.readability_grade, 1)],
			['density', 'lexical density', fmt(p.lexical_density, 2)],
			['compression', 'compression', fmt(p.compression_ratio, 2)]
		];
		const said = rows.map(([, long, v]) => `${long} ${v ?? 'none'}`).join(', ');
		const note: Element = {
			type: 'element',
			tagName: 'div',
			properties: {
				className: ['measure'],
				role: 'note',
				ariaLabel: `matra's measures of the next paragraph: ${said}`,
				dataPagefindIgnore: '',
				dataParagraph: String(at + 1),
				dataGrade: p.readability_grade ?? '',
				dataDensity: p.lexical_density ?? '',
				dataCompression: p.compression_ratio ?? ''
			},
			children: rows.map(([short, , v]) => ({
				type: 'element',
				tagName: 'span',
				properties: { className: ['m-row'] },
				children: [
					{ type: 'element', tagName: 'span', properties: { className: ['m-k'] }, children: [{ type: 'text', value: short }] },
					{
						type: 'element',
						tagName: 'span',
						properties: { className: v === null ? ['m-v', 'm-none'] : ['m-v'] },
						children: [{ type: 'text', value: v ?? 'none' }]
					}
				]
			}))
		};
		tree.children.splice(index, 0, note);
	}
	return { mapped: true, measured: pairs.length, generator: m.generator, dataUrl };
}

/**
 * A figure tag on a line of its own: `<figure-name attr="value" ... />`.
 * Only a whole Markdown block matches; a figure tag anywhere else is left as
 * raw HTML, and checkTags then fails the build on it with the reason.
 */
const FIGURE_TAG = /^<((?:figure|example)-[a-z][a-z-]*)((?:\s+[a-z][a-z-]*="[^"<>]*")*)\s*\/>$/;
const FIGURE_ATTR = /([a-z][a-z-]*)="([^"<>]*)"/g;

const figureTags: Plugin<[], MdRoot> = () => (tree) => {
	tree.children = tree.children.map((node) => {
		if (node.type !== 'html') return node;
		const m = FIGURE_TAG.exec(node.value.trim());
		const kind = m ? REGISTRY[m[1]]?.kind : undefined;
		if (!m || (kind !== 'figure' && kind !== 'example')) return node;
		const properties: Record<string, string> = { dataMatraFigure: 'true' };
		for (const [, name, value] of m[2].matchAll(FIGURE_ATTR)) properties[name] = value;
		// An mdast node remark-rehype does not know becomes the element its
		// data names, with no children.
		return {
			type: 'figureTag',
			data: { hName: m[1], hProperties: properties },
			children: [],
			position: node.position
		} as unknown as typeof node;
	});
};

/**
 * The body as segments: each registered figure element at the top level
 * becomes a figure segment carrying its data, and the nodes between figures
 * are serialised to HTML runs. Every figure's attributes and data are checked
 * here; anything wrong fails the build with the page and line.
 */
function toSegments(
	tree: Root,
	ctx: RenderContext,
	highlight: (code: string, lang: string) => string
): Segment[] {
	const segments: Segment[] = [];
	const errors: string[] = [];
	let run: RootContent[] = [];
	let count = 0;
	const flush = () => {
		if (run.length === 0) return;
		const html = toHtml({ type: 'root', children: run });
		if (html.trim() !== '') segments.push({ kind: 'html', html });
		run = [];
	};

	for (const node of tree.children) {
		const entry = node.type === 'element' ? REGISTRY[node.tagName] : undefined;
		if (node.type !== 'element' || (entry?.kind !== 'figure' && entry?.kind !== 'example')) {
			run.push(node);
			continue;
		}
		const at = `${ctx.file}${node.position ? `:${node.position.start.line}` : ''}: <${node.tagName}>`;
		const props = node.properties ?? {};
		const attrs: Record<string, string> = {};
		for (const [key, value] of Object.entries(props)) {
			if (key === 'dataMatraFigure') continue;
			attrs[key] = String(value);
		}
		if (entry.kind === 'example') {
			const unknown = Object.keys(attrs).filter((k) => k !== 'name');
			const example = attrs.name === undefined ? undefined : ctx.examples.get(attrs.name);
			if (unknown.length || !example) {
				errors.push(
					`  ${at}: ${unknown.length ? `unknown attribute ${unknown.join(', ')}; ` : ''}` +
						(example ? '' : `no example called "${attrs.name ?? ''}" in site/examples/ ` +
							`(known: ${[...ctx.examples.keys()].join(', ') || 'none'})`)
				);
				continue;
			}
			flush();
			count += 1;
			segments.push({
				kind: 'example',
				part: entry.part,
				id: `ex-${example.name}-${entry.part}`,
				example: exampleView(example, ctx.figures, ctx.base, highlight)
			});
			continue;
		}
		const unknown = Object.keys(attrs).filter((k) => !(k in entry.attributes));
		const missing = Object.entries(entry.attributes)
			.filter(([k, need]) => need === 'required' && !(k in attrs))
			.map(([k]) => k);
		if (unknown.length || missing.length) {
			errors.push(
				`  ${at}: ${unknown.length ? `unknown attribute ${unknown.join(', ')}` : ''}` +
					`${unknown.length && missing.length ? '; ' : ''}` +
					`${missing.length ? `missing ${missing.join(', ')}` : ''}`
			);
			continue;
		}
		const key = `${attrs.input}/${entry.figure}`;
		const file = ctx.figures.get(key);
		if (!file) {
			const known = [...ctx.figures.keys()].filter((k) => k.endsWith(`/${entry.figure}`));
			errors.push(
				`  ${at}: no data at site/src/lib/figures/${key}.json ` +
					`(add site/inputs/${attrs.input}.txt and run cargo run --example docsite_figures; ` +
					`known: ${known.join(', ') || 'none'})`
			);
			continue;
		}
		const base = {
			kind: 'figure' as const,
			id: `fig-${ctx.file.replace(/\.md$/, '').replace(/[^a-z0-9]+/gi, '-')}-${count + 1}`,
			dataUrl: `${ctx.base}/figures/${key}.json`
		};
		let segment: Segment;
		switch (entry.figure) {
			case 'parse': {
				const parse = file as ParseFigureFile;
				const total = parse.data.sentences.length;
				const sentence = attrs.sentence === undefined ? 1 : Number(attrs.sentence);
				if (!Number.isInteger(sentence) || sentence < 1 || sentence > total) {
					errors.push(`  ${at}: sentence="${attrs.sentence}" is not between 1 and ${total}`);
					continue;
				}
				segment = { ...base, figure: 'parse', sentence, file: parse };
				break;
			}
			case 'primitives':
				segment = { ...base, figure: 'primitives', file: file as PrimitivesFigureFile };
				break;
			case 'metrics':
				segment = { ...base, figure: 'metrics', file: file as MetricsFigureFile };
				break;
			case 'keyphrases':
				segment = { ...base, figure: 'keyphrases', file: file as KeyphrasesFigureFile };
				break;
			case 'textrank':
				segment = { ...base, figure: 'textrank', file: file as TextrankFigureFile };
				break;
			case 'clusters': {
				const clusters = file as ClustersFigureFile;
				const grid = clusters.data.grid.map((g) => g.threshold);
				const threshold =
					attrs.threshold === undefined ? clusters.data.default_threshold : Number(attrs.threshold);
				if (!grid.some((t) => Math.abs(t - threshold) < 1e-9)) {
					errors.push(`  ${at}: threshold="${attrs.threshold}" is not on the grid (${grid.join(', ')})`);
					continue;
				}
				segment = { ...base, figure: 'clusters', threshold, file: clusters };
				break;
			}
			case 'pipeline':
				segment = { ...base, figure: 'pipeline', file: file as PipelineFigureFile };
				break;
		}
		flush();
		count += 1;
		segments.push(segment);
	}
	flush();
	if (errors.length > 0) throw new Error(`figures that cannot render:\n${errors.join('\n')}`);
	return segments;
}

/** HTML comments are notes to the author, not content. */
const stripComments: Plugin<[], Root> = () => (tree) => {
	visit(tree, 'comment', (_node, index, parent) => {
		if (parent && index !== undefined) {
			parent.children.splice(index, 1);
			return [SKIP, index];
		}
	});
};

const checkTags: Plugin<[RenderContext], Root> = (ctx) => (tree) => {
	const unknown: string[] = [];
	visit(tree, 'element', (node) => {
		if (MARKDOWN_ELEMENTS.has(node.tagName)) return;
		const entry = REGISTRY[node.tagName];
		if (entry?.kind === 'passthrough') return SKIP;
		const line = node.position?.start.line;
		if (entry?.kind === 'figure' || entry?.kind === 'example') {
			if (node.properties?.dataMatraFigure === 'true') return SKIP;
			unknown.push(
				`  ${ctx.file}${line ? `:${line}` : ''}: <${node.tagName}> must be one self-closing ` +
					'tag on a line of its own, with a blank line before and after'
			);
			return SKIP;
		}
		unknown.push(`  ${ctx.file}${line ? `:${line}` : ''}: <${node.tagName}>`);
	});
	if (unknown.length > 0) {
		throw new Error(
			`unregistered tags in page content:\n${unknown.join('\n')}\n` +
				'Register the tag in site/src/lib/server/markdown/registry.ts, or, ' +
				'if it was meant as text, put it in backticks.'
		);
	}
};

const rewriteLinks: Plugin<[RenderContext], Root> = (ctx) => (tree) => {
	const broken: string[] = [];
	visit(tree, 'element', (node) => {
		if (node.tagName !== 'a') return;
		const href = node.properties?.href;
		if (typeof href !== 'string') return;
		// External, same-page, and mail links stay as written.
		if (/^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith('#')) return;

		const [target, hash] = splitHash(href);
		if (href.startsWith('/') || !target.endsWith('.md')) {
			broken.push(`  ${ctx.file}: ${href} (a relative link to a .md page is the only local form)`);
			return;
		}
		const resolved = posix.normalize(posix.join(posix.dirname(ctx.file), target));
		const route = ctx.routes.get(resolved);
		if (!route) {
			broken.push(`  ${ctx.file}: ${href} (resolves to ${resolved}, which is not in SUMMARY.md)`);
			return;
		}
		node.properties.href = `${ctx.base}${route}${hash}`;
	});
	if (broken.length > 0) {
		throw new Error(`links that resolve to no page:\n${broken.join('\n')}`);
	}
};

function splitHash(href: string): [string, string] {
	const i = href.indexOf('#');
	return i === -1 ? [href, ''] : [href.slice(0, i), href.slice(i)];
}

/**
 * The title, the description, and the table of contents, read off the tree
 * after heading ids are assigned. Each h2 to h6 also gets an anchor link, hidden
 * from assistive technology and search, since the heading itself is announced.
 */
const collectHeadings: Plugin<
	[{ toc: TocEntry[]; meta: { title: string; description: string } }],
	Root
> =
	({ toc, meta }) =>
	(tree) => {
		for (const node of tree.children) {
			if (node.type === 'element' && node.tagName === 'p') {
				meta.description = firstSentence(toString(node));
				break;
			}
		}
		visit(tree, 'element', (node) => {
			const depth = /^h([1-6])$/.exec(node.tagName)?.[1];
			if (!depth) return;
			const text = toString(node).trim();
			if (depth === '1') {
				meta.title ||= text;
				return SKIP;
			}
			const id = node.properties?.id;
			if (typeof id !== 'string') return SKIP;
			if (depth === '2' || depth === '3') toc.push({ depth: Number(depth) as 2 | 3, id, text });
			node.children.push({
				type: 'element',
				tagName: 'a',
				properties: {
					className: ['heading-anchor'],
					href: `#${id}`,
					ariaHidden: 'true',
					tabIndex: -1,
					dataPagefindIgnore: ''
				},
				children: [{ type: 'text', value: '#' }]
			});
			return SKIP;
		});
	};

function firstSentence(text: string): string {
	const flat = text.replace(/\s+/g, ' ').trim();
	const end = /^(.{40,}?[.!?])(\s|$)/.exec(flat);
	const sentence = end ? end[1] : flat;
	return sentence.length > 200 ? `${sentence.slice(0, 197)}...` : sentence;
}

/**
 * mdBook reads `rust,ignore` as the language `rust` with an attribute. Shiki
 * reads the whole string as a language name. Keep the part before the comma,
 * and fail on a language with no grammar loaded.
 */
const normalizeFenceLanguage: Plugin<[RenderContext], Root> = (ctx) => (tree) => {
	visit(tree, 'element', (node) => {
		if (node.tagName !== 'code') return;
		const classes = node.properties?.className;
		if (!Array.isArray(classes)) return;
		node.properties.className = classes.map((c) => {
			const name = String(c);
			if (!name.startsWith('language-')) return name;
			const lang = name.slice('language-'.length).split(',')[0];
			const known = ['text', 'console', ...LANGUAGES];
			if (!known.includes(lang)) {
				const line = node.position?.start.line;
				throw new Error(
					`${ctx.file}${line ? `:${line}` : ''}: code fence language "${lang}" has no grammar ` +
						'loaded. Add it to LANGUAGES in site/src/lib/server/markdown/render.ts.'
				);
			}
			return `language-${lang}`;
		});
	});
};

/**
 * Wide reference tables scroll on their own instead of pushing the page
 * sideways. The wrapper takes focus so the table can be scrolled from the
 * keyboard.
 */
const wrapTables: Plugin<[], Root> = () => (tree) => {
	visit(tree, 'element', (node, index, parent) => {
		if (node.tagName !== 'table' || !parent || index === undefined) return;
		const wrapper: Element = {
			type: 'element',
			tagName: 'div',
			properties: { className: ['table-scroll'], tabIndex: 0 },
			children: [node as ElementContent]
		};
		parent.children[index] = wrapper;
		return SKIP;
	});
};

/**
 * The hand-drawn diagrams are drawn 720 units wide with labels down to about
 * 9.5 units. Shrunk to a phone's width, those labels become unreadable, so a
 * diagram keeps a minimum width (set in app.css) and scrolls sideways inside
 * its own container instead. The page never does.
 *
 * The SVG is untouched; only a wrapper is added. The scroller takes focus so
 * it can be scrolled from the keyboard, and is labelled with the diagram's own
 * label. The hint below it shows only when the container is narrower than the
 * diagram, and is hidden from assistive technology and search.
 */
const wrapDiagrams: Plugin<[], Root> = () => (tree) => {
	tree.children = tree.children.map((node) => {
		if (node.type !== 'element' || node.tagName !== 'svg') return node;
		const label = node.properties?.ariaLabel;
		const scroller: Element = {
			type: 'element',
			tagName: 'div',
			properties: {
				className: ['diagram-scroll'],
				tabIndex: 0,
				role: 'region',
				ariaLabel: typeof label === 'string' ? `Diagram: ${label}` : 'Diagram'
			},
			children: [node]
		};
		const hint: Element = {
			type: 'element',
			tagName: 'p',
			properties: { className: ['diagram-hint'], ariaHidden: 'true', dataPagefindIgnore: '' },
			children: [{ type: 'text', value: 'Scroll sideways to see the whole diagram.' }]
		};
		return {
			type: 'element',
			tagName: 'div',
			properties: { className: ['diagram'] },
			children: [scroller, hint]
		} satisfies Element;
	});
};

/**
 * Every page opens with its `# Title`. The title is lifted out of the body so
 * the layout can set it, with the part it belongs to, above the page. A page
 * whose first element is anything else fails the build.
 */
const liftTitle: Plugin<[{ file: string; heading: { html: string; id: string } }], Root> =
	({ file, heading }) =>
	(tree) => {
		const i = tree.children.findIndex((n) => n.type === 'element');
		const first = tree.children[i];
		if (!first || first.type !== 'element' || first.tagName !== 'h1') {
			throw new Error(`${file}: a page must open with its # title`);
		}
		heading.html = toHtml(first.children);
		heading.id = typeof first.properties.id === 'string' ? first.properties.id : '';
		tree.children.splice(i, 1);
	};
