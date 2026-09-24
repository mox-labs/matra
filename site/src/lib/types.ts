/** One entry in SUMMARY.md: a page, and any pages nested under it. */
export interface NavItem {
	title: string;
	/** The page's path under site/content/, e.g. `guides/cli.md`. */
	file: string;
	/** The route it is served at, without the base path, e.g. `/guides/cli`. */
	route: string;
	children: NavItem[];
}

/** A `# Heading` part of SUMMARY.md. Prefix chapters sit in a part with no title. */
export interface NavPart {
	title: string | null;
	items: NavItem[];
}

export interface TocEntry {
	depth: 2 | 3;
	id: string;
	text: string;
}

export interface PageLink {
	title: string;
	route: string;
}

/** Everything a documentation page renders. Built once, at prerender time. */
export interface Doc {
	title: string;
	/** The title as HTML, keeping inline code. */
	titleHtml: string;
	/** The id of the title heading. */
	titleId: string;
	/** A one-sentence summary for `<meta name="description">`. */
	description: string;
	/** The SUMMARY.md part the page belongs to, shown above its title. */
	part: string | null;
	/** Rendered HTML of the page body, produced at build time. */
	html: string;
	toc: TocEntry[];
	file: string;
	route: string;
	/** GitHub edit link for the file the page is actually stored in. */
	editUrl: string;
	prev: PageLink | null;
	next: PageLink | null;
}
