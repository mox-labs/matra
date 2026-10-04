/**
 * The two Blueprints behaviours that need a script, as progressive
 * enhancements over HTML that already reads in full without one
 * (src/lib/server/markdown/legibility.ts draws it; site/README.md, "The
 * Blueprints components", says why).
 *
 *   chip     a code-line chip is a link to the lines on GitHub at the
 *            record's commit. With a script, a tap or a click opens its
 *            sheet instead: the quoted lines, here, in a popover that a tap
 *            outside, Escape or its close button dismisses, and focus goes
 *            back to the chip. A modified click (a new tab) still follows
 *            the link.
 *   sketch   a sketch figure with two states shows both, one after the
 *            other, without a script. With one, the states share a frame and
 *            a toggle beside the figure swaps them: one eased stage, which a
 *            second press reverses mid-flight (a CSS transition retargets),
 *            and which jumps under reduced motion (app.css).
 *
 * Listeners are delegated from the document, so content that arrives after
 * a client-side navigation is covered without rebinding.
 */

function onClick(e: MouseEvent) {
	const target = e.target as Element | null;
	const chip = target?.closest<HTMLAnchorElement>('a.chip[data-sheet]');
	if (chip) {
		if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
		const sheet = document.getElementById(chip.dataset.sheet ?? '');
		if (!sheet || typeof sheet.showPopover !== 'function') return;
		e.preventDefault();
		if (sheet.matches(':popover-open')) {
			sheet.hidePopover();
			return;
		}
		sheet.showPopover();
		opener.set(sheet, chip);
		sheet.querySelector<HTMLElement>('.sheet-close')?.focus({ preventScroll: true });
		return;
	}
	const button = target?.closest<HTMLButtonElement>('.sk-toggle button[data-show]');
	if (button) {
		const figure = button.closest<HTMLElement>('figure.sketch');
		if (figure) show(figure, button.dataset.show ?? '');
	}
}

/** The chip that opened each sheet, to hand focus back when it closes. */
const opener = new WeakMap<Element, HTMLElement>();

function onToggle(e: Event) {
	const sheet = e.target as HTMLElement;
	if (!(sheet instanceof HTMLElement) || !sheet.classList.contains('sheet')) return;
	if ((e as ToggleEvent).newState === 'closed') {
		const chip = opener.get(sheet);
		opener.delete(sheet);
		// Only when focus was in the sheet: a tap elsewhere keeps its own target.
		if (chip && (document.activeElement === document.body || sheet.contains(document.activeElement))) chip.focus({ preventScroll: true });
	}
}

function show(figure: HTMLElement, state: string) {
	for (const layer of figure.querySelectorAll<HTMLElement>('.sk-state')) {
		if (layer.dataset.state === state) layer.setAttribute('data-current', '');
		else layer.removeAttribute('data-current');
	}
	for (const b of figure.querySelectorAll<HTMLButtonElement>('.sk-toggle button')) {
		b.setAttribute('aria-pressed', String(b.dataset.show === state));
	}
}

/** Turns on the toggles of every sketch figure on the page that has not been yet. */
export function enhanceSketches() {
	for (const figure of document.querySelectorAll<HTMLElement>('figure.sketch:not([data-enhanced])')) {
		const toggle = figure.querySelector<HTMLElement>('.sk-toggle');
		if (!toggle) continue;
		toggle.hidden = false;
		figure.setAttribute('data-enhanced', '');
		const current = figure.querySelector<HTMLElement>('.sk-state[data-current]')?.dataset.state;
		if (current) show(figure, current);
	}
}

/** Installs the delegated listeners once; returns their removal. */
export function installLegibility(): () => void {
	document.addEventListener('click', onClick);
	document.addEventListener('toggle', onToggle, true);
	enhanceSketches();
	return () => {
		document.removeEventListener('click', onClick);
		document.removeEventListener('toggle', onToggle, true);
	};
}
