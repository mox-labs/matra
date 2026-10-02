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
	// The installation page became the how-to "Install matra"; every section
	// heading kept its id.
	'/tutorials/installation': [{ id: 'installation' }],
	// Pragmatics kept its explanation. Its how-to for agents became a guide of
	// its own, and its table of caps repeated the InputTooLarge table.
	'/explanation/pragmatics': [
		{ id: 'for-an-agent', to: '/guides/agent' },
		{ id: 'cost-and-limits', to: '/reference/errors#inputtoolarge' }
	]
};
