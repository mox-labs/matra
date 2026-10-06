/**
 * The Blueprints components, held to their rules on a fixture, so a
 * component no record uses yet is checked as one in use is. Part of
 * `bun run check`, so gate 4.
 *
 *   twins        the fixture (scripts/fixtures/legibility.md) rendered as a
 *                record renders, and every check gate 9 makes of a built page
 *                (legibility-checks.ts) made of it
 *   determinism  rendered twice, with Math.random replaced by a function
 *                that throws: the two renders are byte for byte the same,
 *                and nothing reached for unseeded randomness. A sketch drawn
 *                from an unseeded Rough.js would fail here, and would change
 *                the built HTML on every build
 *   refusals     each mistake a component exists to stop, planted on its
 *                own, fails the render with a message naming it: an observed
 *                claim with no evidence, a citation whose lines lack the text
 *                it names, a likelihood on an observation, a word outside the
 *                closed vocabulary, a sketch with no seed, a decision with no
 *                case against it, a part outside its component, a decision
 *                without `reversible` or with one outside its vocabulary, a
 *                `depends` or `grounds` id that names nothing on the page, a
 *                dependency cycle, a claim id used twice, and a commit not
 *                in this clone
 *
 * Usage: bun scripts/check-legibility.ts
 */
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fromHtml } from 'hast-util-from-html';
import type { Element } from 'hast';
import { assumptionsOf, recordFacts, renderFragment, type LegibilityContext } from '../src/lib/server/markdown/legibility';
import { all, checkPage, COMPARE, prop } from './legibility-checks';

const FIXTURE = readFileSync(join(import.meta.dir, 'fixtures', 'legibility.md'), 'utf8');
// The fixture's made-up record, spelled in parts so the citation check
// (scripts/check-blueprint-refs.sh) does not read it as a citation.
const FIXTURE_ID = ['EPR', '9999'].join('-');
const PIN = '4fcfb4adc85524243c5e392becd4f490e3e42252';

// The fixture's own decisions and assumptions, read as blueprints.ts reads a
// record's, so its card and its queue show what its components say.
const FACTS = recordFacts(FIXTURE, 'scripts/fixtures/legibility.md');

export const fixtureContext: LegibilityContext = {
	file: 'scripts/fixtures/legibility.md',
	// Rendered as if it were a proposal, so its links resolve as a record's do.
	repoFile: 'blueprints/proposals/9999-fixture.md',
	pin: PIN,
	repoUrl: 'https://github.com/mox-labs/matra',
	repoRoot: resolve(import.meta.dir, '..', '..'),
	base: '',
	records: [
		{
			id: FIXTURE_ID,
			title: 'A fixture',
			status: 'proposed',
			route: '/blueprints/proposals/9999-fixture',
			summary: 'A fixture.',
			readAt: PIN,
			pr: { text: '#0', href: 'https://github.com/mox-labs/matra/pull/0' },
			tracking: null,
			decisions: FACTS.decisions,
			assumptions: assumptionsOf(FACTS)
		}
	]
};

/** The fixture as HTML: what the responsive gate lays out. */
export function renderFixture(): Promise<string> {
	return renderFragment(FIXTURE, fixtureContext);
}

const failures: string[] = [];

if (import.meta.main) {
	// Determinism, and no unseeded randomness.
	const random = Math.random;
	Math.random = () => {
		throw new Error('Math.random was called while rendering: something is unseeded');
	};
	let first = '';
	let second = '';
	try {
		first = await renderFixture();
		second = await renderFixture();
	} catch (e) {
		failures.push(`determinism: ${(e as Error).message.split('\n')[0]}`);
	} finally {
		Math.random = random;
	}
	if (first && first !== second) failures.push('determinism: two renders of the fixture differ');

	// Every check gate 9 makes, on the fixture.
	let figures = 0;
	let examined = 0;
	if (first) {
		const tree = fromHtml(first, { fragment: true });
		const kinds = new Set<string>();
		for (const fig of all(tree, (e) => e.properties.dataFigure !== undefined)) {
			const kind = prop(fig, 'dataFigure') ?? '';
			kinds.add(kind);
			figures += 1;
			const table = all(fig, (e: Element) => e.tagName === 'table' && prop(e, 'dataTwinFor') === prop(fig, 'id'))[0];
			if (!table) {
				failures.push(`twins: ${kind} has no text twin`);
				continue;
			}
			COMPARE[kind]?.(fig, table, (msg) => failures.push(`twins: ${kind}: ${msg}`));
		}
		for (const k of Object.keys(COMPARE)) if (!kinds.has(k)) failures.push(`twins: the fixture draws no ${k}; it must hold every component`);
		examined = checkPage(tree, (msg) => failures.push(`twins: ${msg}`));
		for (const c of ['claim', 'assumptions', 'decision', 'pragmatics', 'changed', 'masthead', 'sheet']) {
			if (all(tree, (e) => Array.isArray(e.properties.className) && e.properties.className.includes(c)).length === 0) {
				failures.push(`twins: the fixture renders no ${c}`);
			}
		}
	}

	// Refusals: each planted mistake must fail, with its reason.
	const HEAD = `# ${FIXTURE_ID}: Refusal\n\n- Pinned at: \`${PIN}\`\n- Status: proposed\n\n`;
	const DECISION = (inner: string, attrs = 'id="x" title="X" reversible="yes"') =>
		`<decision ${attrs}>\n\n<choice key="a" title="A">\n\nA.\n\n</choice>\n\n<choice key="b" title="B">\n\nB.\n\n</choice>\n\n${inner}\n\n</decision>\n`;
	const CASE = '<recommendation choice="a">\n\nA.\n\n</recommendation>\n\n<against>\n\nNo.\n\n</against>';
	const EXPERIMENT = (values: string) =>
		`<experiment id="e" title="E">\n\n<hypothesis recorded="2026-10-04" commit="${PIN}">\n\nH.\n\n</hypothesis>\n\n` +
		`<method>\n\nM.\n\n</method>\n\n<outcomes ${values} unit="F1" label="L" />\n\n<result>\n\nR.\n\n</result>\n\n` +
		`<provenance>\n\nP.\n\n</provenance>\n\n<limits>\n\nX.\n\n</limits>\n\n</experiment>\n`;
	const refusals: [string, string, string][] = [
		['an observed claim with no evidence', 'Text <claim basis="observed">no chip</claim>.', 'points at its evidence'],
		['a citation whose lines lack its text', '[x](../../src/lib.rs#L250 "not on that line")', 'do not contain'],
		['a chip with no text to check', 'A <claim basis="observed">x [y](../../src/lib.rs#L250)</claim>.', 'a chip names, as its title'],
		['a citation past the end of its file', '[x](../../Cargo.toml#L99999 "x")', 'past the file'],
		['a likelihood on an observation', 'A <claim basis="observed" likelihood="likely">x [y](../../src/lib.rs#L250 "compose")</claim>.', 'not a prediction'],
		['a likelihood outside the vocabulary', 'A <claim basis="inferred" likelihood="probable">x</claim>.', 'closed vocabulary'],
		['a basis outside the vocabulary', 'A <claim basis="believed">x</claim>.', 'is not one of'],
		['an attribute a component does not take', 'A <claim basis="inferred" confidence="0.8">x</claim>.', 'has no attribute "confidence"'],
		['a sketch with no seed', '<sketch-figure id="s" seed="0" title="S">\n\n```text\nstate a proposed "A"\nbox x "X" 0 0 10 10\n```\n\n</sketch-figure>\n', 'positive integer'],
		['a decision with no case against it', DECISION('<recommendation choice="a">\n\nA.\n\n</recommendation>'), 'the strongest case against'],
		['a recommendation naming no choice', DECISION('<recommendation choice="z">\n\nZ.\n\n</recommendation>\n\n<against>\n\nNo.\n\n</against>'), 'names no <choice'],
		['pragmatics without its silence', '<pragmatics>\n\n<ask>\n\nA.\n\n</ask>\n\n<will>\n\nW.\n\n</will>\n\n<needs>\n\nN.\n\n</needs>\n\n<wont>\n\nX.\n\n</wont>\n\n</pragmatics>\n', 'missing <silence>'],
		['a part outside its component', '<will>\n\nW.\n\n</will>\n', 'belongs directly inside <pragmatics>'],
		['an experiment with no runs', EXPERIMENT('values="   "'), 'lists no run'],
		['an experiment with a run that is not a number', EXPERIMENT('values="0.8 n/a 0.7"'), '"n/a", which is not a number'],
		['a decision without reversible', DECISION(CASE, 'id="x" title="X"'), 'needs reversible="..."'],
		['a reversible outside its vocabulary', DECISION(CASE, 'id="x" title="X" reversible="maybe"'), 'reversible="maybe" is not one of'],
		['a decision depending on no decision on the page', DECISION(CASE, 'id="x" title="X" reversible="yes" depends="nowhere"'), 'depends="nowhere" names no <decision'],
		['a decision resting on no claim on the page', DECISION(CASE, 'id="x" title="X" reversible="yes" grounds="nowhere"'), 'grounds="nowhere" names no <claim'],
		[
			'decisions that depend on each other in a cycle',
			DECISION(CASE, 'id="x" title="X" reversible="yes" depends="y"') + '\n' + DECISION(CASE, 'id="y" title="Y" reversible="yes" depends="x"'),
			'in a cycle: x depends on y depends on x'
		],
		['a claim id used twice', 'A <claim id="c" basis="inferred">x</claim> and <claim id="c" basis="inferred">y</claim>.', 'is used twice on this page']
	];
	for (const [what, body, expect] of refusals) {
		try {
			await renderFragment(HEAD + body, { ...fixtureContext, file: `refusal: ${what}` });
			failures.push(`refusals: ${what} rendered; it must fail`);
		} catch (e) {
			const msg = (e as Error).message;
			if (!msg.includes(expect)) failures.push(`refusals: ${what} failed, but not for its reason ("${expect}"): ${msg.split('\n').slice(0, 2).join(' ')}`);
		}
	}
	// A commit that is not on this history.
	try {
		await renderFragment(HEAD.replace(PIN, 'f'.repeat(40)) + 'Text.', { ...fixtureContext, pin: 'f'.repeat(40) });
		failures.push('refusals: a commit not in this clone rendered; it must fail');
	} catch (e) {
		if (!(e as Error).message.includes('not in this clone')) failures.push(`refusals: an unknown commit failed for another reason: ${(e as Error).message}`);
	}

	console.log(
		`legibility: the fixture draws ${figures} figures and ${examined} claims, chips, assumptions and decisions; ` +
			`rendered twice with Math.random disabled; ${refusals.length + 1} planted mistakes; ${failures.length} failures`
	);
	if (failures.length > 0) {
		console.log('FAIL (legibility):');
		for (const f of failures) console.log(`  ${f}`);
		process.exit(1);
	}
	console.log('PASS (legibility): every component holds to its rules, renders the same twice, and refuses each planted mistake');
}
