# Migrate from 0.2 to 0.3

0.3.0 changes Rust, Python, the command line, the JSON and the config file at once. Nothing keeps its old name. There are no deprecated aliases, no hidden flags, no fields written under both names and no setting that brings back the 0.2 output. A renamed or removed call fails at once in 0.3 rather than running with a different meaning: a compile error in Rust, an `AttributeError` or `TypeError` in Python, an unknown key in the config file, and a new `format_version` and schema major in the JSON. A few results do change for the same call, and [the last section](#results-that-change-with-no-rename) lists them.

This page lists every change a 0.2 caller meets, with the 0.2 form and the 0.3 form side by side. Work through the section for each surface you use.

## Update the version

Move the version requirement first. The old requirement keeps you on 0.2, because cargo reads a change in the first non-zero component as breaking.

```bash
cargo add matra@0.3                                   # was matra@0.2
cargo install matra --version '^0.3' --features cli   # was '^0.2'
pip install 'matra>=0.3'                              # was 'matra>=0.2'
uvx 'matra>=0.3' --skill                              # the skill for 0.3
```

## Every rename at a glance

| 0.2 | 0.3 | Where |
|---|---|---|
| `Engine::new(nlp, decomposers)`, `Engine::with_defaults()` | `Engine::new()`, `Engine::from_config(&Config::resolve()?)` | Rust |
| `Ingest::text`, `Ingest::path` | `Input::text`, `Input::file`, `Inputs::dir` | Rust |
| `Engine::analyze(Ingest)`, `Engine::analyze_one` | `Engine::analyze(Input)`, `Engine::analyze_all(inputs)` | Rust |
| `Engine::annotate`, `Engine::compose` | none: `analyze` is the call | Rust |
| `Ingested`, `RawDocument`, `CorpusEntry`, `Corpus`, `CorpusResult` | none: `Document`, its path in `provenance.source.path` | Rust |
| `extraction::tfidf_summarize`, `textrank_summarize` | `summarize`, `summarize_with(.., SummaryMethod::TextRank)` | Rust |
| `extraction::rake_keyphrases`, `yake_keyphrases` | `keyphrases`, `keyphrases_with(.., KeyphraseMethod::Yake)` | Rust |
| `embed_and_cluster`, `extraction::semantic_clusters` | `clusters` | Rust |
| `ScoredSentence`, `SemanticClusters`, `SemanticCluster`, `SemanticEdge` | `SummarySentence` in a `Summary`, `Clusters`, `Cluster`, `SimilarityEdge` | Rust, Python types |
| `domain::MAX_INPUT_BYTES` | `domain::limits::MAX_INPUT_BYTES` | Rust |
| `Error::InvalidInput(message)` | `Error::InvalidInput { reason, message }` | Rust |
| `Error::ParseFailed(message)` | `Error::ParseFailed { reason, cause }` | Rust |
| `Embedder::identity() -> &str` | `Embedder::identity() -> Identity` | Rust |
| `Config::summarize_n`, `summarize_algorithm`, `keyphrases_algorithm` | `Config::summary_n`, `summary_method`, `keyphrases_method` | Rust |
| `Config::udpipe_model`, `Config::semantic_threshold` | none; `Config::clusters_threshold` is new | Rust |
| `nlp`, `Udpipe`, `NlpProvider`, `metrics`, `hearst`, `extraction`, `decompose`, `source`, `standard_decomposers` | private to the crate | Rust |
| `Matra.english()`, `Matra.from_path(path)` | `matra.Engine()` | Python |
| `Matra.analyze(text)`, `analyze_markdown(text)` | `engine.analyze(text)`, `engine.analyze(text, format="markdown")` | Python |
| `Matra.analyze_path(path)` | `engine.analyze(file=path)`, `engine.analyze_all(dir=path)` | Python |
| `Matra.tfidf_summarize(text, n)`, `textrank_summarize` | `matra.summarize(doc, n, method=...)` | Python |
| `Matra.rake_keyphrases(text, max_phrases)`, `yake_keyphrases` | `matra.keyphrases(doc, n, method=...)` | Python |
| `Matra.semantic_clusters(text, threshold, model)` | `matra.clusters(doc, model, threshold)` | Python |
| `matra.semantic_clusters(vectors, threshold, model_hash)` | none: a cosine over the vectors you hold | Python |
| an embedder's `identity() -> str`, `Model2Vec.identity()` | `identity()` returning an `Identity` dict, `{name, model, sha256, license}` | Python |
| `Model2Vec.potion_base_8m()` reading the config file | `Model2Vec.from_config()` | Python |
| `Document`, `CorpusEntry`, `CorpusItem` (types) | `matra.Document` (a class), `DocumentDict`, and `Document` or `DocumentError` items | Python |
| `paragraph["readability_grade"]` and its siblings | `paragraph["metrics"]["readability_grade"]` | JSON, Python |
| `doc["vocabulary_ttr"]` and its siblings | `doc["metrics"]["vocabulary_ttr"]` | JSON, Python |
| `Paragraph.in_blockquote` | `Paragraph.kind` (`"quote"`) | all |
| `Sentence.reportings`, `Reporting`, `reportings_in` | `Sentence.clausal_complements`, `ClausalComplement`, `clausal_complements_in` | all |
| `Sentence.bare_assertion` | `Sentence.root_mood` | all |
| `Token.feats` (a string) | `Token.features` (a map) | all |
| `Sentence.tree_depth()` returning `usize::MAX` on a cycle | `Sentence.tree_depth`, `None` on a cycle | all |
| `SemanticClusters.model_hash` | `Clusters.provenance.parameters.embedder.sha256` | all |
| `InputTooLarge` `what`: `file_source`, `config_file`, `udpipe_download`, `embedding_download` | `file_read`, `config_file_read`, `parser_model_download`, `embedding_model_download` | all |
| `invalid_input` `details.reason`, the message text | `details.reason`, the name of the check that failed | all |
| `model_invalid`, `parse_failed` and `io` `details.reason`, the error's text | `details.cause` | all |
| `Format` serialized as `"Markdown"`, `"PlainText"` | `"markdown"`, `"plain_text"`, `"blocks"` | Rust serde, JSON |
| `[summarize]`, `algorithm`, `[semantic] threshold`, `[models] udpipe` | `[summary]`, `method`, `[clusters] threshold`, none | config file |
| `--json` envelope `format_version: 1` | `format_version: 2`, with an `error` key | command line |

## Rust

### Build the engine

`Engine::new` takes no arguments and builds the pinned parser itself. It reads no configuration file. `Engine::with_defaults` is gone, because it read one under a name that did not say so.

```rust
// 0.2
let engine = Engine::with_defaults()?;
let engine = Engine::new(Box::new(Udpipe::english(dir)?), standard_decomposers());
```

```rust
// 0.3
let engine = Engine::new()?;
let engine = Engine::from_config(&Config::resolve()?)?;                     // reads the config file
let engine = Engine::from_config(&Config::resolve()?.with_model_dir(dir))?; // a model directory you name
```

The parser and reader traits are private to the crate, so there is no provider to construct and no reader table to pass. No surface loads a UDPipe model file of your choosing: the model is a pin.

### Analyze one document

`Input` is one document. `Engine::analyze` returns its `Document` or the error. There is no stream to unwrap and no `CorpusEntry` around the document.

```rust
// 0.2
let analysis = engine
    .analyze(Ingest::path("essay.md")?)
    .next()
    .unwrap()
    .map_err(|e| e.error)?
    .analysis;
let doc = engine
    .analyze_one(RawDocument::new(text, None, Format::Markdown))
    .map_err(|e| e.error)?
    .analysis;
```

```rust
// 0.3
let doc = engine.analyze(Input::file("essay.md"))?;
let doc = engine.analyze(Input::text(text, Format::Markdown))?;
```

`Input` has five constructors: `text`, `bytes`, `file`, `file_as` (a file read as the format you name) and `blocks` (headings and paragraphs you supply for a format matra has no reader for). `Input::file` on a directory fails with `InvalidInput`, naming `Inputs::dir`.

### Analyze a directory

`Ingest::path` on a directory is `Inputs::dir`, and `analyze_all` streams one result per file. A failed file is an `Err` item carrying its path. The path of a document that analyzed is `doc.provenance.source.path`.

```rust
// 0.2
for item in engine.analyze(Ingest::path("docs/")?) {
    match item {
        Ok(entry) => println!("{:?}: {}", entry.path, entry.analysis.total_words()),
        Err(e) => eprintln!("{:?}: {}", e.path, e.error),
    }
}
```

```rust
// 0.3
for item in engine.analyze_all(Inputs::dir("docs/")?) {
    match item {
        Ok(doc) => println!("{:?}: {}", doc.provenance.source.path, doc.metrics.total_words),
        Err(e) => eprintln!("{e}"),
    }
}
```

`Corpus` and `CorpusResult` are gone, with `Corpus::total_words`, `passive_ratio` and `mean_readability`. Collect the stream yourself. A count such as `total_words` sums across documents. A ratio such as `vocabulary_ttr` does not, because it falls as a document grows.

### Read the measures

The aggregate methods are fields now, so they reach Python and the JSON too. The paragraph measures moved into `Paragraph.metrics`, and the document measures into `Document.metrics`.

```rust
// 0.2
doc.total_words();
doc.passive_ratio();            // 0.0 on a document with no sentences
doc.vocabulary_ttr;
paragraph.readability_grade;
sentence.word_count();
sentence.is_passive();
sentence.tree_depth();          // usize::MAX on a cycle
```

```rust
// 0.3
doc.metrics.total_words;
doc.metrics.passive_ratio;      // None on a document with no sentences
doc.metrics.vocabulary_ttr;
paragraph.metrics.readability_grade;
sentence.word_count;
sentence.passive;
sentence.tree_depth;            // None on a cycle
```

`DocumentMetrics` also carries `total_sentences`, `paragraph_count`, `mean_sentence_length` and `sentence_length_std`. In 0.2 these were methods, so they never reached Python or the JSON.

### Read the record facts

The facts on a sentence are named for what the detector finds. `ClausalComplement` is a verb governing a clausal complement. Which of those verbs report speech is your lexicon, as before.

```rust
// 0.2
sentence.reportings;
sentence.reportings_in(&["say", "claim"]);
sentence.bare_assertion;
token.feats;                    // "Mood=Ind|Tense=Past"
```

```rust
// 0.3
sentence.clausal_complements;
sentence.clausal_complements_in(&["say", "claim"]);
sentence.root_mood;             // Option<RootMood { root_id, mood }>
token.features;                 // BTreeMap: {"Mood": "Ind", "Tense": "Past"}
```

The 0.2 `bare_assertion` flag is `root_mood.mood == "Ind"` with no modal whose `head_id` is `root_mood.root_id`. `Token::feat("Mood")` reads one feature, as before. `Paragraph.in_blockquote` is `Paragraph.kind`: `in_blockquote: true` is `ParagraphKind::Quote`, and a list item is now `ParagraphKind::ListItem`.

### Summaries, keyphrases and clusters

The outcomes are free functions at the crate root. They take the analysed `&Document`, not a slice of its sentences, and need no engine, so they run in a build without UDPipe. The method is optional and never in the name. Each returns a record whose `provenance` names the method that ran and its parameters.

```rust
// 0.2
let sentences: Vec<Sentence> = doc.sentences().cloned().collect();
let top = extraction::tfidf_summarize(&sentences, 3)?;          // Vec<ScoredSentence>
let ranked = extraction::textrank_summarize(&sentences, 3)?;
let phrases = extraction::yake_keyphrases(&sentences, 10)?;     // Vec<Keyphrase>
let groups = embed_and_cluster(&doc, &model, 0.85)?;           // SemanticClusters
```

```rust
// 0.3
let top = matra::summarize(&doc, 3)?;                                    // Summary
let ranked = matra::summarize_with(&doc, 3, SummaryMethod::TextRank)?;
let phrases = matra::keyphrases_with(&doc, 10, KeyphraseMethod::Yake)?; // Keyphrases
let groups = matra::clusters(&doc, &model, 0.85)?;                      // Clusters
```

What moved inside the results:

- `ScoredSentence { text, score, position }` is `SummarySentence { index, span, text, score }`, in `Summary.sentences`. `position` is `index`.
- `Keyphrase` gains `occurrences`, every span the phrase was found in, and sits in `Keyphrases.phrases`.
- `SemanticClusters { model_hash, threshold, clusters }` is `Clusters { provenance, clusters, edges }`. The embedder's identity is `provenance.parameters.embedder`, an `Identity` like the parser's (the old hash is its `sha256`), and the threshold `provenance.parameters.threshold`. The edges moved out of each cluster into one list, ordered by `a` and then `b`. Each `Cluster` holds only `members`.
- The threshold and every score are `f64`, not `f32`.
- `Model2Vec::potion_base_8m(dir)` takes the models directory, the one `MATRA_MODEL_DIR` names, and keeps the three files in `dir/potion-base-8M/`. In 0.2 `dir` was the model's own directory. Pass the parent: `potion_base_8m(cfg.model_dir())` where 0.2 took `potion_base_8m(cfg.model_dir().join("potion-base-8M"))`. Python's `Model2Vec.potion_base_8m(dir)` changed the same way.

### Errors

`Error` keeps its variants and `kind()`. Three shapes changed.

```rust
// 0.2
Error::UnsupportedFormat(format)
Error::InvalidInput(message)
Error::ParseFailed(message)
domain::MAX_INPUT_BYTES
```

```rust
// 0.3
Error::UnsupportedFormat { format, feature }   // both Option
Error::InvalidInput { reason, message }       // reason: "threshold_not_finite", ...
Error::ParseFailed { reason, cause }          // reason: None, or "head_out_of_range", ...
domain::limits::MAX_INPUT_BYTES
```

`InvalidInput`'s `reason` names the check that failed, a stable name the [errors reference](../reference/errors.md#invalidinput) lists, and `details["reason"]` carries it, where 0.2 put the message text there. On `model_invalid`, `parse_failed` and `io` the error's own text moves from `details["reason"]` to `details["cause"]`, so `reason` names a check wherever it appears. Four `InputTooLarge` labels name what you read rather than matra's modules: `file_source` is `file_read`, `config_file` is `config_file_read`, `udpipe_download` is `parser_model_download` and `embedding_download` is `embedding_model_download`. A match on the old strings no longer fires.

`Error::details()` is new. It returns every value in the message by name, so no caller parses a message. `Error` and `DocumentError` serialize as `{kind, message, details}`, and `DocumentError` adds `path`. A failed read now names its path: `io error: cannot read essay.md: No such file or directory (os error 2)`. A file whose bytes are not UTF-8 is `UnsupportedFormat`, not `Io`. Every cap is in `domain::limits`, with the same values as 0.2. [Errors](../reference/errors.md) lists every kind and its details.

### What left the public surface

These modules and items are private to the crate in 0.3: `nlp` with `NlpProvider` and `Udpipe`, `metrics` with `Metric`, `default_suite`, `run_suite` and `readability::flesch_kincaid_grade`, `hearst`, `extraction`, `decompose` with `MarkdownDecomposer`, `PlainTextDecomposer` and `Decomposers`, `source` with `FileSource` and `DirectorySource`, and `standard_decomposers`. The measures stay public as fields of the `Document`. For another measure, write a function over `&Document`. `Embedder` stays public: it is the one port you implement. Its `identity` returns a `domain::Identity`, the shape the parser reports, where it returned `&str`: `Identity::new("my-service", Some("v3".into()), None, None)` in place of `"my-service/v3"`. Return `ModelInvalid`, `Io` or `InputTooLarge` for your own failures, as the trait documents.

### Configuration

`Config` reads the renamed sections, so its accessors follow them.

```rust
// 0.2
cfg.summarize_n();
cfg.summarize_algorithm();
cfg.keyphrases_algorithm();
cfg.semantic_threshold();   // f32
cfg.udpipe_model();
```

```rust
// 0.3
cfg.summary_n();
cfg.summary_method();
cfg.keyphrases_method();
cfg.clusters_threshold();   // f64
```

`Config::from_file(path)` is new: the configuration with a file you name, where a missing file is an error.

## Python

### Build the engine

`Matra` is `Engine`. A model directory is named by `MATRA_MODEL_DIR`, or by a config file you pass to `Engine.from_config`. `Matra.from_path` is gone: no surface loads a model file of your choosing.

```python
# 0.2
from matra import Matra

m = Matra.english()
m = Matra.english("/opt/matra/models")
```

```python
# 0.3
import matra

engine = matra.Engine()                                  # MATRA_MODEL_DIR, else the default
engine = matra.Engine.from_config("matra.toml")          # a config file you name
engine = matra.Engine.from_config()                      # the config file the environment names
```

### Analyze

`engine.analyze` takes exactly one input. A `str` alone is text, never a path, and reads as plain text unless you name a `format`. It returns a `matra.Document`, not a dict.

```python
# 0.2
doc = m.analyze(text)
doc = m.analyze_markdown(text)
items = m.analyze_path("essay.md")              # a list of one
items = m.analyze_path("docs/")                 # a list, built in full
```

```python
# 0.3
doc = engine.analyze(text)
doc = engine.analyze(text, format="markdown")
doc = engine.analyze(file="essay.md")           # read as its extension and bytes say
for item in engine.analyze_all(dir="docs/"):    # lazy, one item per file
    ...
```

`analyze_path` returned `CorpusEntry` dicts (`{"path", "analysis"}`) and `DocumentError` dicts (`{"path", "error": {"kind", "message"}}`). `analyze_all` yields a `matra.Document` or a `DocumentError` dict, `{"path", "kind", "message", "details"}`. Tell them apart with `isinstance(item, matra.Document)`. The path of a document is `item["provenance"]["source"]["path"]`.

```python
# 0.2
for item in m.analyze_path("docs/"):
    if "error" in item:
        print(item["path"], item["error"]["kind"])
    else:
        print(item["path"], item["analysis"]["passive_ratio"])
```

```python
# 0.3
for item in engine.analyze_all(dir="docs/"):
    if isinstance(item, matra.Document):
        print(item["provenance"]["source"]["path"], item["metrics"]["passive_ratio"])
    else:
        print(item["path"], item["kind"])
```

### Read a document

A `matra.Document` reads like the dict it replaced: `doc["metrics"]["total_words"]`, `list(doc)` and `len(doc)` work. It is read-only. Call `doc.to_dict()` where your code needs a dict, changes the result, or pickles it. The keys follow the schema 2.0 record, so the measures moved:

```python
# 0.2
doc["passive_ratio"]
doc["sections"][0]["paragraphs"][0]["readability_grade"]
doc["sections"][0]["paragraphs"][0]["in_blockquote"]
```

```python
# 0.3
doc["metrics"]["passive_ratio"]
doc["sections"][0]["paragraphs"][0]["metrics"]["readability_grade"]
doc["sections"][0]["paragraphs"][0]["kind"] == "quote"
```

### Summaries, keyphrases and clusters

The four algorithm-named methods and `semantic_clusters` are module functions over a `Document`. A text is parsed once however many outcomes you ask of it, and in the format you analysed it in: 0.2's methods read every text as plain text, so a README's headings could land in its summary. Analyse a markdown file with `file=`, or a markdown string with `format="markdown"`, and its headings stay headings. `max_phrases` is `n`. Each returns a dict whose `provenance` names the method and its parameters.

```python
# 0.2
top = m.tfidf_summarize(text, 3)                   # [{"text", "score", "position"}]
ranked = m.textrank_summarize(text, 3)
phrases = m.yake_keyphrases(text, max_phrases=10)  # [{"phrase", "score"}]
groups = m.semantic_clusters(text, 0.85, model)
```

```python
# 0.3
doc = engine.analyze(text)
top = matra.summarize(doc, 3)                       # {"provenance", "sentences": [{"index", "span", "text", "score"}]}
ranked = matra.summarize(doc, 3, method="textrank")
phrases = matra.keyphrases(doc, 10, method="yake")  # {"provenance", "phrases": [{"phrase", "score", "occurrences"}]}
groups = matra.clusters(doc, model, 0.85)
```

The vectors-in `matra.semantic_clusters(vectors, threshold, model_hash)` is gone, because Rust never had it. To compare vectors you already hold, compute the cosine yourself, as [the semantic clusters guide](semantic-clusters.md#comparing-whole-documents) shows.

### Exceptions

Every exception is a `matra.MatraError` subclass with `kind` and `details`. Each also derives from the builtin class 0.2 raised, so `except ValueError` and `except OSError` catch what they caught.

```python
# 0.2
try:
    m.analyze(text)
except ValueError as e:          # which ValueError: read the message
    ...
```

```python
# 0.3
try:
    engine.analyze(text)
except matra.InputTooLargeError as e:   # also a ValueError
    print(e.kind, e.details["limit"], e.details["actual"])
```

A missing file was a bare `OSError` in 0.2. It is now `matra.IoNotFoundError`, which is a `FileNotFoundError` and an `OSError`, so `except FileNotFoundError` now catches it. A permission failure is `matra.IoPermissionError`, a `PermissionError`.

### Types

`matra.types` follows the record. `ScoredSentence` is `SummarySentence` inside `Summary`; `SemanticClusters`, `SemanticCluster` and `SemanticEdge` are `Clusters`, `Cluster` and `SimilarityEdge`; `Reporting` is `ClausalComplement`; `CorpusEntry` and `CorpusItem` are gone. The `TypedDict` that was named `Document` is `DocumentDict`, the shape `to_dict()` returns, and `matra.Document` is the class.

## Command line

### The config file

A config file from 0.2 does not load. Every command that reads one fails with `invalid_input`, naming the file and each key's replacement. Edit the file, or write a fresh one with `matra config init --force`.

```toml
# 0.2
[models]
udpipe = "english-ewt-ud-2.5-191206"
embedding = "potion-base-8M"

[semantic]
threshold = 0.85

[summarize]
n = 3
algorithm = "tfidf"

[keyphrases]
n = 10
algorithm = "rake"
```

```toml
# 0.3
[models]
embedding = "potion-base-8M"

[summary]
n = 3
method = "tfidf"

[keyphrases]
n = 10
method = "rake"

[clusters]
threshold = 0.85
```

`models.udpipe` goes without a replacement, because the parsing model is pinned in the source beside its digest.

### The JSON envelope

`format_version` is `2`, and the envelope gains a fifth key, `error`. All five are always present, and exactly one of `result` and `error` is null. A consumer pinned to `format_version: 1` should fail, and then move.

```json
{ "format_version": 1, "command": "analyze", "input": "essay.md", "result": {} }
```

```json
{ "format_version": 2, "command": "analyze", "input": "essay.md", "result": {}, "error": null }
```

`result` changed too:

- `analyze`: a schema 2.0 `Document`. See [The JSON record](#the-json-record) below.
- `summarize`: a `Summary` record, `{provenance, sentences}`, not a bare list of `{text, score, position}`.
- `keyphrases`: a `Keyphrases` record, `{provenance, phrases}`, not a bare list of `{phrase, score}`.

```bash
# 0.2
matra summarize essay.md --json | jq '.result[0].text'
matra keyphrases essay.md --json | jq '.result[].phrase'
```

```bash
# 0.3
matra summarize essay.md --json | jq '.result.sentences[0].text'
matra keyphrases essay.md --json | jq '.result.phrases[].phrase'
```

### Failures under `--json`

In 0.2 a failure under `--json` wrote nothing to stdout. In 0.3 it writes the envelope there, with `result` null and `error` holding the `kind`, `message` and `details`. It still writes `matra: <message>` to stderr and still exits 2, so a script that reads only the exit code is unchanged. A usage error under `--json` is an envelope too.

```bash
# 0.3
matra analyze missing.md --json | jq -r '.error.kind'      # io
```

The message for a missing file changed with it: `io error: cannot read missing.md: ...` where 0.2 printed `no such file: missing.md`. Branch on `error.kind` and read `error.details`, never the message.

### Standard input

Standard input is read as bytes, and its format is detected as a file's is. `--stdin-filename` still names it, and its extension still picks the format. Bytes that are not UTF-8 fail as `unsupported_format`, as a file's do, where 0.2 failed with a read error that had no kind.

### What is new

These do not break a 0.2 command, and are listed so you know they exist: `matra analyze DIR`, one result per file; `matra clusters`; `--format` on every document command; `--depth` on `analyze --json`; and `matra --schema`. The [CLI guide](cli.md) has each.

## The JSON record

The same record is the Rust `Document`, the Python `matra.Document` and `analyze --json`'s `result`. It now names its own schema version in `provenance.schema`, `"2.0"`. The schema is published: `matra --schema document` prints it.

| 0.2 | 0.3 |
|---|---|
| no provenance | `provenance`: the matra version, the schema, the parser and its model's SHA-256, the format, and the input's path, size and SHA-256 |
| `vocabulary_ttr`, `nominalization_ratio`, `passive_ratio` | `metrics.vocabulary_ttr`, `metrics.nominalization_ratio`, `metrics.passive_ratio` |
| absent | `metrics.total_words`, `total_sentences`, `paragraph_count`, `mean_sentence_length`, `sentence_length_std` |
| absent | `skipped`: each construct the reader saw and did not analyse, with its `kind` and `span` |
| `sections[].heading`, `level`, `paragraphs` | the same, and `span` (the heading's, null with no heading) |
| `paragraph.readability_grade`, `lexical_density`, `compression_ratio` | `paragraph.metrics.readability_grade`, and so on |
| `paragraph.in_blockquote` | `paragraph.kind`: `paragraph`, `quote` or `list_item` |
| absent | `span` on every paragraph, sentence and token, in UTF-8 bytes of the analysed text |
| absent | `sentence.word_count`, `tree_depth` (null on a cycle), `passive` |
| `sentence.reportings` | `sentence.clausal_complements` |
| `sentence.bare_assertion` | `sentence.root_mood`: `{root_id, mood}`, or null |
| `token.feats`: one string of pipe-separated features | `token.features`: `{"Mood": "Ind", "Tense": "Past"}` |

Read the record as the schema asks: ignore a field you do not recognize, treat every string set you receive as open, and reject a major you do not know.

## Results that change with no rename

A few results differ for the same input, and nothing about the call tells you so.

- **Markdown is read by a CommonMark parser.** 0.2 dropped tables without a trace, ended the document at a `## References` line, read `#hashtag` as a heading, and passed inline markup such as `**` and link syntax to the parser as words. In 0.3 tables, code, front matter and HTML are `skipped` entries, a `## References` heading is a heading like any other, a heading needs a space after its `#`, list items are paragraphs of kind `list_item`, and inline markup is removed from the text the parser reads. The counts and measures of a markdown document can change.
- **Plain text** splits paragraphs at a line of only whitespace, so `\r\n` endings and a line of spaces separate paragraphs as an empty line does.
- **Keyphrases with equal scores** come in the same order on every run, by first occurrence, where 0.2 followed a hash map's order.
- **TF-IDF scores** are the same to the last bit on every run.
- **A binary file** is `unsupported_format`, naming the format when it is a PDF or DOCX. In 0.2 it was an I/O error.
- **The size cap's message** reads `input too large`, where 0.2 read `input input too large`.
- **`Model2Vec.potion_base_8m(dir)`** looks in `dir/potion-base-8M/`, where 0.2 looked in `dir` itself. A 0.2 call that passed the model's own directory still runs, and downloads a second copy one level down. Pass the models directory instead.
- **`Model2Vec.potion_base_8m()` with no argument reads no config file.** It finds the model directory from the environment, as `Engine()` does, so a `model_dir` or `models.embedding` set only in the config file no longer moves it. `Model2Vec.from_config()` reads the file.
- **Python's constructors announce a download.** `Engine()`, `Engine.from_config()` and `Model2Vec.potion_base_8m()` write two lines to `sys.stderr` before they download a model, the ones the command line prints. Once the model is on disk they write nothing.
- **An empty result says so.** `matra summarize`, `keyphrases` and `clusters` still exit 1 when they find nothing, and now also write one line to stderr saying so, unless `--json` or `--quiet` is given.
