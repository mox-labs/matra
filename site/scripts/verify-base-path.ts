/**
 * Fail the build when the emitted asset URLs or internal links do not match
 * the base path the site was built for.
 *
 * Built with the wrong BASE_PATH, the site works locally and deploys as a
 * blank page whose /_app/* requests all 404. Nothing about the artifact looks
 * wrong, so the failure would only show once it was live. This makes it a
 * build failure instead.
 *
 * Usage: bun scripts/verify-base-path.ts <build-dir>
 * BASE_PATH is read from the environment, as vite.config.ts reads it.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2];
if (!dir) {
	console.error('usage: bun scripts/verify-base-path.ts <build-dir>');
	process.exit(2);
}

const base = process.env.BASE_PATH ?? '';
const expected = `${base}/_app/`;

function htmlFiles(d: string): string[] {
	return readdirSync(d).flatMap((name) => {
		const path = join(d, name);
		if (statSync(path).isDirectory()) return htmlFiles(path);
		return name.endsWith('.html') ? [path] : [];
	});
}

const docs = htmlFiles(dir);
if (docs.length === 0) {
	console.error(`verify-base-path: no .html files under ${dir}`);
	process.exit(1);
}

// vite.config.ts sets paths.relative = false, so every asset URL is
// absolute. Finding none at all means the output shape changed, and a check
// that passes on an output it does not recognise is worse than no check.
const ASSET = /["']((?:\.{1,2})?\/[^"']*?_app\/[^"']*)["']/g;
const offenders: { doc: string; url: string }[] = [];
let checked = 0;
for (const doc of docs) {
	for (const [, url] of readFileSync(doc, 'utf8').matchAll(ASSET)) {
		checked++;
		if (!url.startsWith(expected)) offenders.push({ doc, url });
	}
}

if (checked === 0) {
	console.error(`verify-base-path: found no _app/ references in ${docs.length} documents`);
	process.exit(1);
}
if (offenders.length > 0) {
	console.error(`verify-base-path: BASE_PATH is "${base}", so every asset URL must start with "${expected}".`);
	for (const { doc, url } of offenders.slice(0, 10)) console.error(`  ${doc}: ${url}`);
	console.error(`  ${offenders.length} of ${checked} references do not.`);
	process.exit(1);
}
console.log(`verify-base-path: ${checked} asset references in ${docs.length} documents start with "${expected}"`);

// Every internal link and source, not only the assets. The site writes the
// base into its own hrefs (src/lib/paths.ts, and ctx.base in the Markdown
// renderer), so a wrong base there leaves the assets loading and every link
// broken. An internal URL is root-absolute ("/..." but not "//..."). With a
// base, each must start with it; with or without one, what follows the base
// must be something this build holds, which is what catches a stray or
// doubled prefix when BASE_PATH is unset. /api is the exception: rustdoc,
// assembled beside the site at deploy time (handleHttpError in vite.config.ts).
const LINK = /\s(?:href|src)="(\/(?!\/)[^"]*)"/g;
function served(path: string): boolean {
	if (path === '/api' || path.startsWith('/api/')) return true;
	const p = decodeURIComponent(path).replace(/\/$/, '');
	return [p, `${p}.html`, `${p}/index.html`].some((f) => {
		try {
			return statSync(join(dir, f)).isFile();
		} catch {
			return false;
		}
	});
}
const wrong: { doc: string; url: string; why: string }[] = [];
let links = 0;
for (const doc of docs) {
	for (const [, raw] of readFileSync(doc, 'utf8').matchAll(LINK)) {
		links++;
		const url = raw.replace(/&amp;/g, '&');
		const path = url.split(/[?#]/)[0];
		if (base && path !== base && !path.startsWith(`${base}/`)) {
			wrong.push({ doc, url, why: `does not start with "${base}/"` });
		} else if (!served(path.slice(base.length) || '/')) {
			wrong.push({ doc, url, why: `names nothing in ${dir} once "${base}" is taken off` });
		}
	}
}
if (links === 0) {
	console.error(`verify-base-path: found no internal href or src in ${docs.length} documents`);
	process.exit(1);
}
if (wrong.length > 0) {
	console.error(`verify-base-path: BASE_PATH is "${base}", and some internal links do not agree with it.`);
	for (const { doc, url, why } of wrong.slice(0, 10)) console.error(`  ${doc}: ${url} ${why}`);
	console.error(`  ${wrong.length} of ${links} internal links are wrong.`);
	process.exit(1);
}
console.log(`verify-base-path: ${links} internal links in ${docs.length} documents carry "${base}" and name a built file`);
