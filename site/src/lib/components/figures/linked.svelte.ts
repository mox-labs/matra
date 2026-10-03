/**
 * Linked highlighting that a mouse, a keyboard and a finger can each drive.
 *
 * A figure lights an item (a word, a row, a sentence) and whatever it is
 * linked to. A mouse lights it by pointing, as before. A keyboard lights it
 * by focus. A finger has no hover: a tap fires pointerenter and pointerleave
 * together, so a highlight bound to those flashes and is gone. So a tap pins
 * the item instead; a second tap on it, or a tap anywhere else, clears the
 * pin.
 *
 * The three are kept apart and read in order: what the mouse points at, then
 * what has keyboard focus, then what a tap pinned.
 *
 *   mouse     pointerenter and pointerleave, from a mouse only (`pointerType`),
 *             so hover behaves exactly as it did
 *   keyboard  focus that is `:focus-visible`. A tap or a click on a focusable
 *             row also focuses it, and that focus is not visible, so it does
 *             not light the row a second time and outlast the pin
 *   touch     pointerup from a touch or a pen pins and unpins, on the same
 *             event, so the highlight answers within the frame of the tap
 *             rather than after the compatibility click. A pan or a scroll
 *             cancels the pointer and fires no pointerup, so dragging a
 *             figure sideways never pins anything
 *
 * `clickPins` is for items that are buttons and already pin on click (the
 * home page's sentence): there the click pins for every input, a tap
 * included, and only the clearing tap elsewhere is added.
 *
 * Nothing here animates. The change is a class, and the figure's own CSS
 * fades it with a transition the reduced-motion rule in app.css removes.
 */
export class Linked<T extends string | number> {
	#hover = $state<T | null>(null);
	#focus = $state<T | null>(null);
	/** The item a tap (or, with `clickPins`, a click) pinned. */
	pinned = $state<T | null>(null);
	/** The item lit now: pointed at, else focused, else pinned. */
	active = $derived(this.#hover ?? this.#focus ?? this.pinned);

	#clickPins: boolean;
	/** The event that set the pin, so the clearing listener lets it pass. */
	#last: Event | null = null;

	constructor({ clickPins = false }: { clickPins?: boolean } = {}) {
		this.#clickPins = clickPins;
		// While something is pinned, a tap anywhere else clears it.
		$effect(() => {
			if (this.pinned === null) return;
			let touch = false;
			const down = (e: PointerEvent) => (touch = e.pointerType !== 'mouse');
			const away = (e: Event) => {
				const byTouch = e instanceof PointerEvent && e.type === 'pointerup' ? e.pointerType !== 'mouse' : touch;
				if (byTouch && e !== this.#last) this.pinned = null;
			};
			const on = this.#clickPins ? 'click' : 'pointerup';
			document.addEventListener('pointerdown', down, true);
			document.addEventListener(on, away);
			return () => {
				document.removeEventListener('pointerdown', down, true);
				document.removeEventListener(on, away);
			};
		});
	}

	toggle(v: T, e?: Event) {
		this.#last = e ?? null;
		this.pinned = this.pinned === v ? null : v;
	}

	/** The handlers for one item, spread onto its element. */
	on(v: T) {
		return {
			onpointerenter: (e: PointerEvent) => {
				if (e.pointerType === 'mouse') this.#hover = v;
			},
			onpointerleave: (e: PointerEvent) => {
				if (e.pointerType === 'mouse' && this.#hover === v) this.#hover = null;
			},
			onpointerup: (e: PointerEvent) => {
				if (!this.#clickPins && e.pointerType !== 'mouse') this.toggle(v, e);
			},
			onclick: (e: MouseEvent) => {
				if (this.#clickPins) this.toggle(v, e);
			},
			onfocus: (e: FocusEvent) => {
				if ((e.currentTarget as Element).matches(':focus-visible')) this.#focus = v;
			},
			onblur: () => {
				if (this.#focus === v) this.#focus = null;
			}
		};
	}
}
