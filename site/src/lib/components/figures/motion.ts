/**
 * EP-0012's Motion rule, in one place for every figure: a change the reader
 * caused runs in two stages, the old state out and then the new one in, about
 * 0.6s in all, easing in and out. Under `prefers-reduced-motion` both stages
 * take no time, so the new state appears at once. Nothing calls these on
 * page load: a figure's first state is its prerendered one.
 */
import { cubicInOut } from 'svelte/easing';

export function reducedMotion(): boolean {
	return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** The first stage: the state being replaced fades out. */
export function leave() {
	return { duration: reducedMotion() ? 0 : 220, easing: cubicInOut };
}

/** The second stage: the new state fades in once the old one has gone. */
export function enter() {
	const reduced = reducedMotion();
	return { duration: reduced ? 0 : 360, delay: reduced ? 0 : 220, easing: cubicInOut };
}
