/**
 * Build-time switches, read from the environment when the site is prerendered.
 *
 * They are read here rather than through `$env/static/public` because a
 * variable that is unset must mean "off", and a static import of an unset
 * variable is a build error rather than an empty value. Every page is
 * prerendered, so the values are fixed into the HTML at build time either way.
 */
export interface GiscusConfig {
	repo: string;
	repoId: string;
	categoryId: string;
}

/**
 * Comments render only when both ids are set. See site/README.md for how to
 * obtain them and switch comments on.
 */
export function giscusConfig(): GiscusConfig | null {
	const repoId = process.env.PUBLIC_GISCUS_REPO_ID?.trim() ?? '';
	const categoryId = process.env.PUBLIC_GISCUS_CATEGORY_ID?.trim() ?? '';
	if (!repoId || !categoryId) return null;
	return { repo: 'mox-labs/matra', repoId, categoryId };
}
