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
 *   example       one part of a worked example (EP-0012, M6), written the
 *                 same way as a figure and naming a directory in
 *                 site/examples/:
 *
 *                   <example-call name="summarize" />
 *
 *                 `part` says which part the tag renders: the input, the
 *                 call in Rust, Python and the CLI, or what the calls print.
 *
 *   component     a Blueprints component (legibility.ts): a part of a
 *                 design record that makes Claude's understanding readable
 *                 and answerable, from a claim with its grounds to the
 *                 owner's decision slot. Written with a closing tag around
 *                 Markdown, or self-closing on a line of its own where it
 *                 holds nothing. `attributes` lists what it may carry, and
 *                 `within` the component it belongs directly inside. It
 *                 renders on a design record or a Lab page only; on a
 *                 product page it fails the build. Syntax, purpose and
 *                 evidence: site/README.md, "The Blueprints components".
 *
 * Adding a new kind of figure means a new entry here and its component;
 * adding a new instance means a line in a page and an input in site/inputs/.
 * Adding an example means a directory in site/examples/ and a page.
 */
import type { FigureKind } from '$lib/types';

export type TagEntry =
	| { kind: 'passthrough'; reason: string }
	| { kind: 'example'; part: 'input' | 'call' | 'output'; reason: string }
	| {
			kind: 'figure';
			figure: FigureKind;
			reason: string;
			/** Attribute name to whether the tag must carry it. */
			attributes: Readonly<Record<string, 'required' | 'optional'>>;
	  }
	| {
			kind: 'component';
			reason: string;
			/** Attribute name to whether the tag must carry it. */
			attributes: Readonly<Record<string, 'required' | 'optional'>>;
			/** The component this one belongs directly inside, if any. */
			within?: string;
	  };

const part = (within: string, reason: string, attributes: Record<string, 'required' | 'optional'> = {}): TagEntry => ({
	kind: 'component',
	reason,
	attributes,
	within
});

export const REGISTRY: Readonly<Record<string, TagEntry>> = {
	details: {
		kind: 'passthrough',
		reason:
			'A disclosure: the answer to a predict-then-reveal prompt (What the clustering threshold does), ' +
			'kept closed until the reader opens it. It works without a script.'
	},
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
	},
	'example-input': {
		kind: 'example',
		part: 'input',
		reason: "A worked example's input: its text, source and licence, and a link to download it."
	},
	'example-call': {
		kind: 'example',
		part: 'call',
		reason:
			"A worked example's call in Rust, Python and the CLI, as tabs, from the files the " +
			'examples gate runs.'
	},
	'example-output': {
		kind: 'example',
		part: 'output',
		reason: 'What the calls print, trimmed where long, with the whole output linked.'
	},

	// The Blueprints components (legibility.ts; site/README.md, "The Blueprints components").
	claim: {
		kind: 'component',
		reason:
			'A load-bearing claim, its basis as a glyph and a word (observed, inferred, assumed), a likelihood ' +
			'from the closed vocabulary on a prediction only, and chips to its grounds: pinned code lines, ' +
			'pull requests, issues. Grounds beside the claim, cheap to check (Toulmin; Bansal 2021; Vasconcelos 2023). ' +
			'An `id` lets a decision name it among its grounds, and the claim links back to that decision.',
		attributes: { basis: 'required', likelihood: 'optional', id: 'optional' }
	},
	assumptions: {
		kind: 'component',
		reason: 'Every claim on the page marked assumed, collected so the owner can confirm or strike each in one place.',
		attributes: {}
	},
	decision: {
		kind: 'component',
		reason:
			"Options, Claude's recommendation set apart as a judgment, the strongest case against it, and the " +
			"owner's decision slot (dossier-grammar G2; Toulmin's rebuttal). `reversible` (yes, costly or no) says " +
			'how readily the recommended choice can be undone; `depends` names the decisions it waits for, so the ' +
			'order is shown (dossier-grammar G1); `grounds` names the claims it rests on, linked both ways (G2).',
		attributes: { id: 'required', title: 'required', reversible: 'required', depends: 'optional', grounds: 'optional' }
	},
	choice: part('decision', 'One option of a decision, collapsed to its title.', { key: 'required', title: 'required' }),
	recommendation: part('decision', "Claude's recommendation: which choice, and why.", { choice: 'required', basis: 'optional' }),
	against: part('decision', 'The strongest case against the recommendation.'),
	ruling: part('decision', "The owner's decision, once made.", { response: 'required', date: 'required' }),
	pragmatics: {
		kind: 'component',
		reason:
			'What the record asks, what Claude will do if it is accepted, what it needs, what it will not do, and ' +
			'that silence is not assent (Amershi G1, G10, G16; dossier-grammar C3).',
		attributes: {}
	},
	ask: part('pragmatics', 'The ask, in a sentence.'),
	will: part('pragmatics', 'What Claude will do if the record is accepted.'),
	needs: part('pragmatics', 'What Claude needs from the owner, as questions that can be answered.'),
	wont: part('pragmatics', 'What Claude will not do.'),
	silence: part('pragmatics', 'What happens if nobody answers.'),
	changed: {
		kind: 'component',
		reason:
			"What changed in Claude's understanding: was, now, and the evidence that changed it (a grounding " +
			'move made visible).',
		attributes: { date: 'required', since: 'required' }
	},
	was: part('changed', 'What Claude understood before.'),
	now: part('changed', 'What it understands now, with a link to what changed it.'),
	'sketch-figure': {
		kind: 'component',
		reason:
			'Structure drawn sketchy when proposed and crisp when shipped, from a declared structure that is also ' +
			'its text twin, with Rough.js at a fixed seed at build time (Wood 2012; Boukhelifa 2012).',
		attributes: { id: 'required', title: 'required', seed: 'required' }
	},
	experiment: {
		kind: 'component',
		reason:
			'A Lab experiment: the hypothesis recorded before the run, the method, every run as a dot, the result ' +
			'in words, provenance, and what it does not show (Padilla, Kay and Hullman 2022).',
		attributes: { id: 'required', title: 'required' }
	},
	hypothesis: part('experiment', 'The hypothesis and prediction, with the day and commit they were recorded at.', {
		recorded: 'required',
		commit: 'required'
	}),
	method: part('experiment', 'How the experiment was run.'),
	outcomes: part('experiment', "Every run's outcome, never a bare mean.", {
		values: 'required',
		unit: 'required',
		label: 'required',
		min: 'optional',
		max: 'optional'
	}),
	result: part('experiment', 'The result in words, with its interval stated as a sentence.'),
	provenance: part('experiment', 'Inputs and licences, matra version, model digests.'),
	limits: part('experiment', 'What the result does not show.'),
	'record-index': {
		kind: 'component',
		reason:
			"The Blueprints index as cards, each record's status and open decisions at a glance; the index's " +
			"table that follows the tag becomes the cards' text twin.",
		attributes: { kind: 'required' }
	},
	awaiting: {
		kind: 'component',
		reason:
			'What awaits the owner across every proposal, in one place: each open decision, in dependency order, ' +
			'then each assumption to confirm, drawn from the records\' components (triage precedes reading).',
		attributes: {}
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
