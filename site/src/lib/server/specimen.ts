/**
 * The specimen's parse, the one input the mark is drawn from. It is ordinary
 * figure data (site/src/lib/figures/specimen/parse.json), so gate 8 keeps it
 * current with matra and the mark follows it.
 */
import { figures } from './content';
import { specimenTokens } from '$lib/mark';
import type { ParseFigureFile, ParseToken } from '$lib/types';

export function specimen(): { tokens: ParseToken[]; file: ParseFigureFile } {
	const file = figures.get('specimen/parse') as ParseFigureFile | undefined;
	if (!file) throw new Error('no site/src/lib/figures/specimen/parse.json; run just docs-figures');
	return { tokens: specimenTokens(file), file };
}
