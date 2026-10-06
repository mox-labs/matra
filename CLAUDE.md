# Matra

NLP library. Text in, structured analysis out.

UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression, vocab TTR, nominalization, passive ratio), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

Rule evaluation over parsed text structure is part of the intended scope and lands later; document references describe it as planned, not present.

## Session start

On session start in this directory, read `blueprints/README.md`: its index says which proposals are proposed or accepted and the process says which record a change takes. The RFCs and EPs in `blueprints/legacy/` are the reasons behind the code that exists; `RFC-NNNN` and `EP-NNNN` cite them. Then Observe (the branch, open pull requests, and open issues labelled `tracking` or `acp`), Orient (which tracking issue's next milestone is next, or an EP's when parallel work has one), Decide, Act, and continue with the operational sections below.

A local, untracked `.claude/logs/SESSION-RESUME.md` may hold session notes. It is a convenience, not a record: anything a later session must rely on belongs in a proposal, a plan's status log, a tracking issue, or the CHANGELOG.

## Posture

matra is a public OSS package intended as an exemplar for both Claude-managed repositories and human–AI collaborative intelligence. Two disciplines are non-negotiable:

- **ACES** — Adaptable, Composable, Extensible. The structural design philosophy resisting the stasis/drag/opacity cycle. Every structural change is checked against the ACES boundary test. See `.claude/skills/aces/SKILL.md`.
- **Antifragility** — the operational discipline. Size caps at entry, panic boundaries at C/C++ FFI, atomic file writes, TOCTOU closure, cycle-safe graph walks. See `.claude/skills/resilience-floor/SKILL.md`.

The quality bar is high because the public surface is a contract across Rust, Python, and (when the WASM crust lands) TypeScript. Names are forever; the API surface, once published, locks downstream costs in.

For how the project is maintained (the agents and their methods, routing, the verification gate, who decides what, the process grain), see `site/content/explanation/how-matra-is-maintained.md`. For PR mechanics, see `CONTRIBUTING.md`.

## Architecture

Hex architecture. Rust core with PyO3 Python bindings. Single crate, dual publish: `matra` on crates.io, `matra` on PyPI via maturin.

Pipeline: ingest → decompose → compose (RFC-0007, superseding RFC-0002). `abstract` is the reserved empty seam between structure and purpose-fitted output; rule evaluation lands there, and `abstract` is a Rust keyword so it names the tier, never code.

The surface is `Ingest` (source variation as data: a string is a stream of one, a directory a stream of many) into `Engine` (`analyze` over a stream, `analyze_one`, or the stages `annotate` and `compose`). `annotate` is the only route from text to the parser, so the size cap holds pipeline-wide; seven equivalence laws in `src/lib.rs` tests pin the grains together. Trait names (`Source`, `Decomposer`, `NlpProvider`) keep their existing names.

The domain depends on nothing else in matra; the port traits (NlpProvider, Decomposer, Source, Embedder) depend only on the domain, and each adapter implements one of them. UDPipe is the default NLP adapter, behind the `udpipe` feature flag.

Four layers, and the dependency arrows only ever point inward.

- `domain.rs` holds every type the library hands back and depends on `serde`, `thiserror` and `std`. Nothing else.
- Each port is a `mod.rs` (`source/`, `decompose/`, `nlp/`, `embed/`) declaring one trait and importing only `domain`.
- Each adapter implements one port. `nlp/udpipe.rs` is the only file in the tree that imports `udpipe_rs`, because that is where the panic boundary lives.
- `metrics/` and `extraction/` are plain functions over `domain` and `stopwords`. They touch no port, which is why they test without a model.
- `lib.rs` is the composition root: the only file that knows every adapter and every port, and the only place they are wired together.
- `config.rs` sits beside `lib.rs` and answers where things live and what the defaults are. It is the one box an adapter may read from the row above it (`Udpipe::from_config`, `Model2Vec::from_config`).

Above the library, `cli/` is the application tier: it parses arguments, renders, and decides exit codes, behind the non-default `cli` feature. Boundary rule 7 holds it to the public surface (`Engine`, `Ingest`, `extraction`, `config`, `domain`) and never a port or an adapter, so it is a consumer that ships in the same crate rather than a layer of it. `src/bin/matra.rs` and `python/matra/cli.py` are launchers: each collects arguments and calls `matra::cli::run`, so one program answers to both. The application tier lives in the library because a binary target cannot be reached from PyO3, and the alternative was the second implementation that 0.1.0 shipped and drifted.

Run `ls` for the file list. It is not repeated here, because a hand-maintained tree in a context document goes stale the first time a file moves and then quietly misinforms whoever trusted it.

For how a call actually runs through those layers, read `site/content/architecture/design.md`.

## Boundary rules

1. `domain.rs` depends only on `serde`, `thiserror`, and `std`. Adding any other dependency requires an enhancement proposal.
2. Port modules (`source/mod.rs`, `decompose/mod.rs`, `nlp/mod.rs`, `embed/mod.rs`) import only from `domain`.
3. No port module imports another port module.
4. `nlp/udpipe.rs` is the ONLY file that imports `udpipe_rs`.
5. `metrics/` and `extraction/` import only from `domain` and `stopwords`.
6. `cargo check --no-default-features` must compile.
7. Composition root (`lib.rs`) is the only place that knows all adapters and ports. `src/cli/` uses the public surface (`Engine`, `Ingest`), `extraction`, `config` and `domain`, never a port module or an adapter.
8. `tracing` is forbidden in `domain.rs` and port modules (Burner amendment, 2026-04-28).

**Motivation for each rule, what breaks when it is violated, and what to read for when reviewing: [`site/content/reference/boundary-rules.md`](site/content/reference/boundary-rules.md).** That file is canonical; this list is the summary.

Enforcement is mechanical for the forms and review for the intent. Rule 6 is verified by compiling on every push (`ci.yml` rust and MSRV jobs). Rules 1, 2, 3, 4, 5, 7 (its `src/cli/` part) and 8 are semgrep rules in `.semgrep/`, each tested against a fixture of the forms it claims (use lines, brace groups, inline paths, `pub use`), run by `scripts/check-boundaries.sh` from `just check`, the opt-in pre-commit hook (skipped with a warning when semgrep is absent) and the `Boundary check` job in `ci.yml`; a failure message names the rule, why it exists and the section of `boundary-rules.md` to read. Review stays the gate for what an import cannot show: a trait shaped around one adapter (2), a function taking text instead of structure (5), wiring outside `lib.rs` beyond the CLI (7). What each check covers and misses: `blueprints/legacy/eps/0014-architecture-guardrails.md`.

## Things that will bite you

Non-obvious gotchas. Each is a behavior plus the failure mode if you violate it.

- **Domain purity is a semgrep check, not a compiler guarantee.** A non-optional dependency added to `[dependencies]` and used in `domain.rs` compiles clean, including under `--no-default-features` (that flag drops only `udpipe`/`sha2`). The rule 1 checks catch it wherever `domain.rs` names the crate, in a `use` line or an inline path, which is why the domain spells std paths in full (`std::fmt::Display`, not `fmt::Display` after `use std::fmt`). The one route they miss is a macro or derive made crate-wide by `#[macro_use]` in `lib.rs` and used unqualified. Adapters are where deps live; the domain stays pure.
- **Single UDPipe importer.** The rule 4 checks fail `just check`, the pre-commit hook and the `Boundary check` CI job if anything outside `nlp/udpipe.rs` names `udpipe_rs`, in any form, or if the adapter makes a `udpipe_rs` item `pub`. A private alias made public under another name still slips past, so review reads for it. The wrap exists because UDPipe holds non-Send C-side state and a panic at the FFI boundary would otherwise abort the host process. The catch_unwind seam lives inside this file by design; reintroducing direct imports elsewhere puts the panic boundary back in user code.
- **Per-paragraph parse, not whole-document.** The previous join-then-prefix-match approach silently reassigned sentences when two paragraphs shared their first 30 characters (FM1). Don't reintroduce "join paragraphs, parse once, wire sentences back to paragraphs by substring match." The pipeline parses each non-blockquote paragraph individually for a reason.
- **TOCTOU closes in `read_and_verify`.** The function returns `Vec<u8>` and the loader consumes those bytes via `Model::load_from_memory`. Never re-read the disk between hash verify and load — that opens the window a swap attack lives in.
- **Magic numbers in tree walks are forbidden.** `Sentence::tree_depth` returns `usize::MAX` on cycles; cycle detection uses a visited set, not `if depth > 20 { return }`. The previous magic-ceiling silently truncated malformed parses; the sentinel is the loud failure.
- **No `Result<T, String>` anywhere in the library.** Library callers match on concrete `domain::Error` variants. `anyhow` belongs in caller code (a CLI, a service) where erasure is ergonomic; matra itself stays on enums via `thiserror`. `library-no-string-errors` in `.semgrep/` fails on a `String` or `&str` error type outside `src/cli/`, `src/bin/` and test modules, and `library-no-unwrap` on `.unwrap()` or `.expect(` there.
- **A semgrep rule that silently matches nothing is the failure to fear.** Semgrep's Rust AST patterns miss brace groups and deep paths, its test discovery skips `.semgrep/`, and outside a git work tree its path-scoped rules match nothing. That is why every rule is a tested regex, why `scripts/check-boundaries.sh` pairs each rule file with its fixture itself, refuses to run outside a work tree, fails when a rule's `paths: include:` names a file git no longer has (so **when you move a file, update the `.semgrep/` rules that scope to it**; the check tells you which), and fails unless the scan read every Rust file in `src/`. Run it through the script or `just boundary`, never as a bare `semgrep` call, and when adding a rule, plant a violation in the real file and watch it fail.
- **PyErr routing is exhaustive at compile time.** Adding a variant to `domain::Error` will fail to compile until you wire it into `From<MatraError> for PyErr` with a specific Python exception class. The no-wildcard match exists so new variants do not silently route to `PyRuntimeError`.
- **Methods do not cross FFI. Only fields do.** Aggregate Rust methods (`Document::passive_ratio()`, `Corpus::total_words()`) are invisible to Python and (future) WASM consumers. If a value needs to be visible cross-language, materialize it as a field on a summary type, not a method.
- **A `path:line` citation in `.claude/` is checked.** `scripts/check-claude-citations.sh` (from `just check` and the `Boundary check` CI job) fails when one names a missing file or a line past its end, because the agent and skill files drifted from the code twice. When you move a file or shorten one, fix the citations the check names. It cannot see a line that still exists but now says something else, so name the item beside the line, as in `Matra` (`src/lib.rs:461`), and a reader can still find it after the line moves.
- **Em dashes get rejected.** Project convention forbids them in documentation prose. `scripts/check-docsite-floor.sh` gate 5 rejects em dashes in `site/content/`, `skills/`, `blueprints/` and `lab/`; reviewers catch them elsewhere.
- **Publishing is hand-gated.** `cargo publish` and `maturin publish` are always preceded by `--dry-run`. A release publishes only through `release.yml`, whose `crates-io` and `pypi` environments wait for the owner; that approval is the release decision, and an agent never approves a deployment environment. Do not script away the gate; it exists because publishing is irreversible and visible to every downstream consumer.
- **Only the owner accepts a decision.** Claude writes and argues enhancement proposals and API change proposals, and never accepts one. A proposal may merge with its status `proposed`, and Claude may merge it so it renders for review; acceptance is the change of its status line to `accepted`, and only the owner merges a pull request that makes it. Standing merge authority (green CI, a review with no blockers, a rationale comment) covers everything else: code, docs, tooling, dependency updates. `CONTRIBUTING.md` (How decisions are made) has the detail.

## Conventions

- `domain::Result<T>` everywhere in the library. No `Result<T, String>`. No panics in library code (UDPipe panics are converted at the boundary via `catch_unwind`).
- `impl AsRef<Path>` for file paths, not `&str`.
- Feature flags are additive. Enabling `udpipe` adds UDPipe; disabling removes only UDPipe.
- Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `ci:`, `perf:`).
- Tests: `#[cfg(test)]` for unit, `tests/` for integration, `examples/` for usage demos.
- No em dashes in documentation prose.
- Public types use `#[non_exhaustive]` for additive forward compatibility.
- `cargo publish` and `maturin publish` always run with `--dry-run` first; explicit per-publish approval is required per the project memory.

## Build

```bash
just check                                     # runs every CI gate locally
cargo build                                    # default (with udpipe)
cargo build --no-default-features              # without udpipe
cargo test                                     # unit + doctests
cargo test --features cli                      # + the binary's own tests
cargo test --test integration -- --ignored     # integration (needs model)
just conformance                               # every crust against spec/tests/
just docs-floor                                # the twelve docsite gates
maturin develop                                # Python local install
maturin build                                  # Python wheel
```

Features are additive: `udpipe` (default), `model2vec`, `python`, `cli`. **Do not run `cargo test --all-features`.** It enables `python`, which builds against libpython with symbols deliberately left undefined until the interpreter loads them, so it fails at link with an arm64 symbol error that looks like a regression and is not.

## Practitioner agents and skills

The agents live in `.claude/agents/` and the skills in `.claude/skills/`, and each routes by its frontmatter `description`: what it is for and what it is not for. Those descriptions are the one routing table; `/review` (`.claude/commands/review.md`) convenes the agents for a pull request by the paths its diff touches.

## Docsite

The site has three areas. **Docs** is `site/content/`: every page describes what ships today; `site/content/roadmap.md` is the only page describing what does not. **Blueprints** renders `blueprints/README.md` and the proposals and plans in `blueprints/proposals/` and `blueprints/plans/` in place, the only area that shows a status, and the collaborate stance: each proposal and plan has a frame and a navigator (Read, Decide, Confirm) drawn from its parsed components, and a decision names its `depends`, `reversible` and `grounds` (`site/README.md`, The collaborate stance); the legacy records in `blueprints/legacy/` are not rendered, and their old addresses keep a page linking each on GitHub. **Lab** renders `lab/README.md`, the home of evals and experiments. The header names the three areas and the sidebar shows the current one. Blueprints and Lab are not Diátaxis parts and are not in `SUMMARY.md`; gate 3 does not read them, every other gate does.

Local comments: in `just docs-serve` only, text on any page can be selected and commented on; threads are `discussion/<route>.jsonl`, committed. `just comments` lists open threads and `just comment-reply <page> <thread> "<body>"` answers as Claude. The static build carries none of it, and `site/scripts/check-no-dev-comments.ts` fails the build if it does. Check `just comments` at session start when the owner has been reviewing.

Gates run via `just docs-floor`: every page reachable from `SUMMARY.md`, every backticked type name resolving in `src/`, every link resolving, a clean build, no em dashes outside quoted material (in `site/content/`, `skills/`, `blueprints/` and `lab/`), `site/content/llms.txt` current with `SUMMARY.md` (regenerate with `scripts/gen-llms-txt.sh`), every published URL and heading anchor still produced (`site/urls.txt`, `site/anchors.txt`), figure data current with what matra produces (regenerated by `examples/docsite_figures.rs` and diffed; needs the UDPipe model), every figure in the built HTML carrying a text twin of the same data (and each Blueprints record's frame, navigator and queue holding to the page's own decisions and assumptions), every example input in `site/inputs/` carrying a source and a licence, every worked example's Rust, Python and CLI call reproducing its committed output (`site/examples/`), and every docsite page in `site/urls.txt` (rustdoc's `api/` pages excepted) loaded in Chromium at 320, 390, 768 and 1280px, closed and with every `<details>` open, never scrolling sideways, with taps on an emulated touch screen pinning a parse figure's highlight and following a proposal's decision to its grounds and back (`site/scripts/check-responsive.ts`; the gate installs the Playwright browser on first run). Beside them, `scripts/check-blueprint-refs.sh` (from `just check` and the `Docsite floor` CI job) fails when a cited `EPR-`, `EPL-`, `RFC-` or `EP-` number has no file of its own kind (`proposals/`, `plans/`, `legacy/rfcs/`, `legacy/eps/`), or a proposal or plan has no row in the index.

Live preview: `just docs-serve` runs the site in `site/` (see `site/README.md`); `just docs-build` builds it. A `SUMMARY.md` entry without a file on disk fails the build.

Diagrams are hand-authored inline SVG; the rule for choosing a diagram form is in `site/README.md`.

