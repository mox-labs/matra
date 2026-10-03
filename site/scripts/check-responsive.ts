/**
 * Every page works at a phone's width and a desktop's, and the figures answer
 * a finger as they answer a pointer.
 *
 * The rule (site/README.md, Design rules): a table, a block of code or a
 * diagram may scroll inside its own box; the page never scrolls sideways.
 * This loads every docsite page in site/urls.txt (rustdoc's api/ pages
 * excepted) from the built site, in a real browser engine, at 320, 390, 768 and 1280 CSS pixels wide, and fails when:
 *
 *   page       the document is wider than the window (scrollWidth > innerWidth)
 *   element    a rendered element extends past the window's left or right
 *              edge with no ancestor that scrolls or clips sideways
 *              (overflow-x other than visible); a `position: fixed` element
 *              and what it holds are exempt, since they do not scroll the page
 *   details    the same two, with every `<details>` on the page opened, so a
 *              table behind "show the data" is held to the rule too
 *
 * Then it emulates a touch screen at 390 pixels and taps a word in the parse
 * figure. A tap fires pointerenter and pointerleave together, so a figure
 * that lights only on hover flashes and goes dark; this asserts that the tap
 * pins the word's arcs, that a second tap clears them, and that a tap
 * elsewhere clears a pin.
 *
 * The windows are desktop windows of that width (no mobile viewport
 * emulation for the layout pass): a mobile browser zooms out to fit a page
 * that overflows, which would hide exactly what this looks for.
 *
 * The browser is Playwright's Chromium headless shell, at the revision the
 * pinned playwright-core names (site/package.json). Gate 12 of the docsite
 * floor installs it on first run; by hand, from site/:
 * `bun node_modules/playwright-core/cli.js install --no-remove chromium-headless-shell`.
 *
 * Usage: bun scripts/check-responsive.ts <build-dir>
 */
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join, resolve, sep } from 'node:path';
import { chromium, type Browser, type Page } from 'playwright-core';

const dir = process.argv[2];
if (!dir) {
	console.error('usage: bun scripts/check-responsive.ts <build-dir>');
	process.exit(2);
}
const root = resolve(dir);

const WIDTHS = [320, 390, 768, 1280];
const HEIGHT = 800;
/** Sub-pixel layout can put an edge a fraction past the window. */
const SLACK = 1;

// The pages: every .html path in the URL contract.
const manifest = readFileSync(join(import.meta.dir, '..', 'urls.txt'), 'utf8');
const paths = manifest
	.split('\n')
	.map((l) => l.trim())
	.filter((l) => l && !l.startsWith('#') && l.endsWith('.html') && !l.startsWith('api/'));
if (paths.length === 0) {
	console.log('FAIL (responsive): urls.txt lists no pages; a check that examined nothing has not passed');
	process.exit(1);
}

// The build, served from its own root as gate 1 reads it: an extensionless
// route resolves to its .html file, as GitHub Pages does.
const server = Bun.serve({
	hostname: '127.0.0.1',
	port: 0,
	fetch(req) {
		let p = decodeURIComponent(new URL(req.url).pathname);
		if (p.endsWith('/')) p += 'index.html';
		let file = join(root, p);
		// Inside the build only: a sibling such as build-x shares the prefix.
		if (!(file === root || file.startsWith(root + sep))) return new Response('not found', { status: 404 });
		if (!existsSync(file) || statSync(file).isDirectory()) {
			if (existsSync(file + '.html')) file += '.html';
			else return new Response('not found', { status: 404 });
		}
		return new Response(Bun.file(file));
	}
});
const origin = `http://127.0.0.1:${server.port}`;

// The server must not reach outside the build, even into a sibling whose
// name starts with the build's: one is made for the probe, then removed.
{
	const sibling = `${root}-x${process.pid}`;
	mkdirSync(sibling);
	writeFileSync(join(sibling, 'index.html'), 'outside the build');
	let status: number;
	try {
		const name = encodeURIComponent(sibling.split(sep).pop() ?? '');
		status = (await fetch(`${origin}/..%2F${name}/index.html`)).status;
	} finally {
		rmSync(sibling, { recursive: true });
	}
	if (status !== 404) {
		console.log(`FAIL (responsive): the test server answered ${status} for a path outside the build`);
		server.stop();
		process.exit(1);
	}
}

/** What the page itself measures: run in the browser. */
function measure(slack: number) {
	const vw = window.innerWidth;
	const out: { scroll: number; vw: number; spills: string[] } = {
		scroll: document.documentElement.scrollWidth,
		vw,
		spills: []
	};
	const describe = (el: Element) => {
		const id = el.id ? `#${el.id}` : '';
		const cls =
			typeof el.className === 'string' && el.className.trim()
				? '.' + el.className.trim().split(/\s+/).filter((c) => !c.startsWith('svelte-')).slice(0, 3).join('.')
				: '';
		return `${el.tagName.toLowerCase()}${id}${cls === '.' ? '' : cls}`;
	};
	const path = (el: Element) => {
		const parts: string[] = [];
		for (let e: Element | null = el; e && e !== document.body && parts.length < 4; e = e.parentElement)
			parts.unshift(describe(e));
		return parts.join(' > ');
	};
	/** Whether the element is held: fixed, or inside a box that scrolls or clips sideways. */
	const held = (el: Element) => {
		for (let e: Element | null = el; e && e !== document.documentElement; e = e.parentElement) {
			const s = getComputedStyle(e);
			if (s.position === 'fixed') return true;
			if (e !== el && e !== document.body && s.overflowX !== 'visible') return true;
		}
		return false;
	};
	const spilled = new Set<Element>();
	for (const el of document.body.querySelectorAll('*')) {
		const r = el.getBoundingClientRect();
		if (r.width === 0 || r.height === 0) continue;
		// Not rendered: hidden, or inside a closed <details>.
		if (!el.checkVisibility({ visibilityProperty: true })) continue;
		if (r.right <= vw + slack && r.left >= -slack) continue;
		if (held(el)) continue;
		// Report the outermost offender only: its descendants spill with it.
		spilled.add(el);
		let inside = false;
		for (let e = el.parentElement; e && !inside; e = e.parentElement) inside = spilled.has(e);
		if (inside) continue;
		out.spills.push(`${path(el)} spans ${Math.round(r.left)} to ${Math.round(r.right)}px`);
	}
	return out;
}

async function settle(page: Page) {
	await page.evaluate(async () => {
		await document.fonts.ready;
		await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
	});
}

const failures: string[] = [];

/** One width, every page; the widths run side by side. */
async function layout(browser: Browser, width: number) {
	let checked = 0;
	const context = await browser.newContext({ viewport: { width, height: HEIGHT } });
	const page = await context.newPage();
	for (const p of paths) {
		const res = await page.goto(`${origin}/${p}`, { waitUntil: 'networkidle' });
		if (!res || (res.status() !== 200 && !(p === '404.html' && res.status() === 404))) {
			failures.push(`${p} @${width}px: HTTP ${res?.status() ?? 'no response'}`);
			continue;
		}
		await settle(page);
		for (const state of ['closed', 'open'] as const) {
			if (state === 'open') {
				const n = await page.evaluate(() => {
					const all = document.querySelectorAll('details');
					all.forEach((d) => (d.open = true));
					return all.length;
				});
				if (n === 0) continue;
				await settle(page);
			}
			const m = await page.evaluate(measure, SLACK);
			const where = `${p} @${width}px${state === 'open' ? ', every <details> open' : ''}`;
			if (m.scroll > m.vw) failures.push(`${where}: the page scrolls sideways (scrollWidth ${m.scroll} > ${m.vw})`);
			for (const s of m.spills.slice(0, 5)) failures.push(`${where}: ${s}, past the window with nothing to scroll it`);
			if (m.spills.length > 5) failures.push(`${where}: and ${m.spills.length - 5} more elements`);
			checked++;
		}
	}
	await context.close();
	return checked;
}

/** Taps a word in the parse figure on an emulated touch screen. */
async function touch(browser: Browser) {
	const page_ = 'explanation/concepts.html';
	const context = await browser.newContext({
		viewport: { width: 390, height: 844 },
		deviceScaleFactor: 3,
		isMobile: true,
		hasTouch: true
	});
	const page = await context.newPage();
	await page.goto(`${origin}/${page_}`, { waitUntil: 'networkidle' });
	await settle(page);
	const figure = page.locator('[data-figure="parse"]').first();
	if ((await figure.count()) === 0) {
		failures.push(`touch: ${page_} has no parse figure to tap`);
		await context.close();
		return;
	}
	const word = figure.locator('svg g.tok').nth(1);
	const lit = () => figure.locator('svg g.tok.lit').count();
	const where = `touch @390px, ${page_}`;
	await word.scrollIntoViewIfNeeded();
	await word.tap();
	// Long enough for any enter-then-leave flash to have cleared it.
	await page.waitForTimeout(300);
	if ((await word.evaluate((e) => e.classList.contains('lit'))) === false)
		failures.push(`${where}: tapping a word in the parse figure does not keep its highlight`);
	await word.tap();
	await page.waitForTimeout(300);
	if ((await lit()) !== 0) failures.push(`${where}: a second tap on the word does not clear its highlight`);
	await word.tap();
	await page.waitForTimeout(100);
	await page.locator('main p').first().tap();
	await page.waitForTimeout(300);
	if ((await lit()) !== 0) failures.push(`${where}: a tap elsewhere does not clear a pinned word`);
	await context.close();
}

let browser: Browser;
try {
	browser = await chromium.launch();
} catch (e) {
	console.log('FAIL (responsive): could not launch the Chromium headless shell');
	console.log(`        ${String(e).split('\n')[0]}`);
	console.log('        install it, from site/: bun node_modules/playwright-core/cli.js install --no-remove chromium-headless-shell');
	server.stop();
	process.exit(1);
}
let checked = 0;
try {
	const counts = await Promise.all([...WIDTHS.map((w) => layout(browser, w)), touch(browser).then(() => 0)]);
	checked = counts.reduce((a, b) => a + b, 0);
} finally {
	await browser.close();
	server.stop();
}

console.log(
	`  ${paths.length} pages at ${WIDTHS.join(', ')}px, ${checked} layouts measured (closed and with every <details> open); touch at 390px`
);
if (failures.length > 0) {
	// The widths run side by side; sorted, the report reads page by page.
	failures.sort();
	console.log('FAIL (responsive):');
	for (const f of failures) console.log(`  ${f}`);
	process.exit(1);
}
console.log('PASS (responsive): no page scrolls sideways at any width, and a tap pins a parse word');
