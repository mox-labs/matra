# Programming model

## Two values: `Ingest` into `Engine`

`Ingest` is where documents come from. `Engine` is what happens to each one.

| Value | Constructors | Meaning |
|---|---|---|
| `Ingest` | `text(string, format)`, `path(file or directory)` | a string is a stream of one, a file is a stream of one, a directory is a stream of many |
| `Engine` | `new(provider, decomposer table)` | the assembled pipeline |

Source variation lives in the data, not in the function namespace, which is why `Engine::analyze` is one function rather than six.

Three ways to run it:

| Call | Does |
|---|---|
| `analyze(ingest)` | a lazy iterator, one result per document; collect into `CorpusResult` to partition successes from failures |
| `analyze_one(raw)` | one document, annotate then compose, returning a `CorpusEntry` |
| `annotate(raw)` then `compose(&mut doc)` | the two stages separately |

Reads are lazy. `Ingest::path` on a directory lists entries up front, and a listing failure is the constructor's `Err`, but no file is read until the iterator is pulled. A per-file failure (unreadable, oversized, a symlink) arrives as an `Err` item carrying the path, so one bad file cannot abort a directory walk.

`compose` is total. It reads what is attached and skips what is not, so it has no failure path.

## Configuration

`Engine::new` takes a provider and a decomposer table, which means the caller decides where the UDPipe model lives. `Config` answers that question once so no caller has to invent a path.

```rust
let engine = Engine::with_defaults()?;
```

That is `Config::resolve()` followed by `Engine::from_config(&cfg)`. Both are additive: `Engine::new`, `Udpipe::english` and `Udpipe::from_path` are unchanged, and an explicit path still wins over everything below.

`Config` resolves locations and defaults, never behavior. It carries the model directory, the semantic threshold, and the default counts and algorithm names, which are all things a caller could pass as arguments instead. It does not carry which metrics run or how output is shaped.

Resolution order, per key: an explicit argument, then the environment, then the config file, then the defaults compiled into the crate from `config/default.toml`. The argument rung has one producer, `Config::with_model_dir`, which returns the same configuration with the model directory replaced and that one key's source changed. It is the rung a command line's `--model-dir` lands on, so the flag reaches the adapter through `Config` rather than past it.

| Variable | Overrides |
|---|---|
| `MATRA_CONFIG_FILE` | the config file path |
| `MATRA_DATA_DIR` | the data root |
| `MATRA_MODEL_DIR` | the model directory |

Paths follow the XDG conventions on Linux and on macOS.

| What | Default | Env override |
|---|---|---|
| config file | `$XDG_CONFIG_HOME/matra/config.toml`, else `~/.config/matra/config.toml` | `MATRA_CONFIG_FILE` |
| data root | `$XDG_DATA_HOME/matra`, else `~/.local/share/matra` | `MATRA_DATA_DIR` |
| models | the data root's `models` subdirectory | `MATRA_MODEL_DIR` |

One exception to the model directory rule: if the data root has no `models` subdirectory yet, and `~/.matra/models` exists and is not empty, that older location is used. matra never creates `~/.matra`. When an existing, non-empty legacy cache is selected it is used as the model directory, downloads and re-downloads included, because the resolved directory is the one handed to `Udpipe::english` and that is where it writes. Create the new location, or set `MATRA_MODEL_DIR`, to move off it.

An empty `~/.matra/models` is not selected. There is no cache in it to keep working, and picking it would send every later download into a directory matra would otherwise never create. A fresh install therefore lands in the new place.

A missing config file is not an error; the built-in defaults stand. A malformed one is `Error::InvalidInput` naming the file and the line, and an algorithm name the build does not know is rejected the same way at resolve time rather than at call time. Every resolved value records which rung it came from, readable through `Config::sources` as a `ValueSource`.

## `annotate` is the only route to the parser

`Engine::annotate` runs the size check, then hands each non-blockquote paragraph to `NlpProvider::parse`. Nothing else in the library calls the parser. That is what makes the 8 MiB cap a property of the pipeline rather than of each entry point: no text over `MAX_INPUT_BYTES` can reach a provider, whichever call you started from.

It is also the cheap route when you want the extractors. Annotate once, read the sentences off the tree, and hand the same slice to every extractor. Nothing is parsed twice.

## Ports and adapters

The domain depends on port traits. Adapters implement them.

| Port | Trait | Adapters this build ships | Feature |
|---|---|---|---|
| `source/` | `Source` | `FileSource`, `DirectorySource` | none |
| `decompose/` | `Decomposer` | `MarkdownDecomposer`, `PlainTextDecomposer` | none |
| `nlp/` | `NlpProvider` | `Udpipe` | `udpipe` (default) |
| `embed/` | `Embedder` | `Model2Vec` | `model2vec` |

Features are additive. Enabling `udpipe` adds UDPipe; disabling it removes UDPipe and nothing else. `cargo check --no-default-features` compiles. The other two features are `python` (the PyO3 bindings) and `cli` (the binary, which implies `udpipe`).

Your own adapter is a trait implementation plus a change at the call site. `Engine::new` takes any `Box<dyn NlpProvider>`; `Decomposers::with` builds a table other than `standard_decomposers()`; `embed_and_cluster` takes any `&dyn Embedder`, and with your own implementation it needs no feature flag at all.

## Determinism and bounds

Every gate returns a typed variant of `Error`. There is no `Result<T, String>` anywhere in the library, and library code does not panic: UDPipe panics at the C boundary are caught in `nlp/udpipe.rs` and converted.

| Gate | Cap | Variant |
|---|---|---|
| `annotate` input text | 8 MiB (`MAX_INPUT_BYTES`) | `InputTooLarge`, `what` is `"input"` |
| file read | 8 MiB | `InputTooLarge`, `what` is `"file_source"` |
| `tfidf_summarize`, `textrank_summarize` | 2,000 sentences | `InputTooLarge`, `what` is `"tfidf"` or `"textrank"` |
| `embed_and_cluster`, `semantic_clusters` | 2,000 sentences | `InputTooLarge`, `what` is `"semantic_clusters"` |
| `rake_keyphrases`, `yake_keyphrases` | 200,000 tokens | `InputTooLarge`, `what` is `"rake"` or `"yake"` |

The `what` discriminator names which gate fired, so a caller can route a document-too-big differently from a corpus-too-big.

Cycle safety is a sentinel, not a ceiling: `Sentence::tree_depth` returns `usize::MAX` on a malformed parse containing a cycle rather than silently truncating.

## Fields cross languages, methods do not

Values reach Python through serde and pythonize. A field has a serde representation and crosses with its name unchanged. A method has none, so there is nothing for it to cross with.

Rust-only, therefore: `Document::total_words`, `total_sentences`, `paragraph_count`, `mean_sentence_length`, `sentence_length_std`; every `Corpus` aggregate (`total_words`, `passive_ratio`, `mean_readability`); the `Sentence` tree walks (`root_token`, `head_of`, `children_of`, `subtree`, `tree_depth`, `is_passive`, `content_tokens`, `word_count`, `reportings_in`, `root_adverbials_in`); `Paragraph::word_count` and `sentence_count`; and `Token::feat`.

When an aggregate needs to be visible cross-language it is materialized as a field. `Document::passive_ratio` is the worked case: the method still exists for Rust callers, and the metric suite writes its value into the `passive_ratio` field, which is what crosses.

## What Python exposes

`Matra`, `Model2Vec`, and one module-level function.

| Python | Rust equivalent |
|---|---|
| `Matra.from_path`, `Matra.english` | `Udpipe::from_path`, `Udpipe::english` wired into an `Engine`; `Matra.english()` with no argument is `Udpipe::from_config` |
| `Matra.analyze`, `Matra.analyze_markdown` | `analyze_one` on a `RawDocument` of that format |
| `Matra.tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases` | `annotate`, then the matching extraction function |
| `Matra.semantic_clusters` | `annotate`, then `embed_and_cluster` |
| `semantic_clusters(vectors, threshold, model_hash)` | `extraction::semantic_clusters` |
| `Model2Vec.from_dir`, `.model_hash`, `.dimensions`, `.embed` | `Model2Vec` behind the `Embedder` port |

Not on the Python surface: directory ingestion (there is no `Ingest` and no path-taking call), the corpus types (`Corpus`, `CorpusEntry`, `CorpusResult`, `DocumentError`), the separate `annotate` and `compose` stages, a custom `NlpProvider` or `Embedder`, and a replaceable metric suite. Python analyzes one in-memory string at a time with the standard wiring; iterating a directory and aggregating is caller code.

`Matra` is `unsendable`, because the UDPipe model holds C-side state that is not thread safe. Multi-process Python is fine; sharing one instance across threads is not.
