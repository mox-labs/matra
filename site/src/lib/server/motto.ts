/**
 * The motto's parse, the one input the mark is drawn from. It is ordinary
 * figure data (site/src/lib/figures/motto/parse.json), so gate 8 keeps it
 * current with matra and the mark follows it.
 */
import { figures } from './content';
import { mottoTokens } from '$lib/mark';
import type { ParseFigureFile, ParseToken } from '$lib/types';

export function motto(): { tokens: ParseToken[]; file: ParseFigureFile } {
	const file = figures.get('motto/parse') as ParseFigureFile | undefined;
	if (!file) throw new Error('no site/src/lib/figures/motto/parse.json; run just docs-figures');
	return { tokens: mottoTokens(file), file };
}
