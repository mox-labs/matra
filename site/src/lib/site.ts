/** Where the project lives. One place, so a move is a one-line change. */
export const REPO_URL = 'https://github.com/mox-labs/matra';
export const ISSUES_URL = `${REPO_URL}/issues`;
export const DISCUSSIONS_URL = `${REPO_URL}/discussions`;
export const SITE_NAME = 'matra';
/** Where the site is published, for commands a reader copies. */
export const SITE_URL = 'https://mox-labs.github.io/matra';

/** The GitHub edit link for a file, given its path from the repository root. */
export function editUrl(repoPath: string): string {
	return `${REPO_URL}/edit/main/${repoPath}`;
}
