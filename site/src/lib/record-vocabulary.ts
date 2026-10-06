/**
 * The words and glyphs of the collaborate stance: what a decision's state,
 * its reversibility and a record's place in the dossier anatomy are called,
 * and how each is marked. One list, read by the build (legibility.ts, which
 * draws the decision blocks and the queue), by the layout (the record frame
 * and the record navigator) and by gate 9, so the same state never has two
 * spellings. Nothing here imports from the server, so a Svelte component can
 * read it. site/README.md, "The collaborate stance", has the reasoning.
 *
 * Colour is a role and never the only signal: every glyph here goes with its
 * word. Spark marks what awaits the owner's touch (an open decision, an
 * objection or a redirect, an assumption to confirm); Emergence a decision
 * the owner accepted. Reversibility is neutral ink: it is a property of the
 * decision, not a state anyone acts on.
 */

/** A decision's state: open until the record carries a ruling, then the ruling's response. */
export const DECISION_STATES = {
	open: { glyph: '●', word: 'open', role: 'spark', settled: false },
	accept: { glyph: '✓', word: 'accepted', role: 'emergence', settled: true },
	'accept-with-reservation': { glyph: '✓', word: 'accepted with a reservation', role: 'emergence', settled: true },
	object: { glyph: '●', word: 'objected', role: 'spark', settled: false },
	redirect: { glyph: '●', word: 'redirected', role: 'spark', settled: false }
} as const;
export type DecisionState = keyof typeof DECISION_STATES;

/**
 * How readily the recommended choice can be undone once carried out. The
 * square fills as undoing it costs more, as a claim's circle fills as its
 * grounds firm up.
 */
export const REVERSIBLE = {
	yes: { glyph: '□', word: 'reversible' },
	costly: { glyph: '◧', word: 'costly to reverse' },
	no: { glyph: '■', word: 'irreversible' }
} as const;
export type Reversible = keyof typeof REVERSIBLE;

/** An assumption's mark: the assumed basis, which awaits the owner's touch. */
export const ASSUMED = { glyph: '○', word: 'assumed' } as const;

/**
 * The dossier anatomy, mapped onto the Rust template's sections without
 * reordering them: orientation (the brief), the case, the queue of
 * decisions, the references, and the margin where comments land. A section
 * the anatomy has no slot for (Future possibilities) carries no role.
 */
export const ANATOMY: Readonly<Record<string, 'orientation' | 'case' | 'queue' | 'references'>> = {
	Summary: 'orientation',
	Motivation: 'case',
	'Guide-level explanation': 'case',
	'Reference-level explanation': 'case',
	Drawbacks: 'case',
	'Rationale and alternatives': 'case',
	'Prior art': 'references',
	'Unresolved questions': 'queue'
};

/**
 * A record's status as a glyph beside its word, read from the status's
 * first word (`superseded by` a later record is superseded). Neutral ink: the
 * status is the record's, and what awaits the owner is marked by the tallies.
 */
export function statusGlyph(status: string): string {
	const first = status.split(/\s+/)[0];
	return { proposed: '○', planned: '○', 'in': '◐', accepted: '●', implemented: '✓', shipped: '✓', superseded: '→', dropped: '×' }[first] ?? '○';
}

/** A record's kind, as its badge names it. */
export const KIND_LABEL = { EPR: 'Enhancement proposal', EPL: 'Enhancement plan' } as const;

/** One decision as the frame, the navigator and the queue show it. */
export interface DecisionView {
	/** The element id on the record's page, `decision-<id>`. */
	id: string;
	/** Its place in the record, 1-based, in document order. */
	n: number;
	title: string;
	state: DecisionState;
	reversible: Reversible;
	/** The element ids of the decisions it depends on. */
	depends: string[];
	/** The element ids of the claims it rests on. */
	grounds: string[];
}

/** One assumed claim as the navigator and the queue show it. */
export interface AssumptionView {
	/** The claim's element id on the record's page. */
	id: string;
	text: string;
	/** The element ids of the decisions that rest on it. */
	decisions: string[];
}

/**
 * The decisions in dependency order: each after every decision it depends
 * on, and otherwise in document order. The build has already refused a
 * cycle, so every decision is placed.
 */
export function dependencyOrder<T extends { id: string; depends: string[] }>(decisions: readonly T[]): T[] {
	const placed = new Set<string>();
	const out: T[] = [];
	while (out.length < decisions.length) {
		const next = decisions.find((d) => !placed.has(d.id) && d.depends.every((x) => placed.has(x)));
		if (!next) break;
		placed.add(next.id);
		out.push(next);
	}
	return out;
}

/** The first words of a text, for a line of the navigator or the queue. */
export function firstWords(text: string, max = 72): string {
	if (text.length <= max) return text;
	const cut = text.slice(0, max);
	return `${cut.slice(0, Math.max(cut.lastIndexOf(' '), max / 2)).replace(/[,;:.]$/, '')}…`;
}
