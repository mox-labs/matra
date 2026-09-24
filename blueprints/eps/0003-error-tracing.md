# I3: Error restructure + tracing PR1

**Status:** not-started
**Boundary:** **MVP**. at the end of this iteration, matra is correct, bounded, has a recovery contract, and is observable.
**Depends on:** I2 (resilience floor)
**Branch:** `i3/error-tracing` off the I2 commit

## Why this iteration exists

Two things land together because they co-design.

**Errors carry the recovery contract.** Today's `Error::ParseFailed(String)` and `Error::ModelInvalid(String)` are stringly-typed. A downstream consumer cannot distinguish "skip this document" from "the model is gone, abort the batch" without parsing the error message. Each consumer reinvents string-matching against matra's error wording, breaking on the first message rewrite.

**Errors carry the diagnosis nowhere visible.** Today, by the time the caller sees an error, the structured context (which file, which sentence index, which token) is gone. A substrate library that loses context at the boundary makes its consumers debug blind.

The fix is paired:
- **Variant carries the recovery contract** (Taleb + Dijkstra + Ace + Burner + Chesterton converged).
- **Tracing carries the diagnosis** (Wolf): a structured `tracing::error!` event before every `Err(...)` propagation, with path, kind, and span context.

Splitting these into two iterations creates two passes over the same call sites. Doing them together is one pass.

K's verdict (2026-04-28): "PR3: Error restructure + Wolf PR1 together. They co-design: `Error::ParseFailed { kind }` and `is_skip_doc/is_fatal` are exactly what the `tracing::error!` events need to carry. Splitting them creates two passes over the same call sites."

## What lands

### Task A: `Error` restructure

**Files:** `src/domain.rs:14-65`, every `Err(Error::*)` site outside `domain.rs` and tests.

**Why (Taleb + Dijkstra + Chesterton):**
- Taleb: "Add one boolean and one variant — no rewrite. `ParseFailKind` distinguishes `Empty` / `MalformedInput` / `ProviderInternal` / `ResourceLimit`. `recoverable: bool` on `ModelInvalid` separates `retry-with-redownload` from `model is gone`."
- Dijkstra: "`is_fatal()` is **not** decidable from variant alone today. Split `Error::Io` into `Error::SourceIo { path, kind }` (skip-doc class) and `Error::ModelIo { path, kind }` (fatal class)."
- Chesterton fence 7: "No pre-existing matchable variants (`ModelNotFound`, `ModelInvalid`, `Io`, `InputTooLarge`, `UnsupportedFormat`) get folded inside `ParseFailKind` — those are publicly named in CHANGELOG."

**Steps:**

1. Define new variants in `src/domain.rs`:
   ```rust
   #[non_exhaustive]
   pub enum ParseFailKind {
       Empty,
       MalformedInput,
       ProviderInternal,
       ResourceLimit,
   }

   #[non_exhaustive]
   pub enum Error {
       ModelNotFound(PathBuf),
       ModelInvalid { message: String, recoverable: bool },
       ParseFailed { kind: ParseFailKind, message: String },
       InputTooLarge { limit: usize, actual: usize, what: &'static str },
       UnsupportedFormat(Format),
       SourceIo { path: PathBuf, kind: io::ErrorKind },
       ModelIo { path: PathBuf, kind: io::ErrorKind },
   }
   ```
2. **Confirm `#[non_exhaustive]` on every variant of both enums** (Chesterton fence 2). Verify with `cargo expand`.
3. Implement accessors:
   ```rust
   impl Error {
       pub fn is_skip_doc(&self) -> bool { /* per truth table in domain-model.md */ }
       pub fn is_fatal(&self) -> bool { /* per truth table */ }
       pub fn parse_kind(&self) -> Option<&ParseFailKind> { /* extracts kind from ParseFailed */ }
   }
   ```
4. Update every existing `Err(Error::Io(e))` site:
   - In `src/source/file.rs`, `src/source/directory.rs`: `SourceIo { path, kind: e.kind() }`.
   - In `src/nlp/udpipe.rs` model-load paths: `ModelIo { path, kind: e.kind() }`.
   - In any other I/O site: classify by what it touches. When in doubt, `SourceIo` (skip-doc default).
5. Update every existing `Err(Error::ParseFailed(s))`:
   - In `src/nlp/udpipe.rs`: classify the failure cause.
     - Empty input → `ParseFailKind::Empty`.
     - `udpipe-rs` returned no sentences for non-empty input → `MalformedInput`.
     - C-side panic (caught by I2 task E) → `ProviderInternal`.
     - Input over `MAX_INPUT_BYTES` (already gated at composition root, but if it slips through) → `ResourceLimit`.
6. Update every existing `Err(Error::ModelInvalid(s))`:
   - Hash mismatch on fresh download → `recoverable: false`.
   - Truncated file or mid-download corruption → `recoverable: true`.
7. Update PyO3 mapping in `src/lib.rs::python`:
   - Define `MatraError` exception class with attributes `kind`, `is_fatal`, `is_skip_doc`, `path` (per Ace's recommendation).
   - Map `Error::ModelNotFound` → `PyFileNotFoundError` (already done).
   - Map `Error::InputTooLarge` → `PyValueError`.
   - Map everything else → `MatraError` with attributes populated.
8. Truth-table tests in `src/domain.rs#[cfg(test)]`:
   - Each variant's `is_skip_doc`/`is_fatal` matches the truth table.
   - `parse_kind` returns `Some(&kind)` for `ParseFailed`, `None` otherwise.

**Acceptance:**
- `cargo expand` confirms `#[non_exhaustive]` on `Error` and `ParseFailKind`.
- Truth-table test passes.
- No `Err(Error::Io(_))` sites remain in `src/`. `rg 'Error::Io\b' src/` returns hits only in `domain.rs` (the variant definition history) and possibly the `From<io::Error>` impl if kept for legacy compatibility (recommend removing).
- Python `MatraError` exception preserves `kind`, `is_fatal`, `is_skip_doc` across FFI.

### Task B: install `tracing` as an always-on dependency

**Files:** `Cargo.toml`, `Cargo.lock`.

**Why (Wolf):** "`tracing` is always-on. Cost is small: `tracing` + `tracing-core` + `pin-project-lite` + `once_cell`, all already pulled or trivially small. With no subscriber installed, the macros compile to near-nothing — substrate-safe."

**Steps:**

1. In `Cargo.toml`:
   ```toml
   [dependencies]
   tracing = { version = "0.1", default-features = false, features = ["std", "attributes"] }
   tracing-opentelemetry = { version = "0.27", optional = true }  # gated under `otel` feature for I5
   opentelemetry = { version = "0.27", optional = true }

   [features]
   otel = ["dep:tracing-opentelemetry", "dep:opentelemetry"]
   ```
2. Update `CLAUDE.md` to record the rule 8 amendment (Burner):
   > Rule 8: `tracing` lives only in adapters and `lib.rs`. Never in `domain.rs` or port modules.
3. Add `tracing` to no other section of the codebase yet (Task C handles that).

**Acceptance:** `Cargo.toml` updated. `cargo check` passes (the dep is present but not used yet).

### Task C: span topology

**Files:** `src/lib.rs`, `src/source/file.rs`, `src/source/directory.rs`, `src/decompose/markdown.rs`, `src/decompose/plain.rs`, `src/nlp/udpipe.rs`, `src/metrics/mod.rs`, `src/metrics/{readability,lexical,compression,document}.rs`, `src/extraction/{tfidf,textrank,rake,yake}.rs`.

**Forbidden:** `src/domain.rs`, `src/source/mod.rs`, `src/decompose/mod.rs`, `src/nlp/mod.rs` (rule 8).

**Why (Wolf):** "INFO spans on every pipeline-stage entry point, DEBUG per metric, structured event before every `Err(...)`. Manual spans only — no `#[instrument]` because it captures full text args."

**Steps:**

1. Wrap each composition-root entry in an INFO span:
   ```rust
   pub fn analyze(text: &str, nlp: &dyn NlpProvider) -> Result<Analysis> {
       let span = tracing::info_span!("matra.analyze", bytes = text.len(), format = "plain");
       let _enter = span.enter();
       /* existing logic */
   }
   ```
   Apply to `analyze`, `analyze_markdown`, `analyze_file`, `analyze_directory`, `parse`, `analyze_from`. The fields on close should record `paragraph_count`, `sentence_count`, `total_words`.
2. Wrap `Udpipe::parse` in `tracing::info_span!("matra.nlp.parse", provider = "udpipe", bytes = text.len())`.
3. Wrap `metrics::run_suite` in `tracing::info_span!("matra.metrics.run_suite", metric_count = suite.len())`.
4. Per-metric DEBUG spans: in each `metrics/{readability,lexical,compression,document}.rs::compute`, wrap in `tracing::debug_span!("matra.metric", metric = "readability")` etc.
5. Per-extractor INFO spans: in each `extraction/{tfidf,textrank,rake,yake}.rs`, wrap the entry in `tracing::info_span!("matra.extract", algo = "tfidf", n_in = sentences.len())`.
6. Source spans: `tracing::info_span!("matra.source.read", path = ?path)` on `FileSource::read` and `DirectorySource::read`.
7. **No `#[instrument]` macro on public API.** Manual spans with explicit fields only. The text content is not a span field; only `bytes = text.len()`.
8. **No spans inside `Sentence::tree_depth`/`subtree`/`children_of`** (Wolf forbidden).

**Acceptance:**
- A test using `tracing-subscriber::fmt::test::TestWriter` captures emitted spans for one `analyze` call. Asserts the expected span hierarchy: `matra.analyze` > `matra.nlp.parse` > `matra.metrics.run_suite` > per-metric DEBUG.
- `bash scripts/check-boundaries.sh` passes (rule 8 still holds).

### Task D: error events at every `Err(...)` site

**Files:** every adapter and `src/lib.rs`. Not `src/domain.rs` (rule 8).

**Why (Wolf):** "Every `return Err(...)` outside `domain.rs` and outside test modules must have a sibling `tracing::error!` (or `warn!` for recoverable) on the same path. The `analyze_directory` per-file errors must each emit `matra.document.failed { path, error_kind }` so downstream OTel sees a per-failure event, not just the aggregate count."

**Steps:**

1. For every `Err(Error::*)` return in `src/lib.rs`, `src/source/*.rs`, `src/decompose/*.rs`, `src/nlp/udpipe.rs`, `src/extraction/*.rs`, `src/metrics/*.rs`: precede the return with a structured event.
   - For `is_skip_doc()` errors: `tracing::warn!(?path, ?kind, "matra.document.skipped")`.
   - For `is_fatal()` errors: `tracing::error!(?path, ?kind, recoverable = false, "matra.fatal")`.
   - For other errors (`InputTooLarge`, etc.): `tracing::warn!(what, limit, actual, "matra.input.too_large")`.
2. The event field names match the variant fields. A consumer can correlate a span event to the returned `Error` by structured fields, not by message strings.
3. Tests: a `TestWriter`-based test captures events for a fault-injected fixture. Asserts the right event level and field shape.

**Acceptance:**
- `rg 'return Err\(' src/ --glob '!src/domain.rs' --glob '!**/tests.rs'`, every line has a sibling `tracing::warn!` or `tracing::error!` within 5 lines above it. Manual review checklist; consider a custom clippy lint as a follow-up in I5.
- `matra.document.failed { path, error_kind }` event emitted on every `analyze_directory` per-file error.

### Task E: examples and docs

**Files:** `examples/observability.rs` (new), `README.md`, `CHANGELOG.md`.

**Why:** the consumer needs a working example to opt into observability without reading the source.

**Steps:**

1. `examples/observability.rs` shows two patterns:
   - Local dev: `tracing_subscriber::fmt().with_env_filter("matra=info").init()`.
   - Production-bound non-blocking writer: `let (writer, _guard) = tracing_appender::non_blocking(std::io::stderr())`.
2. README: brief "Observability" section pointing at the example.
3. CHANGELOG `## [Unreleased]`:
   ```
   ### Added

   - `tracing` instrumentation across the pipeline. INFO spans on every public entry point; DEBUG per-metric; structured events before every `Err(...)` propagation.
   - `Error::is_skip_doc()` and `Error::is_fatal()` accessors. Variant carries the recovery contract; `tracing` carries the diagnosis.
   - `Error::SourceIo` and `Error::ModelIo` (split from `Error::Io`) for clean recoverability classification.
   - `ParseFailKind` enum with `Empty`, `MalformedInput`, `ProviderInternal`, `ResourceLimit` variants.
   - `MatraError` Python exception class preserving `kind`, `is_fatal`, `is_skip_doc` attributes across FFI.

   ### Changed

   - `Error::ParseFailed(String)` → `Error::ParseFailed { kind, message }`. Breaking; matches against the old variant must be updated.
   - `Error::ModelInvalid(String)` → `Error::ModelInvalid { message, recoverable }`. Breaking.
   - `Error::Io(io::Error)` removed; replaced by `SourceIo` and `ModelIo`. Breaking.
   ```

**Acceptance:** `cargo run --example observability` runs end-to-end; emits readable trace output. README has the section.

## Validation

- Truth-table test for every `Error` variant's `is_skip_doc`/`is_fatal` (Task A).
- `cargo expand` confirms `#[non_exhaustive]` (Chesterton fence 2).
- TestWriter-based span assertion test (Task C).
- Per-`Err` event presence verified by manual review checklist (Task D).
- `examples/observability.rs` runs (Task E).
- `bash scripts/check-boundaries.sh` exits 0 (rule 8 still holds; `tracing` not in `domain.rs` or port modules).
- Cross-iteration regression matrix items 1–9 pass.
- `cargo test --features udpipe` count ≥ N₀ + truth-table tests + tracing tests.

## Acceptance gate

matra is **MVP-correct** at the end of I3 if:
- Variant-based recovery contract is in place (truth-table green).
- Every `Err(...)` outside `domain.rs` has a sibling structured event.
- A consumer running with `RUST_LOG=matra=info` sees one INFO span per pipeline stage and one event per error.
- `cargo publish --dry-run` succeeds.
- The Chesterton fence 7 matrix is run against the post-restructure surface; zero contradictions.

## Risks

- **Risk:** the `Error::Io` split breaks existing consumer code. But: there are no consumers yet (pre-publish). Acceptable.
  - **Mitigation:** the CHANGELOG documents the breaking change. After 0.1.0 ships, this is a major-version-bump-only change.

- **Risk:** Burner's CLAUDE.md amendment (rule 8) is forgotten and `tracing` slips into `domain.rs` during the restructure.
  - **Mitigation:** boundary check script runs in CI. Failure blocks the merge.

- **Risk:** `#[non_exhaustive]` is forgotten on `ParseFailKind`, locking the variant set.
  - **Mitigation:** Chesterton fence 2 explicit; `cargo expand` verification step.

- **Risk:** PyO3 `MatraError` mapping leaks Rust internals (the `Debug` of `io::ErrorKind`) and creates an unstable Python attribute surface.
  - **Mitigation:** `kind` on `MatraError` is a Python string, mapped from the Rust `ParseFailKind` via a clean `as_str()` method, not a `Debug` derivation. Document in the implementation comment.

- **Consult:** Burner if the `tracing` integration tempts a port-module instrumentation. Wolf if a span site is unclear.
