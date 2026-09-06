# Use matra from Python

You installed matra with `uv add matra`. That gave you two things: the library described on this page, and the `matra` command, which is the Rust CLI reached through the extension module rather than a Python program of its own. Everything the command does is in the [CLI guide](cli.md); the Python package adds no behavior to it.

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

model_dir = str(Path.home() / ".matra" / "models")
v = Matra.english(model_dir)
```

Pass a real path, not a shell shorthand. The string goes straight to Rust's `create_dir_all`, which does not expand `~`. `Matra.english("~/.matra/models")` creates a directory literally named `~` under your current working directory and caches a 16 MB model inside it.

`Matra.english` downloads the English UDPipe model into that directory on first use, verifies it against a pinned SHA-256 hash, and loads from the cache on every call after that. A download or verification failure raises `RuntimeError`. If you already have a model file, load it directly; a missing path raises `FileNotFoundError`, a corrupt file raises `RuntimeError`:

```python
v = Matra.from_path("/path/to/english-ewt-ud-2.5-191206.udpipe")
```

Construction is the expensive step. Build one `Matra` and reuse it across calls.

## Threads and processes

The underlying model holds C-side state that is not thread-safe, so the Rust binding marks the class `unsendable`. Touching one `Matra` instance from any thread other than the one that created it raises `pyo3_runtime.PanicException`. That exception derives from `BaseException`, not `Exception`, so a bare `except Exception` will not catch it and a thread pool will not report it the way you expect. Do not share an instance across threads.

Separate processes are fine, because each process gets its own model state. Build the engine once per worker with an initializer rather than once per task, so the model is loaded as many times as you have workers and not as many times as you have chunks:

```python
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

from matra import Matra

MODEL_DIR = str(Path.home() / ".matra" / "models")
_engine: Matra | None = None


def _init() -> None:
    global _engine
    _engine = Matra.english(MODEL_DIR)


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
| `tfidf_summarize(text, n)` | text, sentence count | `list[ScoredSentence]` |
| `textrank_summarize(text, n)` | text, sentence count | `list[ScoredSentence]` |
| `rake_keyphrases(text, max_phrases)` | text, phrase count | `list[Keyphrase]` |
| `yake_keyphrases(text, max_phrases)` | text, phrase count | `list[Keyphrase]` |

```python
result = v.analyze("The committee approved the proposal without debate.")
summary = v.tfidf_summarize(text, 3)
phrases = v.rake_keyphrases(text, 10)
```

Two more methods take something other than a string: `analyze_path` takes a path, and `semantic_clusters` takes text plus an embedding model. Both are below.

There is no standalone parse method on the Python surface. Each of the four extraction methods runs the text it is given through the pipeline on its own, treating it as plain text. Calling `analyze` and then `tfidf_summarize` on the same string parses that string twice, and there is no parse-once-use-many path from Python today. If the second parse matters for your workload, do that work in Rust and expose the result through your own binding.

The two summarizers return their selection in document order, not score order. Read `position` to see where a sentence sits in the source and `score` to see how it ranked. The two keyphrase methods return score order, highest first, because `Keyphrase` has no position to sort back into.

Keyphrases come back as lowercased lemmas joined with spaces, not as the surface text. A document about "Dependency Parses" yields the phrase `dependency parse`. Phrases with identical scores can also change relative order between runs, and a tie straddling the `max_phrases` cutoff can change which phrase is included, because the internal candidate map has no stable iteration order. Sort or filter on your side if you need a reproducible list.

## Analyze a directory

`analyze_path` takes a file or a directory and returns one item per document, in path order:

```python
for item in v.analyze_path("docs/"):
    if "error" in item:
        print(f"{item['path']}: {item['error']['kind']}: {item['error']['message']}")
    else:
        print(item["path"], item["analysis"]["vocabulary_ttr"])
```

One unreadable file costs one item, not the walk. A document that analyzed arrives as a `CorpusEntry` (`path`, `analysis`, where `analysis` is the same shape `analyze` returns); one that did not arrives as a `DocumentError` (`path`, `error`), holding the position the document would have had. The `error` object carries a `kind` you can branch on and a `message` for a human to read. `matra.ERROR_KINDS` is the whole vocabulary, in the order the Rust enum declares it: `model_not_found`, `model_invalid`, `parse_failed`, `input_too_large`, `unsupported_format`, `invalid_input`, `io`. Testing `"error" in item` is also what narrows the union for a type checker.

`path` is a `str` decoded the way Python decodes any filesystem path; on Unix, `os.fsencode` on it hands back the bytes the name came from and opens the file even when the name is not valid UTF-8 (elsewhere an undecodable name is decoded lossily). Every path argument on the surface goes the other way through the same encoding: `analyze_path`, `Matra.english`, `Matra.from_path`, `Model2Vec.from_dir` and `Model2Vec.potion_base_8m` take a `str` or a `pathlib.Path` alike.

The walk is not recursive, and symlinks and subdirectories are skipped rather than followed. Only a failure listing the path itself raises: a missing directory is `OSError`, because there is no per-document result for it to travel in.

## Bring your own embeddings

`semantic_clusters` takes any object with two methods. `Model2Vec` is one of them, and so is anything you write:

```python
class MyEmbedder:
    def embed(self, texts: list[str]) -> list[list[float]]:
        return my_service.encode(texts)     # one vector per text, in order

    def identity(self) -> str:
        return "my-service/v3"              # names the geometry, not the run


clusters = v.semantic_clusters(text, 0.85, MyEmbedder())
assert clusters["model_hash"] == "my-service/v3"
```

`Embedder` in `matra.types` is the protocol those two methods satisfy, and it is what `semantic_clusters` is annotated with. Importing it is optional and buys you a type checker's opinion, not a runtime check.

The object is asked for its identity before the text is parsed, so an embedder that cannot name its geometry is refused for the price of one method call.

The contract is the one the Rust port carries: exactly one vector per input text, in input order, every vector the same length. Break it and the call raises `ValueError` with the same message a Rust implementor gets, because the check is in the library and not in the binding. An exception raised inside your `embed` arrives as `ValueError` too, with your exception's own text inside the message.

`identity` is read once, when the object is handed over, and travels into the result as `model_hash`. Two embedders that can disagree must not return the same string, or scores end up attributed to a geometry that did not produce them.

## Size limits

Every method that takes text rejects text over 8 MiB with `ValueError`. The gate lives in the pipeline stage every method routes through, not in each method, so there is no method that skips it.

The per-algorithm caps apply on top: 2000 sentences for `tfidf_summarize` and `textrank_summarize`, and 200000 tokens for `rake_keyphrases` and `yake_keyphrases`, each raising `ValueError` when exceeded.

## The returned dicts and their shape

Every method returns a plain Python dict (or a list of them), not a custom object. `Document`, `Section`, `Paragraph`, `Sentence`, `Token`, `ScoredSentence`, and `Keyphrase` are `TypedDict` definitions in `matra.types` that describe the shape at both runtime and type-check time:

```python
from matra import Document, Keyphrase, Paragraph, ScoredSentence, Section, Sentence, Token
```

`Document` nests down to the token level:

```python
for section in result["sections"]:            # heading, level, paragraphs
    for para in section["paragraphs"]:        # text, in_blockquote, sentences,
                                              # readability_grade, lexical_density,
                                              # compression_ratio
        for sentence in para["sentences"]:    # text, tokens, negations,
                                              # modals, bare_assertion,
                                              # reportings, root_adverbials,
                                              # hearst_pairs
            for token in sentence["tokens"]:
                print(token["lemma"], token["pos"], token["dep"])

print(result["vocabulary_ttr"])        # float or None
print(result["nominalization_ratio"])  # float or None
```

A `Token` carries the ten CoNLL-U columns under these keys: `id`, `text`, `lemma`, `pos`, `xpos`, `feats`, `head`, `dep`, `deps`, `misc`, plus the derived `is_punct`. `head` is the `id` of the governing token in the same sentence, and `0` marks the root.

`ScoredSentence` (`text`, `score`, `position`) is the shape for `tfidf_summarize` and `textrank_summarize`. `Keyphrase` (`phrase`, `score`) is the shape for `rake_keyphrases` and `yake_keyphrases`.

## Why a metric field is `None`

Each metric declines to run below its own threshold, and a `None` records that decision rather than a failed computation.

| Field | Filled when |
|---|---|
| `readability_grade` | the paragraph has more than 10 words and `in_blockquote` is false |
| `lexical_density` | the paragraph has at least 1 word and `in_blockquote` is false |
| `compression_ratio` | the paragraph has more than 50 words, `in_blockquote` is false, and the paragraph text is at most 256 KiB |
| `vocabulary_ttr` | the document has at least one non-punctuation token |
| `nominalization_ratio` | the same condition as `vocabulary_ttr` |

"Words" means the count of tokens with `is_punct` false across the paragraph's sentences. Blockquote paragraphs are never parsed at all: their `sentences` list is empty, their three metric fields are `None`, and they contribute nothing to any document total.

`analyze_markdown` also drops content before parsing. YAML frontmatter, fenced code blocks, and table rows beginning with `|` are removed, and a line reading `## References` or `*References*` ends decomposition for the rest of the document. Use `analyze` if your markdown uses that heading for something other than a trailing bibliography.

## Compute the document-level metrics yourself

Methods do not cross the FFI boundary, only fields do. Rust's `Document` has `passive_ratio()`, `mean_sentence_length()`, `total_sentences()`, `total_words()`, and `sentence_length_std()` as methods, and none of them are reachable from Python. Compute what you need from the fields you already have. The shipped CLI does exactly this, and the same shape works in your code:

```python
sentences = [
    s
    for sec in result["sections"]
    for para in sec["paragraphs"]
    for s in para["sentences"]
]

total = len(sentences)
total_words = sum(
    sum(1 for t in s["tokens"] if not t["is_punct"]) for s in sentences
)
passive = sum(
    1
    for s in sentences
    if any(t["dep"] in ("nsubj:pass", "nsubjpass", "aux:pass") for t in s["tokens"])
)

passive_ratio = passive / total if total else 0.0
mean_sentence_length = total_words / total if total else 0.0
```

The passive test matches the Rust definition exactly: a sentence counts as passive when any of its tokens carries `nsubj:pass`, `nsubjpass`, or `aux:pass`. Keep the tuple in sync if you copy this into your own code.

## Exceptions

Every `domain::Error` variant crosses the FFI boundary as a specific Python exception class, not a generic error:

| Situation | Rust variant | Python exception |
|---|---|---|
| Model path does not exist | `ModelNotFound` | `FileNotFoundError` |
| Input exceeds the 8 MiB cap, or a per-extractor cap | `InputTooLarge` | `ValueError` |
| Input format has no decomposer | `UnsupportedFormat` | `ValueError` |
| A caller broke a documented contract, such as an embedder returning the wrong number of vectors | `InvalidInput` | `ValueError` |
| File I/O error | `Io` | `OSError` |
| Model file corrupt, wrong format, or download failed | `ModelInvalid` | `RuntimeError` |
| NLP parsing failed, including a panic caught at the UDPipe boundary | `ParseFailed` | `RuntimeError` |

```python
try:
    result = v.analyze(text)
except FileNotFoundError as e:
    print(f"model missing: {e}")
except ValueError as e:
    print(f"bad input: {e}")
except RuntimeError as e:
    print(f"parse failed: {e}")
```

The message on each exception is the Rust error's own `Display` output, so an `InputTooLarge` message names the gate that fired: `input`, `file_source`, `tfidf`, `textrank`, `rake`, or `yake`.

The mapping is exhaustive and enforced at compile time on the Rust side. The match that builds the Python exception has no wildcard arm, so adding a new `domain::Error` variant without also wiring it to a Python exception class fails to build. A new variant cannot fall through to `RuntimeError` by accident.

The one failure that does not follow this table is the cross-thread access described above, which arrives as `pyo3_runtime.PanicException` and bypasses `except Exception` entirely.
