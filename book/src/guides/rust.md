# Rust guide

You have a parsed document. You want metrics, summaries, or keyphrases. This guide covers the day-to-day patterns: which function to call when, and why the shape is what it is.

## Analyze a file

The common case: point vaani at a file and get a `Document` back.

```rust
use vaani::nlp::udpipe::Udpipe;

let nlp = Udpipe::english("/tmp/vaani-models")?;
let doc = vaani::analyze_file("essay.md", &nlp)?;

println!("Sentences: {}", doc.total_sentences());
println!("Passive ratio: {:.1}%", doc.passive_ratio() * 100.0);
println!("Vocabulary TTR: {:.2}", doc.vocabulary_ttr.unwrap_or(0.0));
```

`analyze_file` detects the format from the extension (`.md` routes to the markdown decomposer; everything else is plain text) and rejects symlinks and files over 8 MiB before reading. Format detection is extension-only; the file contents are not inspected.

If you have text in memory rather than on disk, use `analyze` (plain text) or `analyze_markdown` directly:

```rust
let doc = vaani::analyze(text, &nlp)?;
let doc = vaani::analyze_markdown(text, &nlp)?;
```

## Parse once, use many

`analyze_file` and `analyze` each run the NLP parser once. When you want both a `Document` and extraction results from the same text, you can parse once and hand the sentences to multiple consumers:

```rust
use vaani::decompose::Decomposer;
use vaani::decompose::markdown::MarkdownDecomposer;

let sections = MarkdownDecomposer.decompose(text);
let sentences = vaani::parse(text, &nlp)?;

let doc     = vaani::analyze_from(sections, &sentences)?;
let summary = vaani::extraction::tfidf_summarize(&sentences, 3)?;
let phrases = vaani::extraction::rake_keyphrases(&sentences, 10)?;
```

`parse` is the expensive step: UDPipe runs full dependency analysis on every sentence. `tfidf_summarize`, `rake_keyphrases`, and the metric suite all read from the already-parsed sentences. One parse call feeds all three.

If you call `analyze_markdown` and then `tfidf_summarize` on the same text, you parse twice. For a single extraction that is fine. For a pipeline producing multiple outputs from the same text, parse-once-use-many is the right shape.

## Per-paragraph control

When you need paragraph-level metrics (readability grade per paragraph, lexical density per section), iterate the section tree:

```rust
for section in &doc.sections {
    let heading = section.heading.as_deref().unwrap_or("(intro)");
    for para in &section.paragraphs {
        if let Some(grade) = para.readability_grade {
            println!("{heading}: FK grade {grade:.1}");
        }
    }
}
```

Blockquote paragraphs (`para.in_blockquote == true`) are skipped by the metric suite; their `Option<f64>` slots stay `None`. The document-level `vocabulary_ttr` and `nominalization_ratio` fields are computed over the entire text after per-paragraph parse.

## Corpus analysis

To analyze a directory of files with per-file error tolerance:

```rust
let (corpus, errors) = vaani::analyze_directory("./docs", &nlp)?;

println!("Analyzed {} documents", corpus.entries.len());
println!("Corpus passive ratio: {:.1}%", corpus.passive_ratio() * 100.0);

for (path, err) in &errors {
    eprintln!("Failed: {}: {err}", path.display());
}
```

Per-file failures (symlinks, oversized files, UDPipe parse errors) do not abort the walk. The outer `Result` is `Err` only for top-level failures like the directory not existing.

## Custom NlpProvider

The `NlpProvider` trait is the seam between vaani and the underlying parser. If you want to bring a different model, a mock for testing, or a no-op stub:

```rust
use vaani::domain::{Result, Sentence};
use vaani::nlp::NlpProvider;

struct MockNlp;

impl NlpProvider for MockNlp {
    fn parse(&self, text: &str) -> Result<Vec<Sentence>> {
        // Return empty sentences — metrics will be zero/None.
        Ok(Vec::new())
    }
}

let doc = vaani::analyze("Some text", &MockNlp)?;
```

`NlpProvider` requires `Send`. UDPipe holds C-side state that is not thread-safe, which is why the UDPipe adapter wraps `Model::parse` in `catch_unwind` and marks itself as non-`Sync`. A custom provider that owns thread-safe state can implement both `Send` and `Sync` freely.

`Token` has a `TokenBuilder` for constructing tokens outside the crate (the struct is `#[non_exhaustive]`, so struct literal syntax is not available to external callers):

```rust
use vaani::domain::Token;

let token = Token::builder(1, "committee".into(), "committee".into(), "NOUN".into(), 0, "root".into())
    .build();
```

## Error handling

Every public function returns `domain::Result<T>`. Match on concrete variants:

```rust
use vaani::domain::Error;

match vaani::analyze_file("big.bin", &nlp) {
    Err(Error::InputTooLarge { limit, actual, what }) => {
        eprintln!("{what} too large: {actual} bytes (limit {limit})");
    }
    Err(Error::UnsupportedFormat(fmt)) => {
        eprintln!("No decomposer for {fmt:?}");
    }
    Err(Error::Io(e)) => eprintln!("I/O: {e}"),
    Err(e) => eprintln!("Other: {e}"),
    Ok(doc) => { /* ... */ }
}
```

`InputTooLarge.what` tells you which gate fired: `"input"` for the text size gate, `"file_source"` for the pre-read file size check, `"tfidf"` / `"textrank"` / `"rake"` / `"yake"` for per-extractor caps.

Do not match with a wildcard that catches `InputTooLarge` and `UnsupportedFormat` together: the `what` field lets you route per gate. `domain::Error` is `#[non_exhaustive]`, so a `_ =>` arm is required for forward compatibility; put your specific arms first.

## Feature flags

| Flag | What it adds | Default |
|---|---|---|
| `udpipe` | UDPipe adapter + SHA-256 model verification | yes |
| `python` | PyO3 + pythonize bindings | no |

To compile without UDPipe (for a custom provider only):

```toml
[dependencies]
vaani = { version = "0.0.1", default-features = false }
```

`cargo check --no-default-features` compiles the domain, metrics, and extraction layers cleanly. The UDPipe adapter is absent; `NlpProvider` is still there as a port you can implement.
