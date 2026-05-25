# Errors Reference

vaani uses a single `domain::Error` enum. Every public function returns `domain::Result<T>`, which is `std::result::Result<T, domain::Error>`. No function returns `Result<T, String>`; no function panics in library code (UDPipe panics are caught at the FFI boundary and converted to `ParseFailed`).

The enum is `#[non_exhaustive]`. New variants may be added in minor versions; match arms must include a catch-all when this is compiled as a dependency.

---

## Variant Table

| Variant | Rust signature | When it fires |
|---|---|---|
| `ModelNotFound` | `ModelNotFound(PathBuf)` | The model file does not exist at the given path |
| `ModelInvalid` | `ModelInvalid(String)` | The model file exists but could not be loaded (corrupt, wrong format, or wrong version) |
| `ParseFailed` | `ParseFailed(String)` | The NLP provider returned an error during parse; also used when a UDPipe panic is caught at the FFI boundary |
| `InputTooLarge` | `InputTooLarge { limit: usize, actual: usize, what: &'static str }` | Input exceeded a size cap; which cap is named by `what` |
| `UnsupportedFormat` | `UnsupportedFormat(Format)` | A `Pdf` or `Docx` file was submitted and no decomposer is registered for it |
| `Io` | `Io(#[from] std::io::Error)` | File I/O error during source ingestion or model loading |

---

## InputTooLarge: `what` discriminator

`InputTooLarge` fires from multiple independent gates. The `what: &'static str` field identifies which gate fired so consumers can route differently per limit.

| `what` value | Gate | Limit |
|---|---|---|
| `"input"` | Top-level text input (all `analyze*` and `parse` entry points) | 8 MiB (8,388,608 bytes) |
| `"file_source"` | Per-file size check in `FileSource` | same 8 MiB cap |
| `"tfidf"` | TF-IDF summarizer sentence count | 2,000 sentences |
| `"textrank"` | TextRank summarizer sentence count | 2,000 sentences |
| `"rake"` | RAKE keyphrase extractor token count | 200,000 tokens |
| `"yake"` | YAKE keyphrase extractor token count | 200,000 tokens |

The 8 MiB input cap accommodates book-length English (a typical novel is roughly 1.5 MiB) with headroom for structured documents. The 2,000-sentence cap on TextRank bounds a dense similarity matrix that would be ~32 MiB of `f64` at the limit. The 200,000-token cap on RAKE and YAKE bounds their candidate-phrase maps.

---

## Python exception mapping

The PyO3 binding converts `domain::Error` to Python exceptions in `lib.rs`. The match is exhaustive at compile time; adding a new variant without updating the mapping is a compile error.

| `domain::Error` variant(s) | Python exception |
|---|---|
| `ModelNotFound` | `FileNotFoundError` |
| `InputTooLarge`, `UnsupportedFormat` | `ValueError` |
| `Io` | `OSError` |
| `ModelInvalid`, `ParseFailed` | `RuntimeError` |

`FileNotFoundError` is chosen for `ModelNotFound` so Python callers can use the conventional `try ... except FileNotFoundError` idiom.

---

## Display strings

The `Display` implementation produces human-readable messages. These are the strings Python's `str(exc)` returns.

| Variant | Format |
|---|---|
| `ModelNotFound(p)` | `"model not found: {path}"` |
| `ModelInvalid(s)` | `"invalid model: {s}"` |
| `ParseFailed(s)` | `"parse failed: {s}"` |
| `InputTooLarge { what, actual, limit }` | `"{what} input too large: {actual} > limit {limit}"` |
| `UnsupportedFormat(f)` | `"unsupported format: {f:?}"` |
| `Io(e)` | `"io error: {e}"` |

---

## Matching in Rust

```rust
match vaani::analyze(text, nlp) {
    Ok(doc) => { /* use doc */ }
    Err(domain::Error::InputTooLarge { what, actual, limit }) => {
        eprintln!("{what}: {actual} bytes, limit {limit}");
    }
    Err(domain::Error::ModelNotFound(path)) => {
        eprintln!("model missing: {}", path.display());
    }
    Err(e) => return Err(e),
}
```

---

*For the full type hierarchy, see [reference/domain-types.md](domain-types.md). For format variants, see `Format` in domain-types.md.*
