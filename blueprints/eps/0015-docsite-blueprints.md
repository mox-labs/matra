# EP-0015: Blueprints on the docsite, and local comments to converge on them

- EP: EP-0015
- Implements: none (standalone: this changes how matra's design is documented and discussed, not matra)
- Start Date: 2026-10-03
- EP PR: [#130](https://github.com/mox-labs/matra/pull/130)
- Status: in progress
- Shipped in: not shipped

## Summary

The docsite renders `blueprints/` as a Blueprints part: the process and
index, every RFC and every EP, read in place from the files the process
edits, each with its status. In the dev server, and only there, any page
can be commented on: select text, comment, reply, resolve. The comments are
files in the repository, one per page, so a later session reads them and
Claude answers in them from the command line. The owner and Claude converge
on a design record where it is read, and the record of that convergence is
committed beside it. The EP template moves to the Rust RFC layout, as the RFC
template already has.

## Motivation

The owner reviews design records, and until now the place to read one was
GitHub's file view or an editor, and the place to discuss it was a pull
request, a chat transcript, or nowhere. A remark on a paragraph of an
accepted RFC has no pull request to land on. A remark made in a session is
lost when the session ends unless someone copies it into a file, and the
next session does not know it was made. The docsite already renders
Markdown with the project's typography and navigation, so it is the natural
place to read a record; what it lacked was the records themselves, and a
way to answer a paragraph in place.

Three owner decisions (2026-10-03) shape this:

- The docsite renders `blueprints/` as a Blueprints part, public and
  read-only on GitHub Pages, and that part is the owner and Claude's
  convergence surface.
- The product pages keep their rule: they describe what ships, with no
  status markers. Statuses appear only in the Blueprints part.
- Local comments live in the dev server only, as committed files; giscus
  stays the public channel. EPs use the Rust RFC template, as RFCs do.

The EP template change is part of the same move. An EP and an RFC are read
side by side on the site now, and the RFC's layout (a teaching explanation,
then a technical one, then the reasons against) is the one a reviewer
already knows. The earlier EP layout (Goals, Non-goals, milestones) kept the
plan but had nowhere to put a standalone EP's design except a free-form
Design section.

## Guide-level explanation

**Reading a record.** The navigation has a part after Reference, titled
Blueprints and marked as design records. Under it are the process and
index (`/blueprints/`), the RFCs and the EPs, each group folding open on the
record being read, and each record with its status in words beside it
(`accepted`, `implemented`, `superseded by RFC-0019`, `shipped in 0.2.0`,
`in progress`). Under a record's title is the same status: "design record,
RFC: implemented". A link to another record stays on the site; a link to
code (`../../src/lib.rs`) opens the file on GitHub. Every record is also
served as its Markdown source (`/blueprints/rfcs/0007-one-pipeline.md`), and
`llms.txt` lists them under a Blueprints heading.

**Commenting, as the owner.** Run `just docs-serve` and open any page, a
record or not. Select a phrase and a "Comment" button appears under it;
write the comment and post it. A marker with the thread's message count
sits in the margin level with the quote, which is lit while the marker is
pointed at, focused or tapped. The marker opens the thread: the messages,
a reply box, and Resolve (Reopen once resolved). "Comments (n open)" in the
corner lists every thread on the page, including any whose quote has since
been edited away, and it reaches everything from the keyboard, including
commenting on the current selection.

**Answering, as Claude.**

```bash
just comments                 # every open thread: page, quote, last message
just comments --all           # resolved ones too
just comment-reply /blueprints/eps/0015-docsite-blueprints 1a2b3c4d "Done in M2."
```

The reply appears in the open page without a reload. Comments are files
under `discussion/`, committed like anything else; reading them needs no
server.

**Publishing.** `just docs-build` and the deploy build none of this: no
endpoint, no marker, no panel. The build fails if any of it gets in.

## Reference-level explanation

### The Blueprints part

`site/src/lib/server/blueprints.ts` globs `blueprints/**/*.md` from the
repository root at build time (Vite `import.meta.glob`, with `blueprints/`
added to the dev server's `fs.allow`), so the files are read where they
are. Each becomes a page:

| File | Route | Served as |
|---|---|---|
| `blueprints/README.md` | `/blueprints/index` | `blueprints/index.html`, which Pages serves at `/blueprints/` |
| `blueprints/rfcs/<name>.md` | `/blueprints/rfcs/<name>` | `.html` and `.md` |
| `blueprints/eps/<name>.md` | `/blueprints/eps/<name>` | `.html` and `.md` |

A record's status is its header's `- Status:` line with links reduced to
their text and any parenthetical dropped; a record without one fails the
build. The templates are listed last in their group, with the status
`template`. The part is appended to the navigation after the `SUMMARY.md`
parts, not listed in `SUMMARY.md`, so gate 2 (every page under
`site/content/` is in `SUMMARY.md`) and the product half of `llms.txt` stay
about the product pages. The records have their own previous and next
chain; the last product page does not lead into them.

Links resolve in the repository's tree: every page, product or record, is
keyed by its path from the repository root, so a relative link that works
on GitHub works on the site. In a record, a relative link that resolves to
no page but to a file that exists in the repository becomes that file's
GitHub URL on `main`, fragment kept; one that resolves to nothing fails the
build. Product pages keep their rule that a local link names a `.md` page.

The records are not measured (the self-measuring margin is for pages under
`site/content/`), and gate 3 does not read them: it holds backticked type
names to `src/`, and a record names types that do not exist yet or no longer
exist. The exemption is named in the gate (`gate3_exempt`) with that reason.
Every other gate covers them: the em-dash gate already read `blueprints/`;
lychee reads the built HTML, and offline the records' Markdown too; every
`.html` and `.md` path is in `site/urls.txt`; each record's section anchors
are in `site/anchors.txt`; gate 12 loads every page in `urls.txt` at four
widths; and `scripts/gen-llms-txt.sh` writes a Blueprints section, reading
each record's first paragraph whole although it is hard-wrapped.

### Local comments

**Dev only, by construction.** The endpoint is a Vite plugin
(`site/src/lib/dev/comments/plugin.ts`) with `apply: 'serve'`, so `vite
build` never runs it. The layout imports the UI
(`site/src/lib/dev/comments/DevComments.svelte`) inside `if
(import.meta.env.DEV)`, which a build replaces with `false`, so the import is
dead code and is dropped. `site/scripts/check-no-dev-comments.ts` runs in
`bun run build` (so in gate 4 and in `docs.yml`) and fails when any emitted
script or stylesheet holds the endpoint's path, the UI's attribute or its
highlight names, or any built page has an element with that attribute. It
reads code and elements, not page text, because this record's own prose
names the endpoint.

**Storage.** `discussion/<route>.jsonl` at the repository root, one file per
page, one JSON object per line, appended and never rewritten. A comment:

```json
{"id":"3f9c2a1b","thread":"3f9c2a1b","parent":null,"author":"owner","created":"2026-10-03T14:02:11.120Z","page":"/blueprints/eps/0015-docsite-blueprints","selector":{"type":"TextQuoteSelector","exact":"the record of that convergence","prefix":"gn record where it is read, and ","suffix":" is committed beside it. The EP ","heading":"summary"},"body":"Say where the files live."}
```

A reply has the thread's id as `thread`, the message it answers as
`parent`, and `selector` null. A status event is a line of its own,
`{"id", "thread", "parent": <thread>, "author", "created", "page",
"status": "resolved" | "open"}`; the last one wins. `author` is `owner` or
`claude`: the endpoint writes `owner` whatever the request says, and the
script writes `claude`. `site/src/lib/dev/comments/model.ts` checks every
field of every line it reads or writes, and a line that fails is reported
with its number, never dropped.

**Writes.** `site/src/lib/dev/comments/store.ts` serves both writers. A write
takes a lock file with O_EXCL, reads the file, writes the file plus the new
line to a temporary file beside it, flushes it and renames it over the
original, so a reader sees the file before or after the line, never part
of it. A lock older than ten seconds is a dead writer's and is taken over,
but only by the holder of a second O_EXCL guard file, who looks at the lock
again under the guard, so two waiters that both saw it stale cannot both
remove it and let two writers in. `site/scripts/test-comments-store.ts`
(in `bun run check`) runs two writer processes against a planted stale lock
50 times and fails on any lost or torn line; with the guard removed it loses
lines. The route is checked against the shape of a site route (lowercase
segments, no `.` or `..`, at most 200 characters), the file is resolved and
held inside `discussion/`, and the deepest existing part of the path must
resolve inside it too, so a symlink cannot lead out. A body is at most 8,000
characters, a quote 2,000, its context 64 either side; control characters
other than tab and newline are refused; a page's file stops at 4 MiB.

**The endpoint.** `GET /__comments?page=<route>` returns the page's threads;
`POST /__comments` takes `{op: "comment", page, selector, body}`,
`{op: "reply", page, thread, body}` or `{op: "status", page, thread,
status}` and returns the threads after it. The dev server binds `localhost`.
A request must come from a loopback address with a loopback `Host` (so a
DNS-rebinding page cannot reach it); a POST must carry this server's own
`Origin`, `Sec-Fetch-Site: same-origin` where the browser sends it, and
`Content-Type: application/json`, which a page elsewhere cannot send
without a CORS preflight the server never answers; and its body is at most
64 KiB. The plugin watches `discussion/` and tells the page over Vite's
socket when its file changes, from either writer.

**Anchoring.** The selector is the W3C Web Annotation TextQuoteSelector
(`exact`, `prefix`, `suffix`, 32 characters of context each side) plus the id
of the nearest heading before the quote. `site/src/lib/dev/comments/anchor.ts`
reads the page's text from the DOM with whitespace collapsed (so rewrapping
the Markdown moves nothing), leaving out the margin notes, heading anchors
and buttons, and keeps each character's text node and offset. On every
render, and whenever the page's DOM changes, each thread's `exact` is found
again: every occurrence is scored by how much of `prefix` ends just before
it and how much of `suffix` starts just after it, with a bonus under the
same heading, and the best wins. A quote that no longer occurs leaves its
thread orphaned: listed in the panel, marked so, with its messages and its
reply box, never dropped. Quotes are lit with the CSS Custom Highlight API,
so the page's DOM is not touched.

**The UI.** Bodies are set as text through Svelte's escaping, with line
breaks kept by CSS; nothing in a comment is read as HTML. The controls use
the site's tokens (Spark for what the reader points at), and nothing
animates. A marker is a 28px button, past the 24px target minimum, placed
inside the window at every width; a popover is a sheet along the bottom
below 40rem. A marker and a panel row light their quote through the
figures' `Linked` (`site/src/lib/components/figures/linked.svelte.ts`): a
mouse by pointing, a keyboard by focus, a finger by a tap that pins, so a
tap does not flash and vanish. Escape closes a popover and returns focus
to the control that opened it.

**Claude's side.** `site/scripts/comments.ts`, run by `just comments` and
`just comment-reply`, reads and writes through the same store. It does not
decide whether a thread still anchors; only the rendered page can. A
comment may name an RFC or EP number that does not exist yet, so
`scripts/check-blueprint-refs.sh` leaves `discussion/` out.

### The EP template

`blueprints/eps/0000-template.md` takes the Rust RFC sections: Summary,
Motivation, Guide-level explanation, Reference-level explanation, Drawbacks,
Rationale and alternatives, Prior art, Unresolved questions, Future
possibilities. The plan is subsections of the reference-level explanation:
milestones and iterations, test plan, ship criteria, risks. The header keeps
`EP`, `Implements` (an RFC, or `none`), `Status` and `Shipped in`, and adds
`Start Date` and `EP PR` as the RFC header has them. A Status log follows
as the one appendix. `blueprints/README.md` says so, and says that EP-0007
to EP-0014 are records and keep their layout. RFC-0019, which described the
earlier layout and kept the records off the docsite, has a dated note
saying both changed and pointing here.

### Milestones and iterations

#### M1: the Blueprints part

- **Deliverable.** `blueprints.ts`, the navigation part with statuses, the
  record line under a record's title, link rewriting, `toml` highlighting
  for the one record that needs it, the `urls.txt`, `anchors.txt` and
  `llms.txt` entries, gate 3's named exemption and lychee over the records.
- **Exit criterion.** `just docs-floor` passes with every record built, its
  internal links on the site and its repository links on GitHub.

#### M2: local comments

- **Deliverable.** The model, store, plugin, UI, script and `just` recipes,
  `check-no-dev-comments.ts` in the build, and the Local comments section of
  `site/README.md`.
- **Exit criterion.** In the dev server, a comment posted on a record
  re-renders, re-anchors after an edit elsewhere on the page, takes a reply
  from `just comment-reply` that appears without a reload, and resolves;
  and the check fails a build into which the UI was planted.

#### M3: the EP template

- **Deliverable.** The template, the README's process text, RFC-0019's
  dated note, and this EP written in the new layout.
- **Exit criterion.** This file follows the template section for section,
  and `scripts/check-blueprint-refs.sh` passes.

#### M4: in use

- **Deliverable.** The owner reviews at least one record through local
  comments, and Claude answers there; any friction found is fixed or
  recorded here.
- **Exit criterion.** A resolved thread from that review is committed on
  `main`, and the docsite deploy that follows passes, with the dev-only
  check, the URL manifest and the live URL check.

### Test plan

Every gate in `just docs-floor`, including gate 12 over the record pages
and gate 4's build with the dev-only check; `just check`;
`scripts/check-blueprint-refs.sh`. The dev-only check is shown failing on a
build with the UI planted. The comment round trip (post, re-render,
re-anchor after an edit elsewhere on the page, a reply from the script,
resolve) is run in the dev server with a real browser, and screenshots at
1280 and 390 pixels of a record with an open thread are attached to the
pull request's report.

### Ship criteria

M1 to M4 are done: the Blueprints part is live on GitHub Pages with every
gate passing, the local comments have carried one real review to a resolved
thread on `main`, and the deployed site holds none of their code.

### Risks

- **The UI ships.** A refactor of the layout drops the `import.meta.env.DEV`
  guard. Signal: the build fails in `check-no-dev-comments.ts`. Response:
  restore the guard; the check is the point.
- **Quotes drift.** A record is edited and threads orphan. Signal: the panel
  marks them. Response: the thread is kept and answered as it is; an
  orphaned, resolved thread is history, not debt.
- **Comment files grow.** Signal: a page nears 4 MiB. Response: none is
  likely to; if one is, the history is in git, and the file can be
  archived under a new name by hand.
- **Records become a second product docs.** Signal: a product page links a
  record for how something works today. Response: the product page states
  it; records stay the history of why.

## Drawbacks

- Design records, including superseded ones, are now one click from the
  product pages. The part's label, the status beside every record, and the
  separate reading order are the defence; a reader can still land on a
  superseded RFC from search.
- The repository gains `discussion/`, which will hold conversation, not
  only conclusions. That is the point, and it is also noise in `git log`.
- The dev server now writes files. It is held to loopback and to
  `discussion/`, but it is a writer that did not exist before.

## Rationale and alternatives

- **Render in place, not copy.** Copying records into `site/content/` would
  make two sources, one of which drifts. Rendering from `blueprints/` keeps
  the file the process edits and the page the owner reads the same bytes.
- **A navigation part outside `SUMMARY.md`.** Listing records in
  `SUMMARY.md` would make gate 2, the product `llms.txt` sections and the
  product reading order carry them. Deriving the part from the directory
  means a new record appears without an edit anywhere else, and the index
  row the citation check already demands is the one manual step.
- **A Vite plugin, not a SvelteKit route.** A `+server.ts` route is part of
  the app and would need excluding from prerendering by configuration that a
  later change could undo. A plugin with `apply: 'serve'` is not part of the
  build at all.
- **Files, not a database or GitHub.** giscus and GitHub Discussions need a
  network, an account and a deploy, and keep the conversation outside the
  repository a session reads. A SQLite file would need a reader. JSON lines
  are readable with `cat`, diff in review, and merge as appended lines.
- **TextQuoteSelector, not offsets.** A character offset breaks on the first
  edit above it; the W3C model calls the position selector "very brittle"
  for this reason and offers the quote selector for robustness. Hypothesis
  and the Web Annotation tools anchor the same way.
- **Not done:** comments on the published site (giscus is that), editing or
  deleting a comment (append-only; a correction is a reply), threads across
  pages, notifications, and fuzzy re-anchoring of an edited quote.

## Prior art

- The W3C Web Annotation Data Model's TextQuoteSelector and
  TextPositionSelector (section 4.2), and Hypothesis, which anchors by quote
  with position as a hint.
- The Rust RFC process, whose template RFCs here already use, and whose
  `text/` directory is rendered as a book.
- EP-0012 (the docsite and its gates) and EP-0013 (its identity), whose
  rules this part follows; RFC-0019 (the process), which this changes in two
  points, by a dated note.

## Unresolved questions

- Whether a thread should link to the commit that resolved it. For now a
  reply says so in words.
- Whether search should keep indexing the records beside the product pages,
  or filter them. For now they are indexed, and labelled on the page.
- Out of scope: comments on rustdoc's `api/` pages, which this site does not
  render.

## Future possibilities

A record's threads could be summarised on the record itself at build time
(open count, last answer), read-only, if the owner wants the published site
to show that a record is under discussion. Nothing here depends on it.

## Status log

- 2026-10-03: planned, by owner decision.
- 2026-10-03: in progress. M1 to M3 in [#130](https://github.com/mox-labs/matra/pull/130).
