# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
