# Domain Types Reference

vaani's public type hierarchy mirrors the structural units of text: tokens nest into sentences, sentences into paragraphs, paragraphs into sections, sections into a document. Extraction algorithms produce `ScoredSentence` and `Keyphrase`. The pipeline's ingestion stage produces `RawDocument`. A corpus is a `Corpus` of `CorpusEntry` records.

All types derive `serde::Serialize` and `serde::Deserialize` except `RawDocument` (which is not serialized; it is a transient pipeline input). All types carry `#[non_exhaustive]` for forward compatibility across SemVer minor versions.

For the reasoning behind the `Document` rename from `Analysis`, see [ADR-0006](../architecture/hex.md).

---

## Token

One parsed token. Carries the full CoNLL-U annotation set plus one derived field.

| Field | Type | CoNLL-U column | Meaning |
|---|---|---|---|
| `id` | `usize` | 1 | 1-based position within the sentence |
| `text` | `String` | 2 | Surface form as it appears in the source text |
| `lemma` | `String` | 3 | Dictionary (base) form |
| `pos` | `String` | 4 | Universal POS tag (e.g. `NOUN`, `VERB`, `ADJ`) |
| `xpos` | `String` | 5 | Language-specific POS tag |
| `feats` | `String` | 6 | Morphological features, pipe-separated (e.g. `Number=Sing\|Case=Nom`) |
| `head` | `usize` | 7 | Head token `id`; `0` signals the syntactic root |
| `dep` | `String` | 8 | Dependency relation to head (e.g. `nsubj`, `obj`, `aux:pass`) |
| `deps` | `String` | 9 | Enhanced dependency graph (may be empty) |
| `misc` | `String` | 10 | Miscellaneous annotations (may be empty) |
| `is_punct` | `bool` | derived | `true` when `pos == "PUNCT"`; used to gate metric computation |

**Construction.** Use `Token::builder(id, text, lemma, pos, head, dep)` and call `.build()`. The six required fields are the CoNLL-U essentials; optional fields default to empty strings, `is_punct` defaults to `false`.

`Token` has no public methods beyond the builder. Fields are `pub`; `#[non_exhaustive]` prevents struct literal construction outside the crate.

---

## Sentence

One parsed sentence: its verbatim text plus id-sorted tokens.

| Field | Type | Meaning |
|---|---|---|
| `text` | `String` | Verbatim sentence text as produced by the NLP provider |
| `tokens` | `Vec<Token>` | CoNLL-U tokens in ascending `id` order |

**Invariants.** Tokens are id-sorted. Exactly one token has `head == 0` (the syntactic root). All `head` references point to another token in the same sentence or to `0`.

### Methods

| Method | Returns | Rust only | Notes |
|---|---|---|---|
| `new(text, tokens)` | `Sentence` | no (constructor) | Caller upholds the invariants |
| `content_tokens()` | `Vec<&Token>` | yes | Tokens where `is_punct == false` |
| `word_count()` | `usize` | yes | Non-punctuation token count |
| `is_passive()` | `bool` | yes | `true` if any token has `dep` of `nsubj:pass`, `nsubjpass`, or `aux:pass` |
| `tree_depth()` | `usize` | yes | Max depth of the dependency tree; returns `usize::MAX` on malformed cyclic parses |
| `root_token()` | `Option<&Token>` | yes | Token with `head == 0` |
| `children_of(id)` | `Vec<&Token>` | yes | Direct dependents of the given token id |
| `head_of(id)` | `Option<&Token>` | yes | Head token of the given token id; `None` for root |
| `subtree(id)` | `Vec<&Token>` | yes | All tokens in the subtree rooted at id, sorted by id; cycle-safe |

All methods are Rust-only. Python consumers receive `Sentence` as a `TypedDict` with `text` and `tokens` fields; method results are not available.

---

## Paragraph

One paragraph with optional per-paragraph metric slots. Slots are `None` until the pipeline's `measure` stage runs.

| Field | Type | Meaning |
|---|---|---|
| `text` | `String` | Verbatim paragraph text |
| `in_blockquote` | `bool` | When `true`, the paragraph is inside a blockquote and metrics are skipped |
| `sentences` | `Vec<Sentence>` | Sentences parsed from this paragraph by the `parse` stage |
| `readability_grade` | `Option<f64>` | Flesch-Kincaid grade level; `None` if not measured or word count <= 10 |
| `lexical_density` | `Option<f64>` | Content-word ratio (0.0 to 1.0); `None` if not measured |
| `compression_ratio` | `Option<f64>` | Brotli compression ratio; `None` if not measured, word count <= 50, or paragraph > 256 KiB |

**Deprecation notice.** `in_blockquote: bool` is planned for replacement by `kind: ParagraphKind` in v0.2. See [ADR-0006](../architecture/hex.md).

### Methods

| Method | Returns | Rust only | Notes |
|---|---|---|---|
| `new(text, in_blockquote)` | `Paragraph` | no (constructor) | Metric slots initialize to `None` |
| `word_count()` | `usize` | yes | Total non-punctuation tokens across all sentences |
| `sentence_count()` | `usize` | yes | Number of sentences in this paragraph |

---

## Section

A structural section: an optional heading plus its paragraphs.

| Field | Type | Meaning |
|---|---|---|
| `heading` | `Option<String>` | Section heading text; `None` for intro sections with no leading heading, and for plain-text decomposition |
| `level` | `usize` | Heading depth (0 for plain text, 1 for `#`, 2 for `##`, etc.) |
| `paragraphs` | `Vec<Paragraph>` | Paragraphs in document order |

`Section::new(heading, level, paragraphs)` is the only constructor. No additional public methods.

---

## Document

The full analysis output. Sections are the single source of truth for paragraph and sentence ownership.

| Field | Type | Meaning |
|---|---|---|
| `sections` | `Vec<Section>` | Section tree; owns all paragraphs and sentences |
| `vocabulary_ttr` | `Option<f64>` | Type-token ratio over lemmas (excluding punct), document-level; `None` until `measure` runs |
| `nominalization_ratio` | `Option<f64>` | Ratio of nominalizing-suffix nouns to total non-punct lemmas, document-level; `None` until `measure` runs |

### Methods

| Method | Returns | Rust only | Notes |
|---|---|---|---|
| `new(sections)` | `Document` | no (constructor) | Metric slots initialize to `None` |
| `paragraphs()` | `impl Iterator<Item = &Paragraph>` | yes | Flat iterator over all paragraphs |
| `paragraphs_mut()` | `impl Iterator<Item = &mut Paragraph>` | yes | Mutable flat iterator |
| `paragraph_count()` | `usize` | yes | Total paragraph count |
| `sentences()` | `impl Iterator<Item = &Sentence>` | yes | Flat iterator over all sentences across all paragraphs |
| `tokens()` | `impl Iterator<Item = &Token>` | yes | Flat iterator over all tokens across all sentences |
| `total_sentences()` | `usize` | yes | Sum of sentence counts across all paragraphs |
| `total_words()` | `usize` | yes | Sum of non-punctuation tokens across all sentences |
| `passive_ratio()` | `f64` | yes | Fraction of passive sentences; `0.0` when no sentences |
| `mean_sentence_length()` | `f64` | yes | Mean words per sentence; `0.0` when no sentences |
| `sentence_length_std()` | `f64` | yes | Sample standard deviation of sentence length; `0.0` when fewer than two sentences |

All methods are Rust-only. Python consumers receive `Document` as a `TypedDict` with `sections`, `vocabulary_ttr`, and `nominalization_ratio`; compute aggregate values from `sections` if needed.

**Transitional alias.** `pub type Analysis = Document` ships with a `#[deprecated]` annotation for in-flight branches. It will be removed in 0.1.0.

---

## ScoredSentence

Output of TF-IDF summarization and TextRank summarization.

| Field | Type | Meaning |
|---|---|---|
| `text` | `String` | Verbatim sentence text |
| `score` | `f64` | Relevance score; higher is more relevant |
| `position` | `usize` | Original sentence index in the input slice; enables re-anchoring in document order |

Results from both summarizers are returned in document order (ascending `position`), not score order.

---

## Keyphrase

Output of RAKE keyphrase extraction and YAKE keyphrase extraction.

| Field | Type | Meaning |
|---|---|---|
| `phrase` | `String` | The keyphrase text (one to three words for YAKE; variable for RAKE) |
| `score` | `f64` | Relevance score; higher is more relevant |

---

## RawDocument

A document before decomposition. Output of `Source`, input to `Decomposer`.

| Field | Type | Meaning |
|---|---|---|
| `text` | `String` | Document text |
| `path` | `Option<PathBuf>` | Source path if the document came from disk; `None` for in-memory text |
| `format` | `Format` | Detected format |

`RawDocument` is not `Serialize`/`Deserialize`. It is a transient pipeline type; it does not appear in Python or in any stored output.

---

## Format

Document format enum, detected from file extension.

| Variant | Meaning |
|---|---|
| `Markdown` | Markdown source |
| `PlainText` | Plain text |
| `Pdf` | Reserved; no decomposer ships today; returns `Error::UnsupportedFormat` |
| `Docx` | Reserved; no decomposer ships today; returns `Error::UnsupportedFormat` |

---

## CorpusEntry

One analyzed document in a corpus.

| Field | Type | Meaning |
|---|---|---|
| `path` | `Option<PathBuf>` | Source path if from disk; `None` for in-memory input |
| `analysis` | `Document` | The document's full analysis output |

---

## Corpus

A collection of analyzed documents.

| Field | Type | Meaning |
|---|---|---|
| `entries` | `Vec<CorpusEntry>` | One entry per successfully analyzed document |

### Methods

| Method | Returns | Rust only | Notes |
|---|---|---|---|
| `new(entries)` | `Corpus` | no (constructor) | |
| `total_words()` | `usize` | yes | Total non-punct tokens across every entry |
| `passive_ratio()` | `f64` | yes | Fraction of passive sentences across all entries; `0.0` when no sentences |
| `mean_readability()` | `f64` | yes | Mean readability grade across all paragraphs carrying a `readability_grade` value; `0.0` when no paragraphs measured |

All methods are Rust-only.

---

## Python TypedDict shapes

The Python surface exposes `Token`, `Sentence`, `Paragraph`, `Section`, `Document`, `ScoredSentence`, and `Keyphrase` as `TypedDict` shapes in `vaani.types`. Import at runtime:

```python
from vaani.types import Document, ScoredSentence, Keyphrase
```

`RawDocument`, `CorpusEntry`, `Corpus`, and `Format` are not exposed on the Python surface in v0.1.

---

*For formulas behind the metric slots, see [reference/methodology.md](methodology.md). For error handling, see [reference/errors.md](errors.md).*
