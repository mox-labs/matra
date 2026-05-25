# Python guide

Install vaani, create a `Vaani` instance, call a method. That is the whole shape. This guide fills in the details: what the methods return, how to work with the dict structure, and where the FFI boundary matters.

## Load the model

```python
from vaani import Vaani

v = Vaani.english("~/.vaani/models")
```

`Vaani.english` downloads the English UDPipe model on first use, verifies its SHA-256 against a pinned hash, and caches it at the path you provide. Subsequent calls load from cache. If you already have a model file:

```python
v = Vaani.from_path("/path/to/english-ewt-ud-2.5-191206.udpipe")
```

Create one `Vaani` instance and reuse it. The underlying model holds C-side state; construction is the expensive step.

## Analyze text

```python
result = v.analyze("The committee approved the proposal without debate.")
# or for markdown:
result = v.analyze_markdown(text)
```

`result` is a plain Python dict matching the `Document` TypedDict shape. It is not a special object. You access it with dict syntax:

```python
print(result["vocabulary_ttr"])       # float or None
print(result["nominalization_ratio"]) # float or None

for section in result["sections"]:
    for para in section["paragraphs"]:
        print(para["readability_grade"])  # float or None
        for sentence in para["sentences"]:
            print(sentence["text"])
            for token in sentence["tokens"]:
                print(token["lemma"], token["pos"], token["dep"])
```

Import the TypedDict shapes for type-checker support:

```python
from vaani import Document, Section, Paragraph, Sentence, Token
```

These are runtime `TypedDict` definitions that mirror the Rust structs in `domain.rs`.

## Passive ratio and mean sentence length

`Document` in Rust has `passive_ratio()` and `mean_sentence_length()` as methods. Those methods do not cross the FFI boundary. Only fields serialize across; methods do not. Compute them from the dict:

```python
sentences = [
    s
    for sec in result["sections"]
    for para in sec["paragraphs"]
    for s in para["sentences"]
]

total = len(sentences)
passive = sum(
    1
    for s in sentences
    if any(t["dep"] in ("nsubj:pass", "nsubjpass", "aux:pass") for t in s["tokens"])
)

passive_ratio = passive / total if total else 0.0
```

This is the same computation the CLI uses in `cli.py`.

## Summarization and keyphrases

```python
# TF-IDF or TextRank: top-3 sentences
summary = v.tfidf_summarize(text, 3)
summary = v.textrank_summarize(text, 3)

for sent in summary:
    print(f"[{sent['score']:.3f}] {sent['text']}")

# RAKE or YAKE: top-10 phrases
phrases = v.rake_keyphrases(text, 10)
phrases = v.yake_keyphrases(text, 10)

for kp in phrases:
    print(f"{kp['score']:.2f}  {kp['phrase']}")
```

Each method returns a list of dicts: `ScoredSentence` (`text`, `score`, `position`) for summaries; `Keyphrase` (`phrase`, `score`) for keyphrases.

Import the TypedDict shapes:

```python
from vaani import ScoredSentence, Keyphrase
```

## Exception classes

| Situation | Exception |
|---|---|
| Model path does not exist | `FileNotFoundError` |
| Model file corrupt or wrong format | `RuntimeError` |
| Input exceeds 8 MiB | `ValueError` |
| Input format not supported | `ValueError` |
| NLP parsing failed | `RuntimeError` |
| File I/O error | `OSError` |

```python
try:
    result = v.analyze(text)
except ValueError as e:
    print(f"Bad input: {e}")
except RuntimeError as e:
    print(f"Parse failed: {e}")
```

The mapping is exhaustive and enforced at compile time in the Rust binding: adding a new `domain::Error` variant without wiring it to a Python exception class is a compile error.

## Thread safety

`Vaani` is `#[pyclass(unsendable)]`. Do not share a single `Vaani` instance across threads:

```python
# Fine: each process has its own Vaani instance
from concurrent.futures import ProcessPoolExecutor

def analyze_chunk(text):
    v = Vaani.english("~/.vaani/models")
    return v.analyze(text)

with ProcessPoolExecutor() as pool:
    results = list(pool.map(analyze_chunk, chunks))

# Not fine: sharing a Vaani across threads
# executor = ThreadPoolExecutor()  # do not do this
```

Multi-process (`ProcessPoolExecutor`) is fine. Each process gets its own C-side model state. Multi-thread (`ThreadPoolExecutor`) sharing a single `Vaani` instance panics at runtime.

## Methods that do not exist in Python

Rust `Document` has several methods that compute over the struct fields: `total_sentences()`, `total_words()`, `passive_ratio()`, `mean_sentence_length()`, `sentence_length_std()`. None of these are exposed in Python. vaani crosses the FFI boundary with fields only. Compute what you need from `result["sections"]`.

The same applies to `Sentence` methods (`word_count()`, `is_passive()`, `tree_depth()`, `children_of()`, `subtree()`). You have the full `tokens` list in each sentence dict; query it directly.
