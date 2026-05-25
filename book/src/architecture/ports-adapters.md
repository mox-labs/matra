# Ports and adapters

vaani has three ports. Each port is a minimal trait; each port has one or more adapter implementations.

## Source

```rust
pub trait Source: Send {
    fn read(&self, input: &Path) -> domain::Result<Vec<RawDocument>>;
    fn accepts(&self, input: &Path) -> bool;
}
```

Adapters:

- **FileSource:** reads one file. Rejects symlinks (uses `symlink_metadata`, non-traversing). Enforces a per-file size cap (`MAX_INPUT_BYTES`) before reading.
- **DirectorySource:** walks a directory non-recursively. Skips symlinks. Sorts paths lexicographically. Tolerates per-file failures via `read_collecting_errors`.

## Decomposer

```rust
pub trait Decomposer {
    fn decompose(&self, text: &str) -> Vec<Section>;
}
```

Adapters:

- **MarkdownDecomposer:** honors heading hierarchy, tracks blockquote membership. Treats malformed markdown as plain text.
- **PlainTextDecomposer:** one heading-less section, paragraphs split on blank lines.

Decomposers are infallible.

## NlpProvider

```rust
pub trait NlpProvider: Send {
    fn parse(&self, text: &str) -> domain::Result<Vec<Sentence>>;
}
```

Adapter:

- **Udpipe:** wraps the `udpipe-rs` C++ bindings. The only file in the codebase allowed to import `udpipe_rs`. C-side panics are caught via `catch_unwind` and converted to `Err(ParseFailed(_))`.

Contracts on the returned `Vec<Sentence>`:

- Sentences in document order.
- Tokens within each sentence id-sorted ascending.
- Exactly one token per sentence has `head = 0`.
- All `head` references are valid (point to another token in the same sentence, or 0).

These are load-bearing. Downstream metrics and extractors assume them.

## Adapter rules

Every adapter:

1. Implements exactly one port (or one inherent helper like `DirectorySource::read_collecting_errors` that the composition root exposes).
2. Imports only from `domain` and the port module it implements. Never from another adapter.
3. Lives in the module that owns its port (`source/`, `decompose/`, `nlp/`).
4. Translates external errors into `domain::Error` variants.
5. Documents contract overrides at the impl site (e.g., `DirectorySource`'s sort order, `Udpipe`'s panic boundary).

## Writing a new adapter

See [Writing a new adapter](../guides/new-adapter.md) for the recipe.

## Adapters that don't exist yet

These are deliberate gaps, not oversights.

| Adapter | Why deferred |
|---|---|
| `PdfDecomposer` | PDF is a format family, not a format. Half-shipping a PDF adapter would lock a bad shape into the public surface. |
| `DocxDecomposer` | Same reasoning. |
| WASM `NlpProvider` | Pure-Rust tagger + parser for browser/Node use. Lands when a TypeScript consumer commits. |
| Streaming `Source` (websocket, filesystem watch) | Only if a consumer needs push semantics. See [Future direction](../architecture/future-direction.md). |

`Format::Pdf` and `Format::Docx` return `Error::UnsupportedFormat` until adapters exist for them.
