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
 *   fixture    every Blueprints component, from the components' fixture
 *              (scripts/fixtures/legibility.md, rendered by
 *              check-legibility.ts), set into the Blueprints index's body
 *              and measured the same way, with no script's enhancements and
 *              then with the sketch's toggle shown, so a component no record
 *              uses yet (the Lab's experiment card) is held to the rule too
 *
 * Then it emulates a touch screen at 390 pixels and taps a word in the parse
 * figure. A tap fires pointerenter and pointerleave together, so a figure
 * that lights only on hover flashes and goes dark; this asserts that the tap
 * pins the word's arcs, that a second tap clears them, and that a tap
 * elsewhere clears a pin. On a proposal it taps a code chip, which must open
 * its sheet inside the window, and the sheet's close button, which must
 * close it; and the sketch's toggle, which must swap the state shown. On
 * EPR-0006 it opens the record frame's sheet (inside the window), taps a
 * decision in its Decide group (the sheet closes, the decision lands in view
 * below the sticky header and frame), taps one of that decision's grounds
 * (its claim lands in view, uncovered) and the claim's way back.
 *
 * Then, at 390 pixels, it checks the menu (#site-nav) for each area's
 * directory-index address (`/blueprints/`, `/lab/`) and its `/index` route
 * (`/blueprints/index`, `/lab/index`): GitHub Pages serves both forms from
 * the same `index.html`, so the header must mark the same area current and
 * the menu must offer the same tree for both, with a Blueprints menu always
 * reaching EPR-0006.
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
import { renderFixture } from './check-legibility';

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
const fixture = await renderFixture();

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
	// The components' fixture, in the Blueprints index's body.
	await page.goto(`${origin}/blueprints/index.html`, { waitUntil: 'networkidle' });
	await page.evaluate((html) => {
		const body = document.querySelector('.prose .body');
		if (body) body.innerHTML = html;
	}, fixture);
	await settle(page);
	for (const state of ['plain', 'enhanced', 'open'] as const) {
		if (state === 'enhanced') {
			await page.evaluate(() => {
				for (const f of document.querySelectorAll('figure.sketch')) {
					f.setAttribute('data-enhanced', '');
					f.querySelector<HTMLElement>('.sk-toggle')?.removeAttribute('hidden');
				}
			});
		}
		if (state === 'open') await page.evaluate(() => document.querySelectorAll('details').forEach((d) => (d.open = true)));
		await settle(page);
		const m = await page.evaluate(measure, SLACK);
		const where = `the components fixture @${width}px (${state})`;
		if (m.scroll > m.vw) failures.push(`${where}: the page scrolls sideways (scrollWidth ${m.scroll} > ${m.vw})`);
		for (const s of m.spills.slice(0, 5)) failures.push(`${where}: ${s}, past the window with nothing to scroll it`);
		checked++;
	}
	await context.close();
	return checked;
}

/** A proposal's chip and sketch, on an emulated touch screen. */
async function touchProposal(browser: Browser) {
	// A proposal that has a code chip, preferring one whose sketch has a
	// toggle: a sketch of one state (a baseline drawing what ships) has no
	// toggle to tap, and the toggle is the half of this check that needs one.
	const withChip = paths.filter(
		(p) => p.startsWith('blueprints/proposals/') && readFileSync(join(root, p), 'utf8').includes('data-sheet=')
	);
	const page_ = withChip.find((p) => readFileSync(join(root, p), 'utf8').includes('class="sk-toggle"')) ?? withChip[0];
	if (!page_) {
		failures.push('touch: no proposal in urls.txt carries a code chip to tap; the chip and sketch go untested');
		return;
	}
	const context = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, isMobile: true, hasTouch: true });
	const page = await context.newPage();
	await page.goto(`${origin}/${page_}`, { waitUntil: 'networkidle' });
	await settle(page);
	const where = `touch @390px, ${page_}`;
	const chip = page.locator('a.chip[data-sheet]').first();
	if ((await chip.count()) === 0) {
		failures.push(`${where}: no code chip to tap`);
	} else {
		await chip.scrollIntoViewIfNeeded();
		await chip.tap();
		await page.waitForTimeout(200);
		const id = (await chip.getAttribute('data-sheet')) ?? '';
		const sheet = page.locator(`[id="${id}"]`);
		const open = await sheet.evaluate((e) => e.matches(':popover-open'));
		if (!open) failures.push(`${where}: tapping a chip does not open its sheet`);
		else {
			const box = await sheet.boundingBox();
			if (!box || box.x < -SLACK || box.x + box.width > 390 + SLACK || box.y + box.height > 844 + SLACK) {
				failures.push(`${where}: the chip's sheet is not inside the window`);
			}
			await sheet.locator('.sheet-close').tap();
			await page.waitForTimeout(200);
			if (await sheet.evaluate((e) => e.matches(':popover-open'))) failures.push(`${where}: the sheet's close button does not close it`);
		}
	}
	const figure = page.locator('figure.sketch[data-enhanced]').first();
	if ((await page.locator('figure.sketch:has(.sk-toggle)').count()) > 0) {
		if ((await figure.count()) === 0) failures.push(`${where}: the sketch's toggle was not turned on`);
		else {
			const buttons = figure.locator('.sk-toggle button');
			const last = buttons.last();
			const target = (await last.getAttribute('data-show')) ?? '';
			await last.scrollIntoViewIfNeeded();
			await last.tap();
			await page.waitForTimeout(400);
			const shown = await figure.locator('.sk-state[data-current]').getAttribute('data-state');
			if (shown !== target) failures.push(`${where}: tapping "${target}" shows "${shown}"`);
			if ((await last.getAttribute('aria-pressed')) !== 'true') failures.push(`${where}: the pressed toggle does not say so (aria-pressed)`);
		}
	}
	await context.close();
}

/**
 * A proposal's frame, navigator and grounds on an emulated phone: the
 * frame's sheet opens inside the window; a decision tapped in its Decide
 * group closes the sheet and lands in view, below the sticky header and
 * frame; a ground tapped in that decision lands on its claim, uncovered; and
 * the claim's way back lands on the decision again.
 */
async function touchRecord(browser: Browser) {
	const page_ = 'blueprints/proposals/0006-the-0-3-0-surface.html';
	const decision = 'decision-release-split';
	const context = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, isMobile: true, hasTouch: true });
	const page = await context.newPage();
	await page.goto(`${origin}/${page_}`, { waitUntil: 'networkidle' });
	await settle(page);
	const where = `touch @390px, ${page_}`;
	/** Whether the element's first box starts in the window and is the topmost thing at its top edge, not under the sticky header or frame. */
	const uncovered = (id: string) =>
		page.evaluate((id) => {
			const el = document.getElementById(id);
			const r = el?.getClientRects()[0];
			if (!el || !r) return 'missing';
			if (r.top < 0 || r.top >= window.innerHeight) return `outside the window (top ${Math.round(r.top)})`;
			const hit = document.elementFromPoint(Math.min(r.left + 4, window.innerWidth - 1), r.top + 4);
			return hit && (el === hit || el.contains(hit)) ? 'ok' : `covered by ${hit?.closest('[class]')?.className ?? hit?.tagName}`;
		}, id);
	const frame = page.locator('details.frame-compact');
	if ((await frame.count()) === 0) {
		failures.push(`${where}: no frame to open`);
		await context.close();
		return;
	}
	await frame.locator('summary').tap();
	await page.waitForTimeout(200);
	if (!(await frame.evaluate((d) => (d as HTMLDetailsElement).open))) failures.push(`${where}: tapping the frame does not open its sheet`);
	const box = await frame.locator('.sheet-body').boundingBox();
	if (!box || box.x < -SLACK || box.x + box.width > 390 + SLACK || box.y + box.height > 844 + SLACK) failures.push(`${where}: the frame's sheet is not inside the window`);
	await frame.locator(`[data-nav-decision="${decision}"] > a`).tap();
	await page.waitForTimeout(400);
	if (await frame.evaluate((d) => (d as HTMLDetailsElement).open)) failures.push(`${where}: following a decision from the sheet leaves the sheet open`);
	if ((await page.evaluate(() => location.hash)) !== `#${decision}`) failures.push(`${where}: the sheet's Decide link does not lead to ${decision}`);
	const atDecision = await uncovered(decision);
	if (atDecision !== 'ok') failures.push(`${where}: ${decision}, followed from the sheet, is ${atDecision}`);
	const ground = page.locator(`#${decision} a.ground`).first();
	const claim = (await ground.getAttribute('data-ground')) ?? '';
	await ground.scrollIntoViewIfNeeded();
	await ground.tap();
	await page.waitForTimeout(400);
	if ((await page.evaluate(() => location.hash)) !== `#${claim}`) failures.push(`${where}: tapping a ground in ${decision} does not lead to ${claim}`);
	const atClaim = await uncovered(claim);
	if (atClaim !== 'ok') failures.push(`${where}: ${claim}, tapped from ${decision}, is ${atClaim}`);
	const back = page.locator(`#${claim} a.grounds-for[data-decision="${decision}"]`);
	if ((await back.count()) === 0) failures.push(`${where}: ${claim} has no way back to ${decision}`);
	else {
		await back.scrollIntoViewIfNeeded();
		await back.tap();
		await page.waitForTimeout(400);
		if ((await page.evaluate(() => location.hash)) !== `#${decision}`) failures.push(`${where}: ${claim}'s way back does not lead to ${decision}`);
	}
	await context.close();
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

/**
 * The menu at 390px, for each area's directory-index address and its
 * `/index` route: GitHub Pages serves `/blueprints/` and `/lab/` from the
 * directory's `index.html`, same file as `/blueprints/index` and
 * `/lab/index`, so a reader who only ever types or follows the trailing-slash
 * address must see the right area's tree too. Regression for the bug where
 * `+layout.svelte` read the area from `page.url.pathname` before normalizing
 * its trailing slash, so `/blueprints/` showed the Docs tree with no route to
 * any proposal.
 */
async function areaNav(browser: Browser) {
	const cases: { path: string; area: string; needsEpr0006: boolean }[] = [
		{ path: 'blueprints/', area: 'Blueprints', needsEpr0006: true },
		{ path: 'blueprints/index', area: 'Blueprints', needsEpr0006: true },
		{ path: 'lab/', area: 'Lab', needsEpr0006: false },
		{ path: 'lab/index', area: 'Lab', needsEpr0006: false }
	];
	const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
	const page = await context.newPage();
	for (const { path, area, needsEpr0006 } of cases) {
		await page.goto(`${origin}/${path}`, { waitUntil: 'networkidle' });
		await settle(page);
		const where = `areaNav @390px, /${path}`;
		const current = await page.locator('.areas a[aria-current]').textContent();
		if (current?.trim() !== area) failures.push(`${where}: the header marks "${current?.trim()}" current, not "${area}"`);
		if (needsEpr0006 && (await page.locator('#site-nav a', { hasText: 'EPR-0006' }).count()) === 0)
			failures.push(`${where}: #site-nav offers no link to EPR-0006`);
	}
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
	const counts = await Promise.all([
		...WIDTHS.map((w) => layout(browser, w)),
		touch(browser).then(() => 0),
		touchProposal(browser).then(() => 0),
		touchRecord(browser).then(() => 0),
		areaNav(browser).then(() => 0)
	]);
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
console.log(
	'PASS (responsive): no page or component scrolls sideways at any width, a tap pins a parse word, ' +
		"a proposal's chip and sketch answer a finger, a decision leads to its grounds and back on a phone, " +
		"and an area's trailing-slash address shows its own menu"
);
