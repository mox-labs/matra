# Errors

Every fallible function in matra returns `domain::Result<T>`, which is `std::result::Result<T, domain::Error>`. No function in the library returns `Result<T, String>`. matra signals failure by returning `Error`, not by unwinding: a panic raised inside the UDPipe C++ boundary is caught at the adapter and converted into `Error::ParseFailed`.

`Error` is `#[non_exhaustive]`. Variants can be added in a minor release, so a match on it from another crate needs a catch-all arm. `Error` derives neither `Clone` nor serde; it is moved, not copied. The stream surface wraps it in `DocumentError`, which adds the path the failure occurred at and moves the error into `CorpusResult::errors`.

## The variants

| Variant | Payload | Returned when |
|---|---|---|
| `ModelNotFound` | `PathBuf` | A model file does not exist at the given path |
| `ModelInvalid` | `String` | A model file exists but could not be loaded, downloaded, or verified |
| `ParseFailed` | `String` | The NLP provider failed on the input, or panicked, or produced an unusable token id |
| `InputTooLarge` | `{ limit: usize, actual: usize, what: &'static str }` | A size gate rejected the input. `what` names the gate |
| `UnsupportedFormat` | `Format` | The document's format has no registered decomposer |
| `InvalidInput` | `String` | A caller violated a documented API contract |
| `Io` | `std::io::Error` | A filesystem operation failed |

`Error` implements `std::error::Error` and `Display` through `thiserror`, and `From<std::io::Error>`, so `?` converts I/O failures automatically.

### ModelNotFound

Returned by `Udpipe::from_path` when the path does not exist. The payload is the path as given. `Udpipe::english` does not return this variant: it downloads the model when the file is absent.

### ModelInvalid

Returned by:

- `Udpipe::from_path` and `Udpipe::from_bytes` when the loader rejects the bytes. The payload is the loader's message.
- `Udpipe::english` when the download fails. The payload is the download error's message.
- `Udpipe::english` when the file still fails SHA-256 verification after one delete and re-download. The payload is `SHA-256 mismatch after re-download: <path>`.

A file whose hash does not match the pinned constant is treated as untrusted and is never loaded.

### ParseFailed

Returned by the UDPipe adapter's `parse` in three situations:

- The underlying parser returns an error. The payload is that error's message.
- The underlying parser panics. The payload is `udpipe panicked: <message>`. The `catch_unwind` boundary that produces this lives in `nlp/udpipe.rs`, and it exists because an unhandled panic crossing the C++ boundary aborts the host process instead of unwinding.
- A token id or head value cannot be represented as `usize`. The payload names the token and its sentence.

### InputTooLarge

Eight gate labels produce this variant. The `what` field carries the label so a caller can route each gate differently.

| `what` | Gate | Limit | Measured over |
|---|---|---|---|
| `"input"` | `Engine::annotate`, the only route from text to the parser | `MAX_INPUT_BYTES`, 8 MiB (8,388,608) | UTF-8 byte length of the text |
| `"file_source"` | `FileSource::read`, which `Ingest::path` reads through | 8,388,608 bytes | File size reported by the filesystem, checked before any read |
| `"tfidf"` | `tfidf_summarize` | 2,000 | Number of sentences in the slice |
| `"textrank"` | `textrank_summarize` | 2,000 | Number of sentences in the slice |
| `"rake"` | `rake_keyphrases` | 200,000 | Total tokens across the slice, punctuation included |
| `"yake"` | `yake_keyphrases` | 200,000 | Total tokens across the slice, punctuation included |
| `"semantic_clusters"` | `semantic_clusters` | 2,000 | Number of sentences in the slice |
| `"embedding_download"` | `Model2Vec::potion_base_8m`, per artifact | 64 MiB (67,108,864) | Bytes read from the response, which stops one past the cap |

`limit` carries the cap and `actual` carries the measured size, so an error message can be built without hardcoding the constants.

The caps bound worst-case memory and time. TextRank builds a dense similarity matrix that reaches roughly 32 MB of `f64` at 2,000 sentences. RAKE and YAKE build phrase-keyed maps whose size follows token count rather than sentence count, which is why their caps are stated in tokens. The download cap is the one gate whose input is not the caller's: it bounds what a redirected or misbehaving server can make the process hold, and the read stops at the bound rather than continuing, so `actual` reports the bound that was breached rather than the response's full length.

A document from disk crosses both text gates in sequence: `"file_source"` when `Ingest` reads it, `"input"` inside `annotate`. In practice `"file_source"` fires first, since both carry the same 8 MiB limit and the file size is checked before the read.

Calling a provider's `parse` directly bypasses the `"input"` gate. The gate belongs to `Engine::annotate`, not to the `NlpProvider` trait.

The four ranking extractors check their caps after their empty-result checks. `tfidf_summarize(sentences, 0)`, `textrank_summarize(sentences, 0)`, `rake_keyphrases(sentences, 0)`, and `yake_keyphrases(sentences, 0)` return an empty vector without evaluating the cap, whatever the size of the slice. The same holds for an empty slice. `semantic_clusters` has no count parameter; its contract checks run first, then its cap, and an empty slice returns empty clusters.

A metric has no gate of its own. The compression ratio skips any paragraph over 262,144 bytes and leaves its metric slot at `None` rather than returning an error.

### InvalidInput

Returned by `semantic_clusters` when a caller breaks its documented contract (embeddings disagreeing on dimension, an embedding containing a non-finite value, a non-finite threshold), and by `embed_and_cluster` when an embedder violates its own length contract. The payload names the violation. This variant means a call site or a provider implementation is wrong, not the input data; nothing about the analyzed text produces it.

### UnsupportedFormat

Returned by `Engine::annotate` when the document's `Format` has no entry in the engine's decomposer table. With `standard_decomposers()` that means `Pdf` or `Docx`; `Markdown` and `PlainText` always have an entry. The payload is the `Format` value.

### Io

Wraps `std::io::Error`, produced by:

- `FileSource` rejecting a symlink, with `ErrorKind::Unsupported` and the message `refusing to read symlink: <path>`.
- `FileSource` rejecting a path that is not a regular file, with `ErrorKind::InvalidInput` and the message `not a regular file: <path>`.
- Any read, directory listing, directory creation, file removal, or rename that fails.
- `Ingest` when a source yields no document, with `ErrorKind::InvalidData` and the message `source returned no documents`.

## Per-document failures: `DocumentError` and `CorpusResult`

The stream surface never lets one bad document abort the rest. `Engine::analyze` yields `Result<CorpusEntry, DocumentError>` per document, where `DocumentError` pairs the `Error` with the path it occurred at (`None` for in-memory text, so a path-less document is distinguishable from one at an empty path). Its `Display` prints `path: error` when a path exists and the bare error otherwise, and its `source()` is the wrapped `Error`.

Collecting the stream into `CorpusResult` partitions it: every success into `corpus`, every failure into `errors`, in order, with entries plus errors equal to documents consumed. `Ingest::path` is `Err` only when the listing itself fails; both read failures and analysis failures travel as stream items.

Two kinds of entry never appear on either side. The directory listing skips symlinks, so a symlink produces no document and no error. It also skips anything that is not a regular file, and the walk is one level deep, so a subdirectory is skipped the same way.

Neither `DocumentError` nor `CorpusResult` is `Serialize`, because `Error` wraps `std::io::Error`. Crossing a language boundary needs a projection with stable kind strings, which does not exist yet; the types stay Rust-side until it does.

## Display strings

These are the strings `Display` produces, and the strings Python's `str(exc)` returns.

| Variant | Format |
|---|---|
| `ModelNotFound(path)` | `model not found: {path}` |
| `ModelInvalid(s)` | `invalid model: {s}` |
| `ParseFailed(s)` | `parse failed: {s}` |
| `InputTooLarge { limit, actual, what }` | `{what} input too large: {actual} > limit {limit}` |
| `UnsupportedFormat(format)` | `unsupported format: {format:?}` |
| `InvalidInput(s)` | `invalid input: {s}` |
| `Io(e)` | `io error: {e}` |

`UnsupportedFormat` renders the variant name, so the string reads `unsupported format: Pdf`.

## Matching in Rust

```rust
use matra::Engine;
use matra::domain::{Error, Format, RawDocument};

fn report(text: &str, engine: &Engine) {
    let raw = RawDocument::new(text.to_string(), None, Format::PlainText);
    match engine.analyze_one(raw) {
        Ok(entry) => println!("{} sentences", entry.analysis.total_sentences()),
        Err(doc_err) => match doc_err.error {
            Error::InputTooLarge {
                what,
                actual,
                limit,
            } => eprintln!("{what} gate: {actual} over limit {limit}"),
            Error::ModelNotFound(path) => {
                eprintln!("model missing at {}", path.display())
            }
            other => eprintln!("{other}"),
        },
    }
}
```

## Python exception mapping

The PyO3 binding converts `Error` into a Python exception class. The conversion is a match with no wildcard arm, so adding a variant to `Error` without assigning it an exception class fails to compile.

| Variant | Python exception |
|---|---|
| `ModelNotFound` | `FileNotFoundError` |
| `InputTooLarge` | `ValueError` |
| `UnsupportedFormat` | `ValueError` |
| `InvalidInput` | `ValueError` |
| `Io` | `OSError` |
| `ModelInvalid` | `RuntimeError` |
| `ParseFailed` | `RuntimeError` |

`ModelNotFound` maps to `FileNotFoundError` so that the conventional Python idiom works:

```python
from matra import Matra

try:
    engine = Matra.from_path("models/english-ewt-ud-2.5-191206.udpipe")
except FileNotFoundError as exc:
    print(exc)  # model not found: models/english-ewt-ud-2.5-191206.udpipe
```

The exception message is the `Display` string from the table above. Variant identity beyond the exception class is not carried across the boundary; a caller that needs to distinguish `InputTooLarge` from `UnsupportedFormat` inspects the message.

Every Python method that takes text routes through `Engine::annotate`, so the 8 MiB `"input"` gate applies uniformly: to `analyze` and `analyze_markdown`, and to the four extraction methods, whose per-extractor caps apply on top.

One further failure has no `Error` variant behind it. The `Matra` class is `#[pyclass(unsendable)]`, because the loaded model holds C-side state that is not thread-safe. Accessing one instance from a thread other than the one that created it fails at runtime. Multi-process use is unaffected.

## At the command line

Two programs are installed under the name `matra`. The Rust binary comes from `cargo install matra --features cli`. The Python console script comes from the wheel and wraps the same engine through the binding. The exit-code contract below belongs to the Rust binary.

| Exit code | Meaning |
|---|---|
| 0 | The command succeeded, and where applicable something was found |
| 1 | The command succeeded and found nothing |
| 2 | An error occurred |

On exit code 2 the binary writes `matra: ` followed by the error's `Display` string to standard error. A broken pipe, which is what happens when the reading end of `matra analyze file.md | head` goes away, exits 0 and prints nothing.

The Python console script does not implement that contract. It exits 1 when the model cannot be loaded, and otherwise surfaces the exception classes from the table above.
