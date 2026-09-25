/**
 * The void or paper. The inline script in app.html sets `data-theme` on
 * <html> before the first paint; this module reads it back after hydration
 * and owns every change from then on. The void is the default (the system is
 * dark-first); paper is the reader's choice, remembered.
 */
export type Theme = 'light' | 'dark';

const KEY = 'matra-theme';

export const theme = $state<{ current: Theme }>({ current: 'dark' });

/** Read the theme the page was painted with. */
export function initTheme(): void {
	theme.current = document.documentElement.dataset.theme === 'light' ? 'light' : 'dark';
}

/** The reader's explicit choice, remembered across visits. */
export function chooseTheme(next: Theme): void {
	try {
		localStorage.setItem(KEY, next);
	} catch {
		// Storage can be disabled; the choice then lasts for this page only.
	}
	theme.current = next;
	document.documentElement.dataset.theme = next;
}
