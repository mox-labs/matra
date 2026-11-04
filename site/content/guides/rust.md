# Use matra from Rust

You already have `matra` in your `Cargo.toml`.

Every snippet below assumes it runs inside a function returning `matra::domain::Result<()>`, which is why `?` works.

## The no-setup path

One line, no arguments, no model on disk yet:

```rust
use matra::Engine;

let engine = Engine::new()?;
```

`Engine::new` locates the model directory from the environment, downloads the English UDPipe model into it if it is not already there, and wires the readers this build ships. The directory is `MATRA_MODEL_DIR` if you set it, otherwise the `models` subdirectory of `$XDG_DATA_HOME/matra`, which defaults to `~/.local/share/matra`. It reads no configuration file. [Programming model](../explanation/programming-model.md#configuration) has the full resolution order.

When you want your configuration file read, or the resolved values but not all of them, hold the `Config` yourself:

```rust
use matra::config::Config;

let cfg = Config::resolve()?.with_model_dir("/opt/matra/models");
let engine = Engine::from_config(&cfg)?;
```

`with_model_dir` puts the directory on the `Argument` rung, which outranks the environment and the defaults, and `cfg.sources()` reports where every other value came from. `Config` is also what `Model2Vec::from_config` reads. `Engine::from_config_with_notice` takes a closure called once before each model download, with what is about to arrive and its licence, so a caller that renders can say so.

## Where the model lives

The parser is UDPipe with one pinned English model, `english-ewt-ud-2.5-191206`, and the engine builds it for you: the parser and reader traits are private to the crate, so there is no provider to construct. Name a directory you own: provisioning trusts what it finds there, and a shared one such as `/tmp` is writable by anybody on the machine. The engine creates the directory if it is missing, downloads the model (about 16 MB) on first use, verifies it against a pinned SHA-256 hash, and loads from the cached file on every call after that. The verified bytes are written to a temporary subdirectory whose name is unique to the call (the process id, the clock in nanoseconds, and a counter) and moved into place with a single rename, so two processes pointed at the same directory cannot corrupt each other's file, even in containers where every process is pid 1.

The path is passed to `std::fs::create_dir_all` unchanged, and `~` in it is taken literally: neither Rust nor matra expands it, so `with_model_dir("~/models")` creates a directory literally named `~` under your current working directory. The same holds for `MATRA_MODEL_DIR` and the other variables when the shell has not already expanded them. Pass an absolute path, or expand the home directory yourself.

Construction is the expensive step: it loads and validates the model once. Build one engine and reuse it for every document. An `Engine` is `Send` but not `Sync`: it moves to another thread, but it is not shared between two, and a stream borrowed from it stays on the thread that holds it. That is a compile error, not a runtime surprise. To parallelize, build one engine on each worker thread, or serialize calls behind a mutex.

## The pipeline

matra's public Rust surface is one pipeline assembled from two values. `Input` says what one document is; `Engine` says what happens to it.

```rust
use matra::domain::Format;
use matra::{Engine, Input, Inputs};

let engine = Engine::new()?;

// One document.
let doc = engine.analyze(Input::text("Plain text in memory.", Format::PlainText))?;
println!("{} sentences", doc.metrics.total_sentences);

// Many documents, lazily and in order. A failure is one item, and the
// walk goes on past it.
for item in engine.analyze_all(Inputs::dir("./corpus")?) {
    match item {
        Ok(doc) => println!("{:?}", doc.provenance.source.path),
        Err(e) => eprintln!("skipped: {e}"),
    }
}
```

`Input` has five constructors. None of them reads anything or fails: the read, the format and the size cap are settled when the engine reaches the input.

| Constructor | The document is | Format from |
|---|---|---|
| `Input::text(string, format)` | the text itself | the argument |
| `Input::bytes(data, format)` | bytes from anywhere | a PDF or DOCX signature, else the argument, else plain text |
| `Input::file(path)` | the file, read when analysed | a PDF or DOCX signature, else the extension |
| `Input::file_as(path, format)` | the file, read when analysed | a PDF or DOCX signature, else the argument |
| `Input::blocks(blocks)` | headings and paragraphs you supply (`Block::heading`, `Block::paragraph`) | `blocks` |

`.md` and `.markdown` read as markdown; any other extension, or none, reads as plain text when the bytes are UTF-8. `Format::Blocks` names the same headings and paragraphs as a JSON array, for `Input::text`, `Input::bytes` or `Input::file_as`, since no extension implies it, with the schema in `spec/schema/v2/blocks.schema.json`. `Inputs::dir(path)` lists one level of a directory, up front, and is an iterator of `Input::file`; its `Err` is the listing failing. `Input::file` on a directory fails with `Error::InvalidInput` naming `Inputs::dir`.

`Engine` exposes the pipeline at two grains:

| Method | Takes | Returns | Use it when |
|---|---|---|---|
| `analyze` | one `Input` | `Result<Document>` | one document, end to end |
| `analyze_all` | anything yielding `Input` | a lazy stream of `Result<Document, DocumentError>` | any number of documents; a failure is one item carrying the path, and the stream goes on |

The grains always agree: `analyze_all` is `analyze` mapped over the inputs, for every `Input` constructor. Inside, `analyze` reads and parses the document, then measures it; those two stages are the engine's own, and the library's test suite pins that analysis is exactly the one followed by the other, so the grains cannot drift apart silently.

The readers this build ships are markdown, plain text and block input. `Format::Pdf` and `Format::Docx` are recognised, by extension and by signature, and have no reader: analysing one returns `Error::UnsupportedFormat` with that format, and so do bytes that are not UTF-8 and carry no signature matra recognises (with `format: None`). The signature outranks the extension and the format you name, so a PDF renamed `notes.txt` is still refused as a PDF rather than read as text.

## What the size gates reject

Seven caps guard the pipeline and the functions over its output. Each one returns `Error::InputTooLarge`, and the `what` field names which gate fired so you can route on the label instead of guessing from context.

| `what` | Limit | Fires in |
|---|---|---|
| `"input"` | 8 MiB (`domain::limits::MAX_INPUT_BYTES`) | the engine's reading stage, which is the only route from text to the parser, on the bytes handed in and on the analysed text, so every `Input` constructor and every pipeline call inherits it |
| `"file_source"` | 8 MiB on disk | `Input::file`, checked against file metadata before any read |
| `"tfidf"` | 2000 sentences | `summarize`, and `summarize_with` with `SummaryMethod::TfIdf` |
| `"textrank"` | 2000 sentences | `summarize_with` with `SummaryMethod::TextRank` |
| `"rake"` | 200000 tokens summed across all sentences | `keyphrases`, and `keyphrases_with` with `KeyphraseMethod::Rake` |
| `"yake"` | 200000 tokens summed across all sentences | `keyphrases_with` with `KeyphraseMethod::Yake` |
| `"clusters"` | 2000 sentences | `clusters`, before the embedding pass |

Three more labels guard what matra reads that is not your text: `"udpipe_download"` and `"embedding_download"` cap a model download at 64 MiB, and `"config_file"` caps the config file at 64 KiB. [Errors](../reference/errors.md#inputtoolarge) lists all ten.

A document from disk passes the gates in sequence: the metadata check when the file is read, then the byte and text checks before it is parsed. Block input is checked on its analysed text, the blocks joined, so blocks each under the cap that join to more than it are refused. A file at exactly the cap is accepted; one byte over is rejected.

The outcome caps are counted over the parsed document, so a document under 8 MiB can still exceed the TF-IDF sentence cap. Check `doc.metrics.total_sentences` before calling if you need to decide between summarizing and chunking.

## Parse once, use many

The parse inside `analyze` is the expensive step: it runs the parser once per paragraph. The outcome functions are free functions over the `Document` it returns, and never parse anything themselves, so they need no engine and run in a build without UDPipe. Analyze once, and hand the same document to as many outcomes as you need:

```rust
use matra::{Input, KeyphraseMethod, SummaryMethod};
use matra::domain::Format;

let doc = engine.analyze(Input::text(text, Format::Markdown))?;

let summary = matra::summarize(&doc, 3)?;
let ranked = matra::summarize_with(&doc, 3, SummaryMethod::TextRank)?;
let phrases = matra::keyphrases_with(&doc, 10, KeyphraseMethod::Yake)?;
```

Each outcome reads the same document; nothing re-parses. The sentences it reads are the ones the reader kept, so markdown headings, code, tables and quotes never reach a summary as if they were prose.

The method is optional. `summarize` runs TF-IDF and `keyphrases` runs RAKE, the defaults `config/default.toml` ships; `summarize_with` and `keyphrases_with` take a `SummaryMethod` or `KeyphraseMethod`, both `#[non_exhaustive]`. Every result is a record carrying the document's `provenance` plus the `method` that ran (`tfidf`, `textrank`, `rake`, `yake`) and the `parameters` it ran with (`n`), because the method decides what a score means: TF-IDF and TextRank, or RAKE and YAKE, score on unrelated scales.

| Function | Returns | Holds |
|---|---|---|
| `summarize`, `summarize_with` | `Summary` | `sentences`, at most `n`, in document order: each a `SummarySentence` with its `index` among the document's sentences, its `span`, `text` and `score` |
| `keyphrases`, `keyphrases_with` | `Keyphrases` | `phrases`, at most `n`, highest score first: each a `Keyphrase` with its `phrase` (lowercased lemmas), `score` and `occurrences`, the span of every place it was found |
| `clusters` | `Clusters` | `clusters` of sentence indices and every `edges` pair that cleared the threshold; see [Cluster sentences by meaning](semantic-clusters.md) |

Keyphrases with equal scores come back in the same order on every run, the order the phrases are first met, so a tie at the requested count is decided the same way each time.

## Why a metric slot is `None`

Every ratio in `ParagraphMetrics` and `DocumentMetrics` is an `Option`, and each metric has its own threshold. A `None` means the metric declined to run, not that it computed nothing. The counts in `DocumentMetrics` (`total_words`, `total_sentences`, `paragraph_count`) are plain integers.

| Slot | Filled when |
|---|---|
| `ParagraphMetrics::readability_grade` | the paragraph has more than 10 words and is not a quote |
| `ParagraphMetrics::lexical_density` | the paragraph has at least 1 word and is not a quote |
| `ParagraphMetrics::compression_ratio` | the paragraph has more than 50 words, is not a quote, and its text is at most 256 KiB |
| `DocumentMetrics::vocabulary_ttr` | the document's attached sentences hold at least one non-punctuation token |
| `DocumentMetrics::nominalization_ratio` | the same condition as `vocabulary_ttr` |
| `DocumentMetrics::passive_ratio`, `DocumentMetrics::mean_sentence_length` | the document holds at least one sentence |
| `DocumentMetrics::sentence_length_std` | the document holds at least two sentences |

"Words" here means `Paragraph::word_count()`, the non-punctuation token count summed over the paragraph's attached sentences. A paragraph whose `sentences` vector is empty has a word count of zero and therefore no per-paragraph metrics at all.

Quotes are never parsed. A paragraph inside a block quote, at any depth, has `kind` `ParagraphKind::Quote`; the parser never sees it, so its `sentences` stays empty, its metric slots stay `None`, and it contributes nothing to `total_words` or `passive_ratio`, though it counts in `paragraph_count`. It stays in the section tree with its text, and its `span` covers the `>` markers.

## What the markdown reader skips

A markdown document (by format argument or by `.md` extension) is read as CommonMark, with tables, strikethrough, task lists and front matter. Headings, paragraphs, list items and quotes become the section tree; a `#` with no space after it, as in `#hashtag`, is text, and `## References` is a heading like any other. Each paragraph's `text` is what the parser reads, with the inline markup removed (emphasis, link and image syntax, the backticks of a code span) and escapes and entities decoded.

What it does not analyse it records in `Document::skipped`, each entry a `SkippedKind` and a `span`, never dropping it silently:

- `FrontMatter`: YAML between `---` lines, or TOML between `+++` lines, at the very start
- `Code`: fenced and indented code blocks
- `Table`: a table, header and rows
- `Html`: an HTML block
- `ThematicBreak`: `---`, `***` or `___` on a line of its own
- `LinkDefinition`: a link reference definition, `[label]: https://example.com`

Every byte of the input that is not whitespace lies in exactly one heading span, paragraph span or skipped span. The test suite checks that over every conformance fixture and thousands of generated documents.

## Spans

Every heading (`Section::span`), paragraph, skipped construct, sentence and token carries a `Span { start, end }` into the analysed text, in UTF-8 bytes, so `&text[span.start..span.end]` is what it came from. For markdown and plain text the analysed text is the input; for block input it is the blocks' texts joined by a blank line. The text fields are normalized (a sentence's `text` is rebuilt from its tokens, a paragraph's has its markup removed); the span is the authority on where a value came from. `provenance.source` records the analysed text's length and SHA-256.

## Handle `domain::Error`

`analyze` and the outcome functions return `domain::Result<T>`, an alias for `Result<T, domain::Error>`. The stream wraps the same error with its path: `analyze_all` yields `DocumentError`, whose `path` field is `Some` for a file input and whose `error` field is the `domain::Error` to match on:

```rust
use matra::domain::Error;

for outcome in engine.analyze_all(matra::Inputs::dir("notes/")?) {
    match outcome {
        Ok(doc) => { /* doc.metrics */ }
        Err(doc_err) => match doc_err.error {
            Error::InputTooLarge { limit, actual, what } => {
                eprintln!("{what} gate: {actual} bytes over the {limit} cap");
            }
            Error::UnsupportedFormat { format, .. } => {
                eprintln!("no reader for {format:?}");
            }
            Error::Io(e) => eprintln!("could not read the file: {e}"),
            Error::ParseFailed(msg) => eprintln!("the parser failed: {msg}"),
            e => eprintln!("analysis failed: {e}"),
        },
    }
}
```

`Error::ParseFailed` also carries panics that crossed the UDPipe FFI boundary. The adapter wraps the C-side call in `catch_unwind` and converts a panic into `ParseFailed` with the payload message, so a bug inside UDPipe surfaces as a matchable error instead of aborting your process. It is also what the parser returns when it cannot place a token in the text it was given, rather than reporting a span it guessed.

To pass an error on to another program, use what every channel carries rather than the message. `e.kind()` is a stable string such as `"input_too_large"`. `e.details()` holds every dynamic value in the message by name, so `e.details().get("limit")` reads a cap without matching the variant. `serde_json::to_value(&e)` is `{"kind", "message", "details"}`, and a `DocumentError` serializes the same way with its `path` beside them. [Errors](../reference/errors.md#kinds-and-details) lists the keys for every kind.

`domain::Error` is `#[non_exhaustive]`, so a trailing `_ =>` (or a named catch-all binding, as above) is required even once you have named every variant that exists today. A future variant becomes a compile requirement for code that matches without a wildcard, and a routed catch-all for code that has one.

The domain structs carry `#[non_exhaustive]` for the same reason. From outside the crate you cannot write `Token { .. }` or `Sentence { .. }` as struct literals. Use the constructors, which matters most when you are building fixtures for tests:

```text
Token::builder(id: usize, text: String, lemma: String, pos: String, head: usize, dep: String) -> TokenBuilder
Sentence::new(text: String, tokens: Vec<Token>) -> Sentence
```

Every string argument is an owned `String`, not a `&str`, so a literal needs `.to_string()` on the way in. `TokenBuilder` takes the remaining CoNLL-U columns through `xpos`, `feats`, `deps` and `misc`, each also a `String`, plus `is_punct` for the derived flag and `span` for where the token lies in its text, and `build()` hands back the `Token`. `Sentence::new` derives the sentence's span from its first and last token. `feats` takes column 6 as CoNLL-U writes it and converts it into the token's `features` map:

```rust
use matra::domain::Token;

let token = Token::builder(
    1,
    "approved".to_string(),
    "approve".to_string(),
    "VERB".to_string(),
    0,
    "root".to_string(),
)
.feats("Mood=Ind|Tense=Past".to_string())
.build();

assert_eq!(token.feat("Tense"), Some("Past"));
assert_eq!(token.features.get("Mood").map(String::as_str), Some("Ind"));
```

## Analyze a directory

`Inputs::dir` lists one level of a directory, and `analyze_all` streams it, one result per file:

```rust
let (documents, errors): (Vec<_>, Vec<_>) = engine
    .analyze_all(matra::Inputs::dir("./corpus")?)
    .partition(Result::is_ok);

let words: usize = documents
    .iter()
    .flatten()
    .map(|doc| doc.metrics.total_words)
    .sum();
println!("{} documents, {words} words", documents.len());
for err in errors.iter().filter_map(|r| r.as_ref().err()) {
    eprintln!("skipped: {err}");
}
```

`Inputs::dir` is `Err` only when the listing itself fails, such as the directory not existing. Everything else is an `Err` item carrying its path: an unreadable file as `Io`, a file over the cap as `InputTooLarge` with `what = "file_source"`, a non-UTF-8 file, a PDF or a DOCX (by signature or by name) as `UnsupportedFormat`, and a parser failure as `ParseFailed`. Documents plus errors always equals inputs consumed, and each document names its file in `provenance.source.path`.

Two kinds of entry are skipped without an error entry, so they will not appear in either list: subdirectories, because the walk is one level deep, and symlinks, which are filtered out with `symlink_metadata` so a link cannot redirect the read elsewhere. Files are attempted in sorted path order. How to aggregate across documents is yours to decide: a count such as `total_words` sums, and a ratio such as `vocabulary_ttr` does not, since it falls as a document grows.

You do not have to collect. The stream from `analyze_all` is lazy, so a loop that renders each document as it completes holds one document in memory at a time, not the corpus.

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

Token fields carry the CoNLL-U columns under Rust names: `text` is the surface form, `pos` is the universal POS tag, `xpos` is the language-specific tag, and `dep` is the dependency relation. `features` is the FEATS column as a map from feature name to value (`token.feat("Mood")` reads one), `deps` and `misc` hold their raw column strings, and `is_punct` is derived by the adapter.

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

The remaining navigation methods: `head_of(id)` returns the head token, or `None` for the root and for an id that is not in the sentence; `content_tokens()` gives you the non-punctuation view. Three counts are fields, computed when the sentence is built: `word_count`, the non-punctuation token count; `passive`, whether any token carries `nsubj:pass`, `nsubjpass`, or `aux:pass`; and `tree_depth`.

`tree_depth` is the longest root-to-leaf path in the sentence, an `Option<usize>`. On a malformed parse where tokens form a cycle it is `None` rather than a depth truncated at some arbitrary ceiling, so a broken parse is a value the compiler makes you handle rather than a number that reads as a deep tree. `subtree` and `children_of` stay safe on the same malformed input; both use a visited set.
