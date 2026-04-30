# I5 — Streaming iterator + Engine + CorpusResult

**Status:** not-started
**Boundary:** **MLP** — at the end of this iteration, vaani scales to corpus-sized work without OOM and ships a delightful Rust DX.
**Depends on:** I4 (workspace + `rumi-nlp` skeleton)
**Branch:** `i5/streaming` off the I4 commit

## Open decision: cut vs deprecate `analyze_directory`

K (recovery 13-agent review, recovery-3.md:782) recommended **cutting `analyze_directory` from 0.1.0 entirely** rather than deprecating it. Argument: unvalidated error policy, forced on callers. Today's plan deprecates-but-keeps; reconsider before this iteration starts.

- **Deprecate-and-keep** (current default): users who already wrote `analyze_directory` calls don't break. Migration path is explicit (`#[deprecated]` annotation guides them to `analyze_directory_iter`).
- **Cut entirely:** smaller public surface at 0.1.0; consumers compose `DirectorySource::read_iter` + the per-doc analysis themselves; cleaner architecture.

**Default for this implan: deprecate-and-keep.** Confirm or redirect before starting Task C.

## Why this iteration exists

The buffered `analyze_directory` is a flow defect (Erlang OBJECTION, 2026-04-28): "5KB × 1M files = 5GB resident text → ~50GB of `Analysis` after parse. OOM at ~10k docs on most machines. Worse: aborts on first I/O error after possibly reading 999,999 files."

The streaming iterator is the drainage primitive. It changes vaani from "works on test fixtures" to "deployable substrate." Erlang and K converged: this is the right primitive **now**, and the reactor decision is deferred.

Two additional surface improvements come along (Ace, 2026-04-28) because they belong to the same DX upgrade:
- **`Engine` struct** for Rust DX parity with the PyO3 `Vaani` class.
- **`CorpusResult { corpus, errors }`** wrapper for clean serialization across FFI.

## What lands

### Task A: `DirectorySource::read_iter` inherent method

**Files:** `crates/vaani-core/src/source/directory.rs`.

**Why (Burner, 2026-04-28):** "`Source::read_iter` on the trait forces every adapter (including `FileSource`, which yields exactly one doc) to implement an iterator with no benefit, and `impl Iterator` in trait return position constrains the trait object. Put `read_iter` as an *inherent* method on `DirectorySource`."

**Steps:**

1. On the `DirectorySource` struct, add an inherent method:
   ```rust
   impl DirectorySource {
       pub fn read_iter(
           &self,
           path: &Path,
       ) -> Result<impl Iterator<Item = Result<RawDocument, (PathBuf, Error)>> + '_> {
           /* lazily iterate sorted paths; per-file errors collected into the iterator */
       }
   }
   ```
2. The iterator yields per-path lazily. The directory listing is collected eagerly (cheap), but file reads happen on `next()`.
3. Path order: lexicographic, matching the existing `read` (Lamport contract).
4. Error semantics: per-file I/O failures become `Err((path, Error::SourceIo { .. }))` items in the stream; iteration continues. A fatal error at the directory level (e.g., directory does not exist) is the outer `Result<impl Iterator>` `Err` at construction time.
5. Tests:
   - 5-file directory iterated returns 5 `Ok` items in lex order.
   - 3 valid + 1 chmod-000 file returns 3 `Ok` and 1 `Err` items.
   - Lazy semantics: instrument `FileSource::read` with a counter; consume only 2 items from a 5-file iterator; assert exactly 2 reads.

**Acceptance:**
- `read_iter` exists on `DirectorySource`. **Not on the `Source` trait.**
- `rg 'fn read_iter' crates/vaani-core/src/source/mod.rs` returns empty.
- `rg 'fn read_iter' crates/vaani-core/src/source/directory.rs` returns exactly one hit.
- Lazy-read counter test passes.

### Task B: `analyze_directory_iter` in the composition root

**Files:** `crates/vaani-core/src/lib.rs`.

**Why (Erlang):** "`analyze_directory_iter(path, nlp) -> impl Iterator<Item = Result<CorpusEntry, (PathBuf, Error)>>` — drops working set to one doc at a time. Survives the rename to `ingest` cleanly. If a reactor ever lands later, the iterator boundary is exactly where a channel slots in."

**Steps:**

1. Public function:
   ```rust
   pub fn analyze_directory_iter<'a>(
       path: impl AsRef<Path>,
       nlp: &'a dyn NlpProvider,
   ) -> Result<impl Iterator<Item = Result<CorpusEntry, (PathBuf, Error)>> + 'a> {
       let raw_iter = DirectorySource.read_iter(path.as_ref())?;
       Ok(raw_iter.map(move |raw_result| {
           raw_result.and_then(|doc| {
               let path = doc.path.clone().unwrap_or_default();
               analyze_raw(&doc.text, doc.format, nlp)
                   .map(|analysis| CorpusEntry { path: doc.path, analysis })
                   .map_err(|e| (path, e))
           })
       }))
   }
   ```
2. Wrap the entire iterator chain in a `vaani.corpus.analyze_iter` INFO span (recorded fields update on close).
3. Per-document failure event at the iterator: `tracing::warn!(?path, ?error_kind, "vaani.document.failed")`.
4. Doc comment locks the order contract: "Documents are yielded in `DirectorySource` order (path-sorted). Consumers requiring deterministic order under future parallelization must `.collect().sort_by_key(|e| e.path.clone())`."
5. Tests:
   - 5-file directory: collected vec equals existing `analyze_directory` output.
   - Memory: process a 1000-file fixture twice. Iterator path peak RSS < buffered path peak RSS by at least the size of one full collected `Corpus`. (Use `psutil` or a similar test harness.)
   - Lazy: stop after 100 items; assert FileSource read count is 100, not 1000.

**Acceptance:**
- `analyze_directory_iter` exists. Order matches `analyze_directory`.
- Memory test confirms streaming behavior (peak RSS lower).
- Lazy test confirms only consumed items were read.

### Task C: `analyze_directory` deprecation shim returning `CorpusResult`

**Files:** `crates/vaani-core/src/lib.rs`, `crates/vaani-core/src/domain.rs` (new `CorpusResult`).

**Why (Ace, 2026-04-28):** "The current return shape `Result<(Corpus, Vec<(PathBuf, Error)>)>` is awkward for Python — flatten to a `CorpusResult { corpus, errors }` struct so `pythonize` produces a clean dict instead of a tuple. `analyze_directory` carries `#[deprecated(since = "0.1.0", note = "use analyze_directory_iter")]` — but still compiles and passes its existing test."

**Steps:**

1. New domain type:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[non_exhaustive]
   pub struct CorpusResult {
       pub corpus: Corpus,
       pub errors: Vec<(PathBuf, Error)>,
   }
   ```
   (Note: `Error` is not currently `Serialize`. Either implement `Serialize` for `Error` (returning a structured shape with `kind`, `path`, `message`) or use a serializable error projection. Recommend: implement `Serialize` for `Error` with structured output.)
2. Deprecate `analyze_directory`:
   ```rust
   #[deprecated(since = "0.1.0", note = "use analyze_directory_iter; collect into CorpusResult if you need the buffered shape")]
   pub fn analyze_directory(...) -> Result<CorpusResult> { /* delegate to iter + collect */ }
   ```
3. Replace the old `(Corpus, Vec<...>)` return with `CorpusResult`.
4. PyO3: the Python `analyze_directory` returns `dict[str, Any]` shaped as `{"corpus": ..., "errors": [...]}`.
5. Tests: existing `analyze_directory` tests pass with the new return type.

**Acceptance:**
- `CorpusResult` exists with `#[non_exhaustive]` and serde derives.
- `analyze_directory` is deprecated but functional.
- `Error` implements `Serialize` (or a wrapper does).
- PyO3 surface returns dict-shaped result.

### Task D: `pub struct Engine` for Rust DX parity

**Files:** `crates/vaani-core/src/lib.rs`.

**Why (Ace):** "`analyze` takes `&dyn NlpProvider` while `Vaani` is the PyO3 class — Rust users have no top-level convenience type. Recommend a `pub struct Engine { nlp: Box<dyn NlpProvider> }` in Rust mirroring the Python `Vaani` shape, with `Engine::english(dir)` and `engine.analyze(text)`. This is the door handle."

**Steps:**

1. Public struct:
   ```rust
   pub struct Engine {
       nlp: Box<dyn NlpProvider>,
   }

   impl Engine {
       #[cfg(feature = "udpipe")]
       pub fn english(model_dir: impl AsRef<Path>) -> Result<Self> { /* ... */ }

       #[cfg(feature = "udpipe")]
       pub fn from_path(model_path: impl AsRef<Path>) -> Result<Self> { /* ... */ }

       pub fn from_provider(nlp: Box<dyn NlpProvider>) -> Self { /* ... */ }

       pub fn analyze(&self, text: &str) -> Result<Analysis> { /* delegate to free fn */ }
       pub fn analyze_markdown(&self, text: &str) -> Result<Analysis> { /* delegate */ }
       pub fn analyze_file(&self, path: impl AsRef<Path>) -> Result<Analysis> { /* delegate */ }
       pub fn analyze_directory(&self, path: impl AsRef<Path>) -> Result<CorpusResult> { /* delegate */ }
       pub fn analyze_directory_iter<'a>(&'a self, path: impl AsRef<Path>) -> Result<impl Iterator<Item = Result<CorpusEntry, (PathBuf, Error)>> + 'a> { /* delegate */ }
       pub fn parse(&self, text: &str) -> Result<Vec<Sentence>> { /* delegate */ }
       pub fn analyze_from(&self, sections: Vec<Section>, sentences: &[Sentence]) -> Analysis { /* delegate */ }
   }
   ```
2. The free functions stay (Ace's two-layer surface). `Engine` methods delegate; the free functions are the power-user API.
3. Doctest at the top of `Engine`:
   ```rust
   /// ```no_run
   /// let engine = vaani::Engine::english("./models")?;
   /// let analysis = engine.analyze("The team shipped the product.")?;
   /// # Ok::<(), vaani::Error>(())
   /// ```
   ```
4. Three-line "Quick start" in README using `Engine` (Ace).

**Acceptance:**
- `Engine` exists and tests against the doctest.
- Engine methods produce identical output to free-function equivalents on the same input. Test: `engine.analyze(text) == analyze(text, &*engine.nlp_for_test_only())`.

### Task E: `pub mod prelude`

**Files:** `crates/vaani-core/src/lib.rs` (or a new `crates/vaani-core/src/prelude.rs`).

**Why (Ace):** "Recommend a `pub mod prelude` re-exporting only `analyze`, `Vaani`/`NlpProvider`, `Error`. Document `analyze_from` as 'advanced: skip double-parse'."

**Steps:**

1. Create `prelude` module re-exporting:
   - `Engine` (Rust DX entry)
   - `Error`, `ParseFailKind`
   - `NlpProvider` trait
   - `Analysis`, `Sentence`, `Token`, `Paragraph`, `Section`, `Corpus`, `CorpusEntry`, `CorpusResult`, `RawDocument`, `Format`
   - `ScoredSentence`, `Keyphrase`
   - The free functions (`analyze`, `analyze_markdown`, `analyze_file`, `analyze_directory`, `analyze_directory_iter`, `parse`, `analyze_from`).
2. Update README Quick Start to `use vaani::prelude::*;`.

**Acceptance:** `use vaani::prelude::*` compiles and gives access to the 90% surface.

## Validation

- Iterator order matches `analyze_directory` (Lamport contract).
- Memory: streaming iterator's peak RSS on 1000-file fixture is materially lower than buffered.
- Lazy: stop-after-N test confirms only N reads.
- `Engine` produces identical output to free functions on same inputs.
- `CorpusResult` serializes cleanly via `pythonize` (Python test: result is a dict with `corpus` and `errors` keys).
- `cargo public-api` diff vs I3: shows added types (`Engine`, `CorpusResult`) and the deprecation marker on `analyze_directory`. No removed types.
- `cargo doc --no-deps`: `Engine` and `prelude` rendered with examples.
- Cross-iteration regression matrix items 1–9 pass.

## Acceptance gate

vaani is **MLP-shippable** at the end of I4 if:
- `analyze_directory_iter` streams; memory test confirms peak RSS reduction.
- `Engine` is the door handle in Rust; mirror of PyO3 `Vaani` shape.
- `CorpusResult` replaces the awkward tuple return.
- `prelude` exists.
- All MVP acceptance gates from I3 still hold.
- README's Quick Start is 3 lines after the `Engine::english` setup.

## Risks

- **Risk:** `Error: Serialize` in `CorpusResult` leaks `io::ErrorKind` `Debug` representation, producing unstable JSON keys.
  - **Mitigation:** custom `Serialize` impl for `Error` that uses stable string representations (`"source_io"`, `"model_io"`, etc.) and structured fields (`path`, `kind` as a string).
  - **Consult:** Ace on the JSON shape; it's the same concern as the `VaaniError` Python attribute mapping.

- **Risk:** the `impl Iterator` return on `analyze_directory_iter` causes lifetime gymnastics that make downstream consumers struggle.
  - **Mitigation:** the lifetime ties to `&dyn NlpProvider`. Consumers hold the `Engine` for the iterator's lifetime, which is the natural pattern.

- **Risk:** Adding `Engine` doubles the surface (free fns + methods). Consumers wonder which is preferred.
  - **Mitigation:** README Quick Start uses `Engine`. `cargo doc` lands `Engine` first. Free fns documented as "power-user; skip double-parse" (Ace).

- **Risk:** `analyze_directory` deprecation message is wrong if users genuinely need the buffered shape (e.g., for `pythonize` output that requires `Vec<CorpusEntry>` materialized).
  - **Mitigation:** the deprecation message points at `analyze_directory_iter` *and* `.collect()` to get the buffered shape. Both work.

- **Consult:** Erlang if memory measurement is unclear (the streaming claim is the load-bearing one). Ace if the surface feels overloaded.
