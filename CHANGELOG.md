# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
Per-release sections follow this shape:

  ## [X.Y.Z] - YYYY-MM-DD

  ### Highlights

  Two to four prose entries explaining the load-bearing changes for
  this release. Each entry teaches the mental model the change is
  built on, not just what shipped. Reserved for architectural decisions,
  breaking changes, security-relevant fixes, and deferred-vs-shipped
  tradeoffs. Aim for 150-400 words per entry. Bug fixes and minor
  refactors live in the structured sections below, not here.

  ### Added / Changed / Deprecated / Removed / Fixed / Security

  Terse Keep-a-Changelog bullets for everything else.

Style: no em dashes (project convention).
Rollover: scripts/changelog-release.sh moves [Unreleased] -> [X.Y.Z]
at release time.
-->

## [Unreleased]

### Highlights

#### Why `Analysis` became `Document`

The type that holds parsed output (sections, paragraphs, sentences, tokens, metric slots) was named `Analysis`. That name reads as a verb, an analytical act. Vaani does not analyze. Vaani structures. The interpreter analyzes. Naming the substrate type after the act it does not perform collapses the line the conviction page argues most carefully.

The rename ships across `src/`, `python/`, `book/`, `docs/`, `examples/`, `tests/`, `.claude/arch/`, and this changelog. A transitional `pub type Analysis = Document;` with a `#[deprecated]` annotation keeps in-flight branches and just-published downstream snippets compiling through the 0.0.x line. The alias removes in 0.1.0.

ADR-0006 locks the abstract-tier vocabulary at the same time. Twelve names reserved: `Document` (live), `Finding`, `SourceSpan`, `Relation`, `Schema`, `Modality`, `SpeechAct`, `Stylometry`, `Rule`, `Predicate`, `Pattern`, `ParagraphKind`, `ParagraphRole`, `ParagraphMetrics`, `DocumentMetrics`. The umbrella for extraction outputs is `Finding`, NOT `Frame`; ADR-0002 reserved `Frame` for Fillmore semantic frames and this ADR preserves that reservation by routing the umbrella through `Finding`. Shape decisions (trait vs enum for `Finding`, enum for `ParagraphKind`, outer-vs-inner Option for metric grouping) are deferred to v0.2 when concrete consumer sites force them. Names settle now; shapes settle later.

`Paragraph.in_blockquote` carries a rustdoc deprecation note pointing at the planned `ParagraphKind` enum. The boolean stays in 0.0.x and 0.1.x because its job (gate measure or not) is binary today. The migration ships when the variant inventory is justified by real consumer semantics.

The M0 floor gate's type-name parity check (`scripts/check-docsite-floor.sh`) protects the rename mechanically: any stale `Analysis` reference in `book/src/` that did not flow through the rename becomes a CI failure. This is the rename's stress test against the floor gates established in M0.

#### Why supply-chain hardening lands now

Vaani is a substrate. Downstream consumers inherit vaani's supply-chain posture transitively: if vaani publishes from a workflow with a long-lived crates.io API token, every consumer transitively depends on that token never leaking. If vaani's GitHub Actions are pinned to floating tags, a hostile force-push to one of those tags can be injected into every downstream build through vaani's CI cache. The substrate's trust posture is load-bearing across the dependency tree.

The deep-research pass on 2026-05-20 surfaced `sbom-tool/gh-guard` — a Claude Code plugin codifying current best practices for Rust supply-chain hardening (OpenSSF Scorecard, CodeQL, action SHA-pinning, Trusted Publishing, SLSA L3 provenance, signed tags). i9 adopts the lower-friction, higher-signal half of that posture: SHA-pinned actions, persist-credentials false on every checkout, workflow-level `permissions: read-all`, weekly Scorecard + CodeQL analyses, and a tag-driven publish workflow gated by a GitHub `crates-io` environment with required-reviewer approval. SLSA L3 provenance ships alongside every release via `slsa-github-generator@v2.1.0`.

The non-negotiable per-publish approval rule (memory: never publish without explicit approval) moves from "I ran cargo publish on my laptop" to "I approved deployment NNNN in the Actions UI". Same human intent, stronger enforcement, better audit trail. The workflow cannot fire `cargo publish` without the human click.

ADR-0005 records the decision, the manual setup the maintainer performs once on GitHub.com and crates.io, and the items deliberately deferred (cargo-vet, fuzz testing, osv-scanner config, binary releases, CII badge). Each deferred item carries an explicit trigger to revisit.

#### Why vaani's framing reset to "NLP library"

The repository previously described itself as a "prose metrics engine." That framing is wrong. Vaani is an NLP library: UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics, summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE), with rule evaluation over parsed text structure as part of the intended scope (planned, not yet shipped). The package is intended as an exemplar for Claude-managed open-source repositories and human–AI collaborative intelligence; the public surface is a contract across Rust and Python (and TypeScript when the WASM crust lands), so identity precision matters.

The reset sweeps `Cargo.toml` description and keywords, `pyproject.toml` description, README's elevator pitch, `CLAUDE.md`, the Python crust docstrings, and the entire `.claude/arch/` set. The aspirational two-crate workspace (`vaani-core` + a sibling matcher-bridge crate), an `Engine` struct, an `analyze_directory_iter` API, a `VaaniError` type with `kind`/`is_fatal`/`is_skip_doc` accessors, an `otel` feature, and "tracing always on" — none of which exist in `src/` — are all purged from the docs. Each `.claude/arch/*.md` now describes the actual single-crate shipping code; the rust-mastery audit at `.claude/arch/rust-mastery-audit.md` is the read-only gap analysis that informs which Frame prescriptions vaani follows and which are deferred.

ADR-0003 (proposed workspace split) is superseded by ADR-0004 (stay single-crate). The decision is re-openable when Pattern 6 from the rust-mastery corpus fires — i.e., when a third-party `NlpProvider` implementor crate ships.

#### Why the domain Error enum is now derived via thiserror

The previous `domain::Error` was an enum with hand-rolled `impl Display`, `impl Error`, and `impl From<std::io::Error>` — roughly 35 lines of structural plumbing that `#[derive(thiserror::Error)]` would have emitted. The boundary rule "only `serde`, `std` in `domain.rs`" forbade thiserror, but per the rust-mastery corpus's M1.i6 ecosystem Frame, thiserror uses `__private<patch>` versioning (axis 1: internal helpers) so multi-version dep graphs are safe by design, and the derive output never appears in vaani's public API. Relax the rule, adopt the derive, delete the plumbing.

At the PyO3 boundary, `From<VaaniError> for PyErr` now routes per variant: `ModelNotFound` → `PyFileNotFoundError`, `InputTooLarge` / `UnsupportedFormat` → `PyValueError`, `Io(_)` → `PyOSError`, `ModelInvalid` / `ParseFailed` → `PyRuntimeError`. The match is exhaustive (no wildcard) so a new variant added to `domain::Error` becomes a compile error at the routing site, surfacing the choice rather than silently routing everything to `PyRuntimeError`. Concrete variant identity now survives the FFI as the right Python exception class — Python consumers can write `try ... except FileNotFoundError` and have it work.

#### Why vaani now has a DAO and a typed Python surface

Vaani is a public OSS package and an intended exemplar. The standards need to be navigable, not implicit. Two structural changes land:

A **diverse agent organization** under `.claude/agents/` (6 practitioner agents: maintainer, reviewer, portsmith, ffi-keeper, resilience, archivist) plus a **skill library** under `.claude/skills/` (7 skills: `aces`, `rust-craft`, `testing`, `architecture`, `ffi-surface`, `resilience-floor`, `docs-lockstep`). Each agent has a defined scope grounded in vaani's actual surface; each skill cites the specific Frames from the rust-mastery corpus that ground its disciplines. **ACES** (Adaptable, Composable, Extensible — resisting the stasis/drag/opacity decay cycle) and **antifragility** (size caps, panic boundaries, atomic operations, TOCTOU closure) are non-negotiable foundations, called out explicitly in the reviewer's check gates and the maintainer's discipline list.

A **fully typed Python crust** with stubs at `python/vaani/_core.pyi` describing the PyO3 extension's TypedDict shapes (`Token`, `Sentence`, `Paragraph`, `Section`, `Document`, `ScoredSentence`, `Keyphrase`) mirroring the Rust domain types. The `Vaani` class has full method signatures with the Python exception classes the PyO3 boundary now raises per variant. `python/vaani/py.typed` declares the package typed per PEP 561; `pyproject.toml` configures `mypy --strict` over the `python/vaani` tree; `justfile` adds a `typecheck` recipe; CI runs `mypy` in a new `pytype` job after building the extension with `maturin develop`. Downstream Python consumers get full IDE autocomplete and type-checking on the public surface.

#### Why model loads are now TOCTOU-safe

The previous flow had two separate disk reads of the same file: `verify_file(path)` would read the bytes, hash them, and confirm the SHA-256 matched the pinned constant; then `Model::load(path)` would re-read the file from disk to build the model. The window between those reads is a Time-Of-Check to Time-Of-Use race. An attacker with write access to the model directory could let the verify pass on the legitimate bytes, then swap the file with a malicious model before the loader's read. The hash check would have done its job on bytes A, but the loaded model would be bytes B.

The fix is structural rather than procedural: read once, hash the in-memory bytes, then pass *those same bytes* directly to `Model::load_from_memory`. There is no second disk read for an attacker to interpose on. The property is provable from the code shape: there is no second read, so there is no swap to make. `read_and_verify` returns `Option<Vec<u8>>` so the verified bytes are the bytes the loader consumes, by construction.

The unit test simulates the attack directly: write known content, verify, swap the file with different content, then assert the in-memory bytes are still the verified ones. Pre-fix the equivalent flow would have failed; post-fix the swap is irrelevant.

This was Vector's MEDIUM finding in the I2 guild review. Cheap to close (one fewer read, in fact), provable from code shape.

#### Why every paragraph is parsed individually now

The previous pipeline joined every non-blockquote paragraph into one prose string, fed it to UDPipe, then ran a wiring step (`metrics::attach_sentences`) that matched returned sentences back to paragraphs by 30-char prefix substring match. Three converging defects: prefix-collision misassigned sentences when paragraphs shared their first 30 characters (formulaic prose like "The system processes X. ..." repeated across paragraphs); inner-substring theft moved a sentence into the wrong paragraph when its prefix happened to appear mid-text in another paragraph; and the wiring scan was O(paragraphs × sentences) with `String::contains`, which on book-length input is roughly 10^10 character comparisons.

The fix eliminates the wiring step entirely. The composition root now calls `nlp.parse(&paragraph.text)` once per non-blockquote paragraph and assigns the returned sentences directly to that paragraph. There is no document-level join, no string-prefix recovery, no ambiguity. A paragraph's sentences are exactly what came back from parsing it.

This is a structural change that closes three defects at once, and it is the kind of fix that compounds: the next contributor who adds a new metric does not need to think about sentence wiring at all, because there is no wiring to think about. The mental model shrinks.

Two regression tests were added: same-prefix paragraphs (FM1) and inner-substring theft. Both would have demonstrated the pre-fix bug; both pass cleanly post-fix.

#### Why each extractor has its own cap with its own label

TextRank has had a 2,000-sentence cap since the original v2 plan, justified by arithmetic: a 2000 × 2000 similarity matrix of f64 is 32 MiB, which is the load-bearing memory cost. The other three extractors had no caps. A naive fix would have been to give them all the same 2,000-sentence cap. That would have been wrong.

TF-IDF, RAKE, and YAKE have different cost drivers. TF-IDF's cost is per-sentence HashMap construction, so a sentence-bound is right (and 2,000 fits the same cost class). RAKE and YAKE's cost is per-token: they build co-occurrence maps and n-gram candidate sets keyed on phrase strings, and their cardinality grows with token count, not sentence count. A chat-log corpus with 50,000 one-token sentences fits comfortably under any sentence cap and still blows up the candidate map. RAKE and YAKE need *token* caps.

Each extractor now returns `Error::InputTooLarge { what: <"tfidf"|"rake"|"yake">, .. }` with a distinct label so consumers can route on the kind. The constants are intentionally separate (not shared with TextRank) because each is justified by its own arithmetic; sharing would couple unrelated cost models and let a future TextRank tuning silently change TF-IDF's behavior.

The discipline this encodes: a cap is not a number. A cap is an arithmetic comment plus a number. The number is the second-class deliverable; the comment is what makes the number defensible.

### Added

- ADR-0006 documenting the rename of `Analysis` to `Document` and locking the abstract-tier vocabulary (`Finding`, `SourceSpan`, `Relation`, `Schema`, `Modality`, `SpeechAct`, `Stylometry`, `Rule`, `Predicate`, `Pattern`, `ParagraphKind`, `ParagraphRole`, `ParagraphMetrics`, `DocumentMetrics`). Shape decisions deferred to v0.2; names reserved now to avoid SemVer-major cost later.
- Transitional `pub type Analysis = Document;` alias in `src/domain.rs` with `#[deprecated(since = "0.0.1")]`. The alias removes in 0.1.0; the M0 floor gate's type-name parity check catches reintroductions of the old name in docs.
- `.github/workflows/scorecard.yml` — OpenSSF Scorecard analysis weekly + on push to main; SARIF uploaded to the Security tab and published publicly to scorecard.dev. Per-job permissions grant only `security-events: write` and `id-token: write`.
- `.github/workflows/codeql.yml` — CodeQL static analysis for Rust + Python on push, PR, and weekly. `build-mode: none` for both. Manual prerequisite: disable the "default setup" in repo Settings → Code security → Code scanning before this workflow can run.
- `.github/workflows/publish.yml` — tag-driven publish to crates.io. The `publish` job declares `environment: crates-io`, which pauses the workflow until the required reviewer (maintainer) approves in the Actions UI. Trusted Publishing (OIDC via `rust-lang/crates-io-auth-action`) mints a scoped token just-in-time, eliminating the long-lived `CARGO_REGISTRY_TOKEN` secret. Pre-publish verifications: tag/Cargo.toml version match, tag-on-main check, cargo test + cargo deny + cargo publish --dry-run. SLSA L3 provenance via `slsa-github-generator@v2.1.0` attached to the auto-generated GitHub Release.
- ADR-0005 documenting the supply-chain hardening posture, the action SHA pin table, manual setup on GitHub.com and crates.io, deferred items with re-visit triggers, and validation criteria.
- `From<domain::Error> for PyErr` at the PyO3 boundary routes per variant: `ModelNotFound` → `PyFileNotFoundError`; `InputTooLarge` / `UnsupportedFormat` → `PyValueError`; `Io(_)` → `PyOSError`; `ModelInvalid` / `ParseFailed` → `PyRuntimeError`. Exhaustive match so adding a new variant becomes a compile error at the routing site.
- Practitioner agents (`.claude/agents/`): `maintainer`, `reviewer`, `portsmith`, `ffi-keeper`, `resilience`, `archivist`. Each agent's scope is vaani-tuned and grounded in specific Frames from the rust-mastery corpus.
- Skills (`.claude/skills/`): `aces`, `rust-craft`, `testing`, `architecture`, `ffi-surface`, `resilience-floor`, `docs-lockstep`. Each skill codifies a discipline with citations to the corpus Frames it grounds in.
- `.claude/arch/rust-mastery-audit.md` — read-only gap analysis against the rust-mastery corpus at `~/radix-workspaces/rust-mastery/`.
- Python type stubs (`python/vaani/_core.pyi`) describing the PyO3 extension's surface with TypedDict shapes for the domain types.
- `python/vaani/py.typed` PEP 561 marker; downstream type checkers honor the stubs.
- `mypy` strict-mode configuration in `pyproject.toml`; `just typecheck` recipe; CI `pytype` job running `mypy` after `maturin develop`.
- ADR-0004 documenting the single-crate decision and the conditions that would re-open the workspace-split question (Pattern 6 criterion: external `NlpProvider` implementor ecosystem).
- `.mise.toml` pinning Rust 1.85 to match `Cargo.toml`'s `rust-version`.
- `MAX_INPUT_BYTES = 8 MiB` constant in `domain.rs` and gates at every composition-root entry point (`analyze`, `analyze_markdown`, `parse`, `analyze_from`). Returns `Error::InputTooLarge { what: "input", .. }`.
- Per-extractor input caps: `tfidf::MAX_SENTENCES = 2000`, `rake::MAX_TOKENS = 200_000`, `yake::MAX_TOKENS = 200_000`. Each with arithmetic comment and a distinct `what:` label on `InputTooLarge`.
- `DirectorySource::read_collecting_errors` inherent method returns `(Vec<RawDocument>, Vec<(PathBuf, Error)>)` so per-file I/O failures can be surfaced without aborting the iteration. `analyze_directory` uses this to merge ingest failures with analysis failures.
- `FileSource` rejects symlinks via `symlink_metadata` and rejects files larger than `MAX_INPUT_BYTES` before any read. Closes the asymmetry where `DirectorySource` skipped symlinks but `FileSource` did not.

### Changed

- `Analysis` type renamed to `Document` across `src/`, `python/`, `book/`, `docs/`, `examples/`, `tests/`, `.claude/arch/`. The Python TypedDict, the PyO3 `Vaani` class methods' return shapes, and every rustdoc + book reference all use the new name. The transitional `Analysis` alias (deprecated) preserves source compatibility through the 0.0.x line. Rustdoc note on `Paragraph.in_blockquote` flags the planned `ParagraphKind` enum migration.
- All GitHub Actions in `.github/workflows/*.yml` pinned to 40-character commit SHAs with `# version` comments. Floating tags (`@v4`, `@stable`, etc.) are no longer allowed; Dependabot keeps the SHAs current via the weekly `github-actions` ecosystem update. Initial pin set follows `gh-guard v0.2.1`'s `templates/versions.json`.
- Every `actions/checkout` step now sets `persist-credentials: false`, preventing a compromised workflow step from pushing back to the repo using the runner's `GITHUB_TOKEN`. Every workflow declares `permissions: read-all` at the workflow level; jobs escalate only what they need (`security-events: write` for SARIF upload, `id-token: write` for OIDC, `contents: write` for release artifacts).
- `justfile`'s `release` recipe no longer prints the `cargo publish` command; the publish itself moves to `.github/workflows/publish.yml`'s `crates-io` environment gate. The recipe now prints the signed-tag (`git tag -s`) and tag-push commands.
- `domain::Error` is now derived via `#[derive(thiserror::Error)]` with per-variant `#[error("…")]` annotations. The hand-rolled `Display`, `Error`, and `From<io::Error>` impls collapse into the derive output (~35 lines removed). Variant identity preserved; behavior unchanged. Boundary rule "only `serde`, `std` in `domain.rs`" is relaxed to admit `thiserror` per the rust-mastery corpus's M1.i6 ecosystem Frame (thiserror is multi-version safe via `__private<patch>` versioning and never appears in the public API).
- `Cargo.toml` and `pyproject.toml` descriptions, README header, `CLAUDE.md` preamble, and `python/vaani/__init__.py` docstring reframed from "Prose metrics engine" to "NLP library" per user direction 2026-05-20. Keywords and categories on `Cargo.toml` updated accordingly (`udpipe`, `parsing`, `summarization` join `nlp`; `science` category added).
- `.claude/arch/*.md` rewritten end-to-end to describe the actual single-crate shipping code. The previous aspirational documentation (two-crate workspace with `vaani-core` + sibling matcher-bridge crate, `Engine` struct, `analyze_directory_iter`, `VaaniError` with `kind`/`is_fatal`/`is_skip_doc`, `otel` feature, "tracing always on") is purged. All `rumi-*` references removed.
- `CLAUDE.md` "Mastery References" section repointed from non-existent `~/oss/research/` paths to the actual rust-mastery corpus at `~/radix-workspaces/rust-mastery/`, citing the specific cross-artifact Frames that ground vaani's architectural decisions.
- ADR-0003 (proposed Cargo workspace with `vaani-core` + `rumi-nlp`) marked Superseded; ADR-0004 documents the single-crate decision and the Pattern 6 re-open condition.
- Pipeline vocabulary settled at `ingest → decompose → parse → measure` with `extract` as a peer stage. The `Source`, `Decomposer`, and `NlpProvider` traits keep their existing names; the renamed verbs appear in stage descriptions and the composition-root surface.
- Removed `decompose::markdown::parse` free function. Markdown decomposition now goes through `decompose::markdown::MarkdownDecomposer.decompose(text)` (the `Decomposer` trait method). This frees the verb `parse` for NLP-only use across the pipeline. Breaking change for callers using the free function.
- `Sentence::tree_depth` is now O(n) per sentence via HashMap-indexed bottom-up DFS with depth memoization. The previous magic `< 20` ceiling is gone. On a malformed parse with a cycle, returns `usize::MAX` (sentinel) rather than silently truncating to 20.
- `tfidf::summarize`, `rake::keyphrases`, `yake::yake_keyphrases` now return `Result<Vec<...>>` instead of `Vec<...>` (breaking). Required so they can return `Error::InputTooLarge` on cap exceedance.
- `analyze_from` now returns `Result<Document>` instead of `Document` (breaking). Required so it can return `Error::InputTooLarge` on aggregate section bytes exceeding the cap.
- The brotli compression metric uses `lgwin = 18` (256 KiB window, down from 4 MiB) and skips paragraphs exceeding `MAX_PARAGRAPH_BYTES = 256 KiB`.
- The composition root parses each non-blockquote paragraph individually and assigns the returned sentences to that paragraph. The previous join-then-prefix-match wiring step is removed.

### Removed

- `metrics::attach_sentences`. The wiring step it performed is no longer needed under per-paragraph parse; sentences arrive at metrics already attached to their originating paragraphs.

### Security

- UDPipe `Model::parse` is wrapped in a `catch_unwind` boundary. A C-side panic at the udpipe-rs FFI boundary becomes `Err(Error::ParseFailed)` with the captured panic message rather than aborting the host process. In Python this manifests as a catchable `PyRuntimeError` instead of interpreter death.
- Model verification is now TOCTOU-safe: `read_and_verify` returns the verified bytes themselves, and `Self::from_bytes` consumes those bytes via `Model::load_from_memory` without a second disk read. An attacker swapping the file post-verify cannot influence the loaded model.
- Concurrent `Udpipe::english(same_dir)` calls no longer race: each downloads to a per-process temporary subdirectory (`.tmp.download.<pid>`), and the rename to the final path is atomic on the same filesystem.
- `FileSource` refuses symlinks and refuses files exceeding `MAX_INPUT_BYTES` before reading.

## [0.1.0] — 2026-04-15

Initial public release.

### Added

- Rust core crate `vaani` with hexagonal architecture:
  - `Source` port (`FileSource`, `DirectorySource`) with format detection
  - `Decomposer` port (`MarkdownDecomposer`, `PlainTextDecomposer`)
  - `NlpProvider` port with `Udpipe` adapter behind the `udpipe` feature flag
  - `metrics/` suite — readability (Flesch-Kincaid), lexical density, compression ratio, vocabulary TTR, nominalization ratio
  - `extraction/` — `tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`
- Domain model (`domain.rs`) with matchable `Error` enum, `#[non_exhaustive]` across public types, `Token::builder` for forward-compatible construction
- Python bindings via PyO3 behind the `python` feature, exposed as the `Vaani` class with `analyze`, `analyze_markdown`, and the four extraction methods
- Python CLI (`vaani`) with `analyze`, `summarize`, `keyphrases` commands
- Automatic English model download + SHA-256 verification against a pinned constant
- `scripts/fetch-model-hash.sh` to refresh the pinned hash when the model version changes

### Security

- `textrank_summarize` caps input at `MAX_SENTENCES = 2000` and returns `Error::InputTooLarge` above that, bounding the O(n²) similarity matrix
- `DirectorySource` skips symlinks to avoid following attacker-controlled paths
- English model download is verified against a pinned SHA-256; mismatched files are removed and re-downloaded once before erroring
- `Error::UnsupportedFormat` returned for `Pdf`/`Docx` sources rather than silently treating binary content as plain text

### Known limitations (tracked for 0.2)

- `DirectorySource` aborts on the first filesystem read error; per-file I/O tolerance is planned
- `analyze_directory` fully materializes results in memory; a streaming iterator API is planned
- Derived analysis metrics (`total_sentences`, `passive_ratio`, `mean_sentence_length`) are Rust methods and are not visible in serialized output; Python/WASM consumers must recompute or a `ProseSummary` sealed struct will be added
- No `abi3` PyO3 feature — wheels are built per Python version
