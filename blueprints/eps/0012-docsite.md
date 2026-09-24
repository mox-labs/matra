# EP-0012: The docsite on SvelteKit, with figures and examples

- EP: EP-0012
- Implements: none (standalone: the docsite changes how matra is documented, not matra)
- Status: in progress
- Shipped in: not shipped (the docsite deploys from `main`, not with a release)

## Summary

Move the docsite from mdBook to SvelteKit with every page, URL and gate
intact, then add the figure pipeline, the figures, and an Examples section.
The page source stays plain Markdown; figures are generated from matra's own
output, committed, checked for currency, and each carries a text twin. Parity comes first and is its own milestone: no
figure lands until the migrated site is indistinguishable from the old one
to every gate and every published URL.

## Design

### Why

The docsite is 17 pages under `book/src/`, accurate and guarded. Six gates in
`scripts/check-docsite-floor.sh` run in CI. `tests/cited_figures.rs` and
`tests/error_tables.rs` pin pages to the code and to measured figures.
`book/src/llms.txt` is generated from `SUMMARY.md`, so an agent can find every
page.

What it cannot do is show matra's output in the shape the output has. A parse
is a tree of arcs between words. Metrics are distributions across the
paragraphs and sections of a document. Keyphrase extractors rank the same
text differently. Semantic clusters form and split as a threshold moves. Today
all of that appears as tables, or as hand-drawn SVGs that show structure
without showing data. There is also no page a newcomer can open to see what
matra does to a real piece of text before installing it, which is the most
common first question.

Five constraints bind any design:

1. **Parsing does not run in the browser.** UDPipe is C++ behind an FFI panic
   boundary. The rest of the crate checks on `wasm32`; the parser does not. A
   figure cannot parse the reader's own text.
2. **The docs describe what ships.** A figure shows real output of the
   released library on a recorded input, never an illustration of what it
   might produce.
3. **Agents read these docs.** Agents do not see a rendered figure: vision
   interfaces take the first frame of an animation, and an SVG drawn by script
   is not text. Whatever a figure shows must also exist as text.
4. **The gates survive the move.** A migration that drops a gate loses its
   protection without anyone noticing, which is the failure class the harness work
   aims to remove.
5. **Published URLs are cited elsewhere.** `Cargo.toml` `homepage`,
   `CITATION.cff`, `README.md`, `scripts/gen-llms-txt.sh` and the generated
   `llms.txt` all name `https://mox-labs.github.io/matra/<path>.html`.

### What readers, agents and contributors meet

#### For a reader

The site opens on one worked example: a short passage, what matra returns for
it, and three paths onward (install, examples, concepts), each labelled with
what the reader will get there. Overview first, and the layout stays put as
the reader moves.

Every figure sits beside a table or list of the same data. The table comes
first wherever the figure uses an encoding a newcomer may not read yet, such
as a dependency tree; the figure is the next step, not the entry. A figure
moves only when the reader asks it to (choosing another sentence, stepping a
threshold, expanding a subtree), and the move is a short transition between
two states. Nothing animates on its own, and with reduced motion requested
every figure shows its end state directly.

The **Examples** section is new. Each example is one real input and one task:

- the input text, with its source and licence
- the call, in Rust, Python and the CLI, as tabs over one example
- the output, as the JSON matra returns, a table, and where it helps a figure
- a short "what to notice", naming the fields a caller would read

Explanation pages may ask a newcomer to predict an outcome before revealing
it. Reference pages never do; they are for readers who already know what
they are looking for.

#### For an agent

Nothing an agent relies on today goes away. `llms.txt` is still generated and
still current. Every page is also served as its Markdown source at the same
path with `.md` (`guides/cli.md` beside `guides/cli.html`), so an agent can
read the page as authored. A figure's data is linked from the page as JSON,
and its text twin is in the HTML.

#### For a contributor

A page is a Markdown file. A figure is one line:

```text
<figure-parse input="examples/negation" sentence="2" />
```

The tag names a registered figure and the data it renders. Adding a new kind
of figure means adding a component to the registry, which is small and
reviewable. Adding a new instance of an existing figure means adding an input
to `site/inputs/` and regenerating. `just docs-figures` regenerates all figure
data; `just docs-floor` runs every gate; `just docs-serve` previews.

### Layout

- `site/`: the SvelteKit app, `adapter-static`, every route prerendered.
- `site/content/`: the Markdown, moved from `book/src/` with `git mv` so
  history follows. `SUMMARY.md` stays the navigation source, so `llms.txt`
  generation and the orphan gate keep their input format.
- `site/inputs/`: example inputs, each a text file with a sidecar naming its
  source and licence. Every input is public domain or written for matra.
- `site/src/lib/figures/`: generated JSON, one file per figure instance,
  committed.
- `examples/docsite_figures.rs`: the generator. It runs the released
  pipeline over `site/inputs/` and writes the JSON.
- `book/` is removed once the new site passes every gate.

### Rendering

Markdown renders at build time through a remark and rehype pipeline (GFM,
heading slugs, syntax highlighting). A known custom tag maps to its Svelte
component; an unknown tag fails the build. Markdown is not compiled as Svelte
(mdsvex is not used), so the source stays readable by every gate, every test
and every agent without a Svelte parser.

d3 is used for layout and scales only (`d3-hierarchy`, `d3-scale`,
`d3-shape`, and `d3-force` run to rest at build time). Svelte owns the DOM and
the transitions. `d3-selection` and `d3-transition` are not used, so there is
one rendering model and every figure prerenders into the static HTML before
any script runs.

The JavaScript toolchain is Bun, with `bun.lock` committed and installs run
frozen in CI. RFC-0005 applies: versions are pinned, and Dependabot covers the
new ecosystem.

### Motion

A transition animates a change between two states that the reader caused. At
most two stages, about a second in total, easing in and out. No motion starts
on its own, loops, or moves with scroll. Under `prefers-reduced-motion` a
figure jumps to its end state. This is stricter than WCAG 2.2 requires
(2.2.2 at Level A concerns self-starting motion over five seconds) and is
stated here as matra's own rule.

### Figures that need a parameter

A figure whose output depends on a parameter the reader can change, such as
the cosine threshold for semantic clusters, is generated at a fixed grid of
values (for example 0.50 to 0.95 in steps of 0.05), and the control snaps to
the grid. The browser never recomputes matra's output, so there is no second
implementation to drift from the first.

### URLs

SvelteKit's `trailingSlash: 'never'` with `adapter-static` prerenders
`/guides/cli` as `guides/cli.html`, the paths mdBook serves today. rustdoc
stays at `/api/`. Search is Pagefind, indexed from the built HTML.

### Gates

| Gate today | Under this RFC |
|---|---|
| 1 Links resolve (lychee, `book/src`) | lychee over `site/content` and the built HTML |
| 2 Every page in `SUMMARY.md` | Same, over `site/content`, following symlinks (today's gate skips `roadmap.md`, a symlink) |
| 3 Backticked type names resolve in `src/` | Same, over `site/content`; figure JSON exempt as output |
| 4 `mdbook build` without warnings | `svelte-check` and the build fail on any warning; every tag used in content is registered |
| 5 No em dashes | Same, over `site/content`, following symlinks |
| 6 `llms.txt` current | Same generator, retargeted to `site/content/SUMMARY.md` |
| new: figures current | Regenerate figure JSON and diff |
| new: every figure has its twin | A test over the prerendered HTML: each figure element has a text twin with the same data |
| new: every example input has a licence | Each file in `site/inputs/` has a sidecar with a source and a licence |

While mdBook still deploys (until M2), gate 4 stays the mdBook build, the
SvelteKit build runs as gate 7, and gate 8 holds the new build to every
`.html` path and heading anchor mdBook serves (`scripts/check-url-parity.sh`).
At the cut-over the SvelteKit build becomes gate 4, and gate 8 gives way to the
check run against the live site after deploy.

`tests/cited_figures.rs`, `tests/error_tables.rs` and
`skills/matra/references/semantic.md` read pages by path. They move to the new
paths in the same pull request as the `git mv`; the pinned digests do not
change because the files do not.

The figures-current gate needs the UDPipe model and the embedding model in the
docs job, fetched through the pinned, hash-verified path of RFC-0011.

### Links out, and comments

The header links to the repository on GitHub, its Issues and Discussions, and
the API reference at `/api/`; on a phone they sit at the foot of the
navigation panel. The footer repeats them beside the contents page, the
single-page view and `llms.txt`. Each page ends with "Edit this page", linking
to its source on GitHub (`site/content/<path>`, or `ROADMAP.md` for the
roadmap, whose page is a symlink to it), and with a link to its Markdown twin.

Readers signed in with GitHub can comment on a page through giscus, which
stores each page's thread as a GitHub Discussion in this repository. Comments
then live where the project's other conversations do, under the same
moderation, with no third-party store. There is one discussion per page,
matched by path; giscus drops the extension before matching, so the
extensionless and the `.html` URL of a page share a thread. The widget loads
nothing until it is scrolled near, follows the site's theme, and renders only
when the build sets `PUBLIC_GISCUS_REPO_ID` and `PUBLIC_GISCUS_CATEGORY_ID`.
`site/README.md` has the steps to switch it on. Comments are a reader
affordance, not content: they are not in the page source, the Markdown twin,
`llms.txt` or the search index.

### Alternatives not taken

**Stay on mdBook and add figures as script islands.** No migration, and the
gates stay as they are. But mdBook has no component model, so every figure is
hand-wired script with no shared layout, no prerendering and no types, and the
figures become the least maintained part of the site.

**SvelteKit with mdsvex.** The common route, and components can sit inline in
prose. But the page source stops being Markdown: `llms.txt`, gates 2, 3 and 5,
`tests/error_tables.rs` and any agent reading the repository would have to
parse Svelte, or would quietly stop seeing whatever lives in components.

**Plain Markdown with named figure tags (chosen).** The Markdown source is the
contract that the gates, the tests, `llms.txt` and agents all read. Keeping it
plain is what lets the site gain interaction without any of them losing sight
of the content. A figure in the source is one line naming its data, so the
data is where the meaning lives.

**Recompute in the browser.** A WASM build of matra's non-parsing code could
recompute some figures live. Rejected for now: parsing is the input to almost
everything, and a figure that recomputes part of the pipeline in a second
build is a second implementation to keep in step.

### Sources

On motion and figures:

- Heer, J. and Robertson, G. (2007). Animated Transitions in Statistical Data
  Graphics. *IEEE TVCG* 13(6). Transitions between states of one graphic aid
  comprehension "with careful design"; heavily staged animation increased
  error.
- Chevalier, F. et al. (2016). Animations 25 Years Later: New Roles and
  Opportunities. *AVI 2016*.
- W3C, WCAG 2.2, Success Criteria 2.2.2 (Pause, Stop, Hide) and 2.3.3
  (Animation from Interactions).
- Victor, B. (2006). *Magic Ink*: interaction earns its cost only when it lets
  a reader ask a question a static page could not answer.
- Segel, E. and Heer, J. (2010). Narrative Visualization: Telling Stories with
  Data. *IEEE TVCG* 16(6). The author-driven then reader-driven ("martini
  glass") structure the home page follows.

On readers:

- Pirolli, P. and Card, S. (1999). Information Foraging. *Psychological
  Review* 106(4). Readers follow scent and leave when it runs out; entry
  points carry a label saying what is behind them.
- Kalyuga, S. (2007). Expertise Reversal Effect and Its Implications for Learner-Tailored Instruction. *Educational Psychology
  Review* 19. Guidance that helps a novice hinders an expert, which is why
  prediction prompts live in explanation pages and never in reference.
- Larkin, J. and Simon, H. (1987). Why a Diagram is (Sometimes) Worth Ten
  Thousand Words. *Cognitive Science* 11. Diagrams cheapen inference for
  readers who can already read them, which is why the table comes first.

On the site: RFC-0005 (supply chain), RFC-0011 (pinned downloads), RFC-0012
(the agent surface and `llms.txt`).

## Goals

- The deployed site is built from `site/`, and `book/` no longer exists.
- Every URL mdBook served still resolves, and rustdoc is still at `/api/`.
- Every gate in the gates table in Design runs in `just docs-floor` and in CI,
  including the three new ones.
- Seven data figures, each on the page whose concept it shows, each with a
  text twin, each generated from matra's output and checked for currency.
- An Examples section of at least six worked examples over licensed inputs.
- A home page that opens on one worked example.

## Non-goals

- Recorded terminal sessions and acceptance scenarios (separate work).
- Rewriting page content beyond what a figure or an example needs. Content
  polish is its own pass, after this plan.
- Live figures over a reader's own text.
- Changing the seven hand-drawn architecture SVGs, which show structure
  rather than data and stay as they are.

## Iterations and milestones

### M1: the site builds at parity, beside mdBook

- **Deliverable.** `site/` with SvelteKit, `adapter-static`, Bun and a
  committed `bun.lock`. `book/src/` moved to `site/content/` with `git mv`.
  The remark and rehype renderer with the tag registry (empty except a
  passthrough for the existing inline SVGs). Navigation from `SUMMARY.md`. The
  CSS from `book/css/matra.css` ported (measure, rhythm, figure and table
  rules). Pagefind search. Every page also emitted as `.md` beside its
  `.html`. The six gates retargeted to `site/content`, gates 2 and 5 now
  following symlinks. `tests/cited_figures.rs`, `tests/error_tables.rs`,
  `skills/matra/references/semantic.md` and `scripts/gen-llms-txt.sh`
  updated to the new paths. `docs.yml` builds the new site into a preview
  artifact while mdBook still deploys.
- **Exit criterion.** `just docs-floor` passes over `site/`. A URL-parity
  script lists every `.html` path in the mdBook build and every one exists in
  the SvelteKit build (zero missing). `cargo test` passes with the moved
  paths. The pinned digests in `tests/cited_figures.rs` are unchanged.

### M2: cut over

- **Deliverable.** `docs.yml` deploys the SvelteKit build with rustdoc at
  `/api/`. `book/`, `book.toml` and the mdBook install steps are removed.
  `CLAUDE.md`, `CONTRIBUTING.md`, `AGENTS.md` and the justfile describe the
  new site.
- **Exit criterion.** The deployed site serves every path from M1's parity
  list, `/llms.txt`, `/api/matra/index.html`, and a `.md` twin for every page,
  checked by a script run against the live URL after deploy.
  `git grep -n 'mdbook\|book/src'` returns only history (released CHANGELOG
  entries, frozen RFC and EP bodies).

### M3: the figure pipeline, and the first figure

- **Deliverable.** `examples/docsite_figures.rs` generating JSON into
  `site/src/lib/figures/` from `site/inputs/`. The figures-current gate
  (regenerate and diff), the twin test over the prerendered HTML, and the
  input-licence gate. The docs CI job fetches the UDPipe and embedding models
  through RFC-0011's pinned path. The first figure: the dependency parse, as
  an arc diagram over the token table, on the Concepts page, where the table
  leads and the figure follows.
- **Exit criterion.** All three new gates pass, and each fails on a planted
  fault: a stale JSON file, a figure without its twin, an input without a
  licence sidecar. The parse figure prerenders with scripts disabled.

### M4: figures for structure and measurement

- **Deliverable.** Structural primitives highlighted over sentences
  (negation, modality, reporting), on the Situation model page. Metrics
  across the paragraphs of one document, as small multiples, on the
  Methodology page. RAKE against YAKE ranking of the same text, as a
  slopegraph, on the keyphrase guidance.
- **Exit criterion.** Each figure passes the three gates and has a reduced
  motion state equal to its end state, checked by the twin test run with
  reduced motion emulated.

### M5: figures for relation and process

- **Deliverable.** TextRank: the sentence graph with the chosen summary
  sentences marked. Semantic clusters over a threshold grid with a stepping
  control. The pipeline (ingest, decompose, compose) as a two-stage stepper
  on the Programming model page.
- **Exit criterion.** As M4. The cluster figure's grid values are the ones
  matra produced; the twin lists the clusters at every grid value.

### M6: examples and the home page

- **Deliverable.** `site/inputs/` with licensed inputs across at least three
  registers (scientific, legal or administrative, narrative). An Examples
  section of at least six worked examples, each with the input, the call in
  Rust, Python and the CLI, the output, and "what to notice". The home page's
  worked example, chosen by the newcomer pass below.
- **Exit criterion.** Every example's call is executed in CI and its output
  compared with the committed output, so an example cannot drift from
  matra's behaviour. The newcomer agent (`.claude/agents/newcomer.md`),
  starting cold from the home page, installs matra and reproduces two
  examples by following the pages literally, and its report lists no step it
  could not follow.

## Test plan

- The gates in `just docs-floor`, extended per Design, in `just check` and
  the `Docsite floor` CI job.
- The URL-parity script (M1) and the live check after deploy (M2).
- The figures-current gate, the twin test and the input-licence gate (M3),
  each demonstrated to fail on a planted fault before it is relied on.
- Example calls executed in CI against committed outputs (M6).
- A newcomer pass at M6, reported as findings rather than a verdict.
- Figures are judged by whether a reader can answer a named question about
  matra's output faster or more correctly with the figure than with the table
  alone. Where the owner's read finds the table does as well, the figure is
  removed. Self-reports such as "that was clear" do not count.

## Ship criteria

The deployed site is built from `site/`, M1's parity list resolves on it,
every gate in the gates table in Design passes in CI, the seven figures and six
examples are live, and the M6 newcomer pass is filed.

## Risks

- **Build-time figure text misplaces without a font shaper.** Early signal:
  labels overlapping in the prerendered SVG. Response: measure text in the
  browser after hydration for label placement only, keeping the data static.
- **Model downloads slow or fail the docs job.** Early signal: docs CI time
  or flakes. Response: cache the pinned models by hash.
- **Parity drift during M1.** Content edits land on `book/src` while M1 is
  open. Response: M1 is one pull request and content edits wait for it.
- **Figure scope creep.** Response: the registry is the gate; a new kind of
  figure needs a named question it answers better than a table.

## Status log

- 2026-09-24: planned.
- 2026-09-24: in progress. M1 delivered: the SvelteKit site under `site/`
  builds every page beside mdBook, with URL and heading-anchor parity checked
  by gate 8. Design gains "Links out, and comments", which the owner asked for;
  comments stay off until a discussion category is chosen.
