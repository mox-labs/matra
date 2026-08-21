# Domain types

`matra::domain` holds every type the library hands back. The module depends on `serde`, `thiserror`, and the standard library, and on nothing else, so a consumer can depend on these types without inheriting a C++ toolchain.

This page is the map of the type graph: which types exist, what each one owns, which values are stored and which are computed on demand, and which of them cross the language boundary. It is not a copy of the generated item-level documentation. For that, run `cargo doc --no-deps --all-features`.

## The type graph

Seven types nest, each owning the next. Nothing is owned twice.

<svg class="mx-own" role="img" aria-label="Nested containment from Corpus down to Token, with stored fields and computed methods for each type" viewBox="0 0 720 460" width="720" height="460" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
<title>Containment from Corpus to Token, stored fields against computed methods</title>
<style>
.mx-own text{fill:currentColor}
.mx-own .box{fill:none;stroke:currentColor;opacity:.35;stroke-width:1px}
.mx-own .ty{font-size:12.5px;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-own .mem{font-size:9.5px;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;opacity:.8}
.mx-own .hd{font-size:10px;font-family:inherit;opacity:.55}
.mx-own .rule{stroke:currentColor;opacity:.18;stroke-width:1px}
</style>
<text class="hd" x="206" y="20">stored fields</text>
<text class="hd" x="462" y="20">computed on call</text>
<line class="rule" x1="452" y1="28" x2="452" y2="448"/>
<rect class="box" x="8" y="36" width="182" height="408" rx="4"/>
<rect class="box" x="20" y="90" width="167" height="348" rx="4"/>
<rect class="box" x="32" y="134" width="152" height="298" rx="4"/>
<rect class="box" x="44" y="202" width="137" height="224" rx="4"/>
<rect class="box" x="56" y="246" width="122" height="174" rx="4"/>
<rect class="box" x="68" y="300" width="107" height="114" rx="4"/>
<rect class="box" x="80" y="368" width="92" height="40" rx="4"/>
<text class="ty" x="17" y="55">Corpus</text>
<text class="ty" x="29" y="109">CorpusEntry</text>
<text class="ty" x="41" y="153">Document</text>
<text class="ty" x="53" y="221">Section</text>
<text class="ty" x="65" y="265">Paragraph</text>
<text class="ty" x="77" y="319">Sentence</text>
<text class="ty" x="89" y="387">Token</text>
<text class="mem" x="206" y="55">entries</text>
<text class="mem" x="462" y="55">total_words · passive_ratio ·</text>
<text class="mem" x="462" y="67">mean_readability</text>
<text class="mem" x="206" y="109">path · analysis</text>
<text class="mem" x="206" y="153">sections · vocabulary_ttr ·</text>
<text class="mem" x="206" y="165">nominalization_ratio</text>
<text class="mem" x="462" y="153">paragraph_count · total_sentences ·</text>
<text class="mem" x="462" y="165">total_words · passive_ratio ·</text>
<text class="mem" x="462" y="177">mean_sentence_length · sentence_length_std</text>
<text class="mem" x="206" y="221">heading · level · paragraphs</text>
<text class="mem" x="206" y="265">text · in_blockquote · sentences ·</text>
<text class="mem" x="206" y="277">three metric slots</text>
<text class="mem" x="462" y="265">word_count · sentence_count</text>
<text class="mem" x="206" y="319">text · tokens</text>
<text class="mem" x="462" y="319">root_token · head_of · children_of ·</text>
<text class="mem" x="462" y="331">subtree · tree_depth · is_passive ·</text>
<text class="mem" x="462" y="343">word_count · content_tokens</text>
<text class="mem" x="206" y="387">the ten CoNLL-U columns · is_punct</text>
</svg>

The left column is what the pipeline writes into the struct. The right column is recomputed from the tree every time you ask, and an empty right column means the type only holds. So there is no second list of paragraphs anywhere in `Document`: the flat iterators walk the section tree, and `vocabulary_ttr` and `nominalization_ratio` are the only two document-level numbers that are stored rather than derived.

Five more types sit outside the nest:

| Type | Role |
|---|---|
| `RawDocument` | output of the ingest stage, input to the decompose stage |
| `ScoredSentence` | output of `tfidf_summarize` and `textrank_summarize` |
| `Keyphrase` | output of `rake_keyphrases` and `yake_keyphrases` |
| `Format` | which decomposer a document needs |
| `Error` | every failure the library can return |

Where each of those enters and leaves the pipeline is drawn in [Architecture](../architecture/design.md#one-call-end-to-end).

## Attributes shared across the module

| Type | Derives | `#[non_exhaustive]` |
|---|---|---|
| `Token` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `TokenBuilder` | none | no |
| `Sentence` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `Paragraph` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `Section` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `Document` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `ScoredSentence` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `Keyphrase` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `Format` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `RawDocument` | `Debug`, `Clone` | yes |
| `CorpusEntry` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `Corpus` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `Error` | `Debug`, `thiserror::Error` | yes |

Three consequences follow from that table.

`RawDocument` is the one structural type without serde derives. It is a transient value between the ingest and decompose stages and never appears in stored output.

No type in the module derives `PartialEq`, `Eq`, or `Hash`. Two `Token` values cannot be compared with `==`, and a `Format` is distinguished by matching on its variants rather than by equality. `Error` additionally has no `Clone` and no serde derives.

`#[non_exhaustive]` has two effects outside the crate. Struct literal syntax is unavailable, so you build values through the associated constructors listed below. Matches on `Format` and `Error` need a catch-all arm, because variants can be added in a minor release.

Every field on these types is `pub` and readable. `TokenBuilder` is the exception: its fields are private and you reach them through its setters.

## Token

One parsed token carrying the full CoNLL-U annotation set plus one derived flag.

| Field | Type | CoNLL-U column | Contents |
|---|---|---|---|
| `id` | `usize` | 1 | Position within the sentence, 1-based |
| `text` | `String` | 2 | Surface form |
| `lemma` | `String` | 3 | Dictionary form |
| `pos` | `String` | 4 | Universal POS tag, for example `NOUN`, `VERB`, `ADJ` |
| `xpos` | `String` | 5 | Language-specific POS tag |
| `feats` | `String` | 6 | Morphological features, pipe-separated |
| `head` | `usize` | 7 | Id of the governing token; `0` marks the root |
| `dep` | `String` | 8 | Dependency relation to the head, for example `nsubj`, `obj`, `aux:pass` |
| `deps` | `String` | 9 | Enhanced dependency graph |
| `misc` | `String` | 10 | Miscellaneous annotations, for example `SpaceAfter=No` |
| `is_punct` | `bool` | derived | Whether the token is punctuation |

Provider-specific notes for the UDPipe adapter, the one adapter that ships:

- `is_punct` is set when the UPOS tag is `PUNCT`.
- `deps` is always the string `_`, the CoNLL-U empty marker. The underlying `udpipe-rs` binding does not surface column 9.
- `xpos`, `feats`, and `misc` carry whatever the model emits, and can be `_` or empty.

A token id or head that does not fit in `usize` is rejected at the adapter with `Error::ParseFailed` rather than being silently coerced.

### Construction

```rust
let token = matra::domain::Token::builder(
        1,                       // id
        "committee".to_string(), // text
        "committee".to_string(), // lemma
        "NOUN".to_string(),      // pos
        3,                       // head
        "nsubj".to_string(),     // dep
    )
    .xpos("NN".to_string())
    .feats("Number=Sing".to_string())
    .is_punct(false)
    .build();
```

`Token::builder(id, text, lemma, pos, head, dep) -> TokenBuilder` takes the six CoNLL-U essentials. `TokenBuilder` carries one setter per optional field (`xpos`, `feats`, `deps`, `misc`, each taking `String`; `is_punct` taking `bool`), each consuming and returning the builder. `build()` returns the `Token`. Unset string fields default to empty; `is_punct` defaults to `false`.

### Methods

| Signature | Returns | Behavior |
|---|---|---|
| `feat(&self, key: &str)` | `Option<&str>` | First exact-key match over the pipe-separated `feats` pairs. `None` when the key is absent |

`feat("Mood")` returns `Some("Ind")` when `feats` is `Mood=Ind|Tense=Pres`. The value is borrowed raw from `feats`, so multi-valued features (`Case=Nom,Acc`) come back unsplit: matra exposes what the provider emitted and does not normalise it. Both the empty string and the CoNLL-U placeholder `_` carry no `key=value` pair, so every lookup on them returns `None`.

`feat` is Rust-only by design. `feats` already crosses FFI as a string, so a lookup over it adds no information to the wire; a Python or TypeScript caller splits the same string ([ADR-0009](https://github.com/mox-labs/matra/blob/main/docs/decisions/0009-feats-lookup-accessor.md)).

## Sentence

| Field | Type | Contents |
|---|---|---|
| `text` | `String` | Sentence text as the NLP provider reports it |
| `tokens` | `Vec<Token>` | Tokens in ascending `id` order |

Invariants that downstream code relies on, and that a hand-built `Sentence` is expected to uphold: tokens are id-sorted; exactly one token has `head == 0`; every other `head` names a token in the same sentence.

The UDPipe adapter builds `text` by joining token surface forms, inserting a space unless the preceding token carries `SpaceAfter=No` in `misc`. The string is a reconstruction, not a slice of the input, so whitespace can differ from the source text.

### Methods

Every method below is a walk over the `head` and `dep` columns. [What matra gives you](../capabilities.md#1-structure) draws one parsed sentence as the tree these walk.

| Signature | Returns | Behavior |
|---|---|---|
| `new(text: String, tokens: Vec<Token>)` | `Sentence` | Constructor. Does not validate the invariants |
| `content_tokens(&self)` | `Vec<&Token>` | Tokens where `is_punct` is false |
| `word_count(&self)` | `usize` | Count of non-punctuation tokens |
| `is_passive(&self)` | `bool` | True when any token's `dep` is `nsubj:pass`, `nsubjpass`, or `aux:pass` |
| `tree_depth(&self)` | `usize` | Longest path from any token to the root. Root depth is 0 |
| `root_token(&self)` | `Option<&Token>` | The token with `head == 0`, if there is one |
| `children_of(&self, id: usize)` | `Vec<&Token>` | Tokens whose `head` equals `id`. Empty for an unknown id |
| `head_of(&self, id: usize)` | `Option<&Token>` | The governing token. `None` for the root and for an unknown id |
| `subtree(&self, id: usize)` | `Vec<&Token>` | The token and all its descendants, sorted by `id`. Empty for an unknown id |

`tree_depth` runs in time linear in token count and returns `usize::MAX` when the head references form a cycle, which reports a malformed parse instead of truncating it. An empty sentence returns 0. `subtree` carries a visited set, so it terminates on cyclic input.

`word_count` counts tokens the NLP provider identified, minus punctuation. It is not a whitespace split, and the two counts differ on the same text.

## Paragraph

| Field | Type | Contents |
|---|---|---|
| `text` | `String` | Paragraph text |
| `in_blockquote` | `bool` | Whether the paragraph came from a blockquote |
| `sentences` | `Vec<Sentence>` | Sentences from parsing this paragraph |
| `readability_grade` | `Option<f64>` | Flesch-Kincaid grade level |
| `lexical_density` | `Option<f64>` | Content-word ratio, 0.0 to 1.0 |
| `compression_ratio` | `Option<f64>` | Brotli compressed size over original size |

The three metric slots hold `None` until the measure stage fills them, and stay `None` when the paragraph does not meet a metric's applicability condition.

The pipeline skips blockquote paragraphs at the parse stage, so a paragraph with `in_blockquote == true` reaches the measure stage with no sentences and keeps all three slots at `None`.

### Methods

| Signature | Returns | Behavior |
|---|---|---|
| `new(text: String, in_blockquote: bool)` | `Paragraph` | Constructor. Empty sentences, all metric slots `None` |
| `word_count(&self)` | `usize` | Non-punctuation tokens summed across this paragraph's sentences |
| `sentence_count(&self)` | `usize` | Number of sentences |

## Section

| Field | Type | Contents |
|---|---|---|
| `heading` | `Option<String>` | Heading text |
| `level` | `usize` | Heading depth |
| `paragraphs` | `Vec<Paragraph>` | Paragraphs in document order |

`MarkdownDecomposer` sets `level` to the number of leading `#` characters and `heading` to the rest of the line. Content before the first heading becomes a section with `heading: None` and `level: 0`. `PlainTextDecomposer` produces at most one section, always with `heading: None` and `level: 0`.

`Section::new(heading: Option<String>, level: usize, paragraphs: Vec<Paragraph>) -> Section` is the constructor. There are no other methods.

## Document

The output of the pipeline.

| Field | Type | Contents |
|---|---|---|
| `sections` | `Vec<Section>` | The section tree, which owns every paragraph, sentence, and token |
| `vocabulary_ttr` | `Option<f64>` | Type-token ratio over lemmas |
| `nominalization_ratio` | `Option<f64>` | Share of nominalizing nouns |

### Methods

| Signature | Returns | Behavior |
|---|---|---|
| `new(sections: Vec<Section>)` | `Document` | Constructor. Both metric slots `None` |
| `paragraphs(&self)` | `impl Iterator<Item = &Paragraph>` | Every paragraph, in document order |
| `paragraphs_mut(&mut self)` | `impl Iterator<Item = &mut Paragraph>` | The mutable form, used by the measure stage |
| `paragraph_count(&self)` | `usize` | Number of paragraphs |
| `sentences(&self)` | `impl Iterator<Item = &Sentence>` | Every sentence across every paragraph |
| `tokens(&self)` | `impl Iterator<Item = &Token>` | Every token across every sentence |
| `total_sentences(&self)` | `usize` | Number of sentences |
| `total_words(&self)` | `usize` | Non-punctuation tokens across every sentence |
| `passive_ratio(&self)` | `f64` | Passive sentences over total sentences. 0.0 with no sentences |
| `mean_sentence_length(&self)` | `f64` | `total_words` over `total_sentences`. 0.0 with no sentences |
| `sentence_length_std(&self)` | `f64` | Sample standard deviation of sentence length in words. 0.0 with fewer than two sentences |

## ScoredSentence

| Field | Type | Contents |
|---|---|---|
| `text` | `String` | Sentence text |
| `score` | `f64` | Relevance score, higher is more relevant |
| `position` | `usize` | Index of the sentence in the slice passed to the summarizer |

Both summarizers select the top N by score and then return them in ascending `position`, so the result reads in document order while `score` carries the ranking. `ScoredSentence::new(text: String, score: f64, position: usize) -> ScoredSentence` is the constructor.

## Keyphrase

| Field | Type | Contents |
|---|---|---|
| `phrase` | `String` | The phrase, built from lowercased lemmas joined by single spaces |
| `score` | `f64` | Relevance score, higher is more relevant |

Both extractors return phrases in descending score order. `Keyphrase::new(phrase: String, score: f64) -> Keyphrase` is the constructor.

## Format

Which decomposer a document needs. `FileSource` assigns the variant from the file extension.

| Variant | Assigned to |
|---|---|
| `Markdown` | Extension `.md` or `.markdown` |
| `PlainText` | Every other extension, and a file with no extension |
| `Pdf` | Extension `.pdf` |
| `Docx` | Extension `.docx` |

`Pdf` and `Docx` have no entry in the standard decomposer table, so analyzing such a file returns `Error::UnsupportedFormat`. `Ingest::text` takes the format as an argument; `Ingest::path` assigns one from the extension. `Format` derives `PartialEq` and `Eq`, which is what the decomposer table keys on.

## RawDocument

| Field | Type | Contents |
|---|---|---|
| `text` | `String` | Document text as read |
| `path` | `Option<PathBuf>` | Source path, `None` for in-memory text |
| `format` | `Format` | Detected format |

`RawDocument::new(text: String, path: Option<PathBuf>, format: Format) -> RawDocument` is the constructor. This is the value a `Source` produces and a `Decomposer` consumes.

## CorpusEntry

| Field | Type | Contents |
|---|---|---|
| `path` | `Option<PathBuf>` | Source path of the analyzed document |
| `analysis` | `Document` | The document's analysis output |

`CorpusEntry::new(path: Option<PathBuf>, analysis: Document) -> CorpusEntry` is the constructor.

## Corpus

| Field | Type | Contents |
|---|---|---|
| `entries` | `Vec<CorpusEntry>` | One entry per successfully analyzed document |

Documents that failed do not appear here. They travel as `DocumentError` values and land in `CorpusResult::errors` beside the corpus.

### Methods

| Signature | Returns | Behavior |
|---|---|---|
| `new(entries: Vec<CorpusEntry>)` | `Corpus` | Constructor |
| `total_words(&self)` | `usize` | Non-punctuation tokens across every entry |
| `passive_ratio(&self)` | `f64` | Passive sentences over total sentences across every entry. 0.0 with no sentences |
| `mean_readability(&self)` | `f64` | Mean of the paragraphs that carry a `readability_grade`, across every entry. 0.0 when none do |

## DocumentError

| Field | Type | Contents |
|---|---|---|
| `path` | `Option<PathBuf>` | Where the failure occurred, `None` for in-memory text |
| `error` | `Error` | What went wrong |

One per-document failure. `Display` prints `path: error` when a path exists and the bare error otherwise; `source()` returns the wrapped `Error`. `DocumentError::new(path, error)` is the constructor. Not `Serialize` and not `Clone`, because `Error` wraps `std::io::Error`.

## CorpusResult

| Field | Type | Contents |
|---|---|---|
| `corpus` | `Corpus` | Every successfully analyzed document |
| `errors` | `Vec<DocumentError>` | Every per-document failure, in consumption order |

The partition of a per-document result stream: entries plus errors equals documents consumed. `collect()` is its constructor, via `FromIterator<Result<CorpusEntry, DocumentError>>`, which is how a stream from `Engine::analyze` becomes one value. Inherits `DocumentError`'s serialization gap, so it does not cross the language boundary today.

## Constants, aliases, and the error type

| Item | Definition | Role |
|---|---|---|
| `MAX_INPUT_BYTES` | `usize`, `8 * 1024 * 1024` (8,388,608) | Byte cap the pipeline applies to text input |
| `Result<T>` | `std::result::Result<T, Error>` | Return type of every fallible library function |
| `Error` | enum, `#[non_exhaustive]` | Every failure the library returns. See [Errors](errors.md) |

## What crosses the language boundary

The Python surface serializes values with pythonize. Fields have a serde representation and cross with their names unchanged. Methods have none, so there is nothing for them to cross with.

<svg class="mx-ffi" role="img" aria-label="Document fields cross the FFI boundary into a Python dict; Document methods stop at the boundary" viewBox="0 0 720 300" width="720" height="300" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
<title>Fields cross the FFI boundary, methods stop at it</title>
<style>
.mx-ffi text{fill:currentColor}
.mx-ffi .box{fill:none;stroke:currentColor;opacity:.35;stroke-width:1px}
.mx-ffi .ty{font-size:12.5px;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-ffi .mem{font-size:10px;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;opacity:.85}
.mx-ffi .hd{font-size:9.5px;font-family:inherit;opacity:.55}
.mx-ffi .nt{font-size:10px;font-family:inherit;opacity:.6}
.mx-ffi .wall{stroke:currentColor;opacity:.45;stroke-width:1.4px}
.mx-ffi .cross{stroke:currentColor;opacity:.75;stroke-width:1.2px}
.mx-ffi .stop{stroke:currentColor;opacity:.4;stroke-width:1.2px;stroke-dasharray:3 3}
.mx-ffi .bar{stroke:currentColor;opacity:.85;stroke-width:2.4px}
.mx-ffi marker path{fill:currentColor;opacity:.75}
</style>
<defs><marker id="mx-ffi-a" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto"><path d="M0,0 L8,4 L0,8 z"/></marker></defs>
<text class="hd" x="16" y="30">Rust</text>
<text class="hd" x="452" y="30">Python</text>
<rect class="box" x="16" y="40" width="272" height="232" rx="5"/>
<rect class="box" x="452" y="40" width="252" height="124" rx="5"/>
<text class="ty" x="30" y="64">Document</text>
<text class="ty" x="466" y="64">Document</text>
<text class="hd" x="30" y="86">stored fields</text>
<text class="hd" x="466" y="86">dict keys</text>
<text class="mem" x="30" y="104">sections</text>
<text class="mem" x="30" y="122">vocabulary_ttr</text>
<text class="mem" x="30" y="140">nominalization_ratio</text>
<text class="mem" x="466" y="104">sections</text>
<text class="mem" x="466" y="122">vocabulary_ttr</text>
<text class="mem" x="466" y="140">nominalization_ratio</text>
<text class="hd" x="30" y="172">methods</text>
<text class="mem" x="30" y="190">passive_ratio()</text>
<text class="mem" x="30" y="208">mean_sentence_length()</text>
<text class="mem" x="30" y="226">sentence_length_std()</text>
<text class="mem" x="30" y="244">total_words()</text>
<line class="cross" x1="294" y1="100" x2="444" y2="100" marker-end="url(#mx-ffi-a)"/>
<line class="cross" x1="294" y1="118" x2="444" y2="118" marker-end="url(#mx-ffi-a)"/>
<line class="cross" x1="294" y1="136" x2="444" y2="136" marker-end="url(#mx-ffi-a)"/>
<line class="stop" x1="176" y1="186" x2="386" y2="186"/>
<line class="stop" x1="176" y1="204" x2="386" y2="204"/>
<line class="stop" x1="176" y1="222" x2="386" y2="222"/>
<line class="stop" x1="176" y1="240" x2="386" y2="240"/>
<line class="bar" x1="388" y1="179" x2="388" y2="193"/>
<line class="bar" x1="388" y1="197" x2="388" y2="211"/>
<line class="bar" x1="388" y1="215" x2="388" y2="229"/>
<line class="bar" x1="388" y1="233" x2="388" y2="247"/>
<text class="hd" x="401" y="14" text-anchor="middle">FFI boundary</text>
<text class="hd" x="401" y="26" text-anchor="middle">pythonize + serde</text>
<line class="wall" x1="398" y1="32" x2="398" y2="288"/>
<line class="wall" x1="404" y1="32" x2="404" y2="288"/>
<text class="nt" x="462" y="202">a Python caller computes these</text>
<text class="nt" x="462" y="218">from the sections it already has</text>
</svg>

`Token::feat`, `Document::passive_ratio`, `Corpus::total_words`, `Sentence::tree_depth`, and every other method in the tables above are available to Rust callers only.

| Rust type | Python shape |
|---|---|
| `Token` | `Token` |
| `Sentence` | `Sentence` |
| `Paragraph` | `Paragraph` |
| `Section` | `Section` |
| `Document` | `Document` |
| `ScoredSentence` | `ScoredSentence` |
| `Keyphrase` | `Keyphrase` |
| `Format`, `RawDocument`, `CorpusEntry`, `Corpus` | no Python shape |

The Python shapes are `TypedDict` declarations available at runtime. They are declared in `matra.types` and re-exported from the package root, so either import works:

```python
from matra.types import Document, Keyphrase, ScoredSentence
from matra import Document, Keyphrase, ScoredSentence
```

Field names and nesting match the Rust types exactly. `Option<f64>` becomes `float | None`; `Option<String>` becomes `str | None`; `Vec<T>` becomes `list[T]`; `usize` becomes `int`.
