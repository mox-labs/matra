# Quickstart

Parse a short text, read the result fields, iterate the structure, and run summarization and keyphrase extraction. By the end, you know the full shape of what vaani returns and how to pull what your application needs.

This tutorial uses Python. If you are building in Rust, the guides section covers the Rust API in detail.

---

## Step 1: Load the engine

```python
from pathlib import Path
from vaani import Vaani

v = Vaani.english(str(Path.home() / ".vaani" / "models"))
```

`Vaani.english()` downloads the English model on first call and loads it from cache on every call after that. The model directory is `~/.vaani/models` here; pass any path that suits your project layout.

---

## Step 2: Analyze text

```python
text = (
    "The committee approved the proposal without debate. "
    "Three amendments were submitted by the working group."
)

result = v.analyze(text)
```

`result` is a Python dict. Its structure mirrors the Rust `Document` type, the shape your application traverses to reach tokens, metrics, and dependency annotations:

```
Document
  sections: list[Section]
    Section
      heading: str | None
      level: int
      paragraphs: list[Paragraph]
        Paragraph
          text: str
          readability_grade: float | None
          lexical_density: float | None
          compression_ratio: float | None
          sentences: list[Sentence]
            Sentence
              text: str
              tokens: list[Token]
                Token
                  id, text, lemma, pos, dep, head, is_punct, ...
  vocabulary_ttr: float | None
  nominalization_ratio: float | None
```

For markdown input with headings, use `v.analyze_markdown(text)` instead. Section boundaries and headings come from the `#` / `##` structure.

---

## Step 3: Read document-level fields

```python
print(result["vocabulary_ttr"])         # type-token ratio across all words
print(result["nominalization_ratio"])   # ratio of nominal forms
```

Both fields are `float | None`. They are `None` if the input had no parseable content.

---

## Step 4: Traverse the structure

Walk from document to section to paragraph to sentence to token:

```python
for section in result["sections"]:
    print(f"Section: {section['heading']!r}, level {section['level']}")

    for para in section["paragraphs"]:
        print(f"  Paragraph readability grade: {para['readability_grade']}")

        for sent in para["sentences"]:
            print(f"    Sentence: {sent['text']!r}")

            for token in sent["tokens"]:
                print(
                    f"      {token['text']!r}"
                    f"  lemma={token['lemma']!r}"
                    f"  pos={token['pos']!r}"
                    f"  dep={token['dep']!r}"
                    f"  head={token['head']}"
                )
```

For the two-sentence committee text, the token traversal shows the structural contrast directly: `committee` carries `dep="nsubj"` (active subject); `amendments` carries `dep="nsubj:pass"` (passive subject). That difference is the structure vaani makes accessible to your application.

---

## Step 5: What is and is not in the dict

The dict contains every field defined on the Rust struct: `sections`, `vocabulary_ttr`, `nominalization_ratio`, and all nested fields down to individual token annotations.

Aggregate methods on the Rust struct don't cross the FFI boundary. There is no `result["passive_ratio"]` key. To compute the passive ratio in Python, iterate the sentences:

```python
sentences = [
    sent
    for sec in result["sections"]
    for para in sec["paragraphs"]
    for sent in para["sentences"]
]

passive = sum(
    1 for s in sentences
    if any(t["dep"] in ("nsubj:pass", "nsubjpass", "aux:pass") for t in s["tokens"])
)

passive_ratio = passive / len(sentences) if sentences else 0.0
print(f"Passive ratio: {passive_ratio:.1%}")
```

The tokens give you everything you need. The aggregation is yours to define.

This is the design: fields cross the boundary, methods don't. What gets computed (and when, and how) stays in your application.

---

## Step 6: Summarize and extract keyphrases

Both take a text string and return a ranked list.

```python
summary = v.tfidf_summarize(text, n=1)
for item in summary:
    print(f"Summary sentence (score {item['score']:.3f}): {item['text']!r}")

keyphrases = v.rake_keyphrases(text, max_phrases=5)
for kp in keyphrases:
    print(f"  {kp['phrase']!r}  score={kp['score']:.2f}")
```

`tfidf_summarize` ranks sentences by term-frequency coverage and returns the top `n`. `rake_keyphrases` extracts noun-phrase candidates by co-occurrence and returns up to `max_phrases` ranked results.

Each item in the summary list is a dict with `text`, `score`, and `position`. Each keyphrase is a dict with `phrase` and `score`.

TextRank (`v.textrank_summarize`) and YAKE (`v.yake_keyphrases`) are the alternative algorithms. The guides section covers when to choose one over the other.

---

## What you have now

After this tutorial:

- You can load the engine and call `analyze()` or `analyze_markdown()`.
- You know the full shape of the result dict: Document > Section > Paragraph > Sentence > Token.
- You can read paragraph-level metrics (`readability_grade`, `lexical_density`, `compression_ratio`) and document-level metrics (`vocabulary_ttr`, `nominalization_ratio`).
- You can iterate tokens and check `dep` labels to compute aggregates that the dict does not pre-compute.
- You can extract summaries and keyphrases.

---

## Where to go next

**The structure behind these results:** [What vaani illuminates](../architecture/conviction.md) explains the substrate framing, what the dependency labels mean for your application, and why the metrics are instruments rather than scores.

**All field definitions:** [Domain types](../reference/domain-types.md) is the canonical lookup for every field name, type, and what the `None` cases mean.

**Formula details:** [Methodology](../reference/methodology.md) documents the Flesch-Kincaid formula, the TF-IDF weighting, RAKE's scoring, and the passive-detection dependency labels.

**Python usage in depth:** A Python guide covering the full method surface, error handling, and multi-document patterns. 🛠️ Planned v0.2+.
