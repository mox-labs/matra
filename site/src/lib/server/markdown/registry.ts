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
export type TagEntry =
	| { kind: 'passthrough'; reason: string }
	| {
			kind: 'figure';
			figure: 'parse';
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
