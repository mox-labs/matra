# Vaani

NLP library. Text in, structured analysis out.

UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression, vocab TTR, nominalization, passive ratio), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

Rule evaluation over parsed text structure is part of the intended scope and lands in a later iteration; document references describe it as planned, not present.

## Session start: OODA and resume

On session start in this directory, read `.claude/logs/SESSION-RESUME.md` first. Run the OODA loop it specifies (Observe state, Orient on what is in flight, Decide the next action, Act), then continue with the operational sections below. The resume file is the durable surface that captures where the work stopped and what the next move is; keep it updated after each batch ships or any meaningful state change.

## Posture

vaani is a public OSS package intended as an exemplar for both Claude-managed repositories and human–AI collaborative intelligence. Two disciplines are non-negotiable:

- **ACES** — Adaptable, Composable, Extensible. The structural design philosophy resisting the stasis/drag/opacity cycle. Every structural change is checked against the ACES boundary test. See `.claude/skills/aces/SKILL.md`.
- **Antifragility** — the operational discipline. Size caps at entry, panic boundaries at C/C++ FFI, atomic file writes, TOCTOU closure, cycle-safe graph walks. See `.claude/skills/resilience-floor/SKILL.md`.

The quality bar is high because the public surface is a contract across Rust, Python, and (when the WASM crust lands) TypeScript. Names are forever; the API surface, once published, locks downstream costs in.

For the working model that frames how humans and AI collaborate on this project (roles, discourse-to-docs-to-code discipline, audit trail), see `docs/collaboration-model.md`. For PR mechanics, see `CONTRIBUTING.md`.

## Architecture

Hex architecture. Rust core with PyO3 Python bindings. Single crate, dual publish: `vaani` on crates.io, `vaani` on PyPI via maturin.

Pipeline: ingest → decompose → parse → measure (+ peer extract)

The five verbs are the public stage vocabulary. Trait names (`Source`, `Decomposer`, `NlpProvider`) keep their existing names; the renamed verbs appear in stage descriptions and composition-root function names.

Domain depends on port traits (NlpProvider, Decomposer, Source), not on adapters directly. UDPipe is the default NLP adapter, behind the `udpipe` feature flag.

```
src/
  lib.rs                    # composition root + PyO3 module (feature-gated)
  domain.rs                 # all domain types (only serde, thiserror, std)
  source/
    mod.rs                  # Source trait
    file.rs                 # FileSource adapter (symlink-rejecting, size-capped)
    directory.rs            # DirectorySource adapter (skips symlinks, sorted, per-file error tolerance)
  decompose/
    mod.rs                  # Decomposer trait
    markdown.rs             # MarkdownDecomposer adapter
    plain.rs                # PlainTextDecomposer adapter
  nlp/
    mod.rs                  # NlpProvider trait
    udpipe.rs               # UDPipe adapter (only file importing udpipe_rs)
  metrics/
    mod.rs                  # Metric alias, default_suite, attach_sentences
    readability.rs          # Flesch-Kincaid
    lexical.rs              # lexical density
    compression.rs          # brotli compression ratio
    document.rs             # vocabulary_ttr + nominalization_ratio
  extraction/
    mod.rs                  # re-exports
    tfidf.rs                # tfidf_summarize
    textrank.rs             # textrank_summarize (capped at MAX_SENTENCES)
    rake.rs                 # rake_keyphrases
    yake.rs                 # yake_keyphrases
  stopwords.rs              # shared utility
python/vaani/
  __init__.py               # re-exports Vaani from _core
  cli.py                    # click + rich CLI, auto-downloads model
scripts/
  fetch-model-hash.sh       # refresh ENGLISH_MODEL_SHA256 when version changes
  check-boundaries.sh       # enforces rules 3, 4, 2 in CI
  install-hooks.sh          # installs the pre-commit hook
  pre-commit-hook.sh        # local pre-commit gates
  changelog-release.sh      # rolls CHANGELOG + bumps version for release
tests/
  integration.rs            # full pipeline tests (require UDPipe model)
examples/
  basic.rs                  # getting-started example
```

## Boundary rules

1. `domain.rs` depends only on `serde`, `thiserror`, and `std`. Adding any other dependency requires an ADR.
2. Port modules (`source/mod.rs`, `decompose/mod.rs`, `nlp/mod.rs`) import only from `domain`.
3. No port module imports another port module.
4. `nlp/udpipe.rs` is the ONLY file that imports `udpipe_rs`.
5. `metrics/` and `extraction/` import only from `domain` and `stopwords`.
6. `cargo check --no-default-features` must compile.
7. Composition root (`lib.rs`) is the only place that knows all adapters and ports.

Rules 2, 3, 4 are enforced by `scripts/check-boundaries.sh` in CI. Rules 1, 5, 6, 7 are enforced by the type system and `cargo check`.

## Things that will bite you

Non-obvious gotchas. Each is a behavior plus the failure mode if you violate it.

- **Domain purity is hard-checked.** Adding `tokio` or `reqwest` (or anything beyond serde/thiserror/std) to `domain.rs` breaks `cargo check --no-default-features` and is caught at review. Adapters are where deps live; the domain stays pure.
- **Single UDPipe importer.** `scripts/check-boundaries.sh` fails CI if anything outside `nlp/udpipe.rs` imports `udpipe_rs`. The wrap exists because UDPipe holds non-Send C-side state and a panic at the FFI boundary would otherwise abort the host process. The catch_unwind seam lives inside this file by design; reintroducing direct imports elsewhere puts the panic boundary back in user code.
- **Per-paragraph parse, not whole-document.** The previous join-then-prefix-match approach silently reassigned sentences when two paragraphs shared their first 30 characters (FM1). Don't reintroduce "join paragraphs, parse once, wire sentences back to paragraphs by substring match." The pipeline parses each non-blockquote paragraph individually for a reason.
- **TOCTOU closes in `read_and_verify`.** The function returns `Vec<u8>` and the loader consumes those bytes via `Model::load_from_memory`. Never re-read the disk between hash verify and load — that opens the window a swap attack lives in.
- **Magic numbers in tree walks are forbidden.** `Sentence::tree_depth` returns `usize::MAX` on cycles; cycle detection uses a visited set, not `if depth > 20 { return }`. The previous magic-ceiling silently truncated malformed parses; the sentinel is the loud failure.
- **No `Result<T, String>` anywhere in the library.** Library callers match on concrete `domain::Error` variants. `anyhow` belongs in caller code (a CLI, a service) where erasure is ergonomic; vaani itself stays on enums via `thiserror`.
- **PyErr routing is exhaustive at compile time.** Adding a variant to `domain::Error` will fail to compile until you wire it into `From<VaaniError> for PyErr` with a specific Python exception class. The no-wildcard match exists so new variants do not silently route to `PyRuntimeError`.
- **Methods do not cross FFI. Only fields do.** Aggregate Rust methods (`Analysis::passive_ratio()`, `Corpus::total_words()`) are invisible to Python and (future) WASM consumers. If a value needs to be visible cross-language, materialize it as a field on a summary type, not a method.
- **Em dashes get rejected.** Project convention forbids them in documentation prose. The orwell voice pass catches them in book content; reviewers catch them elsewhere.
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
cargo test --test integration -- --ignored     # integration (needs model)
maturin develop                                # Python local install
maturin build                                  # Python wheel
```

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

## Docsite generation

For producing docsite content under `book/src/`: load `.claude/logs/bootstrap-fresh-docsite-generation.md`. The bootstrap specifies the IA target, the per-bucket specialist subsets (tufte / karman / burner / jobs / ace / chesterton / researcher applied per Diátaxis bucket), voice invariants, floor-gate requirements, and the cross-architecture verification protocol (`gemini -p` at each batch ship point). Foundational artifacts in `.rhet/` (ground truth, voice anchor, cartography) are the inputs each pipeline step reads.

Floor gates run via `just docs-floor`. Live preview via `cd book && mdbook serve --hostname 0.0.0.0 --port 3000` (mdbook 0.5.3; `create-missing = false` in `book.toml` so SUMMARY entries must exist on disk).

## Mastery references

The rust-mastery corpus at `~/radix-workspaces/rust-mastery/` is the architectural decision substrate. It is closed (12 of 12 milestones complete as of 2026-05-14) with ~150 Frames at file / crate / cross-artifact / milestone scales across 50+ Rust codebases.

For vaani specifically, the load-bearing Frames are:

| Frame | When to consult |
|------|------|
| `frames/cross-artifact/frame__cross-artifact__vaani-readiness.json` | Integrating M1 Frame — the complete architectural prescription for vaani, grounded in 6 cross-artifact + 11 file-Frames |
| `frames/cross-artifact/frame__cross-artifact__errors-tier-lib-vs-app.json` | Error tier discipline (thiserror at library tier, anyhow at app tier when applicable, `?` as the zero-cost seam) |
| `frames/cross-artifact/frame__cross-artifact__rust-python-dual-publish.json` | PyO3 + pythonize + maturin layered disciplines; 0.20 → 0.28 migration archaeology |
| `frames/cross-artifact/frame__cross-artifact__dtolnay-derive-style-ecosystem.json` | The 3-axis rule for `__private<patch>` versioning (internal-helpers / macro-rules / consumer-relationship) |
| `frames/cross-artifact/frame__cross-artifact__cli-ergonomics-and-app-discipline.json` | clap + ripgrep WalkParallel + per-file tolerance + broken-pipe handling |
| `frames/cross-artifact/frame__cross-artifact__typed-extension-config-trio.json` | inventory + typetag + serde for open-set polymorphic dispatch (deferred for vaani; relevant if extensibility surface ships) |
| `frames/cross-artifact/frame__cross-artifact__m8-i3-search-tier-pattern6-substrate-stability.json` | Pattern 6 criterion (separately publish a minimal port crate iff an external implementor ecosystem exists) |
| `frames/cross-artifact/frame__cross-artifact__cross-iteration-pattern-consolidation.json` | The four cross-iteration patterns: co-versioned-coupling, structural-rejection, vertical-layer-composition, isomorphic-dispatch |

The `.claude/arch/rust-mastery-audit.md` document maps these Frames to vaani's actual code and surfaces the remaining gaps. Read it before architectural decisions.

For workflow scaffolds (ci-scaffolds, problem-solving, crafting, collaborating), see the global skills under `~/.claude/`.
