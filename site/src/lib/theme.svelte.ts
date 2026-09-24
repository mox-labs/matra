/**
 * Light or dark. The inline script in app.html sets `data-theme` on <html>
 * before the first paint; this module reads it back after hydration and owns
 * every change from then on.
 */
export type Theme = 'light' | 'dark';

const KEY = 'matra-theme';

export const theme = $state<{ current: Theme }>({ current: 'light' });

/** Read the theme the page was painted with, and follow the system until the reader chooses. */
export function initTheme(): () => void {
	theme.current = document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light';
	const media = matchMedia('(prefers-color-scheme: dark)');
	const follow = (e: MediaQueryListEvent) => {
		if (stored() === null) apply(e.matches ? 'dark' : 'light');
	};
	media.addEventListener('change', follow);
	return () => media.removeEventListener('change', follow);
}

/** The reader's explicit choice, remembered across visits. */
export function chooseTheme(next: Theme): void {
	try {
		localStorage.setItem(KEY, next);
	} catch {
		// Storage can be disabled; the choice then lasts for this page only.
	}
	apply(next);
}

function apply(next: Theme): void {
	theme.current = next;
	document.documentElement.dataset.theme = next;
}

function stored(): string | null {
	try {
		return localStorage.getItem(KEY);
	} catch {
		return null;
	}
}
