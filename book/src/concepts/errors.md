# Errors

vaani's error type is matchable, not opaque. The concrete variants survive across the FFI boundary as specific Python exception classes.

## The `Error` enum

```rust
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("model not found: {}", .0.display())]
    ModelNotFound(PathBuf),

    #[error("invalid model: {0}")]
    ModelInvalid(String),

    #[error("parse failed: {0}")]
    ParseFailed(String),

    #[error("{what} input too large: {actual} > limit {limit}")]
    InputTooLarge { limit: usize, actual: usize, what: &'static str },

    #[error("unsupported format: {0:?}")]
    UnsupportedFormat(Format),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

Every variant has a `#[error("…")]` annotation, so `Display` and `to_string()` produce useful messages without manual `Display` impls. `Io(#[from] std::io::Error)` means `?` converts an I/O error at the boundary automatically.

The enum is `#[non_exhaustive]` — adding a variant later is a minor-version change for downstream pattern matches.

## When each variant fires

| Variant | When |
|---|---|
| `ModelNotFound(path)` | The UDPipe model file does not exist at the given path. |
| `ModelInvalid(msg)` | The model file exists but failed to load (corrupt, wrong format, SHA-256 mismatch after re-download). |
| `ParseFailed(msg)` | NLP parsing failed on the input text, including a converted C-side panic from UDPipe. |
| `InputTooLarge { limit, actual, what }` | Input exceeded a bounded limit. `what` distinguishes which gate fired. |
| `UnsupportedFormat(fmt)` | A `Format::Pdf` or `Format::Docx` reached `analyze_raw` without a registered decomposer. |
| `Io(e)` | File I/O failure. The underlying `std::io::Error` is preserved. |

## The `what` discriminator on `InputTooLarge`

The `what: &'static str` field distinguishes the apex input limit from per-extractor caps. Values:

| `what` | Limit | Where it fires |
|---|---|---|
| `"input"` | `MAX_INPUT_BYTES = 8 MiB` | Top-level public entry points (`analyze`, `parse`, etc.) |
| `"file_source"` | `MAX_INPUT_BYTES` | `FileSource` before reading the file |
| `"tfidf"` | `MAX_SENTENCES = 2000` | TF-IDF summarization |
| `"textrank"` | `MAX_SENTENCES = 2000` | TextRank summarization |
| `"rake"` | `MAX_TOKENS = 200_000` | RAKE keyphrases |
| `"yake"` | `MAX_TOKENS = 200_000` | YAKE keyphrases |

This lets a consumer route differently: "too many sentences for TextRank, but try TF-IDF on a sample" is expressible because the caller can match on `what == "textrank"`.

## Handling errors in Rust

```rust
use vaani::{analyze, domain::Error};

match analyze(&text, &nlp) {
    Ok(analysis) => process(analysis),
    Err(Error::InputTooLarge { actual, limit, .. }) => {
        eprintln!("Input is {actual} bytes; cap is {limit}. Splitting and retrying...");
    }
    Err(Error::ParseFailed(msg)) => {
        eprintln!("Parse failed: {msg}. Skipping this document.");
    }
    Err(e) => return Err(e),
}
```

## Handling errors in Python

The PyO3 boundary routes each Rust variant to a specific Python exception class:

| Rust variant | Python exception |
|---|---|
| `ModelNotFound` | `FileNotFoundError` (`PyFileNotFoundError`) |
| `InputTooLarge` | `ValueError` (`PyValueError`) |
| `UnsupportedFormat` | `ValueError` (`PyValueError`) |
| `Io(_)` | `OSError` (`PyOSError`) |
| `ModelInvalid` | `RuntimeError` (`PyRuntimeError`) |
| `ParseFailed` | `RuntimeError` (`PyRuntimeError`) |

So Python code can write:

```python
from vaani import Vaani

try:
    v = Vaani.english("/nonexistent/dir")
except FileNotFoundError as e:
    # Right exception class; right message format.
    print(f"Model missing: {e}")

try:
    result = v.analyze(huge_text)
except ValueError as e:
    print(f"Input too large: {e}")
```

The routing is exhaustive at compile time on the Rust side — adding a new error variant would fail to compile until the PyO3 boundary is updated to route the new variant. This is intentional: silent routing of new variants to `PyRuntimeError` would erase information at the boundary.

## Why thiserror, not anyhow

vaani is a substrate library. Its callers will match on specific variants — `ModelNotFound` triggers a download prompt, `InputTooLarge` triggers a chunk-and-retry, `Io(_)` triggers a filesystem-specific recovery. Type preservation is required.

`anyhow` is the right choice for **application-tier** code — your code that consumes vaani. There, `.context()` chains add useful diagnostic information for top-level error display. vaani itself stays on concrete enums via `thiserror`.
