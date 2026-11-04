---
name: python
summary: The Python API when the agent writes Python: Engine, analyze and analyze_all, the read-only Document, summarize, keyphrases and clusters, the Embedder protocol, and exception mapping.
---

# The Python API

`pip install 'matra>=0.3'`, or `uv add 'matra>=0.3'`. Python spells what Rust spells: an `Engine` analyzes, and the outcome functions take what it returned.

```python
import matra

engine = matra.Engine()
doc = engine.analyze(file="notes.md")
summary = matra.summarize(doc, n=3)
phrases = matra.keyphrases(doc, n=10, method="yake")
print(doc["metrics"]["passive_ratio"], len(doc["sections"]))
```

A `Document` reads as the JSON in `json` reads, and every outcome returns a plain dict of that shape. `matra.types` declares the shapes as `TypedDict` definitions available at runtime; the whole document's is `DocumentDict`.

## `Engine`

The loaded pipeline. Create it once and reuse it; loading the model is the expensive part.

| Member | Signature | Returns |
|---|---|---|
| `Engine()` | `() -> Engine` | The engine, downloading the pinned English model if it is absent. The directory comes from the environment (`MATRA_MODEL_DIR`, else the data root's `models`); no configuration file is read |
| `Engine.from_config` | `(config_file=None) -> Engine` | The same, with a configuration file read and checked: the one named, which must exist, else the one the environment names |
| `analyze` | `(text=None, *, file=None, bytes=None, blocks=None, format=None) -> Document` | One document |
| `analyze_all` | `(*, dir=None, texts=None, format=None) -> Iterator[Document \| DocumentError]` | Many, lazily, in input order |

`analyze` takes exactly one input; none or two raise `TypeError`. **A `str` is always text, never a path**: name a file with `file=`.

- `text` is read as plain text, or as `format` (`markdown`, `plain_text`, `blocks`).
- `file` is read at the call, as `format` when one is named, else in the format its name and bytes imply: `.md` as markdown, PDF and DOCX refused, any other UTF-8 file as plain text. A JSON file of blocks is `format="blocks"`. Its path is in `doc["provenance"]["source"]["path"]`.
- `bytes` is read as `format`, or detected when none is named.
- `blocks` is a list of `{"kind": "heading", "level": 1, "text": ...}` and `{"kind": "paragraph", "text": ...}`, for text from somewhere matra does not read. An unknown `kind` or key raises `matra.InvalidInputError` naming it.

`format` applies to `text`, `bytes` and `file`, and a PDF or DOCX signature in the bytes outranks it. Every input passes the 8 MiB cap on its way to the parser.

**Threading.** The engine is unsendable: the loaded model holds C-side state that is not thread-safe, so touching one engine from a thread other than the one that created it fails at runtime. Multi-process is fine. Use `ProcessPoolExecutor`, not `ThreadPoolExecutor`, and return `doc.to_dict()` from a worker, since a `Document` does not pickle.

## `analyze_all`

One item per input, in order, and nothing is read until you pull it. `dir=` takes every regular file in a directory, in path order, one level deep; symlinks and subdirectories are skipped, not followed. `texts=` takes an iterable of `str`, read as plain text or as `format`; a bare `str` raises `TypeError`.

```python
import matra

engine = matra.Engine()
for item in engine.analyze_all(dir="docs/"):
    if isinstance(item, matra.Document):
        print(item["provenance"]["source"]["path"], item["metrics"]["passive_ratio"])
    else:
        print(item["path"], item["kind"], item["message"])
```

A document that analyzed arrives as a `Document`. One that did not arrives as a dict, `{"path": str | None, "kind": str, "message": str, "details": dict}`. Test `isinstance(item, matra.Document)` to tell them apart; a type checker narrows the union on that test. One unreadable file costs one item rather than the whole walk. A failure listing the directory itself raises at the call.

On Unix a returned path is decoded with `os.fsdecode`, so `os.fsencode` on it names the same file even when the name is not valid text.

`kind` is one of seven stable strings, exported as `matra.ERROR_KINDS`. Branch on it. The message is for a human to read and is not a contract; `details` holds every value in it by name (the errors reference lists the keys per kind).

## `Document`

Read-only, holding the parse. `doc["provenance"]`, `doc["sections"]`, `doc["metrics"]` and `doc["skipped"]` read one field; `doc.to_dict()` reads the whole. `list(doc)`, `len(doc)` and `"metrics" in doc` work as on a dict. Each read builds a new value, so changing it changes nothing in the document; hold a field in a variable when you read it in a loop.

Spans count UTF-8 bytes. `matra.span_text(text, span)` slices a `str` by one correctly.

## Outcomes

| Function | Signature | Returns |
|---|---|---|
| `summarize` | `(doc, n=3, *, method=None) -> Summary` | The top-n sentences in document order, each with its `index` and `span`, and the method and `n` in its `provenance` |
| `keyphrases` | `(doc, n=10, *, method=None) -> Keyphrases` | Ranked phrases, highest first, each with every `occurrences` span |
| `clusters` | `(doc, embedder, threshold=0.85) -> Clusters` | The sentences grouped by embedding similarity, the embedder's identity and the threshold in its `provenance`. See `semantic` |

`method` is `tfidf` or `textrank` for a summary, `rake` or `yake` for keyphrases; with none, the default runs (`tfidf`, `rake`), the same as the command line's. An unknown name raises `matra.InvalidInputError`. Each takes a `Document`, never a dict, and parses nothing, so ask for as many outcomes of one document as you like.

## `Model2Vec` and the embedder protocol

```python
import matra

model = matra.Model2Vec.potion_base_8m()
found = matra.clusters(doc, model, threshold=0.85)
```

| Member | Signature | Notes |
|---|---|---|
| `Model2Vec.from_dir` | `(dir) -> Model2Vec` | Loads three artifacts from a directory. Never touches the network |
| `Model2Vec.potion_base_8m` | `(dir=None) -> Model2Vec` | Downloads the pinned reference model if absent. With no argument the directory comes from the configuration |
| `model_hash` | property, `str` | SHA-256 over the three artifact files: the model identity |
| `dimensions` | property, `int` | Length of every vector this model produces |
| `embed` | `(texts: list[str]) -> list[list[float]]` | One vector per text, in order, all of `dimensions` length |
| `identity` | `() -> str` | The same string `model_hash` carries |

`embed` plus `identity` is the whole `Embedder` protocol, declared in `matra.types`. Any object with those two methods is accepted where a model is asked for; `Model2Vec` takes a fast path that calls no Python, and your own object goes through the same contract check. Vectors you already hold get in the same way: an object whose `embed` returns them.

```python
class Constant:
    def embed(self, texts: list[str]) -> list[list[float]]:
        return [[1.0, 0.0] for _ in texts]

    def identity(self) -> str:
        return "constant-v1"

found = matra.clusters(doc, Constant(), threshold=0.9)
```

The contract is checked in the library, so a Python embedder returning the wrong number of vectors raises `matra.InvalidInputError`, a `ValueError`, with the same message a Rust implementor would read, and an exception raised inside `embed` arrives as `matra.InvalidInputError` carrying its own text. `identity` is read once, before anything is embedded, so a broken embedder costs one method call.

Two embedders that can disagree must not return the same identity string.

## Exceptions

Every library failure is raised as a class of its own in `matra`, named for the failure with an `Error` suffix, through a match with no catch-all arm, so a new failure variant cannot silently inherit another one's class. Each class derives from `matra.MatraError` and from the builtin class matra raised before 0.3.0, so `except ValueError` still catches an `InputTooLargeError`.

| Failure | Python class | Also a |
|---|---|---|
| model not found | `ModelNotFoundError` | `FileNotFoundError` |
| input too large | `InputTooLargeError` | `ValueError` |
| unsupported format | `UnsupportedFormatError` | `ValueError` |
| invalid input | `InvalidInputError` | `ValueError` |
| io | `IoError` | `OSError` |
| io, a path that does not exist | `IoNotFoundError` | `FileNotFoundError` |
| io, permission denied | `IoPermissionError` | `PermissionError` |
| model invalid | `ModelInvalidError` | `RuntimeError` |
| parse failed | `ParseFailedError` | `RuntimeError` |

Every exception carries `kind`, the stable string, and `details`, a dict of every value in the message. The message, `str(exc)`, is the library's own display string, for a person. Branch on the class or on `kind`, and read values from `details` rather than parsing the message. A call with the wrong arguments raises the builtin `TypeError`, which carries neither.

```python
import matra

try:
    doc = engine.analyze(file="notes.md")
except FileNotFoundError as exc:
    print(exc.details["path"])  # notes.md
except matra.InputTooLargeError as exc:
    print(exc.details["what"], exc.details["limit"], exc.details["actual"])
```

## The command line from Python

The wheel installs a `matra` console script that is the Rust command line reached through the extension module, not a second implementation. `_core.cli_main(argv)` runs it and returns the exit code, with `argv` excluding the program name. Arguments cross with the filesystem encoding, so a path Python decoded with surrogate escapes names the same file it names for the Rust binary. Exit codes are the same: 0 found, 1 nothing found, 2 error.
