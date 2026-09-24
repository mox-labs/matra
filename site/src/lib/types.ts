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

/** One token of a parse, under matra's own field names. */
export interface ParseToken {
	id: number;
	text: string;
	lemma: string;
	pos: string;
	head: number;
	dep: string;
}

export interface ParseSentence {
	paragraph: number;
	text: string;
	tokens: ParseToken[];
}

/**
 * A figure data file as examples/docsite_figures.rs writes it: what the
 * figure shows, the input it came from, and what produced it.
 */
export interface FigureFile<T = unknown> {
	figure: string;
	input: string;
	source: Record<string, string>;
	generator: { matra: string; udpipe_model: { name: string; sha256: string } };
	data: T;
}

export type ParseFigureFile = FigureFile<{ sentences: ParseSentence[] }>;

/**
 * A page body is a run of segments: rendered HTML, and between runs the
 * figures the page's Markdown names, each with the data it renders.
 */
export type Segment =
	| { kind: 'html'; html: string }
	| {
			kind: 'figure';
			tag: 'figure-parse';
			/** Unique on its page. */
			id: string;
			/** The sentence shown first, 1-based. */
			sentence: number;
			/** Where the data file is published, beside the page. */
			dataUrl: string;
			file: ParseFigureFile;
	  };

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
	/** The page body, rendered at build time: HTML runs and figures. */
	segments: Segment[];
	toc: TocEntry[];
	file: string;
	route: string;
	/** GitHub edit link for the file the page is actually stored in. */
	editUrl: string;
	prev: PageLink | null;
	next: PageLink | null;
}
