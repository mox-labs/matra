/**
 * The pages, read from site/content/ at build time.
 *
 * import.meta.glob resolves every Markdown file when the site is built, and
 * the dev server reloads when one changes. Content stays plain Markdown on
 * disk: no preprocessor reads it, and nothing in it is Svelte.
 */
import { realpathSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { error } from '@sveltejs/kit';
import { base } from '$app/paths';
import { editUrl } from '$lib/site';
import type { Doc, FigureFile, NavPart } from '$lib/types';
import { flatten, parseSummary } from './summary';
import { render } from './markdown/render';

const SOURCES = import.meta.glob('/content/**/*.md', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

const LLMS = import.meta.glob('/content/llms.txt', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

/**
 * Figure data, as examples/docsite_figures.rs wrote it. Keyed
 * `<input>/<figure>`; the figures-current gate keeps it in step with matra.
 */
const FIGURE_FILES = import.meta.glob('/src/lib/figures/*/*.json', {
	import: 'default',
	eager: true
}) as Record<string, FigureFile>;

export const figures: ReadonlyMap<string, FigureFile> = new Map(
	Object.entries(FIGURE_FILES).map(([path, file]) => {
		const key = path.replace(/^\/src\/lib\/figures\//, '').replace(/\.json$/, '');
		if (key !== `${file.input}/${file.figure}`) {
			throw new Error(`${path} says it is ${file.input}/${file.figure}; regenerate the figures`);
		}
		return [key, file];
	})
);

function source(file: string): string {
	const text = SOURCES[`/content/${file}`];
	if (text === undefined) {
		throw new Error(`SUMMARY.md names content/${file}, which does not exist`);
	}
	return text;
}

export const nav: NavPart[] = parseSummary(source('SUMMARY.md'));
const order = flatten(nav);

/** Every page in SUMMARY.md, in reading order. */
export const pages = order.map((o) => o.item);

const routes: ReadonlyMap<string, string> = new Map(pages.map((p) => [p.file, p.route]));
for (const p of pages) source(p.file);

export function llmsTxt(): string {
	const text = LLMS['/content/llms.txt'];
	if (text === undefined) throw new Error('content/llms.txt is missing; run scripts/gen-llms-txt.sh');
	return text;
}

/** The raw Markdown of a page, as authored: the `.md` twin agents read. */
export function markdownOf(route: string): string {
	const page = pages.find((p) => p.route === route);
	if (!page) error(404, `No page at ${route}`);
	return source(page.file);
}

const SITE_ROOT = resolve('.');
const REPO_ROOT = resolve('..');

/**
 * The file an edit should land in. For most pages that is the page under
 * site/content/. roadmap.md is a symlink to the repository's ROADMAP.md, and
 * GitHub's editor edits a symlink's target path as text, so its edit link
 * goes to ROADMAP.md itself.
 */
function repoPathOf(file: string): string {
	const real = realpathSync(resolve(SITE_ROOT, 'content', file));
	return relative(realpathSync(REPO_ROOT), real).split('\\').join('/');
}

export async function loadDoc(route: string): Promise<Doc> {
	const i = order.findIndex((o) => o.item.route === route);
	if (i === -1) error(404, `No page at ${route}`);
	const { item, part } = order[i];
	const rendered = await render(source(item.file), { file: item.file, routes, base, figures });
	const link = (j: number) =>
		j >= 0 && j < order.length ? { title: order[j].item.title, route: order[j].item.route } : null;
	return {
		...rendered,
		part,
		file: item.file,
		route: item.route,
		editUrl: editUrl(repoPathOf(item.file)),
		prev: link(i - 1),
		next: link(i + 1)
	};
}
