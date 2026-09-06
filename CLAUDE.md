# Matra

NLP library. Text in, structured analysis out.

UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression, vocab TTR, nominalization, passive ratio), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

Rule evaluation over parsed text structure is part of the intended scope and lands in a later iteration; document references describe it as planned, not present.

## Session start: OODA and resume

On session start in this directory, read `.claude/logs/SESSION-RESUME.md` first. Run the OODA loop it specifies (Observe state, Orient on what is in flight, Decide the next action, Act), then continue with the operational sections below. The resume file is the durable surface that captures where the work stopped and what the next move is; keep it updated after each batch ships or any meaningful state change.

## Posture

matra is a public OSS package intended as an exemplar for both Claude-managed repositories and human–AI collaborative intelligence. Two disciplines are non-negotiable:

- **ACES** — Adaptable, Composable, Extensible. The structural design philosophy resisting the stasis/drag/opacity cycle. Every structural change is checked against the ACES boundary test. See `.claude/skills/aces/SKILL.md`.
- **Antifragility** — the operational discipline. Size caps at entry, panic boundaries at C/C++ FFI, atomic file writes, TOCTOU closure, cycle-safe graph walks. See `.claude/skills/resilience-floor/SKILL.md`.

The quality bar is high because the public surface is a contract across Rust, Python, and (when the WASM crust lands) TypeScript. Names are forever; the API surface, once published, locks downstream costs in.

For the working model that frames how humans and AI collaborate on this project (roles, discourse-to-docs-to-code discipline, audit trail), see `docs/collaboration-model.md`. For PR mechanics, see `CONTRIBUTING.md`.

## Architecture

Hex architecture. Rust core with PyO3 Python bindings. Single crate, dual publish: `matra` on crates.io, `matra` on PyPI via maturin.

Pipeline: ingest → decompose → compose (ADR-0007, superseding ADR-0002). `abstract` is the reserved empty seam between structure and purpose-fitted output; rule evaluation lands there, and `abstract` is a Rust keyword so it names the tier, never code.

The surface is `Ingest` (source variation as data: a string is a stream of one, a directory a stream of many) into `Engine` (`analyze` over a stream, `analyze_one`, or the stages `annotate` and `compose`). `annotate` is the only route from text to the parser, so the size cap holds pipeline-wide; seven equivalence laws in `src/lib.rs` tests pin the grains together. Trait names (`Source`, `Decomposer`, `NlpProvider`) keep their existing names.

Domain depends on port traits (NlpProvider, Decomposer, Source), not on adapters directly. UDPipe is the default NLP adapter, behind the `udpipe` feature flag.

Four layers, and the dependency arrows only ever point inward.

- `domain.rs` holds every type the library hands back and depends on `serde`, `thiserror` and `std`. Nothing else.
- Each port is a `mod.rs` (`source/`, `decompose/`, `nlp/`, `embed/`) declaring one trait and importing only `domain`.
- Each adapter implements one port. `nlp/udpipe.rs` is the only file in the tree that imports `udpipe_rs`, because that is where the panic boundary lives.
- `metrics/` and `extraction/` are plain functions over `domain` and `stopwords`. They touch no port, which is why they test without a model.
- `lib.rs` is the composition root: the only file that knows every adapter and every port, and the only place they are wired together.

Above the library, `bin/matra.rs` is the application tier and decides rendering and exit codes, while `python/matra/` is the crust.

Run `ls` for the file list. It is not repeated here, because a hand-maintained tree in a context document goes stale the first time a file moves and then quietly misinforms whoever trusted it.

For how a call actually runs through those layers, read `book/src/architecture/design.md`.

## Boundary rules

1. `domain.rs` depends only on `serde`, `thiserror`, and `std`. Adding any other dependency requires an ADR.
2. Port modules (`source/mod.rs`, `decompose/mod.rs`, `nlp/mod.rs`, `embed/mod.rs`) import only from `domain`.
3. No port module imports another port module.
4. `nlp/udpipe.rs` is the ONLY file that imports `udpipe_rs`.
5. `metrics/` and `extraction/` import only from `domain` and `stopwords`.
6. `cargo check --no-default-features` must compile.
7. Composition root (`lib.rs`) is the only place that knows all adapters and ports. `src/cli/` uses the public surface (`Engine`, `Ingest`), `extraction`, `config` and `domain`, never a port module or an adapter.
8. `tracing` is forbidden in `domain.rs` and port modules (Burner amendment, 2026-04-28).

**Motivation for each rule, what breaks when it is violated, and what to read for when reviewing: [`book/src/reference/boundary-rules.md`](book/src/reference/boundary-rules.md).** That file is canonical; this list is the summary.

Enforcement is mostly judgment, so review is the gate. Only rule 6 runs on every push (`ci.yml` MSRV job). Rules 3, 4, 8 get a partial grep from `scripts/check-boundaries.sh`, which runs from `just check` and the opt-in pre-commit hook but is **not** wired into any CI workflow. Rules 1, 2, 5, 7 have no mechanical check at all.

## Things that will bite you

Non-obvious gotchas. Each is a behavior plus the failure mode if you violate it.

- **Domain purity rests on review, not the compiler.** A non-optional dependency added to `[dependencies]` and used in `domain.rs` compiles clean, including under `--no-default-features` (that flag drops only `udpipe`/`sha2`). Nothing mechanical catches it. See `book/src/reference/boundary-rules.md` rule 1 for what to read for. Adapters are where deps live; the domain stays pure.
- **Single UDPipe importer.** `scripts/check-boundaries.sh` fails `just check` and the pre-commit hook if anything outside `nlp/udpipe.rs` imports `udpipe_rs`. No CI workflow runs it, so review is the real gate. The wrap exists because UDPipe holds non-Send C-side state and a panic at the FFI boundary would otherwise abort the host process. The catch_unwind seam lives inside this file by design; reintroducing direct imports elsewhere puts the panic boundary back in user code.
- **Per-paragraph parse, not whole-document.** The previous join-then-prefix-match approach silently reassigned sentences when two paragraphs shared their first 30 characters (FM1). Don't reintroduce "join paragraphs, parse once, wire sentences back to paragraphs by substring match." The pipeline parses each non-blockquote paragraph individually for a reason.
- **TOCTOU closes in `read_and_verify`.** The function returns `Vec<u8>` and the loader consumes those bytes via `Model::load_from_memory`. Never re-read the disk between hash verify and load — that opens the window a swap attack lives in.
- **Magic numbers in tree walks are forbidden.** `Sentence::tree_depth` returns `usize::MAX` on cycles; cycle detection uses a visited set, not `if depth > 20 { return }`. The previous magic-ceiling silently truncated malformed parses; the sentinel is the loud failure.
- **No `Result<T, String>` anywhere in the library.** Library callers match on concrete `domain::Error` variants. `anyhow` belongs in caller code (a CLI, a service) where erasure is ergonomic; matra itself stays on enums via `thiserror`.
- **PyErr routing is exhaustive at compile time.** Adding a variant to `domain::Error` will fail to compile until you wire it into `From<MatraError> for PyErr` with a specific Python exception class. The no-wildcard match exists so new variants do not silently route to `PyRuntimeError`.
- **Methods do not cross FFI. Only fields do.** Aggregate Rust methods (`Document::passive_ratio()`, `Corpus::total_words()`) are invisible to Python and (future) WASM consumers. If a value needs to be visible cross-language, materialize it as a field on a summary type, not a method.
- **Em dashes get rejected.** Project convention forbids them in documentation prose. `scripts/check-docsite-floor.sh` gate 5 rejects em dashes in `book/src/`; reviewers catch them elsewhere.
- **Publishing is hand-gated.** `cargo publish` and `maturin publish` are always preceded by `--dry-run`. The publish step itself requires explicit per-publish approval per the project memory. Do not script away the gate; it exists because publishing is irreversible and visible to every downstream consumer.

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
just docs-floor                                # the five docsite gates
maturin develop                                # Python local install
maturin build                                  # Python wheel
```

Features are additive: `udpipe` (default), `model2vec`, `python`, `cli`. **Do not run `cargo test --all-features`.** It enables `python`, which builds against libpython with symbols deliberately left undefined until the interpreter loads them, so it fails at link with an arm64 symbol error that looks like a regression and is not.

## DAO — practitioner agents

| Agent | When to use | File |
|-------|-------------|------|
| `maintainer` | Architectural decisions, adding features, fixing bugs, long-term maintenance | `.claude/agents/maintainer.md` |
| `reviewer` | PR reviews, boundary compliance audits, pre-release readiness checks | `.claude/agents/reviewer.md` |
| `portsmith` | Port trait design, extension points, Pattern 6 evaluation | `.claude/agents/portsmith.md` |
| `ffi-keeper` | PyO3 + future WASM/TS surface integrity, dual-publish discipline | `.claude/agents/ffi-keeper.md` |
| `resilience` | Failure modes, bounds, panics, TOCTOU, security, atomic operations | `.claude/agents/resilience.md` |
| `archivist` | CHANGELOG, ADRs, README, arch docs in lockstep with code | `.claude/agents/archivist.md` |

## Skills

| Skill | When to use | File |
|-------|-------------|------|
| `aces` | **Non-negotiable.** ACES design philosophy: Adaptable, Composable, Extensible. The three counter-forces to stasis/drag/opacity. Run the boundary test on every structural change. | `.claude/skills/aces/SKILL.md` |
| `rust-craft` | Rust design decisions: error tier, dep pin, trait shape, version pin | `.claude/skills/rust-craft/SKILL.md` |
| `testing` | Test strategy: regression discipline, property tests, complexity benches | `.claude/skills/testing/SKILL.md` |
| `architecture` | Hex boundary, port design, composition root, canonical pattern application | `.claude/skills/architecture/SKILL.md` |
| `ffi-surface` | PyO3 dual-publish: unsendable/Bound/pythonize/maturin/pin discipline | `.claude/skills/ffi-surface/SKILL.md` |
| `resilience-floor` | Taleb patterns: catch_unwind, atomic ops, TOCTOU closure, size caps | `.claude/skills/resilience-floor/SKILL.md` |
| `docs-lockstep` | CHANGELOG, ADRs, arch docs in sync with shipping code | `.claude/skills/docs-lockstep/SKILL.md` |

## Docsite

Content lives in `book/src/`. Every page describes what ships today; `book/src/roadmap.md` is the only page describing what does not, and `book/src/plans/` holds the plan for anything whose trigger has fired.

Gates run via `just docs-floor`: every page reachable from `SUMMARY.md`, every backticked type name resolving in `src/` (plans exempt, since a plan names types that do not exist yet), every link resolving, a clean build, and no em dashes outside quoted material.

Live preview: `cd book && mdbook serve --port 3000`. `create-missing = false`, so a `SUMMARY.md` entry without a file on disk fails the build loudly rather than creating a stub.

Diagrams are hand-authored inline SVG. Mermaid is not installed; the rule for choosing between them, and the command to restore mermaid when a sequence or state machine needs it, are in `book/book.toml`.

