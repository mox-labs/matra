/** Where the project lives. One place, so a move is a one-line change. */
export const REPO_URL = 'https://github.com/mox-labs/matra';
export const ISSUES_URL = `${REPO_URL}/issues`;
export const DISCUSSIONS_URL = `${REPO_URL}/discussions`;
export const SITE_NAME = 'matra';
/** Where the site is published, for commands a reader copies. */
export const SITE_URL = 'https://mox-labs.github.io/matra';

/**
 * The accent each SUMMARY part carries in the navigation, by the role of what
 * it holds. Chosen per part, and colour is never the only signal (the part
 * is named beside it):
 *   spark      the reader's own hands: the tutorials, installing and a
 *              first run
 *   emergence  matra's output, run and shown: the how-to guides
 *   planned    not shipped: a dashed neutral rule
 *   neutral    everything else, which is most of it
 */
export type PartRole = 'spark' | 'emergence' | 'planned' | 'neutral';

export const PART_ROLES: Readonly<Record<string, PartRole>> = {
	Tutorials: 'spark',
	'How-to guides': 'emergence',
	'What is planned': 'planned'
};

/** The GitHub edit link for a file, given its path from the repository root. */
export function editUrl(repoPath: string): string {
	return `${REPO_URL}/edit/main/${repoPath}`;
}
