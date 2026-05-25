# API Reference

The authoritative API documentation is generated from the source. This page points to the right surface for each language.

---

## Rust

**docs.rs (when published)**

```
https://docs.rs/vaani
```

Not yet published; the package is in alpha. For now, build locally:

```bash
cargo doc --open
```

This generates rustdoc from `src/` and opens it in the default browser. The `--all-features` flag includes the PyO3 binding surface:

```bash
cargo doc --all-features --open
```

**Public entry points** (defined in `src/lib.rs`)

| Function | Input | Output |
|---|---|---|
| `vaani::analyze(text, nlp)` | `&str`, `&dyn NlpProvider` | `domain::Result<Document>` |
| `vaani::analyze_markdown(text, nlp)` | `&str`, `&dyn NlpProvider` | `domain::Result<Document>` |
| `vaani::analyze_file(path, nlp)` | `impl AsRef<Path>`, `&dyn NlpProvider` | `domain::Result<Document>` |
| `vaani::analyze_directory(path, nlp)` | `impl AsRef<Path>`, `&dyn NlpProvider` | `domain::Result<(Corpus, Vec<(PathBuf, Error)>)>` |
| `vaani::parse(text, nlp)` | `&str`, `&dyn NlpProvider` | `domain::Result<Vec<Sentence>>` |
| `vaani::analyze_from(sections, sentences)` | `Vec<Section>`, `&[Sentence]` | `domain::Result<Document>` |
| `vaani::extraction::tfidf_summarize(sentences, n)` | `&[Sentence]`, `usize` | `domain::Result<Vec<ScoredSentence>>` |
| `vaani::extraction::textrank_summarize(sentences, n)` | `&[Sentence]`, `usize` | `domain::Result<Vec<ScoredSentence>>` |
| `vaani::extraction::rake_keyphrases(sentences, n)` | `&[Sentence]`, `usize` | `domain::Result<Vec<Keyphrase>>` |
| `vaani::extraction::yake_keyphrases(sentences, n)` | `&[Sentence]`, `usize` | `domain::Result<Vec<Keyphrase>>` |

For field-level lookup, see [reference/domain-types.md](domain-types.md).

---

## Python

**Type stubs** (installed with the wheel)

```
python/vaani/_core.pyi    — Vaani class: from_path, english, analyze, analyze_markdown,
                            tfidf_summarize, textrank_summarize, rake_keyphrases, yake_keyphrases
python/vaani/types.py     — TypedDict shapes: Token, Sentence, Paragraph, Section,
                            Document, ScoredSentence, Keyphrase
```

Import the stubs at runtime:

```python
from vaani.types import Document, ScoredSentence, Keyphrase
```

**PyPI (when published)**

```
https://pypi.org/project/vaani/
```

Not yet published.

---

## Version and MSRV

| Item | Value |
|---|---|
| Crate version | 0.0.1 |
| Rust edition | 2024 |
| Minimum Supported Rust Version (MSRV) | 1.85 |
| PyO3 | 0.28 |

---

*For type definitions, see [reference/domain-types.md](domain-types.md). For error variants, see [reference/errors.md](errors.md).*
