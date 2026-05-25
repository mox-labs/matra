# Ports and adapters

A port is a trait. It names what the domain needs, expressed entirely in domain terms. An adapter is an implementation of a port that speaks to a specific piece of infrastructure: a file system, a markdown parser, a C NLP library.

vaani has three port traits and five adapters that ship today.

## The three port traits

**`Source`** (`src/source/mod.rs`)

```rust
pub trait Source: Send {
    fn read(&self, input: &Path) -> domain::Result<Vec<domain::RawDocument>>;
    fn accepts(&self, input: &Path) -> bool;
}
```

`read` takes a path and returns raw documents. `accepts` is a cheap pre-check the composition root uses to pick the right adapter for a given path without reading file contents. Return type is `Vec<RawDocument>` because a single path can yield multiple documents (a directory yields one per file).

**`Decomposer`** (`src/decompose/mod.rs`)

```rust
pub trait Decomposer {
    fn decompose(&self, text: &str) -> Vec<Section>;
}
```

Infallible by design. Malformed input is interpreted as best-effort: malformed markdown is treated as plain text rather than returning an error the caller must handle. The decomposer's job is structural extraction, and structural extraction can always produce something.

**`NlpProvider`** (`src/nlp/mod.rs`)

```rust
pub trait NlpProvider: Send {
    fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>>;
}
```

Fallible because NLP parsing depends on an external model that can fail to load, encounter malformed input, or (in the UDPipe case) panic inside C code. The `Send` bound is required because the composition root passes `&dyn NlpProvider` across function call boundaries that may be used in multi-process contexts.

## The five adapters

**`FileSource`** (`src/source/file.rs`). Reads a single regular file. Two pre-read guards before touching bytes: symlinks are rejected (using `symlink_metadata`, which does not traverse); files larger than `MAX_INPUT_BYTES` are rejected based on metadata-reported size before any read into memory. Format is detected from the file extension: `.md` or `.markdown` → `Format::Markdown`, `.pdf` → `Format::Pdf`, `.docx` → `Format::Docx`, everything else → `Format::PlainText`.

**`DirectorySource`** (`src/source/directory.rs`). Reads all regular files in a directory, non-recursively. Symlinks are skipped. Files are yielded sorted by path (lexicographic); consumers may rely on this ordering. Per-file read failures are tolerated: a single unreadable file does not abort the directory walk. `read_collecting_errors` returns both successful documents and per-file errors; the `Source::read` trait method drops per-file errors silently and returns only successes.

**`MarkdownDecomposer`** (`src/decompose/markdown.rs`). Extracts sections by heading depth, identifies blockquotes (flagging paragraphs as `in_blockquote = true`), skips code blocks and pipe-table rows, and ignores YAML frontmatter. Stops at `## References` or `*References*` to avoid including bibliographic content in the parse.

**`PlainTextDecomposer`** (`src/decompose/plain.rs`). Splits on blank lines, producing a single heading-less section with one paragraph per non-empty block.

**`Udpipe`** (`src/nlp/udpipe.rs`). Wraps the `udpipe-rs` crate, which is a Rust binding over the UDPipe C++ library. This is the only file in vaani that imports `udpipe_rs`; the boundary-check script fails CI if any other file does. Every call to `model.parse()` runs inside `catch_unwind` to convert C-side panics into `Error::ParseFailed`. The English model is downloaded on first use, SHA-256 verified against a pinned constant, and loaded from the verified bytes directly; no second disk read between verify and load.

## Adapter rules

Four rules, enforced by CI:

1. No port module imports another port module. `source/mod.rs` does not import `nlp/mod.rs`.
2. `nlp/udpipe.rs` is the only file that imports `udpipe_rs`.
3. Port modules (`source/mod.rs`, `decompose/mod.rs`, `nlp/mod.rs`) import only from `domain`.
4. `metrics/` and `extraction/` import only from `domain` and `stopwords`.

The composition root (`src/lib.rs`) is the sole exception: it knows all adapters and all ports, because it is the assembly point.

## Adapters that do not exist yet

**`PdfDecomposer` and `DocxDecomposer`.** Both format variants exist in `domain::Format`; `FileSource` detects `.pdf` and `.docx` extensions correctly. But `analyze_file()` in the composition root returns `Error::UnsupportedFormat` for those formats today; there is no decomposer to hand the bytes to. These adapters are deferred because no consumer has required them and the PDF/DOCX parsing ecosystem in Rust is not yet mature enough to choose a dependency confidently. The trigger condition: a documented consumer requirement, or a Rust PDF-extraction crate that is stable enough to carry as a dependency. When the adapter lands, no port trait changes.

**WASM `NlpProvider`.** A WASM build requires an `NlpProvider` that runs in a browser sandbox: no file system, no C FFI, no blocking model downloads. This means either a WASM-compatible model format (WebAssembly-compiled UDPipe or a smaller alternative) or a network-backed provider. Deferred until the WASM crust lands as a defined target. The `NlpProvider` trait requires no change.

**Streaming `Source`.** The current `Source::read` returns `Vec<RawDocument>`, all documents at once. A streaming variant would return an iterator or a channel, enabling processing of document collections that exceed memory. Deferred per ADR-0004 (single-crate, no async reactor): streaming arrives when push-semantics or large-corpus consumers exist and justify the complexity. The trigger condition is a consumer requiring processing of more than a few thousand documents without loading all into memory simultaneously.

See [Hex layout](./hex.md) for how these adapters fit into the dependency graph. See [Boundary rules](../reference/boundary-rules.md) for the enforced import constraints.
