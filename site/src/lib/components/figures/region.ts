/**
 * The name of a figure's scrolling regions, made unique where one document
 * holds several pages.
 *
 * A region is a landmark, and a landmark's role and name have to be unique on
 * the document (axe-core's `landmark-unique`), so a screen reader's list of
 * landmarks tells them apart. On a docsite page they are: each figure names
 * its regions after what they hold. On /print every page shares one document,
 * and a figure that appears on two pages appears twice, so its regions'
 * names repeat. The print page sets a scope for each page's body, and a
 * figure adds that scope to the names of its regions; anywhere else no scope
 * is set and the names are unchanged.
 */
import { getContext, setContext } from 'svelte';

const KEY = Symbol('figure-region-scope');

/** Scope the regions of every figure rendered below this component. */
export function setRegionScope(scope: string): void {
	setContext(KEY, scope);
}

/**
 * A function from a region's name to its name in scope. Call it while the
 * component initialises, where Svelte allows `getContext`.
 */
export function scopedRegion(): (name: string) => string {
	const scope = getContext<string | undefined>(KEY);
	return scope ? (name) => `${name}, on ${scope}` : (name) => name;
}
