/**
 * Anchoring a comment to the text it quotes, in the browser.
 *
 * The page's text is read from the DOM with whitespace collapsed to single
 * spaces, so a paragraph rewrapped in the Markdown, or a line break the
 * renderer moved, does not move the quote. Each character of that text keeps
 * the text node and offset it came from, so a quote found in the text maps
 * back to a DOM Range.
 *
 * Writing a selector: the selection's range, read in that text, gives `exact`
 * and the `AFFIX` characters either side as `prefix` and `suffix` (the W3C
 * TextQuoteSelector), and the nearest heading before it.
 *
 * Re-anchoring: every place `exact` occurs is scored by how much of `prefix`
 * ends right before it and how much of `suffix` starts right after it, with a
 * bonus under the same heading; the best wins. A quote that no longer occurs
 * anywhere does not anchor, and its thread is listed as orphaned, never
 * dropped.
 */
import { AFFIX, type Selector } from './model';

/** Not part of the text a reader comments on: apparatus the page sets beside it. */
const SKIP =
	'.measure, .measures-toggle, .heading-anchor, button, .diagram-hint, script, style, [data-dev-comments]';

interface Index {
	text: string;
	nodes: Text[];
	offsets: number[];
}

export function textIndex(root: Element): Index {
	const nodes: Text[] = [];
	const offsets: number[] = [];
	let text = '';
	let space = true;
	const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
		acceptNode: (n) =>
			n.parentElement?.closest(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT
	});
	for (let n = walker.nextNode() as Text | null; n; n = walker.nextNode() as Text | null) {
		const v = n.data;
		for (let i = 0; i < v.length; i++) {
			const ws = /\s/.test(v[i]);
			if (ws && space) continue;
			text += ws ? ' ' : v[i];
			nodes.push(n);
			offsets.push(i);
			space = ws;
		}
	}
	return { text, nodes, offsets };
}

/** The selector for a range, or null when it holds no text of the page. */
export function describe(root: Element, range: Range): Selector | null {
	const idx = textIndex(root);
	let start = -1;
	let end = -1;
	for (let i = 0; i < idx.text.length; i++) {
		// A character is in the range when its start point is, and the point
		// after it is too.
		const inside =
			range.isPointInRange(idx.nodes[i], idx.offsets[i]) &&
			range.comparePoint(idx.nodes[i], idx.offsets[i] + 1) <= 0;
		if (inside) {
			if (start === -1) start = i;
			end = i + 1;
		} else if (start !== -1) break;
	}
	if (start === -1) return null;
	while (start < end && idx.text[start] === ' ') start++;
	while (end > start && idx.text[end - 1] === ' ') end--;
	if (start === end) return null;
	return {
		type: 'TextQuoteSelector',
		exact: idx.text.slice(start, end),
		prefix: idx.text.slice(Math.max(0, start - AFFIX), start),
		suffix: idx.text.slice(end, end + AFFIX),
		heading: headingBefore(root, idx.nodes[start])
	};
}

function headingBefore(root: Element, node: Node): string | null {
	let found: string | null = null;
	for (const h of root.querySelectorAll('h1[id], h2[id], h3[id], h4[id]')) {
		if (h.compareDocumentPosition(node) & Node.DOCUMENT_POSITION_FOLLOWING) found = h.id;
		else break;
	}
	return found;
}

/** Characters two strings share at the end of `a` and the start of `b`. */
function sharedTail(a: string, b: string): number {
	let n = 0;
	while (n < a.length && n < b.length && a[a.length - 1 - n] === b[b.length - 1 - n]) n++;
	return n;
}
function sharedHead(a: string, b: string): number {
	let n = 0;
	while (n < a.length && n < b.length && a[n] === b[n]) n++;
	return n;
}

/** Find the quote in the page as it is now: a Range, or null when it no longer occurs. */
export function anchor(root: Element, sel: Selector, idx: Index = textIndex(root)): Range | null {
	let best = -1;
	let bestScore = -1;
	for (let at = idx.text.indexOf(sel.exact); at !== -1; at = idx.text.indexOf(sel.exact, at + 1)) {
		const end = at + sel.exact.length;
		let score =
			sharedTail(idx.text.slice(Math.max(0, at - sel.prefix.length), at), sel.prefix) +
			sharedHead(idx.text.slice(end, end + sel.suffix.length), sel.suffix);
		if (sel.heading && headingBefore(root, idx.nodes[at]) === sel.heading) score += AFFIX / 2;
		if (score > bestScore) {
			best = at;
			bestScore = score;
		}
	}
	if (best === -1) return null;
	const last = best + sel.exact.length - 1;
	const range = document.createRange();
	range.setStart(idx.nodes[best], idx.offsets[best]);
	range.setEnd(idx.nodes[last], idx.offsets[last] + 1);
	return range;
}
