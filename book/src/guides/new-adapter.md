# Writing a new adapter

vaani has three port traits: `NlpProvider`, `Decomposer`, and `Source`. Each is a one-method contract. This guide covers all three.

The boundary rule that matters: `domain.rs` imports only `serde`, `thiserror`, and `std`. Port modules import only from `domain`. Adapters belong in their own files; the composition root (`lib.rs`) is the only place that knows about both ports and adapters simultaneously. Do not import one port module from another.

## NlpProvider

`NlpProvider` is the seam between vaani and any NLP backend. Implement it to bring a different parser, a test double, or a model-free stub.

```rust
use vaani::domain::{Result, Sentence, Token};
use vaani::nlp::NlpProvider;

pub struct MyNlp {
    // your model state here
}

impl NlpProvider for MyNlp {
    fn parse(&self, text: &str) -> Result<Vec<Sentence>> {
        // Split on periods as a trivial tokenizer.
        let sentences = text
            .split('.')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Sentence::new(s.to_string(), Vec::new()))
            .collect();
        Ok(sentences)
    }
}
```

**Trait contract:**

- `parse` receives raw text for one paragraph. vaani calls `parse` once per paragraph, not once per document. Do not assume the input is a full document.
- Return one `Sentence` per sentence. Each `Sentence` holds its verbatim text and an ordered `Vec<Token>`.
- Tokens must have `id` values starting at 1. Exactly one token per sentence should have `head = 0` (the root). All other `head` values must point to another token id in the same sentence.
- `NlpProvider` requires `Send`. If your model holds non-`Send` state (C FFI, raw pointers), wrap the call in `std::panic::catch_unwind` and return `Error::ParseFailed` on panic. See `src/nlp/udpipe.rs` for the pattern.

**Token construction** outside the crate uses `Token::builder`, because `Token` is `#[non_exhaustive]`:

```rust
let token = Token::builder(
    1,                          // id (1-based)
    "committee".into(),         // text (surface form)
    "committee".into(),         // lemma
    "NOUN".into(),              // POS tag
    0,                          // head (0 = root)
    "root".into(),              // dep relation
)
.xpos("NN".into())
.feats("Number=Sing".into())
.build();
```

The six required fields are id, text, lemma, pos, head, dep. Optional fields default to empty strings; `is_punct` defaults to `false`.

**Use it:**

```rust
let doc = vaani::analyze(text, &MyNlp { /* ... */ })?;
```

No registration, no plugin table. The trait object goes directly to the analysis functions.

## Decomposer

`Decomposer` splits text into a `Section` tree. Implement it when you have a format vaani does not know about (HTML, reStructuredText, EPUB).

```rust
use vaani::decompose::Decomposer;
use vaani::domain::{Paragraph, Section};

pub struct HtmlDecomposer;

impl Decomposer for HtmlDecomposer {
    fn decompose(&self, text: &str) -> Vec<Section> {
        // Strip tags, treat result as plain text.
        let stripped = strip_html_tags(text);
        let para = Paragraph::new(stripped, false);
        vec![Section::new(None, 0, vec![para])]
    }
}
```

**Trait contract:**

- `decompose` is infallible. Malformed input is handled as best as possible; the trait cannot return an error.
- `Paragraph::new(text, in_blockquote)`: set `in_blockquote = true` for content that should be excluded from metric computation (blockquotes, code blocks).
- `Section::new(heading, level, paragraphs)`: heading is `None` for the intro section before the first heading. Level 0 for plain text; 1+ for heading depth.
- Return paragraphs in document order.

**Use it** via `analyze_from`:

```rust
let sections = HtmlDecomposer.decompose(text);
let sentences = vaani::parse(text, &nlp)?;
let doc = vaani::analyze_from(sections, &sentences)?;
```

Or wire it into your own composition root instead of calling `analyze_markdown` / `analyze`.

## Source

`Source` reads documents from a path. Implement it when you need a different ingestion strategy: in-memory paths, S3 URIs proxied to temp files, database blobs.

```rust
use std::path::Path;
use vaani::domain::{self, Format, RawDocument};
use vaani::source::Source;

pub struct InMemorySource {
    pub content: String,
}

impl Source for InMemorySource {
    fn read(&self, _input: &Path) -> domain::Result<Vec<RawDocument>> {
        Ok(vec![RawDocument::new(
            self.content.clone(),
            None,
            Format::PlainText,
        )])
    }

    fn accepts(&self, _input: &Path) -> bool {
        true
    }
}
```

**Trait contract:**

- `read` translates external errors into `domain::Error` variants. Do not return `Result<T, String>` or `anyhow::Error`. The two I/O-adjacent variants are `Error::Io(std::io::Error)` and `Error::InputTooLarge { limit, actual, what }`.
- `accepts` is a cheap pre-check (no file I/O). The composition root uses it to pick between adapters. It must not read file contents.
- Enforce the 8 MiB cap (`domain::MAX_INPUT_BYTES`) before reading file contents. Check file size via metadata, not by reading. See `src/source/file.rs` for the pattern.
- Reject symlinks before reading. `FileSource` uses `symlink_metadata` (which does not traverse) and rejects any path whose file type is `is_symlink()`.
- `Source` requires `Send`.

## Error translation

All three adapters translate their own failures into `domain::Error`. The full error table:

| Variant | When to use |
|---|---|
| `Error::ModelNotFound(PathBuf)` | NlpProvider: model file absent |
| `Error::ModelInvalid(String)` | NlpProvider: model corrupt or wrong format |
| `Error::ParseFailed(String)` | NlpProvider: parse failed or panicked |
| `Error::InputTooLarge { limit, actual, what }` | Any: input exceeds a size cap; set `what` to your adapter's label |
| `Error::UnsupportedFormat(Format)` | Source/Decomposer: format has no registered handler |
| `Error::Io(std::io::Error)` | Source: file I/O failures |

`Error` is `#[non_exhaustive]`. Do not match it with `_ =>` in adapter code; you should know exactly which variants your adapter can produce.

## Boundary check

Before submitting: run `scripts/check-boundaries.sh`. It enforces that nothing outside `nlp/udpipe.rs` imports `udpipe_rs`, no port module imports another port module, and the composition root is the only place that sees both sides. If you add a new adapter, register it only in `lib.rs`.
