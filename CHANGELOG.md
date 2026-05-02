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

- `MAX_INPUT_BYTES = 8 MiB` constant in `domain.rs` and gates at every composition-root entry point (`analyze`, `analyze_markdown`, `parse`, `analyze_from`). Returns `Error::InputTooLarge { what: "input", .. }`.
- Per-extractor input caps: `tfidf::MAX_SENTENCES = 2000`, `rake::MAX_TOKENS = 200_000`, `yake::MAX_TOKENS = 200_000`. Each with arithmetic comment and a distinct `what:` label on `InputTooLarge`.
- `DirectorySource::read_collecting_errors` inherent method returns `(Vec<RawDocument>, Vec<(PathBuf, Error)>)` so per-file I/O failures can be surfaced without aborting the iteration. `analyze_directory` uses this to merge ingest failures with analysis failures.
- `FileSource` rejects symlinks via `symlink_metadata` and rejects files larger than `MAX_INPUT_BYTES` before any read. Closes the asymmetry where `DirectorySource` skipped symlinks but `FileSource` did not.

### Changed

- Pipeline vocabulary settled at `ingest → decompose → parse → measure` with `extract` as a peer stage. The `Source`, `Decomposer`, and `NlpProvider` traits keep their existing names; the renamed verbs appear in stage descriptions and the composition-root surface.
- Removed `decompose::markdown::parse` free function. Markdown decomposition now goes through `decompose::markdown::MarkdownDecomposer.decompose(text)` (the `Decomposer` trait method). This frees the verb `parse` for NLP-only use across the pipeline. Breaking change for callers using the free function.
- `Sentence::tree_depth` is now O(n) per sentence via HashMap-indexed bottom-up DFS with depth memoization. The previous magic `< 20` ceiling is gone. On a malformed parse with a cycle, returns `usize::MAX` (sentinel) rather than silently truncating to 20.
- `tfidf::summarize`, `rake::keyphrases`, `yake::yake_keyphrases` now return `Result<Vec<...>>` instead of `Vec<...>` (breaking). Required so they can return `Error::InputTooLarge` on cap exceedance.
- `analyze_from` now returns `Result<Analysis>` instead of `Analysis` (breaking). Required so it can return `Error::InputTooLarge` on aggregate section bytes exceeding the cap.
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
