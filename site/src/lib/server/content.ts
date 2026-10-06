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
import { base } from '#lib/paths.ts';
import { ANATOMY } from '#lib/record-vocabulary.ts';
import { editUrl, REPO_URL } from '#lib/site.ts';
import type { Crumb, Doc, FigureFile, NavItem, NavPart, PageMeasures, TocEntry } from '#lib/types.ts';
import { flatten, parseSummary } from './summary';
import { render } from './markdown/render';
import { exampleMarkdown, examples } from './examples';
import { blueprintSources, blueprintsPart, recordCards, records, type Record_ } from './blueprints';
import { labPart, labSources } from './lab';

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

/**
 * matra's measures of each page, keyed by the page's file under content/,
 * as examples/docsite_figures.rs wrote them.
 */
const MEASURE_FILES = import.meta.glob('/src/lib/figures/pages/**/measures.json', {
	import: 'default',
	eager: true
}) as Record<string, PageMeasures>;

export const measures: ReadonlyMap<string, PageMeasures> = new Map(
	Object.values(MEASURE_FILES).map((m) => [m.page, m])
);

function source(file: string): string {
	const text = SOURCES[`/content/${file}`];
	if (text === undefined) {
		throw new Error(`SUMMARY.md names content/${file}, which does not exist`);
	}
	return text;
}

/**
 * The navigation: SUMMARY.md's parts, which are the Docs area, then the
 * Blueprints and Lab areas, read from blueprints/ and lab/ themselves
 * (./blueprints.ts, ./lab.ts) rather than listed in SUMMARY.md, so the
 * product pages' gates and llms.txt sections stay theirs. The layout shows
 * one area's parts at a time.
 */
const summaryNav = parseSummary(source('SUMMARY.md')).map((p) => ({ ...p, area: 'docs' as const }));
export const nav: NavPart[] = [...summaryNav, blueprintsPart, labPart];

/**
 * A reading order per area, each with its own previous and next: the product
 * pages in SUMMARY.md order, the design records, and the Lab. The last page
 * of one area does not lead into the next.
 */
const order = flatten(summaryNav);
const recordOrder = flatten([blueprintsPart]).filter((o) => !o.item.group);
const labOrder = flatten([labPart]);

/** Every page in SUMMARY.md, in reading order. */
export const pages = order.map((o) => o.item);

/** Every page the site renders: the SUMMARY.md pages, the design records and the Lab. */
export const allPages = [...pages, ...recordOrder.map((o) => o.item), ...labOrder.map((o) => o.item)];

const isRecord = (file: string) => file.startsWith('blueprints/');
const isLab = (file: string) => file.startsWith('lab/');
/** A page read from the repository rather than from site/content/. */
const outsideContent = (file: string) => isRecord(file) || isLab(file);

/** A page's path from the repository root. */
function repoFileOf(file: string): string {
	return outsideContent(file) ? file : `site/content/${file}`;
}

/**
 * Every page by its path from the repository root, mapped to its route. Links
 * are resolved in the repository's own tree, so a record can link a page and a
 * page a record with the relative link that works on GitHub.
 */
const routes: ReadonlyMap<string, string> = new Map(allPages.map((p) => [repoFileOf(p.file), p.route]));
for (const p of allPages) pageSource(p.file);
if (records.length !== recordOrder.length) throw new Error('a blueprint is missing from the navigation');

function pageSource(file: string): string {
	if (!outsideContent(file)) return source(file);
	const text = (isLab(file) ? labSources : blueprintSources).get(file);
	if (text === undefined) throw new Error(`no page at ${file}`);
	return text;
}

export function llmsTxt(): string {
	const text = LLMS['/content/llms.txt'];
	if (text === undefined) throw new Error('content/llms.txt is missing; run scripts/gen-llms-txt.sh');
	return text;
}

/**
 * The Markdown of a page, as authored: the `.md` twin agents read. The one
 * change is that a worked example's tags become the Markdown they stand for,
 * the input, the calls and the output, since a tag alone tells an agent
 * nothing it can run.
 */
export function markdownOf(route: string): string {
	const page = allPages.find((p) => p.route === route);
	if (!page) error(404, `No page at ${route}`);
	// A record or a Lab page is served as written: it has no example tags, and
	// its relative links resolve beside it, to the other pages' twins.
	if (outsideContent(page.file)) return pageSource(page.file);
	// The spellings the renderer's tag pattern accepts (FIGURE_TAG in
	// markdown/render.ts), so a tag that renders on the page cannot survive
	// raw in the twin; and if one does anyway, the build fails.
	const twin = source(page.file).replace(
		/^<example-(input|call|output)\s+name="([^"<>]*)"\s*\/>\s*$/gm,
		(tag, part: 'input' | 'call' | 'output', name: string) => {
			const ex = examples.get(name);
			return ex ? exampleMarkdown(ex, part, figures) : tag;
		}
	);
	const left = twin.split('\n').find((line) => /^\s*<example-/.test(line));
	if (left) throw new Error(`${page.file}: an example tag the .md twin could not expand: ${left.trim()}`);
	return twin;
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
	if (outsideContent(file)) return file;
	const real = realpathSync(resolve(SITE_ROOT, 'content', file));
	return relative(realpathSync(REPO_ROOT), real).split('\\').join('/');
}

/** The part a page is in, and the item it is nested under, if any. */
function crumbsOf(target: NavItem): Crumb[] {
	for (const part of nav) {
		const walk = (items: NavItem[], trail: Crumb[]): Crumb[] | null => {
			for (const item of items) {
				if (item === target) return trail;
				const found = walk(item.children, [...trail, { title: item.title, route: item.route }]);
				if (found) return found;
			}
			return null;
		};
		const found = walk(part.items, part.title ? [{ title: part.title, route: null }] : []);
		// A part and its index page often share a name ("Examples"): the page,
		// which can be followed, stands for both.
		if (found) return found.filter((c, i) => !(c.route === null && found[i + 1]?.title === c.title));
	}
	return [];
}

/**
 * A record's frame, or on the index the tally of what awaits the owner: both
 * from what the build parsed (the header and the components, through
 * recordCards), never from the Markdown body.
 */
function frameOf(meta: Record_, toc: TocEntry[]): Pick<NonNullable<Doc['record']>, 'frame' | 'awaiting'> {
	if (meta.kind === 'index') {
		const proposals = recordCards.filter((c) => c.id.startsWith('EPR-'));
		const open = proposals.map((c) => c.decisions.filter((d) => d.state === 'open').length);
		return {
			awaiting: {
				open: open.reduce((a, b) => a + b, 0),
				proposals: open.filter((n) => n > 0).length,
				assumptions: proposals.reduce((a, c) => a + c.assumptions.length, 0),
				href: '#awaiting-you'
			}
		};
	}
	const card = recordCards.find((c) => c.id === meta.id);
	if (!card) throw new Error(`${meta.file}: no card for ${meta.id}`);
	return {
		frame: {
			kind: meta.kind,
			id: card.id,
			title: card.title,
			status: card.status,
			pin: card.readAt ? { sha: card.readAt, href: `${REPO_URL}/tree/${card.readAt}` } : null,
			pr: card.pr,
			tracking: card.tracking,
			trackingNote: card.trackingNote,
			header: card.header,
			decisions: card.decisions,
			assumptions: card.assumptions,
			sections: toc.filter((t) => t.depth === 2).map((t) => ({ id: t.id, text: t.text, role: ANATOMY[t.text] ?? null }))
		}
	};
}

export async function loadDoc(route: string): Promise<Doc> {
	const chain = [order, recordOrder, labOrder].find((c) => c.some((o) => o.item.route === route)) ?? order;
	const i = chain.findIndex((o) => o.item.route === route);
	if (i === -1) error(404, `No page at ${route}`);
	const { item, part } = chain[i];
	const record = outsideContent(item.file);
	const rendered = await render(pageSource(item.file), {
		file: item.file,
		repoFile: repoFileOf(item.file),
		routes,
		base,
		figures,
		examples,
		// The home page is a quick start: the self-measuring margin is an
		// explanation-grade device and stays off it. Every other page keeps it.
		// The records are not measured: matra measures the pages under content/.
		measures: i === 0 || record ? undefined : measures.get(item.file),
		part,
		records: record ? recordCards : undefined
	});
	const link = (j: number) =>
		j >= 0 && j < chain.length ? { title: chain[j].item.title, route: chain[j].item.route } : null;
	const meta = record ? records.find((r) => r.file === item.file) : undefined;
	return {
		record: meta ? { kind: meta.kind, status: meta.status, ...frameOf(meta, rendered.toc) } : null,
		...rendered,
		part,
		crumbs: crumbsOf(item),
		file: item.file,
		route: item.route,
		editUrl: editUrl(repoPathOf(item.file)),
		prev: link(i - 1),
		next: link(i + 1)
	};
}
