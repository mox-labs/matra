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
	generator: {
		matra: string;
		udpipe_model: { name: string; sha256: string };
		/** Present on figures drawn from sentence embeddings. */
		embedding_model?: { name: string; sha256: string };
	};
	data: T;
}

export type ParseFigureFile = FigureFile<{ sentences: ParseSentence[] }>;

/** matra's structural primitive fields, as it serialises them. */
export interface PrimitiveSentence {
	paragraph: number;
	text: string;
	tokens: { id: number; text: string }[];
	root_id: number | null;
	negations: { cue_id: number; cue_lemma: string; head_id: number }[];
	modals: { aux_id: number; aux_lemma: string; head_id: number }[];
	bare_assertion: boolean;
	reportings: {
		verb_id: number;
		verb_lemma: string;
		ccomp_id: number;
		subject_id: number | null;
		subject_lemma: string | null;
	}[];
	root_adverbials: { adv_id: number; adv_lemma: string }[];
	hearst_pairs: {
		pattern: string;
		hypernym: HearstSpan;
		hyponym: HearstSpan;
	}[];
}

export interface HearstSpan {
	head_id: number;
	head_lemma: string;
	first_id: number;
	last_id: number;
}

export type PrimitivesFigureFile = FigureFile<{ sentences: PrimitiveSentence[] }>;

export interface MetricsParagraph {
	index: number;
	opening: string;
	words: number;
	in_blockquote: boolean;
	readability_grade: number | null;
	lexical_density: number | null;
	compression_ratio: number | null;
}

export type MetricsFigureFile = FigureFile<{
	paragraphs: MetricsParagraph[];
	document: {
		mean_readability: number | null;
		vocabulary_ttr: number | null;
		nominalization_ratio: number | null;
		passive_ratio: number | null;
	};
}>;

export interface RankedPhrase {
	phrase: string;
	score: number;
	rank: number;
}

export type KeyphrasesFigureFile = FigureFile<{
	shown: number;
	total: { rake: number; yake: number };
	rake: RankedPhrase[];
	yake: RankedPhrase[];
	phrases: {
		phrase: string;
		rake: { rank: number; score: number } | null;
		yake: { rank: number; score: number } | null;
	}[];
}>;

export type TextrankFigureFile = FigureFile<{
	n: number;
	sentences: {
		position: number;
		paragraph: number | null;
		text: string;
		score: number;
		summary: boolean;
	}[];
}>;

export interface ClusterGridPoint {
	threshold: number;
	clusters: { members: number[]; edges: { a: number; b: number; score: number }[] }[];
}

export type ClustersFigureFile = FigureFile<{
	default_threshold: number;
	grid: ClusterGridPoint[];
	sentences: { index: number; text: string }[];
}>;

export interface PipelineParagraph {
	index: number;
	opening: string;
	in_blockquote: boolean;
	sentences: number;
	tokens: number;
	readability_grade: number | null;
	lexical_density: number | null;
	compression_ratio: number | null;
}

export interface PipelineStage {
	sections: { heading: string | null; level: number; paragraphs: PipelineParagraph[] }[];
	document: {
		vocabulary_ttr: number | null;
		nominalization_ratio: number | null;
		passive_ratio: number | null;
	};
}

export type PipelineFigureFile = FigureFile<{
	raw: { format: string; bytes: number; text: string };
	annotated: PipelineStage;
	composed: PipelineStage;
}>;

/** A figure in a page, resolved against its data at build time. */
interface FigureSegmentBase {
	kind: 'figure';
	/** Unique on its page. */
	id: string;
	/** Where the data file is published, beside the page. */
	dataUrl: string;
}

/** One of a worked example's three calls, highlighted at build time. */
export interface ExampleCall {
	lang: 'rust' | 'python' | 'cli';
	label: string;
	/** The file the call lives in, under site/examples/<name>/. */
	file: string;
	html: string;
}

/**
 * A worked example (EP-0012, M6), as its page shows it: the input, the same
 * call in Rust, Python and the CLI, and what the calls print. Everything here
 * comes from site/examples/<name>/, whose calls the examples gate runs.
 */
export interface ExampleView {
	name: string;
	input: {
		name: string;
		/** The file name the calls read. */
		file: string;
		text: string;
		words: number;
		source: Record<string, string>;
		/** Where the input is published, to download under `file`. */
		url: string;
	};
	calls: ExampleCall[];
	/** What the CLI prints around the result, highlighted, the result elided. */
	cliEnvelopeHtml: string;
	output: {
		html: string;
		/** How the shown output was trimmed, or null when it is whole. */
		trimmed: string | null;
		url: string;
		cliUrl: string;
	};
}

/**
 * A page body is a run of segments: rendered HTML, and between runs the
 * figures and example parts the page's Markdown names, each with its data.
 */
export type Segment =
	| { kind: 'html'; html: string }
	| { kind: 'example'; part: 'input' | 'call' | 'output'; id: string; example: ExampleView }
	| (FigureSegmentBase & {
			figure: 'parse';
			/** The sentence shown first, 1-based. */
			sentence: number;
			file: ParseFigureFile;
	  })
	| (FigureSegmentBase & { figure: 'primitives'; file: PrimitivesFigureFile })
	| (FigureSegmentBase & { figure: 'metrics'; file: MetricsFigureFile })
	| (FigureSegmentBase & { figure: 'keyphrases'; file: KeyphrasesFigureFile })
	| (FigureSegmentBase & { figure: 'textrank'; file: TextrankFigureFile })
	| (FigureSegmentBase & {
			figure: 'clusters';
			/** The threshold shown first, on the data's grid. */
			threshold: number;
			file: ClustersFigureFile;
	  })
	| (FigureSegmentBase & { figure: 'pipeline'; file: PipelineFigureFile });

export type FigureKind = Extract<Segment, { kind: 'figure' }>['figure'];

export interface PageLink {
	title: string;
	route: string;
}

/** matra's measures of one page's paragraphs (site/src/lib/figures/pages/). */
export interface PageMeasures {
	page: string;
	generator: FigureFile['generator'];
	paragraphs: {
		text: string;
		in_blockquote: boolean;
		sentences: number;
		words: number;
		readability_grade: number | null;
		lexical_density: number | null;
		compression_ratio: number | null;
	}[];
}

/** What the page says about its own measurement, beside its title. */
export interface MeasuredLine {
	/** Whether the notes could be set beside the paragraphs they measure. */
	mapped: boolean;
	/** Prose paragraphs given a note. */
	measured: number;
	generator: FigureFile['generator'];
	dataUrl: string;
	/** Why the page shows no notes, when it does not. */
	reason?: string;
}

/** A step in the breadcrumb trail; a part has no page of its own. */
export interface Crumb {
	title: string;
	route: string | null;
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
	/** Where the page sits: its part, and the item it is nested under. */
	crumbs: Crumb[];
	/** matra's measures of this page, or null for a page it has none for. */
	measured: MeasuredLine | null;
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
