/**
 * Heading ids that are published URLs (site/anchors.txt) whose heading has
 * since moved to another page or been reworded.
 *
 * `page.html#section` is cited from outside the site, so a reworded or moved
 * heading must not break it. Each id listed here stays on its page as an
 * empty element, which is all the anchor manifest asks for. When the section
 * now lives on another page, `to` names it, and a link to the old anchor is
 * sent on there by script; without a script it lands at the top of the page
 * it names, whose navigation leads on.
 *
 * Keyed by the page's route. Add an entry when a listed heading is reworded
 * or moved; never delete one, since the link it keeps is still out there.
 */
export const MOVED_ANCHORS: Readonly<Record<string, readonly { id: string; to?: string }[]>> = {
	// Until EP-0012's M6 the front page was the introduction; until the
	// Diátaxis pass the home page carried the clusters figure.
	'/home': [
		{ id: 'four-tiers-of-output', to: '/introduction#four-tiers-of-output' },
		{ id: 'end-to-end', to: '/introduction#end-to-end' },
		{ id: 'one-passage-and-what-matra-returns', to: '/explanation/semantic-clusters#watch-the-threshold-move' }
	],
	// The guide's title became the goal it serves; when the Diátaxis splits
	// moved its reference sections to their own page, its bounds went with them.
	'/guides/semantic-clusters': [
		{ id: 'semantic-clusters' },
		{ id: 'bounds-and-failure', to: '/reference/semantic-clusters#bounds-and-failure' }
	],
	// The Blueprints index: until 2026-10-04 its two tables were the RFCs and
	// the EPs, which are now the legacy records.
	'/blueprints/index': [
		{ id: 'rfcs', to: '/blueprints/index#legacy' },
		{ id: 'eps', to: '/blueprints/index#legacy' }
	],
	// The installation page became the how-to "Install matra"; every section
	// heading kept its id.
	'/tutorials/installation': [{ id: 'installation' }],
	// Pragmatics kept its explanation. Its how-to for agents became a guide of
	// its own, and its table of caps repeated the InputTooLarge table.
	'/explanation/pragmatics': [
		{ id: 'for-an-agent', to: '/guides/agent' },
		{ id: 'cost-and-limits', to: '/reference/errors#inputtoolarge' }
	],
	// EPR-0006 milestone 3: the parser trait is private, so the Rust guide
	// constructs no provider; the outcome types replaced `ScoredSentence`
	// and the bare `Keyphrase` list; `Corpus` and `CorpusResult` are gone,
	// a stream of `Document` or `DocumentError` items in their place.
	'/guides/rust': [{ id: 'construct-a-provider', to: '/guides/rust#where-the-model-lives' }],
	'/reference/domain-types': [
		{ id: 'scoredsentence', to: '/reference/domain-types#summary' },
		{ id: 'keyphrase', to: '/reference/domain-types#keyphrases' },
		{ id: 'corpus', to: '/reference/domain-types#documenterror' },
		{ id: 'methods-4', to: '/reference/domain-types#documenterror' },
		{ id: 'corpusresult', to: '/reference/domain-types#documenterror' }
	],
	'/reference/errors': [
		{
			id: 'per-document-failures-documenterror-and-corpusresult',
			to: '/reference/errors#per-document-failures-documenterror'
		}
	]
};
