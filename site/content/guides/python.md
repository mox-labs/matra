# Use matra from Python

You installed matra with `uv add 'matra>=0.3'` or `pip install 'matra>=0.3'`. That gave you two things: the library described on this page, and the `matra` command, which is the Rust CLI reached through the extension module rather than a Python program of its own. Everything the command does is in the [CLI guide](cli.md); the Python package adds no behavior to it.

The Python surface spells what Rust spells. You ask an `Engine` to `analyze` a document, then ask for an outcome of the result: `summarize`, `keyphrases` or `clusters`.

```python
import matra

engine = matra.Engine()
doc = engine.analyze(file="README.md")
summary = matra.summarize(doc, n=3)
phrases = matra.keyphrases(doc, n=10, method="yake")
print(doc["metrics"]["total_words"])
```

## The no-setup path

No argument, no model on disk yet:

```python
import matra

engine = matra.Engine()
```

`Engine()` downloads the English UDPipe model on first use, verifies it against a pinned SHA-256 hash, and loads from the cache after that. Before the download it writes two lines to `sys.stderr`, the ones the command line prints for the same wait:

```text
matra: downloading english-ewt-ud-2.5-191206.udpipe (16.3 MB) into /home/you/.local/share/matra/models
matra: the model is licensed CC BY-NC-SA 4.0 (non-commercial), separately from matra: https://creativecommons.org/licenses/by-nc-sa/4.0/
```

Once the model is on disk nothing is written. `Engine.from_config()` and `Model2Vec.potion_base_8m()` announce a download the same way. Redirect `sys.stderr` if a notice must not reach your terminal. It finds the model directory from the environment alone: `MATRA_MODEL_DIR` if you set it, otherwise the `models` subdirectory of `$XDG_DATA_HOME/matra`, which defaults to `~/.local/share/matra`. A pre-existing `~/.matra/models` is used instead when the new location does not exist yet and that one is not empty, so a cache from 0.1.0 keeps working. [Programming model](../explanation/programming-model.md#configuration) has the full resolution order.

To put the model somewhere specific, set `MATRA_MODEL_DIR` before you build the engine. Name a directory you own: provisioning trusts what it finds there, and a shared one such as `/tmp` is writable by anybody on the machine. The value is used as a path unchanged, so `~` is taken literally.

`Engine.from_config()` is the same engine with a configuration file read and checked as well: the one the environment names, or the one you pass, `Engine.from_config("matra.toml")`, which must exist. The file does not move the model directory, so most callers want `Engine()`.

The two ways provisioning can fail raise different exceptions, and the split is the one the [errors reference](../reference/errors.md#provisioning-failures) describes. A download that never arrived, or a directory that could not be written, raises `matra.IoError`, an `OSError`. Bytes that arrived and then failed the hash raise `matra.ModelInvalidError`, a `RuntimeError`. A bootstrap that wants to survive both catches `matra.MatraError`. A download that did not arrive carries `details["retryable"]`, which says whether calling again may succeed.

The parser model is pinned. There is no way to load another model file from Python, as there is none from Rust.

Construction is the expensive step. Build one `Engine` and reuse it across calls.

## Threads and processes

The underlying model holds C-side state that is not thread-safe, so the Rust binding marks the class `unsendable`. Touching one `Engine` from any thread other than the one that created it raises `pyo3_runtime.PanicException`. That exception derives from `BaseException`, not `Exception`, so a bare `except Exception` will not catch it and a thread pool will not report it the way you expect. Do not share an engine across threads. A `Document` is not tied to a thread.

Separate processes are fine, because each process gets its own model state. Build the engine once per worker with an initializer rather than once per task, so the model is loaded as many times as you have workers and not as many times as you have chunks:

```python
from concurrent.futures import ProcessPoolExecutor

import matra

_engine: matra.Engine | None = None


def _init() -> None:
    global _engine
    _engine = matra.Engine()


def analyze_chunk(text: str) -> matra.DocumentDict:
    assert _engine is not None
    return _engine.analyze(text).to_dict()


if __name__ == "__main__":
    chunks = ["The report was filed.", "Nobody read it.", "The committee met again on Friday."]
    with ProcessPoolExecutor(initializer=_init) as pool:
        results = list(pool.map(analyze_chunk, chunks))
    print(len(results), [r["metrics"]["total_words"] for r in results])
```

The `if __name__ == "__main__":` guard is required. On macOS and Windows a worker process starts by importing your script afresh (the `spawn` start method), so a pool created at module level would start again inside every worker, and Python stops that with a `RuntimeError` about the bootstrapping phase. Under the guard, only the parent builds the pool. A `Document` does not pickle, so a worker returns `to_dict()`. Concurrent first runs against the same model directory are safe. Each process downloads into its own temporary subdirectory and moves the file into place with a single rename.

## Analyze one document

`Engine.analyze` takes exactly one input, by keyword, and returns a `Document`:

| Call | Reads |
|---|---|
| `analyze(text)` | a `str`, as plain text, or as `format` when you name one |
| `analyze(file=path)` | a file, its format detected from its name and bytes, or as `format` when you name one |
| `analyze(bytes=data)` | `bytes`, as `format`, or detected when you name none |
| `analyze(blocks=[...])` | headings and paragraphs you supply |

```python
doc = engine.analyze("The committee approved the proposal without debate.")
doc = engine.analyze(text, format="markdown")
doc = engine.analyze(file="notes.md")
```

A `str` is always text, never a path: `engine.analyze("notes.md")` analyzes the eight characters `notes.md`. Name a file with `file=`. Passing no input, or two, raises `TypeError` at the call. `format` is one of `markdown`, `plain_text` and `blocks`, and it applies to `text`, `bytes` and `file`. With no `format`, a file is read by what it is: `.md` as markdown, anything else that is UTF-8 as plain text, and a PDF, a DOCX or bytes that are not text as `matra.UnsupportedFormatError`. A JSON file of blocks is `engine.analyze(file="notes.json", format="blocks")`, since no extension implies it; a PDF or DOCX signature still outranks the format you name.

A file's path travels into the result as `doc["provenance"]["source"]["path"]`. On Unix it is decoded the way Python decodes any filesystem path, so `os.fsencode` on it hands back the bytes the name came from, even when the name is not valid UTF-8. Every path argument takes a `str` or a `pathlib.Path` alike.

### Text with its own section breaks

Text that comes from somewhere matra does not read (a PDF you extracted, a CMS, a transcript) can arrive as headings and paragraphs, with no markdown in between:

```python
doc = engine.analyze(blocks=[
    {"kind": "heading", "level": 1, "text": "Field notes"},
    {"kind": "paragraph", "text": "The river rose overnight."},
])
```

Each block is a `BlockDict`, a heading (`kind`, `level`, `text`) or a paragraph (`kind`, `text`). It is the same `blocks` input format the JSON schema in `spec/schema/v2/blocks.schema.json` publishes, read by the same reader, so a `kind` or a key matra does not know raises `matra.InvalidInputError` naming it. The analysed text is the blocks' texts joined by a blank line, and every span points into it.

## Analyze many documents

`Engine.analyze_all` takes a directory or an iterable of texts and returns an iterator, one item per document, in input order:

```python
for item in engine.analyze_all(dir="docs/"):
    if isinstance(item, matra.Document):
        print(item["provenance"]["source"]["path"], item["metrics"]["vocabulary_ttr"])
    else:
        print(f"{item['path']}: {item['kind']}: {item['message']}")
```

Nothing is read or parsed until you pull the next item, so a large directory costs nothing up front. One unreadable file costs one item, not the walk. A document that analyzed arrives as a `Document`. One that did not arrives as a `DocumentError` dict (`path`, `kind`, `message`, `details`), holding the position the document would have had. Test `isinstance(item, matra.Document)` to tell them apart; a type checker narrows the union on that test.

`analyze_all(texts=[...])` does the same for texts you hold, read as plain text or as `format`. A `str` alone raises `TypeError`, since it would otherwise be read one character at a time.

The walk is not recursive, and symlinks and subdirectories are skipped rather than followed. Only a failure listing the directory itself raises: a missing directory is `matra.IoNotFoundError`, a `FileNotFoundError`, because there is no per-document result for it to travel in.

## Read a document

A `Document` holds the parse matra made, and you read it as the JSON reads:

```python
doc["metrics"]["total_words"]
doc["sections"][0]["heading"]
doc["provenance"]["schema"]          # "2.0"
whole = doc.to_dict()                 # the whole document, as a dict
```

The keys are `provenance`, `sections`, `metrics` and `skipped`. `list(doc)`, `len(doc)` and `"metrics" in doc` work as they do on a dict. Each read builds a new value from the parse, so changing what a read returned changes nothing in the document. Hold a field in a variable when you read it in a loop.

A `Document` is read-only. There is nothing to assign to, and the outcome functions take a `Document` and never a dict, so a summary cannot carry provenance no matra run produced. A `Document` does not pickle; `to_dict()` is what you store or send.

## Summaries, keyphrases and clusters

The outcome functions run on a `Document` you already have, so a text is parsed once however many outcomes you ask for:

| Function | Returns |
|---|---|
| `matra.summarize(doc, n=3, *, method=None)` | `Summary` |
| `matra.keyphrases(doc, n=10, *, method=None)` | `Keyphrases` |
| `matra.clusters(doc, embedder, threshold=0.85)` | `Clusters` |

```python
summary = matra.summarize(doc, n=3)
summary = matra.summarize(doc, n=3, method="textrank")
phrases = matra.keyphrases(doc, n=10, method="yake")
```

`method` names how the ranking is done: `tfidf` or `textrank` for a summary, `rake` or `yake` for keyphrases. With none, matra's default runs, which is `tfidf` and `rake` today and the same on the command line. An unknown name raises `matra.InvalidInputError`. The defaults for `n` are the command line's.

Each result is a dict with a `provenance`, the document's own with the `method` that ran and the `parameters` it ran with (`{"n": 3}`), beside the result itself. A summary returns its `sentences` in document order, not score order: read `index` to see where a sentence sits among the document's sentences, `span` for where it lies in the text, and `score` to see how it ranked. Keyphrases return their `phrases` in score order, highest first, each with `occurrences`, the span of every place the phrase was found.

Keyphrases come back as lowercased lemmas joined with spaces, not as the surface text. A document about "Dependency Parses" yields the phrase `dependency parse`; its occurrences' spans point at the surface text. Phrases with identical scores come back in the same order on every run, so a tie straddling the `n` cutoff is decided the same way each time.

The `score` on a `Keyphrase` is an ordering key and nothing more. RAKE's is a sum of per-word ratios that are each at least 1, so a phrase of k words scores at least k, which is a floor per length and not an order across lengths; YAKE's is the reciprocal of a product of per-word scores, so it is unbounded above, carries no unit, and does not track phrase length in either direction. The two scales are unrelated to each other, and neither is comparable across documents or with another tool's. Do not surface either number to a reader as a strength without saying which method produced it. The [CLI guide](cli.md#reading-the-scores) works through what the ranking does and does not tell you, and [Methodology](../reference/methodology.md) carries the formulas.

## Bring your own embeddings

`matra.clusters` takes the embedder as an argument, always. `Model2Vec` is one, and so is any object with two methods:

```python
class MyEmbedder:
    def embed(self, texts: list[str]) -> list[list[float]]:
        return my_service.encode(texts)     # one vector per text, in order

    def identity(self) -> str:
        return "my-service/v3"              # names the geometry, not the run


found = matra.clusters(doc, MyEmbedder(), threshold=0.85)
assert found["provenance"]["parameters"]["embedder"] == "my-service/v3"
```

`Embedder` in `matra.types` is the protocol those two methods satisfy, and it is what `clusters` is annotated with. Importing it is optional and buys you a type checker's opinion, not a runtime check.

The object is asked for its identity before anything is embedded, so an embedder that cannot name its geometry is refused for the price of one method call.

The contract is the one the Rust port carries: exactly one vector per sentence, in order, every vector the same length. Break it and the call raises `matra.InvalidInputError`, a `ValueError`, with the same message a Rust implementor gets, because the check is in the library and not in the binding. An exception raised inside your `embed` arrives as `matra.InvalidInputError` too, with your exception's own text inside the message.

`identity` is read once, when the object is handed over, and travels into the result's provenance as `parameters["embedder"]`. Two embedders that can disagree must not return the same string, or scores end up attributed to a geometry that did not produce them.

## Size limits

Every input over 8 MiB is rejected with `matra.InputTooLargeError`, a `ValueError`, whose `details` carry `what`, `limit` and `actual`. Text, bytes and blocks are checked as `input`; a file is checked before it is read, as `file_source`. The gate lives in the one step every input passes through to reach the parser, so there is no input that skips it. In `analyze_all`, an oversized input is one item whose `kind` is `input_too_large`, and the stream goes on.

The outcome functions add caps of their own: 2000 sentences for a summary and for `clusters`, and 200000 tokens for keyphrases, each raising `matra.InputTooLargeError` with the method in `details["what"]`.

## The shapes of the data

A `Document` reads as a dict, and every outcome returns one. `DocumentDict`, `Section`, `Paragraph`, `Sentence`, `Token`, `Summary`, `Keyphrases` and `Clusters` are `TypedDict` definitions in `matra.types` that describe the shapes at both runtime and type-check time:

```python
from matra import Clusters, DocumentDict, Keyphrases, Paragraph, Section, Sentence, Summary, Token
```

A document nests down to the token level:

```python
for section in doc["sections"]:               # heading, level, span, paragraphs
    for para in section["paragraphs"]:        # span, text, kind, sentences,
                                              # metrics
        print(para["metrics"]["readability_grade"])  # float or None
        for sentence in para["sentences"]:    # span, text, tokens, word_count,
                                              # tree_depth, passive, negations,
                                              # modals, clausal_complements,
                                              # root_adverbials, hearst_pairs,
                                              # root_mood
            for token in sentence["tokens"]:
                print(token["lemma"], token["pos"], token["dep"])

print(doc["metrics"]["vocabulary_ttr"])        # float or None
print(doc["metrics"]["nominalization_ratio"])  # float or None
print(doc["skipped"])                          # [{"kind": "table", "span": {...}}, ...]
```

Every heading, paragraph, skipped construct, sentence and token has a `span`, `{"byte_start": int, "byte_end": int}`, into the analysed text. It counts UTF-8 bytes, not characters, as the field names say: `text[span["byte_start"]:span["byte_end"]]` on a `str` is right on ASCII and wrong, with no error, after the first character outside it. Slice with `matra.span_text(text, span)`, which encodes the text and decodes the stretch for you. The text fields are normalized (a sentence's `text` is rebuilt from its tokens, a markdown paragraph's has its markup removed); the span is where the value came from.

A `Token` carries the ten CoNLL-U columns under these keys: `id`, `text`, `lemma`, `pos`, `xpos`, `features`, `head`, `dep`, `deps`, `misc`, plus its `span` and the derived `is_punct`. `features` is a dict from feature name to value, `{"Number": "Plur"}`. `head` is the `id` of the governing token in the same sentence, and `0` marks the root.

`Summary` is `provenance` and `sentences`, each a `SummarySentence` of `index`, `span`, `text` and `score`. `Keyphrases` is `provenance` and `phrases`, each a `Keyphrase` of `phrase`, `score` and `occurrences`. `Clusters` is `provenance`, `clusters` and `edges`.

## Why a metric field is `None`

Each metric declines to run below its own threshold, and a `None` records that decision rather than a failed computation. The first three are in each paragraph's `metrics`, the rest in the document's.

| Field | Filled when |
|---|---|
| `readability_grade` | the paragraph has more than 10 words and its `kind` is not `quote` |
| `lexical_density` | the paragraph has at least 1 word and its `kind` is not `quote` |
| `compression_ratio` | the paragraph has more than 50 words, its `kind` is not `quote`, and the paragraph text is at most 256 KiB |
| `vocabulary_ttr` | the document has at least one non-punctuation token |
| `nominalization_ratio` | the same condition as `vocabulary_ttr` |
| `passive_ratio`, `mean_sentence_length` | the document has at least one sentence |
| `sentence_length_std` | the document has at least two sentences |

"Words" means the count of tokens with `is_punct` false across the paragraph's sentences. Quotes (a `kind` of `quote`) are never parsed at all: their `sentences` list is empty, their three paragraph metrics are `None`, and they contribute nothing to any document total.

Markdown is read as CommonMark, and what matra does not analyse it lists in `skipped` rather than dropping it: front matter, fenced and indented code, tables, HTML blocks, thematic breaks and link reference definitions, each with its `kind` and `span`. List items are paragraphs of `kind` `list_item`. `## References` is a heading like any other, and a `#hashtag` is text.

## The document-level aggregates

Every document-level aggregate is a field of `metrics`. Read them rather than recomputing them:

```python
doc = engine.analyze("The report was filed. The committee met.")
print(list(doc))
# ['provenance', 'sections', 'metrics', 'skipped']
print(doc["metrics"])
# {'total_words': 7, 'total_sentences': 2, 'paragraph_count': 1,
#  'mean_sentence_length': 3.5, 'sentence_length_std': 0.7071067811865476,
#  'vocabulary_ttr': 0.8571428571428571, 'nominalization_ratio': 0.0,
#  'passive_ratio': 0.5}
```

The counts are always integers. Each ratio is `None` when its condition was not met: `mean_sentence_length` and `passive_ratio` need a sentence, `sentence_length_std` (the sample standard deviation, as `statistics.stdev` computes it) needs two.

Methods do not cross the FFI boundary, only fields do, which is why these are fields: the metric suite materializes them so the aggregates cross as data instead of being re-derived once per language binding (RFC-0008). The same holds per sentence: `word_count`, `tree_depth` and `passive` are keys on each sentence dict.

```python
passive = [
    s["text"]
    for section in doc["sections"]
    for para in section["paragraphs"]
    for s in para["sentences"]
    if s["passive"]
]
```

`doc["provenance"]` says where the document came from: `matra` (the version), `schema` (`"2.0"`, the schema version), `parser` (`name`, `model`, `sha256`, `license`), `format` (`markdown`, `plain_text` or `blocks`), and `source` (`path`, `bytes`, `sha256` of the analysed text). Read `schema` before anything else when a result was stored by another process: ignore keys you do not recognize, and reject a major version you do not know.

## Exceptions

Every `domain::Error` variant crosses the FFI boundary as a class of its own, named for the variant with PEP 8's `Error` suffix; the name it shares with Rust and the JSON is its `kind`. Each derives from `matra.MatraError` and from a builtin class, the one matra raised before 0.3.0, so an `except ValueError` written then still catches what it caught:

| Situation | Python class | Also a |
|---|---|---|
| A model artifact is missing from the directory `Model2Vec.from_dir` was given | `ModelNotFoundError` | `FileNotFoundError` |
| Input exceeds the 8 MiB cap, or an outcome's cap | `InputTooLargeError` | `ValueError` |
| Input format has no reader, or the bytes are not UTF-8 text | `UnsupportedFormatError` | `ValueError` |
| A caller broke a documented contract, such as an unknown method or an embedder returning the wrong number of vectors | `InvalidInputError` | `ValueError` |
| File I/O error, or a download that did not arrive | `IoError` | `OSError` |
| A path that does not exist | `IoNotFoundError` | `FileNotFoundError` |
| A path matra may not read or write | `IoPermissionError` | `PermissionError` |
| Model file corrupt or wrong format, or bytes that arrived and failed the pinned hash | `ModelInvalidError` | `RuntimeError` |
| NLP parsing failed, including a panic caught at the UDPipe boundary | `ParseFailedError` | `RuntimeError` |

`IoNotFoundError` and `IoPermissionError` derive from `IoError`, so `except matra.IoError` catches every I/O failure. `matra.IoError` is not the builtin `IOError`, which is another name for `OSError`. Every exception carries `kind`, the stable string from `matra.ERROR_KINDS`, and `details`, a dict of every value in its message: `what`, `limit` and `actual` on an `InputTooLargeError`, `path` on a missing file, `url` and `retryable` on a download that did not arrive. The [errors reference](../reference/errors.md#kinds-and-details) lists the keys for every kind.

A call with the wrong arguments (no input, two inputs, `format` with `blocks`) raises the builtin `TypeError`, as any Python function does. It is a mistake in the call, not a failure of the analysis.

Loading the model and analyzing text fail in different ways, so catch them separately:

```python
import matra

try:
    engine = matra.Engine()
except matra.MatraError as e:
    raise SystemExit(f"cannot load the model: {e.kind}: {e}")

try:
    doc = engine.analyze(file=path)
except FileNotFoundError as e:
    print(f"no such file: {e.details['path']}")
except matra.InputTooLargeError as e:
    print(f"{e.details['what']} gate: {e.details['actual']} > {e.details['limit']}")
except matra.MatraError as e:
    print(f"{e.kind}: {e}")
```

The message on each exception is the Rust error's own `Display` output, for a person to read. Branch on the class or on `kind`, and read values from `details`. An `InputTooLargeError` names its gate in `details["what"]`: `input` and `file_source` for an input, `tfidf`, `textrank`, `rake`, `yake` and `clusters` for an outcome, and `config_file`, `udpipe_download` and `embedding_download` for the constructors that read a configuration or download a model.

The mapping is exhaustive and enforced at compile time on the Rust side. The match that picks the class has no wildcard arm over the variants, so adding a new `domain::Error` variant without also giving it a class fails to build. A new variant cannot fall through to another variant's class by accident.

The one failure that does not follow this table is the cross-thread access described above, which arrives as `pyo3_runtime.PanicException` and bypasses `except Exception` entirely.
