# Programming model

## Two values: `Input` into `Engine`

`Input` is one document. `Engine` is what happens to it.

| Value | Constructors | Meaning |
|---|---|---|
| `Input` | `text(string, format)`, `bytes(data, format)`, `file(path)`, `file_as(path, format)`, `blocks(blocks)` | one document: text in a named format, bytes from anywhere, a file read as its name says or as a format you name, or headings and paragraphs you supply |
| `Inputs` | `dir(path)` | the files in a directory, an iterator of `Input::file` |
| `Engine` | `new()`, `from_config(cfg)`, `from_config_with_notice(cfg, notice)` | the assembled pipeline: UDPipe with its pinned model, and the readers this build ships |

Source variation lives in the data, not in the function namespace, which is why `Engine::analyze` is one function rather than one per source and format.

Two ways to run it:

| Call | Does |
|---|---|
| `analyze(input)` | one document, end to end, returning a `Document` |
| `analyze_all(inputs)` | a lazy iterator, one result per input, in order; partitioning successes from failures is the caller's |

And what to ask of the result, with free functions over the `Document` that need no engine and no parser:

| Call | Returns |
|---|---|
| `summarize(&doc, n)`, `summarize_with(&doc, n, SummaryMethod)` | `Summary`: the `n` sentences that best represent the document, in document order |
| `keyphrases(&doc, n)`, `keyphrases_with(&doc, n, KeyphraseMethod)` | `Keyphrases`: at most `n` phrases, highest score first, each with every place it occurs |
| `clusters(&doc, &embedder, threshold)` | `Clusters`: sentences grouped by the similarity of their embeddings, with the embedder you pass |

The method is optional and never in a function's name; every result records the method that ran and its parameters in its `provenance`, because which method ran decides what a score means.

Reads are lazy. An `Input` reads nothing until the engine reaches it, and `Inputs::dir` lists entries up front, a listing failure being its `Err`, but reads no file until the iterator is pulled. A per-file failure (unreadable, oversized, a symlink, bytes that are not text) arrives as an `Err` item carrying the path, so one bad file cannot abort a directory walk.

Behind `analyze`, the input side is cut by decision, one crate-private module each: fetching the bytes (with the symlink and size guards), detecting the format (a PDF or DOCX signature, then the format you named, then the extension), reading one format into blocks with their spans, building the section tree from blocks, and parsing. A new format is a reader and a registration line; no published signature moves.

Inside the engine, analysis is two stages: reading and parsing (`annotate`), then measuring (`compose`). Both are crate-private, since a public parse-only call would publish a second shape of `Document` with every metric empty, and the library's tests pin that analysis is exactly the one followed by the other. Measuring is total: it reads what is attached and skips what is not, so it has no failure path.

One short Markdown document at each stage, as matra produces them:

<figure-pipeline input="field-notes" />

## Configuration

The engine needs to know where the UDPipe model lives. `Config` answers that question once so no caller has to invent a path.

```rust
let engine = Engine::new()?;
let engine = Engine::from_config(&Config::resolve()?)?;
```

`Engine::new()` resolves the locations from the environment alone and reads no configuration file; `Engine::from_config(&Config::resolve()?)` reads the file as well. An explicit directory, `Config::with_model_dir`, wins over everything below.

`Config` resolves locations and defaults, never behavior. It carries the model directory, the embedding model's directory name, and the default counts, methods and cluster threshold the command line uses, which are all things a caller could pass as arguments instead. It does not carry which metrics run or how output is shaped. A key 0.3.0 renamed or removed (`[summarize]`, `algorithm`, `[semantic]`, `models.udpipe`) is refused with `Error::InvalidInput` naming what replaced it, so a config file from 0.2 fails to load rather than half-load.

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

One exception to the model directory rule: if the data root has no `models` subdirectory yet, and `~/.matra/models` exists and is not empty, that older location is used. matra never creates `~/.matra`. When an existing, non-empty legacy cache is selected it is used as the model directory, downloads and re-downloads included, because the resolved directory is the one handed to the UDPipe adapter and that is where it writes. Create the new location, or set `MATRA_MODEL_DIR`, to move off it.

An empty `~/.matra/models` is not selected. There is no cache in it to keep working, and picking it would send every later download into a directory matra would otherwise never create. A fresh install therefore lands in the new place.

A missing config file is not an error; the built-in defaults stand. A malformed one is `Error::InvalidInput` naming the file and the line, and an algorithm name the build does not know is rejected the same way at resolve time rather than at call time. Every resolved value records which rung it came from, readable through `Config::sources` as a `ValueSource`.

## `annotate` is the only route to the parser

The engine's reading stage, `annotate`, runs the size check on every `Input`, then hands each paragraph that is not a quote to the parser (`NlpProvider::parse`, a crate-private trait), and maps the token and sentence spans the parser reports against the paragraph's text into the analysed text. Nothing else in the library calls the parser. That is what makes the 8 MiB cap a property of the pipeline rather than of each entry point: no text over `MAX_INPUT_BYTES` can reach the parser, whichever call or constructor you started from.

It is also why the outcomes are cheap. Analyze once, and hand the same document to every outcome function: none of them parses anything. Nothing is parsed twice.

## Ports and adapters

The pipeline depends on port traits, and adapters implement them. The domain depends on neither: the ports import the domain, never the reverse.

| Port | Trait | Adapters this build ships | Feature |
|---|---|---|---|
| `source/` | `Source` (crate-private) | `FileSource`; `DirectorySource` lists a directory for it and reads nothing | none |
| `decompose/` | `Decomposer` (crate-private) | the markdown, plain-text and block-input readers | none |
| `nlp/` | `NlpProvider` (crate-private) | `Udpipe` | `udpipe` (default) |
| `embed/` | `Embedder` | `Model2Vec` | `model2vec` |

Features are additive. Enabling `udpipe` adds UDPipe; disabling it removes UDPipe and nothing else. `cargo check --no-default-features` compiles. The other two features are `cli` (the command line, which implies `udpipe`) and `python` (the PyO3 bindings, which implies `cli` so that a wheel carries the command).

Every port is open to your own adapter. `clusters` takes any `&dyn Embedder`, and with your own implementation it needs no feature flag at all. `Engine::with_parser` takes a parser of your own (`NlpProvider`), in any build; `with_reader` registers a reader (`Decomposer`) for a format, a `Format::Other` you name included; and `with_source` reads the paths inputs name from somewhere else (`Source`). The engine checks what a parser or a reader returns before it reaches a `Document`, so a part of yours cannot hand the outcomes a tree with a cycle or a span outside its text. Without implementing anything, bytes you fetch yourself go in through `Input::bytes`, and a format matra has no reader for goes in as headings and paragraphs through `Input::blocks`.

## Determinism and bounds

Every gate returns a typed variant of `Error`. There is no `Result<T, String>` anywhere in the library, and library code does not panic: UDPipe panics at the C boundary are caught in `nlp/udpipe.rs` and converted.

| Gate | Cap | Variant |
|---|---|---|
| reading, every `Input` | 8 MiB (`MAX_INPUT_BYTES`) | `InputTooLarge`, `what` is `"input"` |
| file read | 8 MiB | `InputTooLarge`, `what` is `"file_read"` |
| `summarize`, `summarize_with` | 2,000 sentences | `InputTooLarge`, `what` is `"tfidf"` or `"textrank"` |
| `clusters` | 2,000 sentences | `InputTooLarge`, `what` is `"clusters"` |
| `keyphrases`, `keyphrases_with` | 200,000 tokens | `InputTooLarge`, `what` is `"rake"` or `"yake"` |
| the pinned UDPipe model being downloaded | 64 MiB | `InputTooLarge`, `what` is `"parser_model_download"` |
| one pinned embedding artifact being downloaded | 64 MiB | `InputTooLarge`, `what` is `"embedding_model_download"` |
| the config file, checked against its metadata | 64 KiB | `InputTooLarge`, `what` is `"config_file_read"` |

The `what` discriminator names which gate fired, so a caller can route a document-too-big differently from a corpus-too-big.

Cycle safety is a value, not a ceiling: `Sentence::tree_depth` is `None` on a malformed parse containing a cycle rather than silently truncated.

## Fields cross languages, methods do not

Values reach Python through serde and pythonize. A field has a serde representation and crosses with its name unchanged. A method has none, so there is nothing for it to cross with.

Rust-only, therefore: the `Document` iterators (`paragraphs`, `sentences`, `tokens`); the `Sentence` tree walks and filters (`root_token`, `head_of`, `children_of`, `subtree`, `content_tokens`, `clausal_complements_in`, `root_adverbials_in`); `Paragraph::word_count` and `sentence_count`; and `Token::feat`.

When an aggregate needs to be visible cross-language it is materialized as a field. The document's counts, sentence-length statistics and ratios are the fields of `DocumentMetrics`, which the metric suite fills, and a sentence's `word_count`, `tree_depth` and `passive` are fields set when it is built.

## What Python exposes

The same words Rust has, in Python's syntax: `Engine`, the read-only `Document`, the outcome functions, `Model2Vec`, and the entry point the `matra` console script launches. A Rust `Input` constructor is a Python keyword, and a Rust `_with` form is Python's `method=`.

| Python | Rust equivalent |
|---|---|
| `Engine()`, `Engine.from_config(config_file=None)` | `Engine::new()`, and `Engine::from_config` over `Config::from_file(path)`, or over `Config::resolve()` with no file named |
| `engine.analyze(text, format=)` | `analyze(Input::text(text, format))`, plain text when no format is named |
| `engine.analyze(file=)`, `bytes=`, `blocks=` | `analyze` on `Input::file` (`Input::file_as` with `format=`), `Input::bytes` or block input |
| `engine.analyze_all(dir=)`, `texts=` | `analyze_all` over `Inputs::dir`, or over `Input::text` per text, lazily |
| `doc["metrics"]`, `doc.to_dict()` | the fields of `Document`, through serde |
| `summarize(doc, n=3, method=)`, `keyphrases(doc, n=10, method=)` | `summarize` or `summarize_with`, `keyphrases` or `keyphrases_with` |
| `clusters(doc, embedder, threshold=0.85)` | `clusters` |
| `span_text(text, span)` | slicing the analysed text by a `Span` |
| `Model2Vec.from_dir`, `.model_hash`, `.dimensions`, `.embed`, `.identity` | `Model2Vec` behind the `Embedder` port |
| `Model2Vec.potion_base_8m` | `Model2Vec::potion_base_8m`; with no argument, over the model directory the environment names, reading no config file, as `Engine()` does |
| `Model2Vec.from_config` | `Model2Vec::from_config` over `Config::from_file` or `Config::resolve`, as `Engine.from_config` is `Engine::from_config` |
| any object with `embed` and `identity` | an `Embedder` implementor, wrapped by an adapter in the binding |
| `_core.cli_main(argv)` | `cli::run`, which is what the `matra` console script launches |

A stream crosses item by item: `analyze_all` yields a `Document` or a `DocumentError` dict per input, so a failure on one file is one item rather than an exception that ends the walk. And `clusters` takes any object with `embed` and `identity`: the `Embedder` port was always the extension point, and the binding now reaches it. `Model2Vec` carries both methods too, so it satisfies the same protocol a caller's own object does rather than sitting beside it as a second accepted type. In the extension module that arm needs no embedding adapter compiled in. The `matra` package itself still requires an extension built with `model2vec`: its `__init__` imports `Model2Vec` and raises `ImportError` naming the feature when it is absent, and every published wheel includes it.

An outcome takes a document you already analyzed, in both languages, so a text is parsed once however many outcomes are asked of it. Python's outcome functions take only a `Document`, never a dict: a dict could carry provenance no matra run produced. Parsing is the one thing no caller substitutes, in either language, because the size cap and the panic boundary live behind it. The Python binding lives in `src/python/` and uses only the public surface, which boundary rule 7 holds it to.

`Engine` is `unsendable` in Python, because the UDPipe model holds C-side state that is not thread safe. Multi-process Python is fine; sharing one engine across threads is not.
