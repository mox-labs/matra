/**
 * The Lab area: evals and experiments, rendered from lab/ at the repository
 * root, as Blueprints is from blueprints/. Today it is its landing page.
 *
 *   lab/README.md   /lab/index   (served as /lab/)
 */
import type { NavPart } from '$lib/types';

const RAW = import.meta.glob('../../../../lab/README.md', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

/** Each Lab page's source, keyed by its path from the repository root. */
export const labSources: ReadonlyMap<string, string> = new Map(
	Object.entries(RAW).map(([key, text]) => [key.replace(/^(\.\.\/)+/, ''), text])
);
if (!labSources.has('lab/README.md')) throw new Error('lab/README.md, the Lab landing page, is missing');

export const labPart: NavPart = {
	title: 'Lab',
	area: 'lab',
	note: 'evals and experiments',
	items: [{ title: 'About the Lab', file: 'lab/README.md', route: '/lab/index', children: [] }]
};
