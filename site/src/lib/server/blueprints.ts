/**
 * The Blueprints area: matra's design records, rendered from blueprints/ at
 * the repository root. Nothing is copied into site/content/; the files the
 * process edits are the files the site reads.
 *
 *   blueprints/README.md             /blueprints/index   (served as /blueprints/)
 *   blueprints/proposals/<name>.md   /blueprints/proposals/<name>
 *   blueprints/plans/<name>.md       /blueprints/plans/<name>
 *
 * The templates (0000-template.md) are not records and are not rendered;
 * the index links them on GitHub. Neither are the legacy records in
 * blueprints/legacy/, the RFCs and EPs written before 2026-10-04: their old
 * addresses keep a page that links each on GitHub (legacyForwards).
 *
 * This is the one area of the site where a status appears. Each record's
 * status is read from the `- Status:` line of its header, so the navigation
 * says what the record says, and nothing is written twice.
 */
import { REPO_URL } from '$lib/site';
import type { NavItem, NavPart } from '$lib/types';
import { assumptionsOf, readPin, recordFacts, type HeaderLink, type RecordCard } from './markdown/legibility';

/** The area's title in the navigation. */
export const BLUEPRINTS_PART = 'Blueprints';

const RAW = import.meta.glob(
	['../../../../blueprints/README.md', '../../../../blueprints/proposals/*.md', '../../../../blueprints/plans/*.md'],
	{ query: '?raw', import: 'default', eager: true }
) as Record<string, string>;

/** Each rendered file's source, keyed by its path from the repository root (`blueprints/proposals/NNNN-name.md`). */
export const blueprintSources: ReadonlyMap<string, string> = new Map(
	Object.entries(RAW)
		.map(([key, text]): [string, string] => [key.replace(/^(\.\.\/)+/, ''), text])
		.filter(([file]) => !file.endsWith('/0000-template.md'))
);

export interface Record_ {
	/** The path from the repository root. */
	file: string;
	route: string;
	kind: 'EPR' | 'EPL' | 'index';
	/** `EPR-NNNN`, or null for the index. */
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
	const m = /^blueprints\/(proposals|plans)\/(\d{4})-[^/]+\.md$/.exec(file);
	if (!index && !m) {
		throw new Error(`${file}: a blueprint is blueprints/README.md or blueprints/{proposals,plans}/NNNN-name.md`);
	}
	const h1 = /^# (.+)$/m.exec(text)?.[1]?.trim();
	if (!h1) throw new Error(`${file}: a record must open with its # title`);
	const route = `/${file.replace(/\.md$/, '').replace(/README$/, 'index')}`;
	if (index) return { file, route, kind: 'index', id: null, title: 'Process and index', status: '' };
	const kind = m![1] === 'proposals' ? 'EPR' : 'EPL';
	const id = `${kind}-${m![2]}`;
	if (!h1.startsWith(`${id}: `)) {
		throw new Error(`${file}: its title begins "${id}: ", the kind and number its path gives it`);
	}
	const status = /^- Status:\s*(.+)$/m.exec(text)?.[1];
	if (!status) throw new Error(`${file}: a record's header carries a "- Status:" line`);
	return { file, route, kind, id, title: h1.slice(id.length + 2), status: shortStatus(status) };
}

const all = [...blueprintSources.entries()].map(([file, text]) => parse(file, text));
const byNumber = (a: Record_, b: Record_) => (a.id ?? '').localeCompare(b.id ?? '');
const index = all.find((r) => r.kind === 'index');
if (!index) throw new Error('blueprints/README.md, the index, is missing');
const proposals = all.filter((r) => r.kind === 'EPR').sort(byNumber);
const plans = all.filter((r) => r.kind === 'EPL').sort(byNumber);

/** Every blueprint page, in reading order: the index, the proposals, the plans. */
export const records: Record_[] = [index, ...proposals, ...plans];

/** The first sentence of a record's Summary, as text. */
function summaryOf(text: string): string {
	const body = /(?:^|\n)## Summary[ \t]*\n+([\s\S]*?)(?:\n[ \t]*\n|$)/.exec(text)?.[1] ?? '';
	const plain = body
		.replace(/<[^>]+>/g, '')
		.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
		.replace(/[`*_]/g, '')
		.replace(/\s+/g, ' ')
		.trim();
	return /^(.{20,}?[.!?])(\s|$)/.exec(plain)?.[1] ?? plain;
}

/**
 * A link a record's header names on a line of its own (`- Proposal PR:
 * [#134](https://...)`), or null when the line names none: a tracking issue
 * not opened yet, or none for a baseline.
 */
function headerLink(text: string, ...keys: string[]): HeaderLink | null {
	for (const key of keys) {
		const line = new RegExp(`^- ${key}:\\s*(.*)$`, 'm').exec(text)?.[1];
		const link = line ? /\[([^\]]+)\]\((https?:[^)\s]+)\)/.exec(line) : null;
		if (link) return { text: link[1].replace(/`/g, ''), href: link[2] };
	}
	return null;
}

/** A header line's value as plain words: links to their text, code to its letters. */
function headerText(text: string, key: string): string | null {
	const line = new RegExp(`^- ${key}:\\s*(.*)$`, 'm').exec(text)?.[1];
	return line === undefined ? null : line.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/`/g, '').trim();
}

/** The header's facts the frame shows besides the commit, the pull request and the tracking issue, labelled as the masthead labels them. */
const HEADER_FIELDS: [string, string][] = [
	['Start Date', 'Started'],
	['Feature Name', 'Feature name'],
	['Implements', 'Implements']
];

/**
 * What the index's cards, the queue and each record's frame show: its
 * status, the first sentence of its Summary, the commit its citations were
 * read at, its pull request and tracking issue, and its decisions and
 * assumptions, read from the components in its source by the same parse the
 * page is rendered from (recordFacts), so a dangling id, a dependency cycle
 * or a decision without `reversible` fails the build here too.
 */
export const recordCards: RecordCard[] = [...proposals, ...plans].map((r) => {
	const text = blueprintSources.get(r.file)!;
	const facts = recordFacts(text, r.file);
	return {
		id: r.id!,
		title: r.title,
		status: r.status,
		route: r.route,
		summary: summaryOf(text),
		readAt: readPin(text, r.file),
		pr: headerLink(text, 'Proposal PR', 'Plan PR'),
		tracking: headerLink(text, 'Tracking issue'),
		trackingNote: headerLink(text, 'Tracking issue') ? null : headerText(text, 'Tracking issue'),
		header: HEADER_FIELDS.flatMap(([key, label]) => {
			const value = headerText(text, key);
			return value ? [{ label, text: value }] : [];
		}),
		decisions: facts.decisions,
		assumptions: assumptionsOf(facts)
	};
});

const item = (r: Record_): NavItem => ({
	title: r.id ? `${r.id} ${r.title}` : r.title,
	file: r.file,
	route: r.route,
	children: [],
	status: r.status || undefined
});

/**
 * The area in the navigation. The proposals and the plans are groups whose
 * own link is their section of the index; each record names its status
 * beside it.
 */
export const blueprintsPart: NavPart = {
	title: BLUEPRINTS_PART,
	area: 'blueprints',
	note: 'design records, with their status',
	items: [
		item(index),
		{ title: 'Proposals', file: index.file, route: `${index.route}#proposals`, group: true, children: proposals.map(item) },
		{ title: 'Plans', file: index.file, route: `${index.route}#plans`, group: true, children: plans.map(item) }
	]
};

/**
 * The addresses the legacy records were published at before 2026-10-04, as
 * the site's own Blueprints part. Each still answers, with a page that links
 * the record on GitHub. The list is fixed; never remove an entry, since the
 * link it keeps is still out there.
 */
const LEGACY: Readonly<Record<'rfcs' | 'eps', readonly string[]>> = {
	rfcs: [
		'0001-record-architectural-decisions',
		'0002-pipeline-vocabulary',
		'0003-workspace-with-rumi-nlp',
		'0004-stay-single-crate',
		'0005-supply-chain-hardening',
		'0006-abstract-tier-vocabulary-lock',
		'0007-one-pipeline',
		'0008-structural-primitives-are-fields',
		'0009-feats-lookup-accessor',
		'0010-embeddings-adapter',
		'0011-out-of-the-box',
		'0012-agent-surface',
		'0013-attribution-and-citation',
		'0014-distribution-matrix',
		'0015-provisioning-failures',
		'0019-rfc-and-ep-process',
		'0020-deprecate-unread-config-keys'
	],
	eps: [
		'0007-structural-primitives',
		'0008-pipeline-surface',
		'0009-embeddings-adapter',
		'0010-foundations',
		'0011-agent-surface',
		'0012-docsite',
		'0013-docsite-identity',
		'0014-architecture-guardrails'
	]
};

const LEGACY_FILES = import.meta.glob('../../../../blueprints/legacy/*/*.md', { query: '?raw', import: 'default' });

export interface LegacyForward {
	/** The old route, `/blueprints/rfcs/0007-one-pipeline`. */
	from: string;
	/** `RFC-0007`, or the template's kind. */
	id: string;
	/** The file it is now, from the repository root. */
	file: string;
	/** The file on GitHub, on main. */
	url: string;
}

function forward(from: string, id: string, file: string): LegacyForward {
	if (!LEGACY_FILES[`../../../../${file}`] && !RAW[`../../../../${file}`]) {
		throw new Error(`${from} forwards to ${file}, which does not exist`);
	}
	return { from, id, file, url: `${REPO_URL}/blob/main/${file}` };
}

/** Every old record address and where it now leads; one that names no file fails the build. */
export const legacyForwards: LegacyForward[] = [
	...LEGACY.rfcs.map((n) => forward(`/blueprints/rfcs/${n}`, `RFC-${n.slice(0, 4)}`, `blueprints/legacy/rfcs/${n}.md`)),
	...LEGACY.eps.map((n) => forward(`/blueprints/eps/${n}`, `EP-${n.slice(0, 4)}`, `blueprints/legacy/eps/${n}.md`)),
	// The templates moved to the new kinds rather than into legacy.
	forward('/blueprints/rfcs/0000-template', 'the proposal template', 'blueprints/proposals/0000-template.md'),
	forward('/blueprints/eps/0000-template', 'the plan template', 'blueprints/plans/0000-template.md')
];
