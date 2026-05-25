# Python usage

## The Vaani class

```python
from vaani import Vaani

# Constructor 1: load from a local file
v = Vaani.from_path("/path/to/english-ewt.udpipe")

# Constructor 2: download the English model on first use (SHA-256-verified, atomic)
v = Vaani.english("/path/to/model/dir")
```

The instance is **not thread-safe**. UDPipe holds C-side state that cannot safely be accessed from multiple threads simultaneously. PyO3 enforces this at runtime: if you pass a `Vaani` instance to a `ThreadPoolExecutor`, it will panic when the worker thread tries to call it. Use `ProcessPoolExecutor` instead: each process gets its own model instance, so there is no sharing.

## Methods

All methods return Python dicts (or lists of dicts) mirroring the Rust domain types. The TypedDict shapes are documented in `_core.pyi` and exported from `vaani` for use as type annotations.

```python
from vaani import Vaani, Document, ScoredSentence, Keyphrase

v: Vaani = Vaani.english(model_dir)

# Full pipeline (parse + metrics)
analysis: Document = v.analyze(text)              # plain text
analysis: Document = v.analyze_markdown(md_text)  # markdown with section awareness

# Just summarization (parses internally)
top3: list[ScoredSentence] = v.tfidf_summarize(text, 3)
top3: list[ScoredSentence] = v.textrank_summarize(text, 3)

# Just keyphrases (parses internally)
phrases: list[Keyphrase] = v.rake_keyphrases(text, 10)
phrases: list[Keyphrase] = v.yake_keyphrases(text, 10)
```

## Dict shapes

```python
class Token(TypedDict):
    id: int
    text: str
    lemma: str
    pos: str
    xpos: str
    feats: str
    head: int
    dep: str
    deps: str
    misc: str
    is_punct: bool

class Sentence(TypedDict):
    text: str
    tokens: list[Token]

class Paragraph(TypedDict):
    text: str
    in_blockquote: bool
    sentences: list[Sentence]
    readability_grade: float | None
    lexical_density: float | None
    compression_ratio: float | None

class Section(TypedDict):
    heading: str | None
    level: int
    paragraphs: list[Paragraph]

class Document(TypedDict):
    sections: list[Section]
    vocabulary_ttr: float | None
    nominalization_ratio: float | None

class ScoredSentence(TypedDict):
    text: str
    score: float
    position: int

class Keyphrase(TypedDict):
    phrase: str
    score: float
```

Iterate the section tree to reach paragraphs, sentences, and tokens:

```python
analysis = v.analyze_markdown(text)
for sec in analysis["sections"]:
    print(f"Section level {sec['level']}: {sec['heading'] or '(intro)'}")
    for para in sec["paragraphs"]:
        if para["readability_grade"] is not None:
            print(f"  grade {para['readability_grade']:.1f}: {para['text'][:50]}")
```

## Exception classes

Each Rust error variant surfaces as a specific Python exception class. See [Errors](../reference/errors.md#handling-errors-in-python) for the full mapping and usage examples.

## Methods do not cross FFI

Aggregate methods on the Rust `Document` (`passive_ratio()`, `mean_sentence_length()`, `total_sentences()`, `total_words()`) are not visible in the serialized Python dict. They are Rust methods, not fields. Compute the aggregates in Python from the section tree, or use the Rust API directly if you need them.

```python
# Computing total sentence count from the Python dict
def total_sentences(analysis: Document) -> int:
    return sum(
        len(para["sentences"])
        for sec in analysis["sections"]
        for para in sec["paragraphs"]
    )
```

See [Cross-language story](../architecture/cross-language.md) for why methods don't cross FFI and what the full type-crossing rules are.

## Type stubs

The Python wheel ships with full type stubs (`py.typed` + `_core.pyi`), so `mypy --strict` and `pyright --strict` catch dict-access mistakes against the TypedDict shapes returned by every method.

If you use `mypy`:

```toml
# pyproject.toml of your consumer project
[tool.mypy]
strict = true
```

`mypy` will pick up vaani's `py.typed` marker automatically and use the typed shapes. Dict access like `analysis["sections"][0]["paragraphs"][0]["readability_grade"]` is type-checked against the TypedDicts.

If you use `pyright` or another type checker, the same `py.typed` + `_core.pyi` mechanism applies (PEP 561).
