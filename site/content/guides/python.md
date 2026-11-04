# Use matra from Python

You installed matra with `uv add 'matra>=0.2'` or `pip install 'matra>=0.2'`. That gave you two things: the library described on this page, and the `matra` command, which is the Rust CLI reached through the extension module rather than a Python program of its own. Everything the command does is in the [CLI guide](cli.md); the Python package adds no behavior to it.

## The no-setup path

No argument, no model on disk yet:

```python
from matra import Matra

v = Matra.english()
```

With no argument the model directory is resolved the way every matra surface resolves it: `MATRA_MODEL_DIR` if you set it, otherwise the `models` subdirectory of `$XDG_DATA_HOME/matra`, which defaults to `~/.local/share/matra`. A pre-existing `~/.matra/models` is used instead when the new location does not exist yet and that one is not empty, so a cache from 0.1.0 keeps working and keeps being written to. [Programming model](../explanation/programming-model.md#configuration) has the full resolution order.

## Load the model from a directory you choose

```python
from pathlib import Path

from matra import Matra

v = Matra.english(Path.home() / "models" / "matra")
```

The argument is a `str` or any `os.PathLike`, such as the `pathlib.Path` above. Pass a real path, not a shell shorthand: `~` is taken literally. The path goes straight to Rust's `create_dir_all`, which does not expand it, so `Matra.english("~/models")` creates a directory literally named `~` under your current working directory and caches a 16 MB model inside it. `Path.home()`, or `os.path.expanduser`, does the expansion in Python. Name a directory you own: provisioning trusts what it finds there, and a shared one such as `/tmp` is writable by anybody on the machine.

`Matra.english` downloads the English UDPipe model into that directory on first use, verifies it against a pinned SHA-256 hash, and loads from the cache on every call after that. The two ways that can fail raise different exceptions, and the split is the one the [errors reference](../reference/errors.md#provisioning-failures) describes: a download that never arrived, or a directory that could not be written, raises `matra.IoError`, an `OSError`; bytes that arrived and then failed the hash raise `matra.ModelInvalidError`, a `RuntimeError`. A bootstrap that wants to survive both catches `matra.MatraError`. A download that did not arrive carries `details["retryable"]`, which says whether calling again may succeed. Before 0.2.0 a failed download also raised `RuntimeError`, so an `except RuntimeError` written against an older version stops catching network failures. If you already have a model file, load it directly; a missing path raises `matra.ModelNotFoundError`, a `FileNotFoundError`, and a corrupt file raises `matra.ModelInvalidError`:

```python
v = Matra.from_path("/path/to/english-ewt-ud-2.5-191206.udpipe")
```

Construction is the expensive step. Build one `Matra` and reuse it across calls.

## Threads and processes

The underlying model holds C-side state that is not thread-safe, so the Rust binding marks the class `unsendable`. Touching one `Matra` instance from any thread other than the one that created it raises `pyo3_runtime.PanicException`. That exception derives from `BaseException`, not `Exception`, so a bare `except Exception` will not catch it and a thread pool will not report it the way you expect. Do not share an instance across threads.

Separate processes are fine, because each process gets its own model state. Build the engine once per worker with an initializer rather than once per task, so the model is loaded as many times as you have workers and not as many times as you have chunks:

```python
from concurrent.futures import ProcessPoolExecutor

from matra import Matra

_engine: Matra | None = None


def _init() -> None:
    global _engine
    _engine = Matra.english()


def analyze_chunk(text: str) -> dict:
    assert _engine is not None
    return _engine.analyze(text)


with ProcessPoolExecutor(initializer=_init) as pool:
    results = list(pool.map(analyze_chunk, chunks))
```

Concurrent `Matra.english` calls against the same model directory are safe. Each process downloads into its own temporary subdirectory and moves the file into place with a single rename.

## The six text methods

| Method | Takes | Returns |
|---|---|---|
| `analyze(text)` | plain text | `Document` |
| `analyze_markdown(text)` | markdown | `Document` |
| `tfidf_summarize(text, n)` | text, sentence count | `Summary` |
| `textrank_summarize(text, n)` | text, sentence count | `Summary` |
| `rake_keyphrases(text, max_phrases)` | text, phrase count | `Keyphrases` |
| `yake_keyphrases(text, max_phrases)` | text, phrase count | `Keyphrases` |

```python
result = v.analyze("The committee approved the proposal without debate.")
summary = v.tfidf_summarize(text, 3)
phrases = v.rake_keyphrases(text, 10)
```

Two more methods take something other than a string: `analyze_path` takes a path, and `semantic_clusters` takes text plus an embedding model. Both are below.

There is no standalone parse method on the Python surface. Each of the four summary and keyphrase methods runs the text it is given through the pipeline on its own, treating it as plain text. Calling `analyze` and then `tfidf_summarize` on the same string parses that string twice, and there is no parse-once-use-many path from Python today. If the second parse matters for your workload, do that work in Rust and expose the result through your own binding.

Each result is a dict with a `provenance`, the parsed text's own with the `method` that ran (`tfidf`, `textrank`, `rake`, `yake`) and the `parameters` it ran with (`{"n": 3}`), beside the result itself. The two summarizers return their `sentences` in document order, not score order: read `index` to see where a sentence sits among the text's sentences, `span` for where it lies in the text, and `score` to see how it ranked. The two keyphrase methods return their `phrases` in score order, highest first, each with `occurrences`, the span of every place the phrase was found.

Keyphrases come back as lowercased lemmas joined with spaces, not as the surface text. A document about "Dependency Parses" yields the phrase `dependency parse`; its occurrences' spans point at the surface text. Phrases with identical scores come back in the same order on every run, so a tie straddling the `max_phrases` cutoff is decided the same way each time.

The `score` on a `Keyphrase` is an ordering key and nothing more. RAKE's is a sum of per-word ratios that are each at least 1, so a phrase of k words scores at least k, which is a floor per length and not an order across lengths; YAKE's is the reciprocal of a product of per-word scores, so it is unbounded above, carries no unit, and does not track phrase length in either direction. The two scales are unrelated to each other, and neither is comparable across documents or with another tool's. Do not surface either number to a reader as a strength without saying which method produced it. The [CLI guide](cli.md#reading-the-scores) works through what the ranking does and does not tell you, and [Methodology](../reference/methodology.md) carries the formulas.

## Analyze a directory

`analyze_path` takes a file or a directory and returns one item per document, in path order:

```python
for item in v.analyze_path("docs/"):
    if "kind" in item:
        print(f"{item['path']}: {item['kind']}: {item['message']}")
    else:
        print(item["provenance"]["source"]["path"], item["metrics"]["vocabulary_ttr"])
```

One unreadable file costs one item, not the walk. A document that analyzed arrives as the same `Document` shape `analyze` returns, naming its file in `provenance.source.path`; one that did not arrives as a `DocumentError` (`path`, `kind`, `message`, `details`), holding the position the document would have had. Each file is read as its bytes and extension say: `.md` as markdown, anything else that is UTF-8 as plain text, and a PDF, a DOCX or bytes that are not text as an `unsupported_format` error. It carries a `kind` you can branch on, a `message` for a human to read, and `details`, every value in the message by name, so nothing has to be parsed out of the message. `matra.ERROR_KINDS` is the whole vocabulary, in the order the Rust enum declares it: `model_not_found`, `model_invalid`, `parse_failed`, `input_too_large`, `unsupported_format`, `invalid_input`, `io`. Testing `"kind" in item` is also what narrows the union for a type checker.

A path in an item is a `str` decoded the way Python decodes any filesystem path; on Unix, `os.fsencode` on it hands back the bytes the name came from and opens the file even when the name is not valid UTF-8 (elsewhere an undecodable name is decoded lossily). Every path argument on the surface goes the other way through the same encoding: `analyze_path`, `Matra.english`, `Matra.from_path`, `Model2Vec.from_dir` and `Model2Vec.potion_base_8m` take a `str` or a `pathlib.Path` alike.

The walk is not recursive, and symlinks and subdirectories are skipped rather than followed. Only a failure listing the path itself raises: a missing directory is `matra.IoNotFoundError`, a `FileNotFoundError`, because there is no per-document result for it to travel in.

## Bring your own embeddings

`semantic_clusters` takes any object with two methods. `Model2Vec` is one of them, and so is anything you write:

```python
class MyEmbedder:
    def embed(self, texts: list[str]) -> list[list[float]]:
        return my_service.encode(texts)     # one vector per text, in order

    def identity(self) -> str:
        return "my-service/v3"              # names the geometry, not the run


clusters = v.semantic_clusters(text, 0.85, MyEmbedder())
assert clusters["provenance"]["parameters"]["embedder"] == "my-service/v3"
```

`Embedder` in `matra.types` is the protocol those two methods satisfy, and it is what `semantic_clusters` is annotated with. Importing it is optional and buys you a type checker's opinion, not a runtime check.

The object is asked for its identity before the text is parsed, so an embedder that cannot name its geometry is refused for the price of one method call.

The contract is the one the Rust port carries: exactly one vector per input text, in input order, every vector the same length. Break it and the call raises `matra.InvalidInputError`, a `ValueError`, with the same message a Rust implementor gets, because the check is in the library and not in the binding. An exception raised inside your `embed` arrives as `matra.InvalidInputError` too, with your exception's own text inside the message.

`identity` is read once, when the object is handed over, and travels into the result's provenance as `parameters["embedder"]`. Two embedders that can disagree must not return the same string, or scores end up attributed to a geometry that did not produce them.

## Size limits

Every method that takes text rejects text over 8 MiB with `matra.InputTooLargeError`, a `ValueError`, whose `details` carry `what` (`"input"`), `limit` and `actual`. The gate lives in the pipeline stage every method routes through, not in each method, so there is no method that skips it.

The per-method caps apply on top: 2000 sentences for `tfidf_summarize`, `textrank_summarize` and `semantic_clusters` (the method and the module-level function alike), and 200000 tokens for `rake_keyphrases` and `yake_keyphrases`, each raising `matra.InputTooLargeError` when exceeded, with the method in `details["what"]`.

## The returned dicts and their shape

Every method returns a plain Python dict (or a list of them), not a custom object. `Document`, `Section`, `Paragraph`, `Sentence`, `Token`, `Summary`, `Keyphrases` and `Clusters` are `TypedDict` definitions in `matra.types` that describe the shape at both runtime and type-check time:

```python
from matra import Clusters, Document, Keyphrases, Paragraph, Section, Sentence, Summary, Token
```

`Document` nests down to the token level:

```python
for section in result["sections"]:            # heading, level, span, paragraphs
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

print(result["metrics"]["vocabulary_ttr"])        # float or None
print(result["metrics"]["nominalization_ratio"])  # float or None
print(result["skipped"])                          # [{"kind": "table", "span": {...}}, ...]
```

Every heading, paragraph, skipped construct, sentence and token has a `span`, `{"start": int, "end": int}`, into the analysed text. It counts UTF-8 bytes, not characters, so slice the encoded text: `text.encode()[span["start"]:span["end"]].decode()`. The text fields are normalized (a sentence's `text` is rebuilt from its tokens, a markdown paragraph's has its markup removed); the span is where the value came from.

A `Token` carries the ten CoNLL-U columns under these keys: `id`, `text`, `lemma`, `pos`, `xpos`, `features`, `head`, `dep`, `deps`, `misc`, plus its `span` and the derived `is_punct`. `features` is a dict from feature name to value, `{"Number": "Plur"}`. `head` is the `id` of the governing token in the same sentence, and `0` marks the root.

`Summary` (`provenance`, `sentences`, each a `SummarySentence` of `index`, `span`, `text` and `score`) is the shape for `tfidf_summarize` and `textrank_summarize`. `Keyphrases` (`provenance`, `phrases`, each a `Keyphrase` of `phrase`, `score` and `occurrences`) is the shape for `rake_keyphrases` and `yake_keyphrases`. `Clusters` (`provenance`, `clusters`, `edges`) is the shape for `Matra.semantic_clusters`; the module-level `semantic_clusters` reads no document, so it returns a `VectorClusters`, `parameters` in place of the provenance.

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

`analyze_markdown` reads CommonMark, and what it does not analyse it lists in `skipped` rather than dropping it: front matter, fenced and indented code, tables, HTML blocks, thematic breaks and link reference definitions, each with its `kind` and `span`. List items are paragraphs of `kind` `list_item`. `## References` is a heading like any other, and a `#hashtag` is text.

## The document-level aggregates

Every document-level aggregate arrives in the dict as a field of `metrics`. Read them rather than recomputing them:

```python
result = v.analyze("The report was filed. The committee met.")
print(sorted(result.keys()))
# ['metrics', 'provenance', 'sections']
print(result["metrics"])
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
    for section in result["sections"]
    for para in section["paragraphs"]
    for s in para["sentences"]
    if s["passive"]
]
```

`result["provenance"]` says where the document came from: `matra` (the version), `schema` (`"2.0"`, the schema version), `parser` (`name`, `model`, `sha256`, `license`), `format` (`markdown` or `plain_text`), and `source` (`path`, `bytes`, `sha256` of the input). Read `schema` before anything else when a result was stored by another process: ignore keys you do not recognize, and reject a major version you do not know.

## Exceptions

Every `domain::Error` variant crosses the FFI boundary as a class of its own, named for the variant with PEP 8's `Error` suffix; the name it shares with Rust and the JSON is its `kind`. Each derives from `matra.MatraError` and from a builtin class, the one matra raised before 0.3.0, so an `except ValueError` written then still catches what it caught:

| Situation | Python class | Also a |
|---|---|---|
| Model path does not exist | `ModelNotFoundError` | `FileNotFoundError` |
| Input exceeds the 8 MiB cap, or a per-extractor cap | `InputTooLargeError` | `ValueError` |
| Input format has no reader, or the bytes are not UTF-8 text | `UnsupportedFormatError` | `ValueError` |
| A caller broke a documented contract, such as an embedder returning the wrong number of vectors | `InvalidInputError` | `ValueError` |
| File I/O error, or a download that did not arrive | `IoError` | `OSError` |
| A path that does not exist | `IoNotFoundError` | `FileNotFoundError` |
| A path matra may not read or write | `IoPermissionError` | `PermissionError` |
| Model file corrupt or wrong format, or bytes that arrived and failed the pinned hash | `ModelInvalidError` | `RuntimeError` |
| NLP parsing failed, including a panic caught at the UDPipe boundary | `ParseFailedError` | `RuntimeError` |

`IoNotFoundError` and `IoPermissionError` derive from `IoError`, so `except matra.IoError` catches every I/O failure. `matra.IoError` is not the builtin `IOError`, which is another name for `OSError`. Every exception carries `kind`, the stable string from `matra.ERROR_KINDS`, and `details`, a dict of every value in its message: `what`, `limit` and `actual` on an `InputTooLarge`, `path` on a missing model, `url` and `retryable` on a download that did not arrive. The [errors reference](../reference/errors.md#kinds-and-details) lists the keys for every kind.

Loading the model and analyzing text fail in different ways, so catch them separately:

```python
import matra
from matra import Matra

try:
    v = Matra.from_path("models/english-ewt-ud-2.5-191206.udpipe")
except FileNotFoundError as e:
    raise SystemExit(f"model missing: {e.details['path']}")

try:
    result = v.analyze(text)
except matra.InputTooLargeError as e:
    print(f"{e.details['what']} gate: {e.details['actual']} > {e.details['limit']}")
except matra.MatraError as e:
    print(f"{e.kind}: {e}")
```

The message on each exception is the Rust error's own `Display` output, for a person to read. Branch on the class or on `kind`, and read values from `details`. An `InputTooLargeError` names its gate in `details["what"]`. The text methods raise `input`, `tfidf`, `textrank`, `rake`, `yake` and `clusters`; the constructors that resolve or download a model raise `config_file`, `udpipe_download` and `embedding_download`. `analyze_path` raises none of them: an oversized file there is an item whose `kind` is `input_too_large`, with `file_source` in its `details`.

The mapping is exhaustive and enforced at compile time on the Rust side. The match that picks the class has no wildcard arm over the variants, so adding a new `domain::Error` variant without also giving it a class fails to build. A new variant cannot fall through to another variant's class by accident.

The one failure that does not follow this table is the cross-thread access described above, which arrives as `pyo3_runtime.PanicException` and bypasses `except Exception` entirely.
