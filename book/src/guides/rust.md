# Use matra from Rust

You already have `matra` in your `Cargo.toml`.

Every snippet below assumes it runs inside a function returning `matra::domain::Result<()>`, which is why `?` works.

## The no-setup path

One line, no arguments, no model on disk yet:

```rust
use matra::Engine;

let engine = Engine::with_defaults()?;
```

`Engine::with_defaults` resolves a `Config` (the environment, then your config file, then the defaults compiled into the crate), downloads the English UDPipe model into the resolved directory if it is not already there, and wires the standard decomposer table. The directory is `MATRA_MODEL_DIR` if you set it, otherwise the `models` subdirectory of `$XDG_DATA_HOME/matra`, which defaults to `~/.local/share/matra`. [Programming model](../explanation/programming-model.md#configuration) has the full resolution order.

Everything below is the explicit path, which still works and still wins over the resolved one. Take it when you want the model somewhere specific, or a provider matra does not ship.

## Construct a provider

The pipeline owns a `Box<dyn NlpProvider>`. `Udpipe` is the adapter that ships with the `udpipe` feature (on by default):

```rust
use matra::nlp::udpipe::Udpipe;

let nlp = Udpipe::english("/tmp/matra-models")?;
```

`Udpipe::english` creates the directory if it is missing, downloads the English model (about 16 MB) on first use, verifies it against a pinned SHA-256 hash, and loads from the cached file on every call after that. The download goes to a per-process temporary subdirectory and is moved into place with a single rename, so two processes pointed at the same directory cannot corrupt each other's file.

The path is passed to `std::fs::create_dir_all` unchanged. Rust does not expand `~`, so `Udpipe::english("~/.matra/models")` creates a directory literally named `~` under your current working directory. Pass an absolute path, or expand the home directory yourself.

If you already have a model file on disk, load it directly:

```rust
let nlp = Udpipe::from_path("/path/to/english-ewt-ud-2.5-191206.udpipe")?;
```

`from_path` returns `Error::ModelNotFound` when the path does not exist and `Error::ModelInvalid` when the file exists but fails to load. A third constructor, `Udpipe::from_bytes`, loads from an in-memory byte slice, which suits a model embedded with `include_bytes!`.

Construction is the expensive step: it loads and validates the model once. Build one provider, hand it to one `Engine`, and reuse that engine for every document.

`NlpProvider` is declared `Send`, not `Sync`. A `Box<dyn NlpProvider>` moves between threads, but a reference to it does not, so an `Engine` (and any stream borrowed from it) belongs to one thread at a time. That is a compile error, not a runtime surprise. Build one engine per worker thread, or serialize calls behind a mutex.

## The pipeline

matra's public Rust surface is one pipeline assembled from two values. `Ingest` says where documents come from; `Engine` says what happens to each one.

```rust
use matra::domain::{CorpusResult, Format};
use matra::nlp::udpipe::Udpipe;
use matra::{Engine, Ingest};

let nlp = Udpipe::english("/tmp/matra-models")?;
let engine = Engine::new(Box::new(nlp), matra::standard_decomposers());

// A string is a stream of one.
let one = engine
    .analyze(Ingest::text("Plain text in memory.", Format::PlainText))
    .next()
    .expect("a stream of one")
    .map_err(|e| e.error)?;

// A directory is a stream of many. Same call.
let many: CorpusResult = engine.analyze(Ingest::path("./corpus")?).collect();
```

`Ingest` has two constructors and they cover every source shape:

| Constructor | Yields | Notes |
|---|---|---|
| `Ingest::text(string, format)` | one document | never fails; the format says which decomposer runs |
| `Ingest::path(path)` | one document for a file, zero or more for a directory | `Err` only when the path does not exist or the directory cannot be listed |

Format comes from the extension for `Ingest::path` (`.md` and `.markdown` route to the markdown decomposer, everything else to plain text) and from the argument for `Ingest::text`. Reads are lazy: `Ingest::path` on a directory lists entries up front but touches no file until the stream is pulled, and a per-file failure (unreadable, oversized, a symlink) becomes an `Err` item carrying its path rather than an abort.

`Engine` exposes the pipeline at three levels:

| Method | Takes | Returns | Use it when |
|---|---|---|---|
| `analyze` | anything yielding `Ingested` items | a lazy stream of `Result<CorpusEntry, DocumentError>` | the default: any number of documents, end to end |
| `analyze_one` | one `RawDocument` | `Result<CorpusEntry, DocumentError>` | you have exactly one document and no stream |
| `annotate` | `&RawDocument` | `Result<Document>` | you want structure and sentences without the metric suite |
| `compose` | `&mut Document` | nothing | run the metric suite over an annotated document; total, no failure path |

The three levels always agree: `analyze` is `analyze_one` mapped over the stream, and `analyze_one` is `annotate` followed by `compose`. The library's test suite pins that agreement as equivalence laws, so the levels cannot drift apart silently.

`standard_decomposers()` is the format table this build ships: markdown and plain text. `Error::UnsupportedFormat` means exactly "no entry in the table", and `Pdf`/`Docx` are reserved variants with no entry today. A caller can build a different table with `Decomposers::new().with(format, decomposer)` and hand it to `Engine::new`, which is also how you plug in your own `Decomposer`.

One ordering detail that shapes which error you actually see: `Ingest::path` reads the file with `read_to_string` before the engine looks at the extension. A genuine binary PDF fails the UTF-8 decode first, so you get `Error::Io` with kind `InvalidData`, not `Error::UnsupportedFormat`. You reach `UnsupportedFormat` only when the file is valid UTF-8 and carries a `.pdf` or `.docx` extension.

## What the size gates reject

Six caps guard the pipeline. Each one returns `Error::InputTooLarge`, and the `what` field names which gate fired so you can route on the label instead of guessing from context.

| `what` | Limit | Fires in |
|---|---|---|
| `"input"` | 8 MiB of text (`domain::MAX_INPUT_BYTES`) | `Engine::annotate`, which is the only route from text to the parser, so every pipeline call inherits it |
| `"file_source"` | 8 MiB on disk | `Ingest::path`, checked against file metadata before any read |
| `"tfidf"` | 2000 sentences | `tfidf_summarize` |
| `"textrank"` | 2000 sentences | `textrank_summarize` |
| `"rake"` | 200000 tokens summed across all sentences | `rake_keyphrases` |
| `"yake"` | 200000 tokens summed across all sentences | `yake_keyphrases` |

A document from disk passes two of these in sequence: the metadata check when `Ingest` reads it, then the text check inside `annotate`. A file at exactly the cap is accepted; one byte over is rejected.

The extraction caps are counted after parsing, so a document under 8 MiB can still exceed the TF-IDF sentence cap. Check `sentences.len()` before calling if you need to decide between summarizing and chunking.

## Parse once, use many

The parse inside `annotate` is the expensive step: it runs the NLP provider once per paragraph. The extraction functions (`tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`) are pure functions over `&[Sentence]` and never call the NLP provider themselves. Run the pipeline once, read the sentences back off the tree, and hand the same slice to as many extractors as you need:

```rust
use matra::domain::{Format, RawDocument, Sentence};

let raw = RawDocument::new(text.to_string(), None, Format::Markdown);
let mut doc = engine.annotate(&raw)?;
engine.compose(&mut doc);

let sentences: Vec<Sentence> = doc.sentences().cloned().collect();
let summary = matra::extraction::tfidf_summarize(&sentences, 3)?;
let phrases = matra::extraction::rake_keyphrases(&sentences, 10)?;
```

Each extractor reads the same slice; nothing re-parses. Skip the `compose` call if you only want the extractions and not the metrics. The sentences you read off the tree are the ones the decomposer kept, so markdown headings, fenced code, and blockquotes never reach the extractors as if they were prose.

## Why a metric slot is `None`

Every metric slot is an `Option`, and each metric has its own threshold. A `None` means the metric declined to run, not that it computed nothing.

| Slot | Filled when |
|---|---|
| `Paragraph::readability_grade` | the paragraph has more than 10 words and is not in a blockquote |
| `Paragraph::lexical_density` | the paragraph has at least 1 word and is not in a blockquote |
| `Paragraph::compression_ratio` | the paragraph has more than 50 words, is not in a blockquote, and its text is at most 256 KiB |
| `Document::vocabulary_ttr` | the document's attached sentences hold at least one non-punctuation token |
| `Document::nominalization_ratio` | the same condition as `vocabulary_ttr` |

"Words" here means `Paragraph::word_count()`, the non-punctuation token count summed over the paragraph's attached sentences. A paragraph whose `sentences` vector is empty has a word count of zero and therefore no per-paragraph metrics at all.

Blockquote paragraphs are never parsed. `annotate` skips them, so their `sentences` stays empty, their metric slots stay `None`, and they contribute nothing to `total_words` or `passive_ratio`. They remain in the section tree with `in_blockquote = true` and their `text` intact.

## What markdown decomposition drops

A markdown document (by format argument or by `.md` extension) runs `MarkdownDecomposer`, which discards several kinds of content before any parsing happens:

- YAML frontmatter, when the very first line is `---`, through to the closing `---`
- fenced code blocks, along with the fence lines
- lines beginning with `|`, which is how table rows are excluded
- everything from a line reading `## References` or `*References*` onward, to the end of the document

That last rule ends decomposition rather than skipping a block. A document with a `## References` heading in the middle loses every section after it, silently. If your documents use that heading for something other than a trailing bibliography, decompose with `PlainTextDecomposer` instead, or write your own `Decomposer`.

## Handle `domain::Error`

`annotate` returns `domain::Result<T>`, an alias for `Result<T, domain::Error>`. The stream methods wrap the same error with its path: `analyze` and `analyze_one` return `DocumentError`, whose `path` field is `Some` for documents that came from disk and whose `error` field is the `domain::Error` to match on:

```rust
use matra::domain::Error;

for outcome in engine.analyze(matra::Ingest::path("notes.txt")?) {
    match outcome {
        Ok(entry) => { /* entry.analysis */ }
        Err(doc_err) => match doc_err.error {
            Error::InputTooLarge { limit, actual, what } => {
                eprintln!("{what} gate: {actual} bytes over the {limit} cap");
            }
            Error::UnsupportedFormat(format) => {
                eprintln!("no decomposer registered for {format:?}");
            }
            Error::Io(e) => eprintln!("could not read the file: {e}"),
            Error::ParseFailed(msg) => eprintln!("the provider failed: {msg}"),
            e => eprintln!("analysis failed: {e}"),
        },
    }
}
```

`Error::ParseFailed` also carries panics that crossed the UDPipe FFI boundary. The adapter wraps the C-side call in `catch_unwind` and converts a panic into `ParseFailed` with the payload message, so a bug inside UDPipe surfaces as a matchable error instead of aborting your process.

`domain::Error` is `#[non_exhaustive]`, so a trailing `_ =>` (or a named catch-all binding, as above) is required even once you have named every variant that exists today. A future variant becomes a compile requirement for code that matches without a wildcard, and a routed catch-all for code that has one.

The domain structs carry `#[non_exhaustive]` for the same reason. From outside the crate you cannot write `Token { .. }` or `Sentence { .. }` as struct literals. Use `Token::builder(id, text, lemma, pos, head, dep).build()` and `Sentence::new(text, tokens)`, which matters most when you are building fixtures for tests.

## Analyze a directory

`Ingest::path` on a directory streams one level of it, and collecting into `CorpusResult` partitions the outcomes:

```rust
use matra::domain::CorpusResult;

let result: CorpusResult = engine.analyze(matra::Ingest::path("./corpus")?).collect();

println!(
    "{} documents, {} words",
    result.corpus.entries.len(),
    result.corpus.total_words()
);
for err in &result.errors {
    eprintln!("skipped: {err}");
}
```

`Ingest::path` is `Err` only when the listing itself fails, such as the directory not existing. Everything else lands in `errors`: an unreadable file as `Io`, a file over the cap as `InputTooLarge` with `what = "file_source"`, a non-UTF-8 file as `Io` with kind `InvalidData`, a `.pdf` or `.docx` name that happens to hold UTF-8 as `UnsupportedFormat`, and a provider failure as `ParseFailed`. The partition always holds: entries plus errors equals documents consumed.

Two kinds of entry are skipped without an error entry, so they will not appear in either list: subdirectories, because the walk is one level deep, and symlinks, which are filtered out with `symlink_metadata` so a link cannot redirect the read elsewhere. Files are attempted in sorted path order, and `Corpus` gives you `total_words()`, `passive_ratio()`, and `mean_readability()` across the whole set.

You do not have to collect. The stream from `analyze` is lazy, so a loop that renders each document as it completes holds one document in memory at a time, not the corpus.

## Walk a `Document`

`Document` holds a `sections: Vec<Section>` tree; sections hold `paragraphs: Vec<Paragraph>`; paragraphs hold `sentences: Vec<Sentence>`. Flat iterators cut across the tree when you do not need the structure:

```rust
for sentence in doc.sentences() {
    println!("{}", sentence.text);
}
```

`doc.paragraphs()` and `doc.tokens()` flatten at the other two levels.

A `Sentence` carries its dependency tree in `tokens: Vec<Token>`, where each token's `head` field points at another token's `id` in the same sentence and `head == 0` marks the root. Navigate it with the methods `Sentence` provides rather than walking `tokens` by hand:

```rust
for sentence in doc.sentences() {
    let Some(root) = sentence.root_token() else { continue };
    println!("root: {} ({})", root.text, root.pos);

    for child in sentence.children_of(root.id) {
        println!("  {} --{}--> {}", child.text, child.dep, root.text);
    }
}
```

Token fields carry the CoNLL-U columns under Rust names: `text` is the surface form, `pos` is the universal POS tag, `xpos` is the language-specific tag, and `dep` is the dependency relation. `feats`, `deps`, and `misc` hold their raw column strings, and `is_punct` is derived by the adapter.

`subtree(id)` collects every token reachable from a given token, sorted back into document order, which is the tool for pulling a phrase out from under a dependency relation rather than a single token. Pulling the direct object out of every sentence that has one:

```rust
for sentence in doc.sentences() {
    for token in &sentence.tokens {
        if token.dep == "obj" {
            let phrase: Vec<&str> = sentence
                .subtree(token.id)
                .iter()
                .map(|t| t.text.as_str())
                .collect();
            println!("object phrase: {}", phrase.join(" "));
        }
    }
}
```

matra reports the dependency label UDPipe assigned (`"obj"` here). It does not interpret what the phrase means. That interpretation is your application's job.

The remaining navigation methods: `head_of(id)` returns the head token, or `None` for the root and for an id that is not in the sentence; `content_tokens()` and `word_count()` give you the non-punctuation view; `is_passive()` reports whether any token carries `nsubj:pass`, `nsubjpass`, or `aux:pass`.

`tree_depth()` returns the longest root-to-leaf path in the sentence. On a malformed parse where tokens form a cycle it returns `usize::MAX` rather than truncating at some arbitrary ceiling, so a broken parse is loud rather than quietly shallow. Compare against `usize::MAX` before you use the value in arithmetic. `subtree` and `children_of` stay safe on the same malformed input; both use a visited set.
