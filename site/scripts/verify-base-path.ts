/**
 * Fail the build when the emitted asset URLs do not match the base path the
 * site was built for.
 *
 * Built with the wrong BASE_PATH, the site works locally and deploys as a
 * blank page whose /_app/* requests all 404. Nothing about the artifact looks
 * wrong, so the failure would only show once it was live. This makes it a
 * build failure instead.
 *
 * Usage: bun scripts/verify-base-path.ts <build-dir>
 * BASE_PATH is read from the environment, as svelte.config.js reads it.
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

// svelte.config.js sets paths.relative = false, so every asset URL is
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
