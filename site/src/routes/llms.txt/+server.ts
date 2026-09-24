import { llmsTxt } from '$lib/server/content';
import type { RequestHandler } from './$types';

/**
 * The agent-facing map of the site, at its root. The file is generated from
 * SUMMARY.md by scripts/gen-llms-txt.sh and committed; gate 6 of the docsite
 * floor fails when it is stale. This route serves it as committed.
 */
export const prerender = true;

export const GET: RequestHandler = () =>
	new Response(llmsTxt(), { headers: { 'content-type': 'text/plain; charset=utf-8' } });
