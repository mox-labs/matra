# Use matra from Rust

You already have `matra` in your `Cargo.toml` and a UDPipe model on disk. This guide covers the entry points, the size gates, the parse-once-use-many pattern, why a metric slot comes back `None`, error handling, and walking a parsed document's dependency trees.

Every snippet below assumes it runs inside a function returning `matra::domain::Result<()>`, which is why `?` works.

## Construct a provider

Every public function that touches text takes `nlp: &dyn NlpProvider`. `Udpipe` is the adapter that ships with the `udpipe` feature (on by default):

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

Construction is the expensive step: it loads and validates the model once. Build one `Udpipe` and pass `&nlp` to every call that needs it, rather than constructing a new one per document.

`NlpProvider` is declared `Send`, not `Sync`. A `Box<dyn NlpProvider>` moves between threads, but `&dyn NlpProvider` does not, so you cannot hand one shared provider to several worker threads by reference. That is a compile error, not a runtime surprise. Build one provider per worker thread, or serialize calls behind a mutex.

## The six entry points

matra's public Rust API is six functions in `lib.rs`, the composition root. Each takes `&dyn NlpProvider`; together they cover every combination of how your text is decomposed and how much of the pipeline you want to run yourself.

| Function | Takes | Returns | Use it when |
|---|---|---|---|
| `analyze` | `&str` (plain text) | `Document` | You have plain text in memory. Paragraphs split on blank lines. |
| `analyze_markdown` | `&str` (markdown) | `Document` | You have markdown in memory and want section and paragraph structure, with blockquotes flagged out of the metric suite. |
| `analyze_file` | a path | `Document` | You have one file on disk. Format is detected from the extension (`.md` and `.markdown` route to the markdown decomposer, everything else to plain text); the file is read only after a symlink check and a size check pass. |
| `analyze_directory` | a directory path | `(Corpus, Vec<(PathBuf, Error)>)` | You have a directory of files and want per-file error tolerance. |
| `parse` | `&str` | `Vec<Sentence>` | You want the parsed sentences without the metric suite, most often to feed one or more extraction functions. |
| `analyze_from` | pre-decomposed `Vec<Section>` + pre-parsed `&[Sentence]` | `Document` | You already ran a `Decomposer` and `parse` separately. Read the caveat below before you reach for this one expecting a fully populated `Document`. |

`analyze_file` and `analyze_directory` return `Err(Error::UnsupportedFormat(Format::Pdf | Format::Docx))` when they hit a format with no registered decomposer. Both formats are reserved variants today, not shipped decomposers.

One ordering detail that shapes which error you actually see: `FileSource` reads the file with `read_to_string` before it looks at the extension. A genuine binary PDF fails the UTF-8 decode first, so you get `Error::Io` with kind `InvalidData`, not `Error::UnsupportedFormat`. You reach `UnsupportedFormat` only when the file is valid UTF-8 and carries a `.pdf` or `.docx` extension.

## What the size gates reject

Six separate caps guard the pipeline. Each one returns `Error::InputTooLarge`, and the `what` field names which gate fired so you can route on the label instead of guessing from context.

| `what` | Limit | Fires in |
|---|---|---|
| `"input"` | 8 MiB of text (`domain::MAX_INPUT_BYTES`) | `analyze`, `analyze_markdown`, `parse`, and `analyze_from` (which sums the byte length of every paragraph it was handed) |
| `"file_source"` | 8 MiB on disk | `analyze_file` and `analyze_directory`, checked against file metadata before any read |
| `"tfidf"` | 2000 sentences | `tfidf_summarize` |
| `"textrank"` | 2000 sentences | `textrank_summarize` |
| `"rake"` | 200000 tokens summed across all sentences | `rake_keyphrases` |
| `"yake"` | 200000 tokens summed across all sentences | `yake_keyphrases` |

`analyze_file` runs two of these in sequence: the metadata check first, then the text check inside `analyze` or `analyze_markdown`. A file at exactly the cap is accepted; one byte over is rejected.

The extraction caps are counted after parsing, so a document under 8 MiB can still exceed the TF-IDF sentence cap. Check `sentences.len()` before calling if you need to decide between summarizing and chunking.

## Parse once, use many

`parse` is the expensive step in the pipeline: it runs the NLP provider once over the text. The extraction functions (`tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`) are pure functions over `&[Sentence]` and never call the NLP provider themselves. Call `parse` once and hand the result to as many extractors as you need:

```rust
let sentences = matra::parse(text, &nlp)?;

let summary = matra::extraction::tfidf_summarize(&sentences, 3)?;
let phrases = matra::extraction::rake_keyphrases(&sentences, 10)?;
```

Each extractor reads the same slice; neither one re-parses. This is the shape to reach for whenever you want more than one extraction result from the same text.

### `analyze_from` does not attach sentences to paragraphs

If you also want a `Document` from that same parse, `analyze_from` takes the sentences plus the `Vec<Section>` a `Decomposer` produced. Be precise about what it gives you, because the obvious reading is wrong:

```rust
use matra::decompose::Decomposer;
use matra::decompose::markdown::MarkdownDecomposer;

let sections = MarkdownDecomposer.decompose(text);
let sentences = matra::parse(text, &nlp)?;
let doc = matra::analyze_from(sections, &sentences)?;
```

`analyze_from` populates `Document::vocabulary_ttr` and `Document::nominalization_ratio` from the `sentences` slice, because those two metrics read that slice directly. It does not attach the sentences to the paragraphs inside `sections`, and no `Decomposer` fills `Paragraph::sentences` either. Both shipped decomposers leave it empty.

So after the snippet above, `doc` is hollow below the document level. Every per-paragraph slot (`readability_grade`, `lexical_density`, `compression_ratio`) stays `None`, and every `Document` method that counts sentences or words (`total_sentences`, `total_words`, `passive_ratio`, `mean_sentence_length`, `sentence_length_std`) reports zero, because they all walk `paragraphs().flat_map(|p| p.sentences.iter())`, which yields nothing.

This is a rough edge in the current API rather than a subtlety you are meant to have inferred. The crate's own rustdoc example on `analyze_from` and the shipped `examples/parse_once_use_many.rs` both present this combination without the caveat, and the example's printed sentence, word, and passive-ratio line reads as zero when you run it.

Two ways out. If you want a fully populated `Document` and do not mind a second pass over the text, call `analyze` or `analyze_markdown`, which parse each non-blockquote paragraph individually and wire the sentences in themselves. If you want to keep a single parse, attach the sentences to their paragraphs yourself, parsing per paragraph:

```rust
let mut sections = MarkdownDecomposer.decompose(text);
for paragraph in sections.iter_mut().flat_map(|s| &mut s.paragraphs) {
    if !paragraph.in_blockquote {
        paragraph.sentences = matra::parse(&paragraph.text, &nlp)?;
    }
}
let flat: Vec<_> = sections
    .iter()
    .flat_map(|s| s.paragraphs.iter())
    .flat_map(|p| p.sentences.iter().cloned())
    .collect();
let doc = matra::analyze_from(sections, &flat)?;
```

Parse per paragraph here, rather than splitting one whole-text parse across paragraphs afterward. Two paragraphs that share a prefix cannot be told apart by substring matching, and the pipeline was changed to per-paragraph parsing for exactly that reason. `Paragraph::sentences` is a public field so callers can wire it this way.

## Why a metric slot is `None`

Every metric slot is an `Option`, and each metric has its own threshold. A `None` means the metric declined to run, not that it computed nothing.

| Slot | Filled when |
|---|---|
| `Paragraph::readability_grade` | the paragraph has more than 10 words and is not in a blockquote |
| `Paragraph::lexical_density` | the paragraph has at least 1 word and is not in a blockquote |
| `Paragraph::compression_ratio` | the paragraph has more than 50 words, is not in a blockquote, and its text is at most 256 KiB |
| `Document::vocabulary_ttr` | the sentence slice holds at least one non-punctuation token |
| `Document::nominalization_ratio` | the same condition as `vocabulary_ttr` |

"Words" here means `Paragraph::word_count()`, the non-punctuation token count summed over the paragraph's attached sentences. A paragraph whose `sentences` vector is empty has a word count of zero and therefore no per-paragraph metrics at all, which is the mechanism behind the `analyze_from` behavior described above.

Blockquote paragraphs are never parsed. `analyze` and `analyze_markdown` skip them, so their `sentences` stays empty, their metric slots stay `None`, and they contribute nothing to `total_words` or `passive_ratio`. They remain in the section tree with `in_blockquote = true` and their `text` intact.

## What markdown decomposition drops

`analyze_markdown` and `analyze_file` on a `.md` file run `MarkdownDecomposer`, which discards several kinds of content before any parsing happens:

- YAML frontmatter, when the very first line is `---`, through to the closing `---`
- fenced code blocks, along with the fence lines
- lines beginning with `|`, which is how table rows are excluded
- everything from a line reading `## References` or `*References*` onward, to the end of the document

That last rule ends decomposition rather than skipping a block. A document with a `## References` heading in the middle loses every section after it, silently. If your documents use that heading for something other than a trailing bibliography, decompose with `PlainTextDecomposer` instead, or write your own `Decomposer`.

## Handle `domain::Error`

Every public function returns `domain::Result<T>`, an alias for `Result<T, domain::Error>`. Match on the concrete variant rather than treating every failure the same way:

```rust
use matra::domain::Error;

match matra::analyze_file("notes.txt", &nlp) {
    Ok(doc) => { /* ... */ }
    Err(Error::InputTooLarge { limit, actual, what }) => {
        eprintln!("{what} gate: {actual} bytes over the {limit} cap");
    }
    Err(Error::UnsupportedFormat(format)) => {
        eprintln!("no decomposer registered for {format:?}");
    }
    Err(Error::Io(e)) => eprintln!("could not read the file: {e}"),
    Err(Error::ParseFailed(msg)) => eprintln!("the provider failed: {msg}"),
    Err(e) => eprintln!("analysis failed: {e}"),
}
```

`Error::ParseFailed` also carries panics that crossed the UDPipe FFI boundary. The adapter wraps the C-side call in `catch_unwind` and converts a panic into `ParseFailed` with the payload message, so a bug inside UDPipe surfaces as a matchable error instead of aborting your process.

`domain::Error` is `#[non_exhaustive]`, so a trailing `_ =>` (or a named catch-all binding, as above) is required even once you have named every variant that exists today. A future variant becomes a compile requirement for code that matches without a wildcard, and a routed catch-all for code that has one.

The domain structs carry `#[non_exhaustive]` for the same reason. From outside the crate you cannot write `Token { .. }` or `Sentence { .. }` as struct literals. Use `Token::builder(id, text, lemma, pos, head, dep).build()` and `Sentence::new(text, tokens)`, which matters most when you are building fixtures for tests.

## Analyze a directory

`analyze_directory` reads one level of a directory and tolerates per-file failure:

```rust
let (corpus, errors) = matra::analyze_directory("./corpus", &nlp)?;

println!("{} documents, {} words", corpus.entries.len(), corpus.total_words());
for (path, error) in &errors {
    eprintln!("skipped {}: {error}", path.display());
}
```

The outer `Result` is `Err` only when the listing itself fails, such as the directory not existing. Everything else lands in the error vector: an unreadable file as `Io`, a file over the cap as `InputTooLarge` with `what = "file_source"`, a non-UTF-8 file as `Io` with kind `InvalidData`, a `.pdf` or `.docx` name that happens to hold UTF-8 as `UnsupportedFormat`, and a provider failure as `ParseFailed`.

Two kinds of entry are skipped without an error entry, so they will not appear in either return value: subdirectories, because the walk is one level deep, and symlinks, which are filtered out with `symlink_metadata` so a link cannot redirect the read elsewhere. Files are attempted in sorted path order, and `Corpus` gives you `total_words()`, `passive_ratio()`, and `mean_readability()` across the whole set.

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
