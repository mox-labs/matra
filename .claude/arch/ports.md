# Ports

A **port** is a boundary trait. It defines what the domain needs from the outside world without knowing how that need is met. Adapters implement ports.

vaani has three ports. They were chosen as the minimum that preserves the boundary discipline; adding more does not increase composability, it increases coordination cost.

## `Source` (the ingest port)

**Verb:** ingest. **Module:** `src/source/mod.rs`.

```rust
pub trait Source: Send {
    fn read(&self, input: &Path) -> domain::Result<Vec<RawDocument>>;
    fn accepts(&self, input: &Path) -> bool;
}
```

A `Source` takes a path and produces zero or more `RawDocument` values. `accepts` lets the composition root pick the right adapter for a given path.

### Contract

- **Precondition:** `path` is a syntactically valid path. The adapter is allowed to exist or not exist on disk; the adapter decides what to do.
- **Postcondition on success:** every returned `RawDocument` has bytes resident, format detected, and either a path that points to the source file or `None` (for in-memory text). Order, when multiple documents are returned, is path-sorted.
- **Postcondition on failure:** returned `Err` is a `domain::Error` variant. `DirectorySource::read_collecting_errors` surfaces per-file failures separately from the top-level result.

### Forbidden imports

- No `udpipe_rs` (rule 4).
- No `pyo3`.
- No imports from `decompose` or `nlp` (rule 3).

## `Decomposer` (the structural port)

**Verb:** decompose. **Module:** `src/decompose/mod.rs`.

```rust
pub trait Decomposer {
    fn decompose(&self, text: &str) -> Vec<Section>;
}
```

A `Decomposer` takes raw text and produces a section tree. Sections contain paragraphs; paragraphs contain blockquote flags but no parsed sentences yet.

### Contract

- **Precondition:** text is valid UTF-8.
- **Postcondition:** paragraphs are in document order. `in_blockquote` is set correctly. Sections respect heading hierarchy where applicable. No `Sentence` data is populated; that arrives later in the pipeline.

### Why not return `Result`?

`Decomposer` is infallible. A markdown decomposer that hits something it doesn't understand (e.g., raw HTML in markdown) treats it as plain text. A plain-text decomposer never fails on text. If a decomposer ever needs to fail, that decision lives in `ingest` (where format detection happens), not in `decompose`.

Infallible decomposers keep the composition root simple.

### Forbidden imports

- No I/O (decomposers operate on `&str`).
- No imports from `source` or `nlp`.

## `NlpProvider` (the parse port)

**Verb:** parse. **Module:** `src/nlp/mod.rs`.

```rust
pub trait NlpProvider: Send {
    /// Parse text into sentences with POS tags and dependency labels.
    fn parse(&self, text: &str) -> domain::Result<Vec<Sentence>>;
}
```

An `NlpProvider` annotates a text region with linguistic structure. The contract is load-bearing: downstream code (`measure`, `extract`) assumes document order and id-sorted tokens.

### Contract

- **Precondition:** text is valid UTF-8. The composition root checks size before calling; the provider does not enforce a bound.
- **Postcondition on success:**
  - Sentences in document order.
  - Tokens within each sentence id-sorted ascending.
  - Exactly one token per sentence has `head = 0`.
  - All `head` references are valid (point to another token in the same sentence, or 0).
  - On panic in the underlying provider: caller sees `Err(ParseFailed(_))`, never a process abort. (The `Udpipe` adapter wraps `Model::parse` in `catch_unwind`.)
- **Postcondition on failure:** `Err` is a `domain::Error` variant — typically `ParseFailed`.

A future `NlpProvider` adapter that violates document order or id-sorting silently breaks every metric and extractor. The contract is documented here and on the trait.

### Why per-paragraph parsing?

The composition root parses **per paragraph**, not per document. Two reasons:

1. **Correctness.** Parsing the whole document then matching sentences back to paragraphs by string prefix is fragile (a regression-tested defect; see `lib.rs` tests `parse_per_paragraph_scopes_sentences_to_originating_paragraph` and `parse_per_paragraph_no_inner_substring_theft`). Per-paragraph parse removes the need to wire back: a paragraph's sentences are exactly what came back from `nlp.parse(paragraph.text)`.
2. **Bounded cost.** UDPipe's intermediate memory grows with input size. Per-paragraph keeps the working set proportional to one paragraph, not the whole document.

The trait does not enforce per-paragraph use; consumers may parse whole documents. The library's composition root chooses per-paragraph for the convenience APIs.

### Forbidden imports

- No `udpipe_rs` in the trait module. Only in the adapter.
- No imports from `source` or `decompose`.

## Why three ports

Three is the minimum that preserves the boundary discipline. Alternatives tested and rejected:

- **Add an `Ingest` port** (separate from `Source`). `Source` already is the ingest port semantically. A separate trait duplicates the surface.
- **Combine `Decomposer` into `Source`** (a `Source` returns sections directly). Format-specific structural breakdown is orthogonal to format-specific ingestion. A directory of markdown files needs one `Source` and one `Decomposer`, not a custom hybrid per format.
- **Add a `Measure` port and an `Extract` port.** Metrics and extraction operate on domain types only; they have no adapter surface. They live as functions in `metrics/` and `extraction/`, not as ports.

## Adding a new port

Don't, unless:

1. You have a real adapter need (a new I/O or service axis that the existing three cannot absorb).
2. The trait is small (one or two methods).
3. The trait imports only from `domain`.
4. There is at least one consumer in the composition root using it.

Adding ports speculatively is the Inner Platform anti-pattern. Real adapters first, port second.
