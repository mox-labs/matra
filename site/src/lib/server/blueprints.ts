/**
 * The Blueprints part: matra's design records, rendered from blueprints/ at
 * the repository root. Nothing is copied into site/content/; the files the
 * process edits are the files the site reads.
 *
 *   blueprints/README.md        /blueprints/index   (served as /blueprints/)
 *   blueprints/rfcs/<name>.md   /blueprints/rfcs/<name>
 *   blueprints/eps/<name>.md    /blueprints/eps/<name>
 *
 * This is the one part of the site where a status appears. Each record's
 * status is read from the `- Status:` line of its header, so the navigation
 * says what the record says, and nothing is written twice.
 */
import type { NavItem, NavPart } from '$lib/types';

/** The part's title in SUMMARY order; it follows every SUMMARY.md part. */
export const BLUEPRINTS_PART = 'Blueprints';

const RAW = import.meta.glob('../../../../blueprints/**/*.md', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

/** Each record's source, keyed by its path from the repository root (`blueprints/rfcs/0007-one-pipeline.md`). */
export const blueprintSources: ReadonlyMap<string, string> = new Map(
	Object.entries(RAW).map(([key, text]) => [key.replace(/^(\.\.\/)+/, ''), text])
);

export interface Record_ {
	/** The path from the repository root. */
	file: string;
	route: string;
	kind: 'RFC' | 'EP' | 'index';
	/** `RFC-0007`, or null for the index. */
	id: string | null;
	title: string;
	status: string;
}

/** The status as the navigation shows it: the header's value, as text, without its parenthetical. */
export function shortStatus(line: string): string {
	return line
		.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
		.replace(/`/g, '')
		.replace(/\s*\(.*$/, '')
		.trim();
}

function parse(file: string, text: string): Record_ {
	const index = file === 'blueprints/README.md';
	const m = /^blueprints\/(rfcs|eps)\/(\d{4})-[^/]+\.md$/.exec(file);
	if (!index && !m) {
		throw new Error(`${file}: a blueprint is blueprints/README.md or blueprints/{rfcs,eps}/NNNN-name.md`);
	}
	const h1 = /^# (.+)$/m.exec(text)?.[1]?.trim();
	if (!h1) throw new Error(`${file}: a record must open with its # title`);
	const route = `/${file.replace(/\.md$/, '').replace(/README$/, 'index')}`;
	if (index) return { file, route, kind: 'index', id: null, title: 'Process and index', status: '' };
	const kind = m![1] === 'rfcs' ? 'RFC' : 'EP';
	const id = `${kind}-${m![2]}`;
	const template = m![2] === '0000';
	const status = /^- Status:\s*(.+)$/m.exec(text)?.[1];
	if (!status && !template) {
		throw new Error(`${file}: a record's header carries a "- Status:" line`);
	}
	return {
		file,
		route,
		kind,
		id,
		title: template ? 'The template' : h1.replace(/^(RFC|EP)-\d{4}:\s*/, ''),
		status: template ? 'template' : shortStatus(status!)
	};
}

const all = [...blueprintSources.entries()].map(([file, text]) => parse(file, text));
// By number, with the template, which is not a record, last (as in the index).
const sortKey = (r: Record_) => (r.status === 'template' ? '9999' : (r.id ?? ''));
const byNumber = (a: Record_, b: Record_) => sortKey(a).localeCompare(sortKey(b));
const index = all.find((r) => r.kind === 'index');
if (!index) throw new Error('blueprints/README.md, the index, is missing');
const rfcs = all.filter((r) => r.kind === 'RFC').sort(byNumber);
const eps = all.filter((r) => r.kind === 'EP').sort(byNumber);

/** Every blueprint page, in reading order: the index, the RFCs, the EPs. */
export const records: Record_[] = [index, ...rfcs, ...eps];

const item = (r: Record_): NavItem => ({
	title: r.id ? `${r.id} ${r.title}` : r.title,
	file: r.file,
	route: r.route,
	children: [],
	status: r.status || undefined
});

/**
 * The part in the navigation. The RFCs and the EPs are groups whose own link
 * is their table in the index; each record names its status beside it.
 */
export const blueprintsPart: NavPart = {
	title: BLUEPRINTS_PART,
	note: 'design records, with their status',
	items: [
		item(index),
		{ title: 'RFCs', file: index.file, route: `${index.route}#rfcs`, group: true, children: rfcs.map(item) },
		{ title: 'EPs', file: index.file, route: `${index.route}#eps`, group: true, children: eps.map(item) }
	]
};
