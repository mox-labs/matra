/**
 * The tag registry.
 *
 * A page is plain Markdown. Markdown produces a known, small set of HTML
 * elements, listed below. Any other element in a page was written into it as
 * raw HTML, and it renders only if its tag is registered here. An unregistered
 * tag fails the build, naming the file, the line and the tag.
 *
 * That is what keeps the page source readable by every gate, every test and
 * every agent: nothing in a page means something they cannot see, unless a
 * reviewer registered it here on purpose. It also catches the plain mistake of
 * a bare `<name>` in prose, which a browser would swallow as an unknown
 * element and show nothing for.
 *
 * Two kinds of entry:
 *
 *   passthrough   the element and everything inside it are emitted as
 *                 written. For hand-authored markup whose meaning is the
 *                 markup itself.
 *
 *   figure        a data figure (EP-0012). Written in a page as one
 *                 self-closing tag on a line of its own, naming its data:
 *
 *                   <figure-parse input="primitives" sentence="1" />
 *
 *                 `figure` names the data file the generator writes,
 *                 site/src/lib/figures/<input>/<figure>.json, and `attributes`
 *                 lists what the tag may carry. A missing input, an unknown
 *                 attribute or a value out of range fails the build. The page
 *                 renders the component for the tag, prerendered with its data.
 *
 * Adding a new kind of figure means a new entry here and its component;
 * adding a new instance means a line in a page and an input in site/inputs/.
 */
import type { FigureKind } from '$lib/types';

export type TagEntry =
	| { kind: 'passthrough'; reason: string }
	| {
			kind: 'figure';
			figure: FigureKind;
			reason: string;
			/** Attribute name to whether the tag must carry it. */
			attributes: Readonly<Record<string, 'required' | 'optional'>>;
	  };

export const REGISTRY: Readonly<Record<string, TagEntry>> = {
	svg: {
		kind: 'passthrough',
		reason:
			'The hand-authored architecture diagrams. Position carries their meaning, ' +
			'each is role="img" with an aria-label, and they show structure, not data.'
	},
	'figure-parse': {
		kind: 'figure',
		figure: 'parse',
		reason:
			'The dependency parse of one input, a sentence at a time: the token table, ' +
			'then an arc diagram of the same tokens. Answers "what does each word ' +
			'depend on, and by which relation?" faster than the table alone.',
		attributes: { input: 'required', sentence: 'optional' }
	},
	'figure-primitives': {
		kind: 'figure',
		figure: 'primitives',
		reason:
			'The structural primitive fields of every sentence of one input, marked on the ' +
			'words they point at. Answers "which words did matra read each primitive off?" ' +
			'faster than cross-referencing token ids in a table.',
		attributes: { input: 'required' }
	},
	'figure-metrics': {
		kind: 'figure',
		figure: 'metrics',
		reason:
			'The paragraph measures of one document as small multiples, one panel a measure, ' +
			'with the document value where matra computes one. Answers "where in the document ' +
			'do the measures move, and together?" faster than a column of numbers.',
		attributes: { input: 'required' }
	},
	'figure-keyphrases': {
		kind: 'figure',
		figure: 'keyphrases',
		reason:
			'RAKE and YAKE ranks of the same phrases as a slopegraph. Answers "do the two ' +
			'methods agree on what ranks high?" faster than two ranked lists side by side.',
		attributes: { input: 'required' }
	},
	'figure-textrank': {
		kind: 'figure',
		figure: 'textrank',
		reason:
			'Every sentence\'s TextRank score in document order, with the summary it picks ' +
			'marked. Answers "where in the document did the summary come from, and how far ' +
			'ahead of the rest were its sentences?" faster than a sorted list.',
		attributes: { input: 'required' }
	},
	'figure-clusters': {
		kind: 'figure',
		figure: 'clusters',
		reason:
			'Semantic clusters at a fixed grid of thresholds, stepped by the reader. Answers ' +
			'"what does moving the threshold do to the clusters?" faster than a table per value.',
		attributes: { input: 'required', threshold: 'optional' }
	},
	'figure-pipeline': {
		kind: 'figure',
		figure: 'pipeline',
		reason:
			'One document at each stage of the pipeline, stepped by the reader: ingested, ' +
			'annotated, composed. Answers "what does each stage add?" faster than reading the ' +
			'stages\' types.',
		attributes: { input: 'required' }
	}
};

/**
 * The elements CommonMark and GitHub Flavored Markdown produce. These need no
 * registration, whether they came from Markdown syntax or were written as HTML.
 */
export const MARKDOWN_ELEMENTS: ReadonlySet<string> = new Set([
	// CommonMark blocks and inlines
	'p',
	'h1',
	'h2',
	'h3',
	'h4',
	'h5',
	'h6',
	'blockquote',
	'ul',
	'ol',
	'li',
	'pre',
	'code',
	'hr',
	'br',
	'a',
	'em',
	'strong',
	'img',
	// GFM: tables, strikethrough, task lists, footnotes
	'table',
	'thead',
	'tbody',
	'tr',
	'th',
	'td',
	'del',
	'input',
	'section',
	'sup'
]);
