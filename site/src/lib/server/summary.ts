import type { NavItem, NavPart } from '$lib/types';

/**
 * Parse SUMMARY.md into navigation.
 *
 * SUMMARY.md stays the navigation source, in the format the mdBook-era site
 * used, because other readers depend on it: scripts/gen-llms-txt.sh builds
 * llms.txt from it, and gate 2 of the docsite floor checks every page is
 * listed in it. The format is the subset of mdBook's this site uses:
 *
 *   # Summary               the title line, ignored
 *   [Title](page.md)        a prefix chapter, before the first part
 *   # Part                  opens a part
 *   - [Title](page.md)      a page in the current part
 *     - [Title](page.md)    nested under the item above, two or four spaces in
 *
 * Any other non-blank line fails the build. A line this parser cannot read
 * would otherwise be a page that silently has no route.
 */
export function parseSummary(source: string): NavPart[] {
	const parts: NavPart[] = [{ title: null, items: [] }];
	const stack: { indent: number; item: NavItem }[] = [];
	let sawTitle = false;

	const lines = source.split('\n');
	for (const [i, raw] of lines.entries()) {
		const line = raw.replace(/\s+$/, '');
		if (line === '') continue;

		const heading = /^#\s+(.+)$/.exec(line);
		if (heading) {
			if (!sawTitle && parts.length === 1 && parts[0].items.length === 0) {
				// The first heading is the book title ("# Summary") and names no part.
				sawTitle = true;
				continue;
			}
			parts.push({ title: heading[1], items: [] });
			stack.length = 0;
			continue;
		}

		const entry = /^(\s*)(?:-\s+)?\[([^\]]+)\]\(([^)]+)\)$/.exec(line);
		if (!entry) {
			throw new Error(`SUMMARY.md:${i + 1}: cannot read this line: ${JSON.stringify(raw)}`);
		}
		const [, indentText, title, target] = entry;
		const isListItem = /^\s*-\s/.test(line);
		const file = target.replace(/^\.\//, '');
		if (!file.endsWith('.md')) {
			throw new Error(`SUMMARY.md:${i + 1}: an entry must name a .md page, got ${target}`);
		}
		const item: NavItem = { title, file, route: routeOf(file), children: [] };
		const part = parts[parts.length - 1];

		if (!isListItem) {
			// A prefix chapter: allowed only before the first part.
			if (parts.length > 1) {
				throw new Error(`SUMMARY.md:${i + 1}: a bare link after the first part: ${raw}`);
			}
			part.items.push(item);
			continue;
		}

		const indent = indentText.length;
		while (stack.length > 0 && stack[stack.length - 1].indent >= indent) stack.pop();
		if (stack.length === 0) {
			if (indent !== 0) {
				throw new Error(`SUMMARY.md:${i + 1}: an indented item with no parent: ${raw}`);
			}
			part.items.push(item);
		} else {
			stack[stack.length - 1].item.children.push(item);
		}
		stack.push({ indent, item });
	}

	return parts.filter((p) => p.title !== null || p.items.length > 0);
}

/**
 * The route a content file is served at. `guides/cli.md` is `/guides/cli`,
 * which the static adapter writes as `guides/cli.html`: the path mdBook served.
 * A directory's README.md is its index, as in mdBook.
 */
export function routeOf(file: string): string {
	const stem = file.replace(/\.md$/, '');
	if (stem === 'README') return '/index';
	if (stem.endsWith('/README')) return `/${stem.slice(0, -'/README'.length)}/index`;
	return `/${stem}`;
}

/** Every page in reading order, nested pages after their parent. */
export function flatten(parts: NavPart[]): { item: NavItem; part: string | null }[] {
	const out: { item: NavItem; part: string | null }[] = [];
	const walk = (items: NavItem[], part: string | null) => {
		for (const item of items) {
			out.push({ item, part });
			walk(item.children, part);
		}
	};
	for (const part of parts) walk(part.items, part.title);
	return out;
}
