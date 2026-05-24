# Rust usage

## Convenience APIs

The composition root exposes seven top-level functions. Pick the one that matches your input shape.

```rust,ignore
use vaani::{analyze, analyze_markdown, analyze_file, analyze_directory, parse, analyze_from};
use vaani::nlp::udpipe::Udpipe;

// 1. Plain text in memory
let analysis = vaani::analyze(&text, &nlp)?;

// 2. Markdown in memory (honors heading hierarchy)
let analysis = vaani::analyze_markdown(&md_text, &nlp)?;

// 3. A file on disk (format detected from extension)
let analysis = vaani::analyze_file("essay.md", &nlp)?;

// 4. A directory of files (returns Corpus + per-file errors)
let (corpus, errors) = vaani::analyze_directory("./essays", &nlp)?;

// 5. Just parse (for parse-once-use-many)
let sentences = vaani::parse(&text, &nlp)?;

// 6. Build the Document from pre-parsed sentences
let analysis = vaani::analyze_from(sections, &sentences)?;
```

Each entry point checks `MAX_INPUT_BYTES` at the gate before doing real work.

## Just parse

```rust,ignore
use vaani::parse;
use vaani::nlp::NlpProvider;

fn token_stream(text: &str, nlp: &dyn NlpProvider) -> vaani::domain::Result<()> {
    let sentences = parse(text, nlp)?;
    for sentence in sentences {
        for token in &sentence.tokens {
            println!("{:>2} {:<15} {:<6} {:<10} head={}", token.id, token.text, token.pos, token.dep, token.head);
        }
    }
    Ok(())
}
```

## Parse once, use many

`parse` is the single expensive step; `analyze_from`, `tfidf_summarize`, and `rake_keyphrases` are cheap consumers of the parsed sentences. When you want both an `Document` and one or more extractions, parse once and hand the sentences to each consumer:

```rust,ignore
use vaani::{parse, analyze_from};
use vaani::extraction::{tfidf_summarize, rake_keyphrases};
use vaani::decompose::{Decomposer, markdown::MarkdownDecomposer};

let nlp = Udpipe::english("./models")?;
let text = std::fs::read_to_string("essay.md")?;

let sections = MarkdownDecomposer.decompose(&text);
let sentences = parse(&text, &nlp)?;

let analysis = analyze_from(sections, &sentences)?;
let summary = tfidf_summarize(&sentences, 3)?;
let phrases = rake_keyphrases(&sentences, 10)?;
```

This is why `measure` and `extract` are both peers of `parse` in the pipeline, not nested under each other. See [The pipeline](../concepts/pipeline.md) for the architectural rationale.

## Custom NlpProvider

vaani depends on the port, not the adapter. If you have a different NLP backend, implement `NlpProvider`:

```rust,ignore
use vaani::nlp::NlpProvider;
use vaani::domain::{Result, Sentence};

struct MyBackend { /* ... */ }

impl NlpProvider for MyBackend {
    fn parse(&self, text: &str) -> Result<Vec<Sentence>> {
        // Your parser. Postcondition contracts:
        //   - sentences in document order
        //   - tokens within each sentence id-sorted ascending
        //   - exactly one head==0 per sentence
        //   - all head references valid
        unimplemented!()
    }
}

// Use it just like Udpipe.
let nlp = MyBackend { /* ... */ };
let analysis = vaani::analyze(&text, &nlp)?;
```

`NlpProvider` requires `Send` (so the value can cross thread boundaries) but not `Sync` (interior state can be per-instance).

## Per-paragraph control

The composition root parses per-paragraph internally. If you want fine-grained control over which paragraphs to parse:

```rust,ignore
use vaani::decompose::{Decomposer, markdown::MarkdownDecomposer};
use vaani::nlp::NlpProvider;
use vaani::domain::{Document, Sentence};

let mut analysis = Document::new(MarkdownDecomposer.decompose(&text));
let mut all_sentences: Vec<Sentence> = Vec::new();

for para in analysis.paragraphs_mut() {
    // Custom paragraph-level logic: skip blockquotes, code fences, etc.
    if para.in_blockquote || looks_like_code(&para.text) { continue; }

    let parsed = nlp.parse(&para.text)?;
    all_sentences.extend(parsed.iter().cloned());
    para.sentences = parsed;
}
```

## Error handling

See [Errors](../concepts/errors.md). `Result<T, vaani::domain::Error>` is the universal return type; pattern-match on variants for fine-grained recovery.

## Feature flags

| Feature | Default | What it enables |
|---|---|---|
| `udpipe` | yes | The `Udpipe` adapter and the `nlp::udpipe` module. |
| `python` | no | PyO3 bindings (`Vaani` class, `_core` module). Built by `maturin`. |

`cargo check --no-default-features` is a hard CI gate. The domain types, metrics, extraction algorithms, and port traits are all usable without `udpipe`.
