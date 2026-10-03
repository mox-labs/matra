/**
 * Fail the build when the static site holds any part of the local comments.
 *
 * Local comments exist in the dev server only (site/README.md, Local
 * comments): the `/__comments` endpoint is a Vite plugin that `vite build`
 * never runs, and the UI is imported only under `import.meta.env.DEV`, which a
 * build replaces with `false`. Both are true by construction, and both break
 * quietly: an unguarded import ships the UI, which then calls an endpoint
 * the published site does not have. So the build is read for them.
 *
 *   code     every script and stylesheet the build emits (`_app/`) is free of
 *            the endpoint's path, the UI's attribute and its highlight
 *            names, and of the event the dev server sends it
 *   pages    no element in any built page carries the UI's attribute
 *
 * Page text is not code: a page may document the endpoint, and that prose
 * would be in the HTML, its data and its Markdown twin. So pages are
 * read for the attribute on an element, never for the words.
 *
 * Usage: bun scripts/check-no-dev-comments.ts <build-dir>
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const dir = process.argv[2];
if (!dir) {
	console.error('usage: bun scripts/check-no-dev-comments.ts <build-dir>');
	process.exit(2);
}

/** What only the comments code holds. */
const CODE_MARKERS = ['/__comments', 'data-dev-comments', 'dev-comment-active', 'matra:comments'];
/** The UI's root element, as an attribute on a tag. */
const ELEMENT = /<[a-z][^<>]*\sdata-dev-comments(?=[\s=>/])/i;

function files(d: string): string[] {
	return readdirSync(d).flatMap((name) => {
		const path = join(d, name);
		return statSync(path).isDirectory() ? files(path) : [path];
	});
}

const all = files(dir);
const code = all.filter((f) => relative(dir, f).startsWith('_app') && /\.(js|mjs|css)$/.test(f));
const pages = all.filter((f) => f.endsWith('.html'));
if (code.length === 0 || pages.length === 0) {
	console.error(`FAIL (dev comments): found ${code.length} scripts and ${pages.length} pages under ${dir}; nothing was checked`);
	process.exit(1);
}

const found: string[] = [];
for (const f of code) {
	const text = readFileSync(f, 'utf8');
	for (const m of CODE_MARKERS) if (text.includes(m)) found.push(`${relative(dir, f)}: ${m}`);
}
for (const f of pages) {
	if (ELEMENT.test(readFileSync(f, 'utf8'))) found.push(`${relative(dir, f)}: an element with data-dev-comments`);
}

if (found.length > 0) {
	console.error('FAIL (dev comments): the static build holds the local comments, which are dev-server only:');
	for (const f of found) console.error(`  ${f}`);
	console.error('        Import the UI only under import.meta.env.DEV (src/routes/+layout.svelte).');
	process.exit(1);
}
console.log(`check-no-dev-comments: ${code.length} scripts and stylesheets and ${pages.length} pages hold no local comments code`);
