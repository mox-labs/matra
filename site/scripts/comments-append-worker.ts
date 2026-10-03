/**
 * One writer for scripts/test-comments-store.ts: waits until a shared start
 * time, so two of these contend for the lock, then appends one reply.
 *
 * Usage: bun scripts/comments-append-worker.ts <repo-root> <route> <thread> <start-ms> <body> <judged-pause-ms>
 */
import { Store } from '../src/lib/dev/comments/store.ts';

const [root, route, thread, start, body, judged] = process.argv.slice(2);
// Widen the race windows (store.ts, testPause): this writer reads the file and
// holds it a while, and the other judges the planted lock stale late, so a
// takeover that removes a live lock lets both writers in and loses a line.
process.env.MATRA_COMMENTS_TEST_HOLD_MS = '40';
process.env.MATRA_COMMENTS_TEST_JUDGED_MS = judged;
while (Date.now() < Number(start)) {
	// Spin to the shared start: a sleep's granularity would stagger the writers.
}
new Store(root).reply(route, 'claude', thread, body);
