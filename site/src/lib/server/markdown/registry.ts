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
 * One kind of entry exists today:
 *
 *   passthrough   the element and everything inside it are emitted as
 *                 written. For hand-authored markup whose meaning is the
 *                 markup itself.
 *
 * Figures (EP-0012, M3) add a second kind that maps a tag to a component.
 */
export type TagEntry = { kind: 'passthrough'; reason: string };

export const REGISTRY: Readonly<Record<string, TagEntry>> = {
	svg: {
		kind: 'passthrough',
		reason:
			'The hand-authored architecture diagrams. Position carries their meaning, ' +
			'each is role="img" with an aria-label, and they show structure, not data.'
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
