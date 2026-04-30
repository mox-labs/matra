# I2 — Resilience floor

**Status:** not-started
**Depends on:** I1 (rename landed)
**Branch:** `i2/resilience-floor` off the I1 commit

## Why this iteration exists

Whatever fragility ships at 0.1.0 propagates to every consumer. Each fragility becomes a contract that binds downstream codebases forever once published.

Taleb's verdict (2026-04-28): "**OBJECTION on shipping 0.1.0 as-is. Five must-do antifragile fixes are mandatory before publish.**" Knuth corrected the metric on extraction caps; Vector added four HIGH security findings; Lamport added a BLOCK on the model download race. This iteration lands all of them.

This is the floor below which 0.1.0 cannot ship. It is not glamorous. It is not optional.

## What lands

### Task A: per-file I/O tolerance in `DirectorySource`

**Files:** `src/source/directory.rs`.

**Why (Taleb #3):** "One unreadable file in a 10k-doc corpus aborts the whole batch. A consumer running an overnight corpus pass loses everything because of one permission-denied. CHANGELOG admits this as '0.2 limitation' — that's fragility theater since the caller cannot work around it."

**Steps:**

1. Change `DirectorySource::read` to return `Ok((Vec<RawDocument>, Vec<(PathBuf, Error)>))` (matching the shape `analyze_directory` already exposes), or alternatively accumulate errors into a side-channel that the composition root reads.
2. Per-file I/O failures (`SourceIo`-class errors) are collected into the side-channel; the iteration continues.
3. Add tests:
   - Directory with 3 valid files plus 1 chmod-000 file: returns 3 documents and 1 error.
   - Directory with 3 valid files plus 1 file with non-UTF-8 bytes: returns 3 documents and 1 error.

**Acceptance:** test fixture above passes. `analyze_directory` and (later, in I4) `analyze_directory_iter` both surface per-file errors without aborting.

### Task B: extraction caps with per-algorithm bounds

**Files:** `src/extraction/tfidf.rs`, `src/extraction/rake.rs`, `src/extraction/yake.rs`, `src/domain.rs` (consts).

**Why (Knuth correction to Taleb, 2026-04-28):** "TF-IDF/RAKE/YAKE are all linear, not quadratic. They need caps for memory and bounded-latency on hostile input, not algorithmic blow-up. TF-IDF: cap on sentences. RAKE/YAKE: cap on tokens." (Chesterton fence 4 confirmed `MAX_SENTENCES = 2000` is computed for TextRank's 32MB matrix and **must not be shared**.)

**Steps:**

1. Define per-extractor constants in `src/extraction/{tfidf,rake,yake}.rs`:
   - `tfidf::MAX_SENTENCES` (own constant, distinct from `textrank::MAX_SENTENCES`). Same value (2000) is acceptable; distinct constants prevent accidental coupling.
   - `rake::MAX_TOKENS = 200_000` (default; see tie-breaker below).
   - `yake::MAX_TOKENS = 200_000` (default).
2. Above each constant, add a comment justifying the value with arithmetic. Example for `rake`:
   ```rust
   // RAKE builds a co-occurrence map keyed on phrase strings. Worst-case
   // unique-phrase cardinality is bounded by token count times mean phrase
   // length k (typically ~4). At 200k tokens the map holds <= ~50k entries
   // at ~64 bytes each = ~3 MB resident. Wall time on Zipf-distributed input
   // measured at <500ms (see scratch/bench-i2.md).
   const MAX_TOKENS: usize = 200_000;
   ```
3. Each extractor's entry returns `Error::InputTooLarge { what: <"tfidf"|"rake"|"yake">, limit, actual }` when input exceeds its cap. **Distinct `what:` labels per extractor** (Chesterton fence 4 explicit).
4. Replicate the existing `textrank.rs` cap-rejection test pattern for each new cap. Each test asserts the right `what:` label.

**Tie-breaker for `MAX_TOKENS = 200_000`** (Ixian protocol):

If empirical bench at 200k shows wall time > 1s p99 or peak RSS > 256 MB, lower the cap until both bounds hold. Document the chosen value in the inline comment. The number is not the deliverable; the **arithmetic comment** is the deliverable.

**Acceptance:**
- `tfidf::MAX_SENTENCES`, `rake::MAX_TOKENS`, `yake::MAX_TOKENS` exist with arithmetic comments.
- Three new tests, each asserting `Error::InputTooLarge` with a distinct `what:` label.
- `textrank::MAX_SENTENCES` unchanged (Chesterton fence 4: do not share).

### Task C: `MAX_INPUT_BYTES` at composition-root entry points

**Files:** `src/domain.rs` (const), `src/lib.rs` (enforcement).

**Why (Taleb #2 + Vector HIGH):** "A single 1 GB doc OOMs the host. `FileSource::read_to_string` loads the whole file before format detection or any limit." Plus: per Burner, enforcement lives at the public surface (composition root), not on the trait, so adapters cannot be bypassed via direct trait calls without paying the same cost.

**Steps:**

1. Add `pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;` in `src/domain.rs` with arithmetic comment:
   ```rust
   /// Default upper bound on text input to public analyze*/parse functions.
   /// 8 MiB accommodates book-length English (~1.5 MiB / 200k words for
   /// a typical novel) with headroom for multilingual and structured prose.
   /// Beyond this, UDPipe's intermediate memory crosses 1 GB at parse time.
   pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
   ```
2. Enforce at every public entry: `analyze`, `analyze_markdown`, `analyze_file`, `analyze_directory` (per-doc check inside the loop), `parse`, `analyze_from` (sum of section text lengths).
3. Return `Error::InputTooLarge { what: "input", limit: MAX_INPUT_BYTES, actual }`.

**Acceptance:**
- `MAX_INPUT_BYTES` present in `domain.rs` with arithmetic comment.
- Test: 1-byte-over-cap input rejected with `InputTooLarge { what: "input", .. }`. 1-byte-under-cap input accepted.
- `analyze_file` on a 10 GB file (test simulated; do not actually create) refuses before reading. Use a `FileSource` trait bypass test: `FileSource::accepts` plus a synthetic large-metadata file in a `tempfile`.

### Task D: O(n) `tree_depth` with HashMap+memo

**Files:** `src/domain.rs:220-239`.

**Why (Knuth, 2026-04-28):** "Build a `HashMap<id, head>` once (O(n)), then for each token walk parents with a `HashSet<id>` visited guard. Or better: single bottom-up DFS from the root with depth memoization in `HashMap<id, usize>` — **O(n)** per sentence, total." Plus Chesterton fence 1: the `< 20` ceiling is unjustified; remove it.

**Steps:**

1. Replace the existing `tree_depth` body with a HashMap-indexed bottom-up DFS:
   - Build `HashMap<id, head>` once.
   - For each token, walk from token to root via the map. Use `HashSet<id>` visited guard.
   - Memoize depth per token in a second `HashMap<id, usize>`.
   - Return the max depth across all tokens.
   - On cycle (token revisited): return `usize::MAX` or surface via a debug-assert. **Never silently truncate.**
2. Add tests:
   - 25-token straight chain: returns depth 24.
   - 1000-token straight chain: returns depth 999, wall time < 50ms (proves O(n)).
   - 100-token chain with a cycle: cycle is detected; returns `usize::MAX` (or whatever sentinel the impl chooses; document it).

**Acceptance:**
- The magic number 20 no longer appears in `domain.rs`.
- Tests above pass.
- `cargo bench` (or a unit test with `Instant::now()`) shows wall time growth is linear, not quadratic, between 100, 500, and 1000 tokens.

### Task E: `catch_unwind` boundary in `Udpipe::parse`

**Files:** `src/nlp/udpipe.rs:118-183`.

**Why (Taleb #1):** "UDPipe is the SPOF and a C library with no panic boundary. A C-level panic/SIGSEGV inside `Model::parse` aborts the process. In Python this manifests as interpreter death, not a catchable exception. WASM consumers see a trap."

**Steps:**

1. Wrap the `self.model.parse(text)` call inside `std::panic::catch_unwind`. Use `AssertUnwindSafe` if needed.
2. On caught panic, return `Err(Error::ParseFailed { kind: ParseFailKind::ProviderInternal, message: <captured panic message or "udpipe panic"> })`.
3. Tests:
   - A fault-injection `NlpProvider` fixture (lives in tests, not in `src/`) that panics on parse: the wrapper returns `ParseFailed`, never aborts the test process. Use `Catch_Panic` test harness or the `should_panic` attribute inverted: assert no panic propagates.

**Acceptance:** the test exists and passes. The process does not abort on panic. `is_skip_doc()` returns true on the resulting error (consumer can skip the document).

### Task F: `FileSource` symlink fix and size cap

**Files:** `src/source/file.rs`.

**Why (Vector HIGH):** "`FileSource::read` calls `std::fs::read_to_string` with no `symlink_metadata` check. `DirectorySource` is documented to skip symlinks, but `analyze_file` does not. Asymmetry between adapters is itself a vulnerability."

**Steps:**

1. In `FileSource::read`, before reading: call `std::fs::symlink_metadata(path)`. If `metadata.file_type().is_symlink()`, return `Err(Error::UnsupportedFormat(Format::Unknown))` or a new `Error::SourceIo { path, kind: io::ErrorKind::Unsupported }`.
2. Check `metadata.len()` against `MAX_INPUT_BYTES`. If over, return `Error::InputTooLarge { what: "file_source", limit, actual: metadata.len() }`.
3. Tests:
   - Symlink to a regular file: `FileSource::read` rejects it.
   - File over the size cap: refuses before `read_to_string`.

**Acceptance:** both tests pass. `FileSource` and `DirectorySource` now both skip symlinks consistently.

### Task G: brotli `lgwin ≤ 18` and per-paragraph cap

**Files:** `src/metrics/compression.rs`.

**Why (Vector HIGH):** "`lgwin=22` = 4 MB sliding window per paragraph, level 6 is mid-CPU. A malicious 100-paragraph document each with 10 MB of crafted noise pegs CPU."

**Steps:**

1. In `metrics/compression.rs`, change brotli config: `lgwin` from 22 to 18 (256 KB window).
2. Before compressing each paragraph, check paragraph byte size against a local `MAX_PARAGRAPH_BYTES` constant (suggest 256 KiB; document arithmetic).
3. If over, set `compression_ratio = None` rather than compress.
4. Test: a 1 MB single-paragraph input completes in < 100 ms with finite ratio (or `None`).

**Acceptance:** wall time for the 1 MB paragraph case bounded; `None` semantics consistent with the v2 plan rule "`Option<f64>` = not applicable" (Chesterton fence 7).

### Task H: TOCTOU fix in `Udpipe::english`

**Files:** `src/nlp/udpipe.rs:72-83`.

**Why (Vector MED):** "After verify, `from_path(&path)` re-reads from disk. Attacker with write access to `model_dir` swaps the file between verify and load → load runs against unverified bytes."

**Steps:**

1. Modify the verify path: read bytes once, hash them, compare. If match, pass the in-memory bytes to `Model::load_from_memory` (or whatever `udpipe-rs` API exposes). Never re-read from disk after verify.
2. If `udpipe-rs` does not have an in-memory load: hold a `File` handle across verify and load with an exclusive lock (`fs2::FileExt::lock_exclusive`).
3. Test: simulate a swap by replacing the file mid-call with a different valid file. Loaded model must correspond to the verified bytes, not the swapped ones.

**Acceptance:** test passes. `udpipe-rs::Model` is loaded from the same bytes that were verified.

### Task I: atomic model download

**Files:** `src/nlp/udpipe.rs` download path.

**Why (Lamport BLOCK):** "Two processes calling `english(model_dir)` race on `path.exists()` → both call `download_english` → both write the same path. Process A reads a half-written file, fails verification, deletes it, process B is mid-write → corruption."

**Steps:**

1. Download path: write to `<model_path>.tmp.<pid>`. After verify, `std::fs::rename(<tmp>, <final>)`. `rename` is atomic on the same filesystem.
2. Alternative if cross-filesystem: hold a file lock on the model_dir during download+verify+rename. Choose one approach; document it.
3. Test: spawn two processes (or two threads with separate `Udpipe::english` calls and a fault-injected slow download). Both succeed. Final file matches expected hash. No `.tmp.<pid>` orphan blocks the second call.

**Acceptance:** concurrent test passes. No orphaned `.tmp.*` files block subsequent runs.

### Task J: `attach_sentences` rewrite — parse per paragraph

**Files:** `src/lib.rs` (composition root), `src/metrics/mod.rs:51-80` (delete or replace `attach_sentences`).

**Why (Knuth + Dijkstra + Lamport, converged):**
- Dijkstra: "30-char prefix collision misassigns sentences, plus inner-substring theft when a sentence prefix appears mid-paragraph. The pre-fix invariant 'every paragraph gets visited' holds, but the prefix-match itself is the bug."
- Knuth: "Don't pass the whole document to UDPipe and guess. Parse paragraph-by-paragraph (skipping blockquotes), and the assignment is implicit."
- Lamport: "Match on `(paragraph_index, char_offset)` rather than prefix-contains" — converges with Knuth's parse-per-paragraph fix.

**Steps:**

1. In `src/lib.rs`'s pipeline (the function currently called `run_analysis` or its successor): for each non-blockquote paragraph, call `nlp.parse(&paragraph.text)` directly. Assign the returned sentences to `paragraph.sentences`.
2. Delete `metrics::attach_sentences` (or repurpose it as a no-op shim that leaves sentences in place).
3. The metrics suite no longer needs the wiring step. Document this in `metrics/mod.rs::default_suite`.
4. Document-level metrics (`vocabulary_ttr`, `nominalization_ratio`) read from the populated `Analysis` directly.
5. Tests:
   - Two paragraphs starting with "The system processes..." (FM1 prefix-collision regression). Each paragraph's sentences are scoped to that paragraph; no leak.
   - One paragraph with a sentence whose prefix appears mid-text in another paragraph (inner-substring theft regression). No misassignment.
   - Empty paragraph followed by valid paragraph: empty paragraph has zero sentences; valid paragraph has its own.

**Acceptance:**
- The two regression tests above pass.
- `cargo test` count: increases by at least 2.
- Wall time on a 100-paragraph document is **not significantly slower** than the pre-rewrite version (parse-per-paragraph has small per-paragraph startup cost; check against I0 baseline). Acceptable delta: within +20% of the I0 wall-time noise floor. Larger delta: investigate.

## Validation

The full validation matrix from Ixian's protocol:

| Sub-task | Acceptance gate | Falsifier |
|---|---|---|
| A | dir with 3 valid + 1 chmod-000 returns 3 docs + 1 error | injecting permission-denied → if `Err`, fail |
| B | TF-IDF/RAKE/YAKE caps with three distinct `what:` labels | `MAX_TOKENS+1` returns `InputTooLarge { what: "rake", .. }` |
| C | 1-byte-over-cap rejected; 1-byte-under accepted | n/a |
| D | 25-depth chain returns 24; 1000-depth returns 999; wall time linear | depth result of 20 means `< 20` was relaxed not removed |
| E | panic-injecting fixture returns `ParseFailed`, no abort | process abort on panic input → fail |
| F | symlink and oversize file both refused at FileSource | symlink read → fail |
| G | 1 MB paragraph compression bounded to < 100 ms | timeout → fail |
| H | swap-after-verify produces verified bytes in loaded model | model corresponds to swapped file → fail |
| I | concurrent `english(same_dir)` both succeed; no orphan blocks restart | one fails or orphan present after kill → fail |
| J | FM1 + inner-substring regressions pass | sentence leak across paragraphs → fail |

Wall-time sanity at I2 landing: `cargo test --features udpipe` within ±2σ of I0 baseline. If outside, investigate (likely Task J cost; acceptable up to +20%, otherwise needs profiling).

## Acceptance gate

After I2 lands, all of:
- All 10 sub-tasks (A–J) acceptance gates pass.
- `cargo test --features udpipe` count increases (new resilience tests added) by at least 12.
- Cross-iteration regression matrix items 1–9 pass.
- `bash scripts/check-boundaries.sh` exits 0.
- Wall time within ±2σ of I0 baseline (or ≤ +20% if degraded).

## Risks

- **Risk:** Task J (parse-per-paragraph) significantly slows `cargo test --features udpipe` because per-paragraph UDPipe startup dominates short-paragraph fixtures.
  - **Mitigation:** measure first. If degradation > +20%, profile UDPipe per-call overhead; consider sharing the `Model` instance across paragraphs (it's already shared via `&self`; the cost is the FFI call, not the model load).
  - **Consult:** Knuth on the algorithm; Erlang on the per-call FFI cost.

- **Risk:** Task H (TOCTOU fix) requires an `udpipe-rs` API surface that does not exist (no in-memory load).
  - **Mitigation:** check `udpipe-rs` docs first. If no in-memory load: file lock + atomic rename approach. Document the choice in the implementation comment.
  - **Consult:** Vector if the file-lock approach exposes a different attack surface.

- **Risk:** Task I (atomic download) introduces a `fs2` dependency, bloating the dependency tree.
  - **Mitigation:** atomic rename via `std::fs::rename` is sufficient if download stays on one filesystem. Use `fs2` only if cross-filesystem support is needed.

- **Risk:** Task B's `MAX_TOKENS = 200_000` is wrong for a real workload.
  - **Mitigation:** the value is the second-class deliverable. The arithmetic comment is the first-class deliverable. If a consumer reports an issue, adjust the constant; the comment's reasoning must be updated alongside.
  - **Consult:** Knuth for the bench protocol. Run the tie-breaker bench before locking the value.
