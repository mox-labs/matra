/**
 * Markdown to HTML, at build time.
 *
 * Every page is rendered here during prerendering, and the HTML is written
 * into the static page. Nothing about a page's content depends on a script
 * running in the reader's browser, and neither the parser nor the syntax
 * highlighter is shipped to it.
 *
 *   remark-parse, remark-gfm      Markdown and GitHub's extensions to it
 *   remark-rehype, rehype-raw     to HTML, parsing any raw HTML in the page
 *   checkTags                     every element Markdown did not produce is
 *                                 in the tag registry, or the build fails
 *   rewriteLinks                  `../guides/cli.md#x` to the route it is
 *                                 served at; a link to no page fails the build
 *   rehype-slug                   heading ids, the same ids mdBook produced
 *   collectHeadings               the table of contents, and heading anchors
 *   shiki                         syntax highlighting, light and dark
 *   wrapTables                    tables scroll on their own on a phone
 *   liftTitle                     the `# Title`, set apart by the layout
 */
import { posix } from 'node:path';
import { unified, type Plugin } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import remarkRehype from 'remark-rehype';
import rehypeRaw from 'rehype-raw';
import rehypeSlug from 'rehype-slug';
import rehypeShikiFromHighlighter from '@shikijs/rehype/core';
import rehypeStringify from 'rehype-stringify';
import { createHighlighter } from 'shiki';
import { SKIP, visit } from 'unist-util-visit';
import { toString } from 'hast-util-to-string';
import { toHtml } from 'hast-util-to-html';
import type { Element, ElementContent, Root } from 'hast';
import type { TocEntry } from '$lib/types';
import { MARKDOWN_ELEMENTS, REGISTRY } from './registry';

/**
 * The grammars the pages use. A fence in a language not listed here fails the
 * build rather than rendering unhighlighted, so adding a language is a
 * decision someone makes here. `text` and an unlabelled fence need no grammar.
 * `console` is shiki's alias for shellsession.
 */
const LANGUAGES = ['rust', 'python', 'bash', 'shellsession', 'json'];
const THEMES = { light: 'github-light', dark: 'github-dark' } as const;

let highlighter: ReturnType<typeof createHighlighter> | undefined;
function getHighlighter() {
	highlighter ??= createHighlighter({ themes: Object.values(THEMES), langs: LANGUAGES });
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
	/** The page body, without its title. */
	html: string;
	toc: TocEntry[];
}

export interface RenderContext {
	/** The page's path under site/content/, for error messages and link resolution. */
	file: string;
	/** Every page in SUMMARY.md, by file, mapped to the route it is served at. */
	routes: ReadonlyMap<string, string>;
	/** The configured base path, `''` locally and `/matra` on GitHub Pages. */
	base: string;
}

export async function render(markdown: string, ctx: RenderContext): Promise<Rendered> {
	const toc: TocEntry[] = [];
	const meta = { title: '', description: '' };
	const heading = { html: '', id: '' };

	const processor = unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(remarkRehype, { allowDangerousHtml: true })
		.use(rehypeRaw)
		.use(stripComments)
		.use(checkTags, ctx)
		.use(rewriteLinks, ctx)
		.use(rehypeSlug)
		.use(collectHeadings, { toc, meta })
		.use(normalizeFenceLanguage, ctx)
		.use(rehypeShikiFromHighlighter, await getHighlighter(), {
			themes: THEMES,
			defaultColor: false,
			defaultLanguage: 'text'
		})
		.use(wrapTables)
		.use(liftTitle, { file: ctx.file, heading })
		.use(rehypeStringify);

	const html = String(await processor.process(markdown));
	return {
		title: meta.title,
		titleHtml: heading.html,
		titleId: heading.id,
		description: meta.description,
		html,
		toc
	};
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
