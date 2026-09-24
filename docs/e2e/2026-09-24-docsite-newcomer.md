# The new docsite, from the home page to two reproduced examples

> **Point-in-time record.** This is the report of the newcomer pass EP-0012's
> M6 calls for, run on 2026-09-24 against the site built from the
> `docs/site-m6` branch before it merged. It describes that site and the
> published matra 0.2.1, not what either is later. The findings marked fixed
> were fixed on the same branch after the pass; the EP-0012 status log and
> the `[Unreleased]` section of `CHANGELOG.md` record them.

**Charter.** Start cold from the home page of the built site. Install matra by
following the pages. Reproduce two worked examples exactly as their pages show
them. Separately, pick the home page's worked example by which of three
candidates (the parse, the structural primitives, the semantic clusters) a
first-time reader understood fastest.

**Who ran it, and what that costs.** There is no separate newcomer agent to
hand this to from where it ran, so the maintainer who wrote the M6 pages
followed `.claude/agents/newcomer.md` itself. That is the weakest part of this
report, and it is said here rather than buried: the person following the pages
knew what they meant. Two things narrow the gap. Every command was run as the
page printed it, and every result was compared with the page by a script
rather than by eye. And the choice of home example, which is a question of
first understanding, was put to three separate cold readers (below), not
judged by the author. Commands ran in small groups per page section, not one
at a time, so this pass measures correctness better than it measures patience.

**Environment.** A fresh `python:3.12` container (Debian, glibc, `g++` and
`curl` present), linux/aarch64 under OrbStack on Apple Silicon: native arm64,
not emulated. No matra, no model, no configuration, no Rust toolchain until
the Rust step installed one with rustup. The site was the branch's build,
served locally with GitHub Pages path semantics and reached from the container
as `host.docker.internal:4191`. matra came from the registries: `pip install
matra` resolved 0.2.1 from PyPI, and `cargo add matra` resolved 0.2.1 from
crates.io.

**Host untouched.** The container mounted nothing from the host, so the pass
could not write to the host's configuration, data or model locations. The
host ran only the static file server and the headless browser that read the
pages.

---

## Summary

| Group | Findings |
|---|---|
| Blocks the charter | none |
| Hurts first impression | 5, all fixed on the branch |
| Cosmetic | 3, recorded, not fixed here |

Both examples reproduced exactly, in every language tried: the summary
(Python, CLI and Rust) and the one-sentence parse (Python and CLI), plus the
readability example as a check on the brotli upgrade since 0.2.1 (Python and
CLI). Every output matched the page's committed output to twelve significant
digits, and every CLI envelope matched its `cli.json`.

## What happened, in order

1. **Home page.** The opening names what matra does, a figure of real output
   follows, and three labelled paths lead on. Followed "Install".
2. **Installation, Python route.** `pip install matra`: 0.2.1 installed in one
   second from the aarch64 wheel. `matra --version` printed `matra 0.2.1`,
   while the page's expected output read `matra 0.2.0`
   (`site/content/tutorials/installation.md:58` and `:79`). Finding 1.
3. **Verify the install.** The snippet printed `sections: 1` and
   `vocabulary_ttr: 0.8571428571428571`, exactly the page's expected output.
   The first run took 4 seconds and printed nothing while it downloaded the
   model, as the page warns; the second took 1 second.
4. **Examples index, then "Summarize a long document".** The page's first
   command is `curl -O https://mox-labs.github.io/matra/example-files/summarize/origin-struggle.txt`.
   Before this branch deploys, that URL is a 404, and `curl -O` exited 0 having
   saved GitHub Pages' 404 HTML as `origin-struggle.txt`: a 15,418-byte HTML
   page that matra would then have analysed as the input. After deploy the URL
   resolves, but any later 404 would fail the same silent way. Finding 2.
   The rest of the pass used the local build's copy of the same path, with
   `curl -fLO`.
5. **Summarize, Python and CLI.** The Python tab, run as shown, printed the
   page's `output.json` exactly (positions 32, 33, 58). The CLI line printed
   the page's `cli.json` exactly.
6. **Summarize, Rust.** `cargo new`, `cargo add matra serde_json` as the tab's
   first line says, the tab as `src/main.rs`: resolved matra 0.2.1, built in
   78 seconds with the container's `g++`, and printed the page's output
   exactly. CI compiles the Rust tabs against the repository, so this is the
   one check that they also work against the published crate.
7. **"Parse one sentence", Python and CLI.** Both matched the page exactly.
8. **"Compare readability across paragraphs", Python and CLI.** Both matched,
   compression ratios included. The branch is ahead of 0.2.1 by a brotli
   upgrade (7 to 9), which could in principle move a compression ratio; on
   this input it did not.

## Findings

### Hurts first impression

1. **The installation page's version banner was stale.** It showed
   `matra 0.2.0` for a command that prints `matra 0.2.1`
   (`site/content/tutorials/installation.md:58`, `:79`). A reader told to
   compare against the page sees a mismatch on their first command. The CLI
   guide had the same banner (`site/content/guides/cli.md:231`). *Fixed*: all
   three read 0.2.1, and `scripts/check-version-sync.sh` now holds the
   installation page's banners to `Cargo.toml`. The CLI guide is a pinned
   source page (`tests/cited_figures.rs`), so it was corrected and its cited
   scores re-measured by the recipe on the semantic clusters page (0.8362,
   0.5907 and 0.6080, unchanged), but it is not held by the check, which
   would force that re-measurement every release.
2. **The download command succeeded on a missing file.** `curl -O` saves an
   HTTP error page under the requested name and exits 0 (step 4). *Fixed*:
   every example page now prints `curl -fLO`, which fails loudly and follows
   a redirect.
3. **The home page promised more than its example showed.** All three cold
   readers (below) said the opening lists readability, keyphrases and
   summaries and the example shows none of them. *Fixed*: the text under the
   figure says it is one of the things matra returns, where it is available,
   and that the Examples show the rest.
4. **The clusters figure hid part of its table and crowded two labels.** The
   cold reader of that candidate could not read the end of the edges column
   ("2-3 (0." at the figure's edge), could not tell that the table scrolled to
   show three more thresholds, and could not tell which of two nested arcs
   the labels 0.85 and 0.88 belonged to. *Fixed*: the edge lists wrap, every
   threshold's row shows, and nested arcs sit far enough apart that each
   label clears its neighbour.
5. **An example page's Markdown twin carried no calls.** The `.md` twin agents
   read showed `<example-call name="summarize" />` where the page shows the
   code. *Fixed*: the twin carries the input's source and download command,
   the three calls and the output as Markdown.

### Cosmetic, recorded

6. **The provenance line names the crate version, not the build.** Figures
   say "Produced by matra 0.2.1", and the branch is ahead of 0.2.1 by
   dependency upgrades. On every input checked the output is the same, and
   the gates hold the committed data to the tree it was built from, so this
   misleads no one today; it would if a library change landed between
   releases.
7. **Two navigation titles meant nothing to a first reader.** All three cold
   readers listed "Situation model" and "Pragmatics" among what they did not
   understand. Rewording pages is outside EP-0012's scope.
8. **The parse figure's relation labels are unexplained on the figure.** The
   parse candidate's reader could only read `nsubj`, `cop` and the rest with
   knowledge of Universal Dependencies it brought with it. The legend names
   the groups, not the labels.

### Nothing found

- The Python route on linux/aarch64: wheel present, install in one second, no
  build.
- The verify snippet: output exactly as printed.
- The Python and CLI tabs of three examples, and the Rust tab of one: output
  exactly as committed.
- The CLI envelope: `input` is the file name as typed, as the page shows.

## The home page's example

Three builds of the home page differed only in the worked example. Each
full-page screenshot went to a separate headless session with no project
context, read-only tools, and one image, and the same five questions: what the
software does, three specific things it returned for the text and what each
means, what to do next, what was not understood, and what took the most
careful reading. Answers were graded against the data, not against how
confident they sounded.

| Candidate | Q2 correct | Time | What the reader could not resolve |
|---|---|---|---|
| Parse (one Austen sentence) | 3 of 3 | 17 s | eleven relation labels, `0 root`, the arc diagram's hidden right edge |
| Primitives (five written sentences) | 3 of 3 | 17 s | Hearst spans, `root adverbial` on `not`, a reported sentence marked `bare_assertion` |
| Clusters (ten short sentences) | 3 of 3 | 20 s | the clipped edges column, the unseen rows, the crowded labels |

Time does not separate them, and all three were answered correctly. What
separates them is the kind of confusion left over. The parse and primitives
readers were stopped by concepts the page would have to teach before the
example makes sense. The clusters reader explained the result in plain words,
unprompted found that sentences 2 and 3 share a cluster only through sentence
1, and found that sentence 8 joins at 0.70, and everything it could not
resolve was a layout defect that could be fixed (finding 4). The home page
uses the clusters.

The cost of that choice is stated on the page: clustering comes with the
Python package and with the Rust library's `model2vec` feature, and the
command line has no clusters command.

**Warrant.** The three readers were sessions of the same model family as the
author, which makes them independent of the author's context but not of its
priors, and a screenshot is not a browser. What they declined to claim is
useful signal: each said where it guessed, and none claimed to understand the
relation labels or the Hearst spans.

## The first ten minutes

The home page shows real output before asking for anything. Install took one
second and the first call four, silent as the page warns. The first example
page's first command then quietly saved an error page as the input, which is
the minute a real reader would have lost, and the one the fix removes. After
that every call in every language printed what its page shows, down to the
last digit.
