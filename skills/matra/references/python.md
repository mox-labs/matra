---
name: python
summary: The Python API when the agent writes Python: every method, the Embedder protocol, Model2Vec.embed for raw vectors, analyze_path, and exception mapping.
---

# The Python API

`pip install matra`, or `uv add matra`. The package exposes two classes, one module-level function, and a set of typed shapes.

```python
from matra import Matra, Model2Vec, semantic_clusters
```

Everything returns plain dictionaries and lists. There are no wrapper objects to learn: the shapes are exactly the JSON shapes in `json`, and `matra.types` declares them as `TypedDict` definitions available at runtime.

## `Matra`

The loaded engine. Create it once and reuse it; loading the model is the expensive part.

```python
from matra import Matra

engine = Matra.english()
doc = engine.analyze_markdown(open("notes.md").read())
print(doc["passive_ratio"], len(doc["sections"]))
```

| Method | Signature | Returns |
|---|---|---|
| `Matra.from_path` | `(model_path) -> Matra` | The engine, loading a UDPipe model file you name. Nothing is verified and nothing is fetched |
| `Matra.english` | `(model_dir=None) -> Matra` | The engine, downloading the pinned English model if it is absent. With no argument the directory resolves the way every surface resolves it |
| `analyze` | `(text: str) -> dict` | A document, decomposed as plain text |
| `analyze_markdown` | `(text: str) -> dict` | A document, decomposed as markdown: sections from headings, frontmatter and fenced code dropped |
| `tfidf_summarize` | `(text: str, n: int) -> list[dict]` | Top-n scored sentences in document order |
| `textrank_summarize` | `(text: str, n: int) -> list[dict]` | The same, ranked by graph centrality |
| `rake_keyphrases` | `(text: str, max_phrases: int) -> list[dict]` | Ranked phrases, highest first |
| `yake_keyphrases` | `(text: str, max_phrases: int) -> list[dict]` | The same, different ranking |
| `semantic_clusters` | `(text: str, threshold: float, model) -> dict` | Clusters over the text's sentences. See `semantic` |
| `analyze_path` | `(path) -> list[dict]` | One item per document a path names |

Every path argument takes a `str` or an `os.PathLike` such as a `pathlib.Path`, and every one that defaults to `None` resolves through the configuration when the argument is absent.

Every method that takes text routes through the same entry point, so the 8 MiB input cap applies uniformly, with the per-extractor caps on top.

**Threading.** The class is unsendable: the loaded model holds C-side state that is not thread-safe, so touching one instance from a thread other than the one that created it fails at runtime. Multi-process is fine. Use `ProcessPoolExecutor`, not `ThreadPoolExecutor`.

## `analyze_path`

One item for a file, one per regular file for a directory, in path order. Symlinks and subdirectories are skipped, not followed, and the walk is one level deep.

```python
from matra import Matra

engine = Matra.english()
for item in engine.analyze_path("docs/"):
    if "error" in item:
        print(item["path"], item["error"]["kind"], item["error"]["message"])
    else:
        print(item["path"], item["analysis"]["passive_ratio"])
```

A document that analyzed arrives as `{"path": str | None, "analysis": dict}`. One that did not arrives as `{"path": str | None, "error": {"kind": str, "message": str}}`. Test `"error" in item` to tell them apart; a type checker narrows the union on that test. One unreadable file costs one item rather than the whole walk. A failure listing the path itself raises instead of arriving as an item.

`path` is `None` only for a document that never came from disk, which a directory walk never produces. On Unix a returned path is decoded with `os.fsdecode`, so `os.fsencode` on it names the same file even when the name is not valid text.

`kind` is one of seven stable strings, exported as `matra.ERROR_KINDS`. Branch on it. The message is for a human to read and is not a contract.

## `Model2Vec` and the embedder protocol

```python
from matra import Matra, Model2Vec

engine = Matra.english()
model = Model2Vec.potion_base_8m()
clusters = engine.semantic_clusters(text, 0.85, model)
```

| Member | Signature | Notes |
|---|---|---|
| `Model2Vec.from_dir` | `(dir) -> Model2Vec` | Loads three artifacts from a directory. Never touches the network |
| `Model2Vec.potion_base_8m` | `(dir=None) -> Model2Vec` | Downloads the pinned reference model if absent. With no argument the directory comes from the configuration |
| `model_hash` | property, `str` | SHA-256 over the three artifact files: the model identity |
| `dimensions` | property, `int` | Length of every vector this model produces |
| `embed` | `(texts: list[str]) -> list[list[float]]` | One vector per text, in order, all of `dimensions` length |
| `identity` | `() -> str` | The same string `model_hash` carries |

`embed` plus `identity` is the whole `Embedder` protocol, declared in `matra.types`. Any object with those two methods is accepted where a model is asked for; `Model2Vec` takes a fast path that calls no Python, and your own object goes through the same contract check.

```python
from matra import Matra

class Constant:
    def embed(self, texts: list[str]) -> list[list[float]]:
        return [[1.0, 0.0] for _ in texts]

    def identity(self) -> str:
        return "constant-v1"

clusters = Matra.english().semantic_clusters(text, 0.9, Constant())
```

The contract is checked in the library, so a Python embedder returning the wrong number of vectors raises `ValueError` with the same message a Rust implementor would read, and an exception raised inside `embed` arrives as `ValueError` carrying its own text. `identity` is read once, before the text is parsed, so a broken embedder costs one method call rather than a document's parse.

Two embedders that can disagree must not return the same identity string.

## Module-level `semantic_clusters`

```python
from matra import semantic_clusters

clusters = semantic_clusters(vectors, 0.85, "my-model-v1")
```

For a caller who already holds embeddings. Indices in the result are positions in the list passed in, and the scores are attributed to the hash given. Raises `ValueError` when vectors disagree on dimension, contain a non-finite value, exceed the 2,000-vector cap, or the threshold is not finite.

## Exceptions

The binding converts every library failure into a Python exception class through a match with no catch-all arm, so a new failure variant cannot silently inherit another one's class.

| Failure | Python exception |
|---|---|
| model not found | `FileNotFoundError` |
| input too large | `ValueError` |
| unsupported format | `ValueError` |
| invalid input | `ValueError` |
| io | `OSError` |
| model invalid | `RuntimeError` |
| parse failed | `RuntimeError` |

The exception message is the library's own display string. Variant identity beyond the exception class does not cross, so a caller separating an oversized input from an unsupported format inspects the message. Where a stable machine-readable key is needed, use `analyze_path`, whose items carry `kind`.

```python
from matra import Matra

try:
    engine = Matra.from_path("models/english-ewt-ud-2.5-191206.udpipe")
except FileNotFoundError as exc:
    print(exc)  # model not found: models/english-ewt-ud-2.5-191206.udpipe
```

## The command line from Python

The wheel installs a `matra` console script that is the Rust command line reached through the extension module, not a second implementation. `_core.cli_main(argv)` runs it and returns the exit code, with `argv` excluding the program name. Arguments cross with the filesystem encoding, so a path Python decoded with surrogate escapes names the same file it names for the Rust binary. Exit codes are the same: 0 found, 1 nothing found, 2 error.
