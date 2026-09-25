/**
 * Motion for the figures, in one place, from the motion research
 * (motion-for-data-graphics, F1 to F10) and EP-0012's rule:
 *
 *   F1  animate a change the reader caused, never a mark at rest; nothing
 *       here runs on page load, since a figure's first state is prerendered
 *   F2  interruptible: a new input retargets a transition mid-flight and
 *       never queues. `swap` is used as a bidirectional Svelte transition,
 *       which reverses from wherever it is; state that lives on persistent
 *       elements uses CSS transitions, which retarget by themselves
 *   F3  a transition between two states eases in and out; a continuous
 *       change the reader drags (a scrub) is not eased at all
 *   F5  one stage, well under a second
 *
 * Under `prefers-reduced-motion` the duration is zero, so states jump.
 */
import { cubicInOut } from 'svelte/easing';

export function reducedMotion(): boolean {
	return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** One stage, old and new crossing: for a swapped state (another sentence, another stage). */
export function swap() {
	return { duration: reducedMotion() ? 0 : 240, easing: cubicInOut };
}

/**
 * Rows the keyboard can walk, as the pointer does, with one tab stop: the
 * first row (or the last focused) takes focus, and the arrow keys, Home and
 * End move it. Each row needs `data-row`; what focusing a row does is the
 * row's own onfocus.
 */
export function roving(node: HTMLElement) {
	const rows = () => [...node.querySelectorAll<HTMLElement>('[data-row]')];
	const init = () => rows().forEach((r, i) => (r.tabIndex = i === 0 ? 0 : -1));
	init();
	const observer = new MutationObserver(init);
	observer.observe(node, { childList: true, subtree: true });
	const onkeydown = (e: KeyboardEvent) => {
		const list = rows();
		const at = list.indexOf(document.activeElement as HTMLElement);
		if (at === -1) return;
		const to =
			e.key === 'ArrowDown'
				? Math.min(list.length - 1, at + 1)
				: e.key === 'ArrowUp'
					? Math.max(0, at - 1)
					: e.key === 'Home'
						? 0
						: e.key === 'End'
							? list.length - 1
							: null;
		if (to === null) return;
		e.preventDefault();
		list[at].tabIndex = -1;
		list[to].tabIndex = 0;
		list[to].focus();
	};
	node.addEventListener('keydown', onkeydown);
	return () => {
		observer.disconnect();
		node.removeEventListener('keydown', onkeydown);
	};
}
