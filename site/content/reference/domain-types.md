# Domain types

`matra::domain` holds every type the library hands back. The module depends on `serde`, `thiserror`, and the standard library, and on nothing else, so a caller can depend on these types without inheriting a C++ toolchain.

It is not a copy of the generated item-level documentation. For that, run `cargo doc --no-deps --all-features`.

## The type graph

Five types nest, each owning the next. Nothing is owned twice.

<svg class="mx-own" role="img" aria-label="Nested containment from Document down to Token, with stored fields and computed methods for each type" viewBox="0 0 720 362" width="720" height="362" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
<title>Containment from Document to Token, stored fields against computed methods</title>
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
<line class="rule" x1="452" y1="28" x2="452" y2="350"/>
<rect class="box" x="32" y="36" width="152" height="298" rx="4"/>
<rect class="box" x="44" y="104" width="137" height="224" rx="4"/>
<rect class="box" x="56" y="148" width="122" height="174" rx="4"/>
<rect class="box" x="68" y="202" width="107" height="114" rx="4"/>
<rect class="box" x="80" y="270" width="92" height="40" rx="4"/>
<text class="ty" x="41" y="55">Document</text>
<text class="ty" x="53" y="123">Section</text>
<text class="ty" x="65" y="167">Paragraph</text>
<text class="ty" x="77" y="221">Sentence</text>
<text class="ty" x="89" y="289">Token</text>
<text class="mem" x="206" y="55">provenance · sections ·</text>
<text class="mem" x="206" y="67">metrics (DocumentMetrics) ·</text>
<text class="mem" x="206" y="79">skipped</text>
<text class="mem" x="206" y="123">heading · level · span ·</text>
<text class="mem" x="206" y="135">paragraphs</text>
<text class="mem" x="206" y="167">span · text · kind · sentences ·</text>
<text class="mem" x="206" y="179">metrics (ParagraphMetrics)</text>
<text class="mem" x="462" y="167">word_count · sentence_count</text>
<text class="mem" x="206" y="221">span · text · tokens · word_count ·</text>
<text class="mem" x="206" y="233">tree_depth · passive ·</text>
<text class="mem" x="206" y="245">six structural fields</text>
<text class="mem" x="462" y="221">root_token · head_of · children_of ·</text>
<text class="mem" x="462" y="233">subtree · content_tokens</text>
<text class="mem" x="206" y="289">the ten CoNLL-U columns · span ·</text>
<text class="mem" x="206" y="301">is_punct</text>
</svg>

The left column is what the pipeline writes into the struct. The right column is recomputed from the tree every time you ask, and an empty right column means the type only holds. So there is no second list of paragraphs anywhere in `Document`: the flat iterators walk the section tree. Every number a reader needs from a document is stored, in `DocumentMetrics` and `ParagraphMetrics` and in a sentence's `word_count`, `tree_depth` and `passive`, because a field crosses the language boundary and a method does not.

The derived structural types (`Negation`, `Modal`, `ClausalComplement`, `RootAdverbial`, `RootMood`, `HearstPair` with its `HearstSpan` and `HearstPattern`) sit inside the nest, owned by `Sentence`, and the metric records `DocumentMetrics` and `ParagraphMetrics` are owned by `Document` and `Paragraph`. `Provenance`, with its `Identity` and `ProvenanceSource`, is owned by `Document`, and so is `skipped`, a list of `Skipped` records, each a `SkippedKind` and a `Span`. Every heading, paragraph, sentence and token holds a `Span`, and a paragraph its `ParagraphKind`. The other types sit outside it:

| Type | Role |
|---|---|
| `Block` | a heading or paragraph a caller supplies, the input to `Input::blocks`; never returned |
| `Summary`, `SummarySentence` | output of `summarize` and `summarize_with` |
| `Keyphrases`, `Keyphrase` | output of `keyphrases` and `keyphrases_with` |
| `Clusters`, `Cluster`, `SimilarityEdge` | output of `clusters`, described in [Semantic clusters](semantic-clusters.md) |
| `Parameters` | what shaped an outcome, recorded in its `provenance` |
| `Format` | which reader a document needs |
| `Error` | every failure the library can return |
| `ErrorDetails`, `DetailValue` | every dynamic value in an error's message, by name: what `Error::details` returns |
| `DocumentError` | one document's failure, an `Err` item of `analyze_all`'s stream |
| `Embedding` | one vector from an `Embedder`, the input to clustering |
| `ProvisionNotice` | what a model fetch is about to download, handed to the `_with_notice` constructors' callback, described in [Errors](errors.md#the-first-run-says-so) |

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
| `DocumentMetrics`, `ParagraphMetrics` | `Debug`, `Clone`, `Default`, `PartialEq`, `Serialize`, `Deserialize` | yes |
| `Provenance` | `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` | yes |
| `Parameters` | `Debug`, `Clone`, `Default`, `PartialEq`, `Serialize`, `Deserialize` | yes |
| `Identity`, `ProvenanceSource` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` | yes |
| `Summary`, `SummarySentence`, `Keyphrases`, `Keyphrase` | `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` | yes |
| `Clusters`, `Cluster`, `SimilarityEdge` | `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` | yes |
| `Format` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` | yes |
| `Span` | `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` | yes |
| `Skipped`, `SkippedKind`, `ParagraphKind` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` | yes |
| `Block` | `Debug`, `Clone`, `PartialEq`, `Eq` | no |
| `Error` | `Debug`, `thiserror::Error`; `Serialize` by hand | yes |
| `ErrorDetails` | `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`; `Serialize` by hand | no |
| `DetailValue` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize` | yes |
| `Negation`, `Modal`, `ClausalComplement`, `RootAdverbial` | `Debug`, `Clone`, `Serialize`, `Deserialize` | yes |
| `RootMood` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` | yes |
| `HearstPair`, `HearstSpan` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` | yes |
| `HearstPattern` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, `Deserialize` | yes |
| `DocumentError` | `Debug`; `Serialize` by hand | yes |
| `Embedding` | `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` | no |
| `ProvisionNotice` | `Debug`, `Clone`, `PartialEq`, `Eq` | yes |

`Block` is the one input type. It has no serde derives and no public fields, so it needs no `#[non_exhaustive]`: a caller builds one with `Block::heading` or `Block::paragraph` and only matra reads it. Its JSON form is the `blocks` format, whose schema is `spec/schema/v2/blocks.schema.json`.

No type in the module derives `Hash`, and the parse tree derives no equality: two `Token`, `Sentence`, `Paragraph`, `Section` or `Document` values cannot be compared with `==`. `Format` derives `PartialEq` and `Eq`, which is what the reader table keys on, and the metric records, the provenance types, `Span`, `Skipped`, the kind enums, `Block`, `RootMood`, the Hearst types, the outcome types, `Embedding`, `ProvisionNotice`, `ErrorDetails` and `DetailValue` derive `PartialEq`. `Error` has no `Clone`, no equality and no serde derives; its `Serialize` is written by hand, as `DocumentError`'s and `ErrorDetails`' are. `ErrorDetails` has no public fields, like `Block`, so it needs no `#[non_exhaustive]`. `Embedding` is the one public type with public fields and without `#[non_exhaustive]`: on a tuple struct the attribute would make the constructor private, and an `Embedder` implemented outside the crate has to build one.

`#[non_exhaustive]` has two effects outside the crate. Struct literal syntax is unavailable, so you build values through the associated constructors listed below. The outcome types have no public constructor: matra produces them, and serde reads them back. Matches on `Format`, `ParagraphKind`, `SkippedKind` and `Error` need a catch-all arm, because variants can be added in a minor release.

Every field on these types is `pub` and readable. `TokenBuilder` and `Block` are the exceptions: their fields are private, and you reach them through the builder's setters and the block's constructors.

## Token

One parsed token carrying the full CoNLL-U annotation set plus one derived flag.

| Field | Type | CoNLL-U column | Contents |
|---|---|---|---|
| `id` | `usize` | 1 | Position within the sentence, 1-based |
| `span` | `Span` | none | Where the surface form lies in the analysed text, in UTF-8 bytes |
| `text` | `String` | 2 | Surface form |
| `lemma` | `String` | 3 | Dictionary form |
| `pos` | `String` | 4 | Universal POS tag, for example `NOUN`, `VERB`, `ADJ` |
| `xpos` | `String` | 5 | Language-specific POS tag |
| `features` | `BTreeMap<String, String>` | 6 | Morphological features as a map from feature name to value |
| `head` | `usize` | 7 | Id of the governing token; `0` marks the root |
| `dep` | `String` | 8 | Dependency relation to the head, for example `nsubj`, `obj`, `aux:pass` |
| `deps` | `String` | 9 | Enhanced dependency graph |
| `misc` | `String` | 10 | Miscellaneous annotations, for example `SpaceAfter=No` |
| `is_punct` | `bool` | derived | Whether the token is punctuation |

Provider-specific notes for the UDPipe adapter, the one adapter that ships:

- `is_punct` is set when the UPOS tag is `PUNCT`.
- `deps` is always the string `_`, the CoNLL-U empty marker. The underlying `udpipe-rs` binding does not surface column 9.
- `xpos` and `misc` carry whatever the model emits, and can be `_` or empty. `features` is converted from the column: `Mood=Ind|Tense=Pres` becomes two entries, a multi-valued feature (`Case=Nom,Acc`) keeps its value unsplit, and `_` becomes an empty map.

A token id or head that does not fit in `usize` is rejected at the adapter with `Error::ParseFailed` rather than being silently coerced.

`span` comes from aligning each form with the paragraph text the parser was given, then mapping it into the analysed text. A form the adapter cannot find where the text says it should be is `Error::ParseFailed` naming the token, never a guessed span. In a markdown paragraph whose markup the reader removed, a token's span can cover markup inside it (`foo**bar**` is one token from the `f` to the last `r`), and a token read from an escape or an entity (`&amp;`) covers all of it.

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

`Token::builder(id, text, lemma, pos, head, dep) -> TokenBuilder` takes the six CoNLL-U essentials. `TokenBuilder` carries one setter per optional field (`xpos`, `deps`, `misc`, each taking `String`; `feats`, taking the CoNLL-U column 6 string and converting it into `features`; `is_punct` taking `bool`; `span` taking a `Span`), each consuming and returning the builder. `build()` returns the `Token`. Unset string fields default to empty, `features` to an empty map, `is_punct` to `false`, and `span` to the empty span at 0.

### Methods

| Signature | Returns | Behavior |
|---|---|---|
| `feat(&self, key: &str)` | `Option<&str>` | The value of one feature in `features`, by exact name. `None` when the feature is absent |

`feat("Mood")` returns `Some("Ind")` when the token carries `Mood=Ind`. The value is what the provider emitted, so a multi-valued feature (`Case=Nom,Acc`) comes back unsplit: matra does not normalise it. A token whose column 6 was `_` or empty has no features, so every lookup on it returns `None`.

`feat` is Rust-only by design. `features` crosses FFI as a map, so a lookup over it adds no information to the wire; a Python or TypeScript caller indexes the same map ([RFC-0009](https://github.com/mox-labs/matra/blob/main/blueprints/legacy/rfcs/0009-feats-lookup-accessor.md)).

## Sentence

| Field | Type | Contents |
|---|---|---|
| `span` | `Span` | Where the sentence lies in the analysed text: from its first token's start to its last token's end, derived at construction |
| `text` | `String` | Sentence text as the NLP provider reports it, normalized, not a slice of the input |
| `tokens` | `Vec<Token>` | Tokens in ascending `id` order |
| `word_count` | `usize` | Non-punctuation tokens, derived at construction |
| `tree_depth` | `Option<usize>` | Longest path from any token to the root, the root at depth 0, derived at construction. `None` when the head references form a cycle |
| `passive` | `bool` | Whether any token's `dep` is `nsubj:pass`, `nsubjpass`, or `aux:pass`, derived at construction |
| `negations` | `Vec<Negation>` | Negation cues, derived at construction |
| `modals` | `Vec<Modal>` | Modal auxiliaries, derived at construction |
| `clausal_complements` | `Vec<ClausalComplement>` | Verbs governing a clausal complement (`ccomp`), each with its subject when the parse has one, derived at construction |
| `root_adverbials` | `Vec<RootAdverbial>` | Adverbial modifiers attached to the root, derived at construction |
| `hearst_pairs` | `Vec<HearstPair>` | Candidate hypernymy pairs from the six Hearst (1992) patterns, filled by the pipeline at the annotate stage |
| `root_mood` | `Option<RootMood>` | The root clause's `Mood` feature with the root's token id, derived at construction. `None` when the root clause carries no mood |

Invariants that downstream code relies on, and that a hand-built `Sentence` and every `NlpProvider` are expected to uphold: tokens are id-sorted; exactly one token has `head == 0`; every other `head` names a token in the same sentence; following `head` never loops. Nothing validates them, and the tree walks stay safe when the last one breaks (see `tree_depth` below).

`tree_depth` is computed in time linear in token count with a visited set, and is `None` when the head references form a cycle, which reports a malformed parse as a value a reader must handle instead of truncating it or returning a number that reads as a very deep tree. An empty sentence has depth `Some(0)`. `word_count` counts tokens the NLP provider identified, minus punctuation. It is not a whitespace split, and the two counts differ on the same text.

The UDPipe adapter builds `text` by joining token surface forms, inserting a space unless the preceding token carries `SpaceAfter=No` in `misc`. The string is a reconstruction, not a slice of the input, so whitespace can differ from the source text; `span` is where the sentence came from.

### Derived structural fields

The fields after `tokens` are computed once from the dependency graph and serialized with the sentence, so every language binding and the CLI's JSON read one Rust detection as data ([RFC-0008](https://github.com/mox-labs/matra/blob/main/blueprints/legacy/rfcs/0008-structural-primitives-are-fields.md)). Most are computed at `Sentence::new`; `hearst_pairs` is the exception, filled by the pipeline at the annotate stage because its detector lives in the `hearst` module outside the domain, so a hand-built `Sentence` carries an empty vector until the caller runs `hearst::hypernymy_pairs`. Each field reports structure only; the reading is the caller's.

`Negation` reports one cue by token id: `cue_id`, `cue_lemma` (`not`, `never`, `no`, `neither`, `nor`), and `head_id`, the token the cue attaches to. A cue fires only on its carrying relation (`advmod`, `det`, `cc`), so `nothing` as a subject does not fire and tokenizer-split `cannot` fires exactly once, on the `not` token. Whether the negation reverses the sentence's claim is not matra's call.

`Modal` reports one modal auxiliary by token id: `aux_id`, `aux_lemma`, and `head_id`, the token the auxiliary attaches to (`0` when the modal is itself the root). The closed class is the ten lemmas the UD English treebank tags `MD`: `can`, `could`, `may`, `might`, `must`, `ought`, `shall`, `should`, `will`, `would`, matched by lemma on the `aux` relation or the `AUX` part of speech. The part-of-speech arm catches the shapes the model does not park on `aux`: VP ellipsis promotes the modal to root ("Yes, you should.") and coordinated auxiliaries parse as root plus `aux` or `conj` ("He can and will succeed."). Precision holds because the model's only other `AUX` lemmas (`be`, `have`, `do`, `get`) are outside the class. Contracted forms fire on their lemma (`wo` in `won't` lemmatizes to `will`), the noun `will` never fires because it never carries `aux` or `AUX`, and a multi-auxiliary chain (`might have been done`) reports its modal with `head_id` on the content verb. English modals are ambiguous across readings, and resolving `must` into obligation or inference takes context matra does not model, so no epistemic, deontic, or dynamic category is assigned: the auxiliary and its arc are reported, the reading is the caller's.

`RootMood` reports the grammatical mood of the root clause: `root_id`, the root's token id, and `mood`, the `Mood` feature as the model emitted it (`Ind`, `Imp`, and an open set beyond). The feature is read off the root token or, when the root carries none, off its first `cop`, `aux`, or `aux:pass` child that does, because the model parks finiteness on the auxiliary in copular ("The sky is blue."), do-support ("He did leave.") and passive clauses. A modal carries `VerbForm=Fin` and no mood, so a clause whose root a modal governs ("You must complete the form.") usually has no root mood; whether a modal governs the root is the separate fact in `modals`. It reads the root clause only, so a mood in a subordinate clause does not reach it. The bare assertoric form, an indicative root no modal governs, is `mood == "Ind"` with no modal whose `head_id` is `root_id`.

`ClausalComplement` reports one verb governing a clausal complement by token id: `verb_id` and `verb_lemma` for the verb, `ccomp_id` for the head of the complement, and `subject_id` plus `subject_lemma` for the verb's subject (`nsubj`, `nsubj:pass`) when the parse has one in the same sentence. The construction fires for every verb that fills it ("Smith reported that the effect vanished.", "These results suggest that the mechanism is shared.", "We show that the method works." all match), because verbs taking a clausal complement are an open class and any list matra shipped would be incomplete while looking authoritative. Which verb lemmas count as reporting or evidential is the caller's lexicon, supplied as a parameter to `clausal_complements_in` or applied to the field directly in any language. A `ccomp` under a non-verb head ("I am sure that it works." parses the complement under the adjective) is outside the construction by decision. The subject is optional by observed fact: UDPipe splits "Smith et al. reported that ..." into two sentences at the period in "et al.", stranding the attribution in the previous sentence while the verb keeps its `ccomp`. That upstream segmentation defect is recorded here and in the test suite rather than papered over.

`RootAdverbial` reports one adverbial modifier attached to the root by token id: `adv_id` and `adv_lemma`. The root-attached `advmod` arc is where sentence-scope adverbs land ("Reportedly, the deal closed."), but the dependency graph does not distinguish sentence scope from manner ("The team quickly shipped." puts `quickly` on the same arc), so every root-attached adverbial is reported and the caller's lexicon selects the evidential ones, via `root_adverbials_in` or a filter over the field. Negation particles on the same arc ("not" in "The plan is not ready.") appear in both `negations` and `root_adverbials`; both report arcs, neither assigns a reading, and the overlap is intentional.

`HearstPair` reports one candidate hypernymy pair from the six lexico-syntactic patterns of Hearst (1992), detected as dependency-arc patterns rather than regex over surface text: `NP such as NP`, `such NP as NP`, `NP, including NP`, `NP, especially NP`, `NP and other NP`, `NP or other NP`. The `pattern` field carries a `HearstPattern` tag naming the construction that matched (`such_as`, `such_np_as`, `including`, `especially`, `and_other`, `or_other` on the wire), and `hypernym` and `hyponym` each carry a `HearstSpan`: `head_id` and `head_lemma` for the noun phrase head, plus `first_id` and `last_id` for the contiguous token range covering the head and its adjacent nominal modifiers, with the pattern's own marker words (`such`, `other`) excluded, so traceability holds against `tokens`. Precision is favoured over recall: each detector requires the full verified arc shape, so a clausal coordination ("She invited John, and other guests brought gifts."), a definite contrastive coordination ("The teacher and the other students laughed."), or an emphasized plain coordination ("France and especially Spain") does not fire, and a sentence the model misparses is missed rather than misreported. The pair is a candidate by design: matra reports that the sentence used a construction which conventionally signals hypernymy, and does not build a taxonomy or assert the relation is true.

### Methods

Most methods below are walks over the `head` and `dep` columns; the two `_in` filters are views over the derived fields against a caller-supplied lexicon. [Concepts](../explanation/concepts.md#the-document-tree) draws one parsed sentence as the tree these walk.

| Signature | Returns | Behavior |
|---|---|---|
| `new(text: String, tokens: Vec<Token>)` | `Sentence` | Constructor. Derives `span` and the structural fields from `tokens`. Does not validate the invariants |
| `content_tokens(&self)` | `Vec<&Token>` | Tokens where `is_punct` is false |
| `root_token(&self)` | `Option<&Token>` | The token with `head == 0`, if there is one |
| `children_of(&self, id: usize)` | `Vec<&Token>` | Tokens whose `head` equals `id`. Empty for an unknown id |
| `head_of(&self, id: usize)` | `Option<&Token>` | The governing token. `None` for the root and for an unknown id |
| `subtree(&self, id: usize)` | `Vec<&Token>` | The token and all its descendants, sorted by `id`. Empty for an unknown id |
| `clausal_complements_in(&self, lexicon: &[&str])` | `Vec<&ClausalComplement>` | Clausal complements whose `verb_lemma` is in the caller-supplied lexicon |
| `root_adverbials_in(&self, lexicon: &[&str])` | `Vec<&RootAdverbial>` | Root adverbials whose `adv_lemma` is in the caller-supplied lexicon |

`subtree` carries a visited set, so it terminates on cyclic input.

## Paragraph

| Field | Type | Contents |
|---|---|---|
| `span` | `Span` | Where the paragraph lies in the analysed text, trailing whitespace excluded. For markdown it covers the quote and list markers on its lines |
| `text` | `String` | The text the parser reads, normalized: for markdown the inline text with the markup removed (emphasis, link and image syntax, code span backticks, quote and list markers), soft line breaks as `\n`, escapes and entities decoded; for plain text and block input a verbatim slice |
| `kind` | `ParagraphKind` | `Paragraph`, `Quote` (inside a block quote at any depth) or `ListItem`; `paragraph`, `quote`, `list_item` on the wire |
| `sentences` | `Vec<Sentence>` | Sentences from parsing this paragraph |
| `metrics` | `ParagraphMetrics` | The paragraph's three measures |

`ParagraphMetrics` holds the three:

| Field | Type | Contents |
|---|---|---|
| `readability_grade` | `Option<f64>` | Flesch-Kincaid grade level, when the paragraph holds more than ten words |
| `lexical_density` | `Option<f64>` | Content-word ratio, 0.0 to 1.0, when the paragraph holds a word |
| `compression_ratio` | `Option<f64>` | Brotli compressed size over original size, when the paragraph holds more than fifty words |

Each slot holds `None` until `Engine::compose` fills it, and stays `None` when the paragraph does not meet the metric's applicability condition; no outer `Option` wraps the record, because each slot gates on its own.

`Engine::annotate` skips quotes when it parses, so a paragraph whose `kind` is `ParagraphKind::Quote` reaches `compose` with no sentences and keeps all three slots at `None`.

### Methods

| Signature | Returns | Behavior |
|---|---|---|
| `new(text: String, kind: ParagraphKind, span: Span)` | `Paragraph` | Constructor. Empty sentences, every metric `None` |
| `word_count(&self)` | `usize` | Non-punctuation tokens summed across this paragraph's sentences |
| `sentence_count(&self)` | `usize` | Number of sentences |

## Section

| Field | Type | Contents |
|---|---|---|
| `heading` | `Option<String>` | Heading text, its markup removed |
| `level` | `usize` | Heading depth |
| `span` | `Option<Span>` | Where the heading lies in the analysed text, markers included; `None` with no heading |
| `paragraphs` | `Vec<Paragraph>` | Paragraphs in document order |

The markdown reader sets `level` to the heading's level (the number of `#` characters, or 1 and 2 for a setext heading) and `heading` to its text. Content before the first heading becomes a section with `heading: None` and `level: 0`. Plain text produces at most one section, always with `heading: None` and `level: 0`. Block input opens a section at each `Block::heading`, at the level it names.

`Section::new(heading: Option<String>, level: usize, span: Option<Span>, paragraphs: Vec<Paragraph>) -> Section` is the constructor. There are no other methods.

## Document

The output of the pipeline.

| Field | Type | Contents |
|---|---|---|
| `provenance` | `Provenance` | Where the document came from and what produced it, recorded by `Engine::annotate` |
| `sections` | `Vec<Section>` | The section tree, which owns every paragraph, sentence, and token |
| `metrics` | `DocumentMetrics` | The document's aggregates, filled by `Engine::compose` |
| `skipped` | `Vec<Skipped>` | What the reader saw and did not analyse, in document order |

`DocumentMetrics` holds what the command line's table prints and what a binding would otherwise rebuild:

| Field | Type | Contents |
|---|---|---|
| `total_words` | `usize` | Non-punctuation tokens across every sentence |
| `total_sentences` | `usize` | Sentences across every paragraph |
| `paragraph_count` | `usize` | Paragraphs, quotes included |
| `mean_sentence_length` | `Option<f64>` | `total_words` over `total_sentences`. `None` with no sentences |
| `sentence_length_std` | `Option<f64>` | Sample standard deviation of sentence length in words. `None` with fewer than two sentences, where it is undefined |
| `vocabulary_ttr` | `Option<f64>` | Type-token ratio over lemmas. Falls as text grows, so documents of different lengths are not comparable on it |
| `nominalization_ratio` | `Option<f64>` | Share of nominalizing nouns |
| `passive_ratio` | `Option<f64>` | Passive sentences over total sentences. `None` with no sentences |

Before the document is measured every count is 0 and every ratio `None`; `None` after it means the document did not meet the metric's condition, never a computed zero. `Engine::analyze` always measures, so this holds for a document you build by hand with `Document::new`.

`Provenance` holds `matra`, the version that produced the document; `schema`, the schema version as `"major.minor"` (`SCHEMA_VERSION`, `"2.0"`), which appears nowhere else; `parser`, the parser's `Identity`; `format`, the `Format` the input was read as; and `source`, a `ProvenanceSource` with the input's `path` (`None` for anything but a file), and the length in `bytes` and the `sha256` of the analysed text every span points into, computed once at annotation. For markdown and plain text that is the input; for block input, the blocks joined. On a document, `method` and `parameters` are `None` and absent from the JSON; an outcome derived from the document carries a copy of its provenance with both set (see [Summary](#summary)). There is no timestamp, so two analyses of the same input by the same build are identical. `Identity` names a model-backed component: `name` (`udpipe`), `model` (`english-ewt-ud-2.5-191206`), `sha256` of the model artifact, and `license`. The UDPipe adapter derives it from the bytes it loaded, so a model matra does not pin reports its digest with `model` and `license` set to `None`; an embedder reports one in the same shape (`model2vec` for `Model2Vec`), recorded in a cluster result's `parameters.embedder`. `Provenance::new(parser, format, source)` fills `matra` and `schema` from the build.

### Methods

| Signature | Returns | Behavior |
|---|---|---|
| `new(provenance: Provenance, sections: Vec<Section>, skipped: Vec<Skipped>)` | `Document` | Constructor. An empty metric record |
| `paragraphs(&self)` | `impl Iterator<Item = &Paragraph>` | Every paragraph, in document order |
| `paragraphs_mut(&mut self)` | `impl Iterator<Item = &mut Paragraph>` | The mutable form, which the engine uses as it parses and measures |
| `sentences(&self)` | `impl Iterator<Item = &Sentence>` | Every sentence across every paragraph |
| `tokens(&self)` | `impl Iterator<Item = &Token>` | Every token across every sentence |

## Summary

What `summarize` and `summarize_with` return. The selector is `matra::SummaryMethod`, at the crate root (`TfIdf`, the default, and `TextRank`, `#[non_exhaustive]`).

| Field | Type | Contents |
|---|---|---|
| `provenance` | `Provenance` | The document's, with `method` (`tfidf` or `textrank`, an open set) and `parameters` (`n`) |
| `sentences` | `Vec<SummarySentence>` | At most `n` sentences, in document order |

`SummarySentence` is one sentence a summary keeps:

| Field | Type | Contents |
|---|---|---|
| `index` | `usize` | The sentence's position among the document's sentences (`Document::sentences`), counting from 0 |
| `span` | `Span` | Where it lies in the analysed text, as on the `Sentence` |
| `text` | `String` | Its text, as on the `Sentence` |
| `score` | `f64` | The method's score, higher is more representative |

Both summarizers select the top `n` by score and then return them in ascending `index`, so the result reads in document order while `score` carries the ranking. A score compares only with another from the same method.

`Parameters` records what shaped an outcome, each field present only when the method takes it: `n` (`Option<usize>`) for a summary and keyphrases, `threshold` (`Option<f64>`) and `embedder` (`Option<Identity>`, the identity the embedder reports, the same shape as the parser's) for clusters. In JSON an absent parameter is an absent key, so a summary's is `{"n": 3}`.

## Keyphrases

What `keyphrases` and `keyphrases_with` return. The selector is `matra::KeyphraseMethod` (`Rake`, the default, and `Yake`, `#[non_exhaustive]`).

| Field | Type | Contents |
|---|---|---|
| `provenance` | `Provenance` | The document's, with `method` (`rake` or `yake`, an open set) and `parameters` (`n`) |
| `phrases` | `Vec<Keyphrase>` | At most `n` phrases, highest score first; equal scores in the order the phrases are first met, the same on every run |

`Keyphrase` is one ranked phrase:

| Field | Type | Contents |
|---|---|---|
| `phrase` | `String` | The phrase, built from lowercased lemmas joined by single spaces |
| `score` | `f64` | Relevance score, higher is more relevant |
| `occurrences` | `Vec<Span>` | Every stretch of the analysed text it was found in, in document order, from its first word's start to its last word's end |

## Clusters

What `clusters` returns: `provenance` (the document's, with `method` `cosine` and `parameters` `threshold` and `embedder`), `clusters` (`Vec<Cluster>`, each `members`, sentence indices) and `edges` (`Vec<SimilarityEdge>`, each `a`, `b` and `score: f64`). [Semantic clusters](semantic-clusters.md) describes each field and what co-membership does and does not mean.

## Format

Which reader a document needs.

| Variant | Wire name | Assigned to |
|---|---|---|
| `Markdown` | `markdown` | Extension `.md` or `.markdown` |
| `PlainText` | `plain_text` | Every other extension, and a file with no extension |
| `Blocks` | `blocks` | No extension: a caller names it, and `Input::blocks` takes it |
| `Pdf` | `pdf` | Extension `.pdf`, or bytes opening with `%PDF` |
| `Docx` | `docx` | Extension `.docx`, or a zip archive holding `[Content_Types].xml` |

`Format::from_path(path: impl AsRef<Path>) -> Format` is the extension column, and it is the only copy of it. Reading a file with `Input::file` applies it, and the command line reads it for the name given to `--stdin-filename` when `--format` names none, so a name means the same thing on both routes. When the bytes are read, a PDF or DOCX signature outranks both the extension and a format the caller names.

`Pdf` and `Docx` have no reader, so analyzing such a document returns `Error::UnsupportedFormat` with that format. `Input::text` and `Input::file_as` take the format as an argument; `Input::bytes` takes one optionally; `Input::file` detects one. `Format` derives `PartialEq` and `Eq`, which is what the reader table keys on.

## Block

A heading or a paragraph a caller supplies, for a document matra has no reader for. Input only: built by `Block::heading(level: u8, text: impl Into<String>)` and `Block::paragraph(text: impl Into<String>)`, handed to `Input::blocks`, and never returned. It has no public fields.

The analysed text of block input is the blocks' texts joined by a blank line (`"\n\n"`), in order, and every span in the result points into it. A heading opens a section at its level; a level of 0 is refused with `Error::InvalidInput` when the input is analysed. The same blocks as JSON are the `blocks` format: an array of `{"kind": "heading", "level": 1, "text": "..."}` and `{"kind": "paragraph", "text": "..."}` objects, with an unknown `kind` or field refused by name.

## Span

| Field | Type | Contents |
|---|---|---|
| `byte_start` | `usize` | The first byte |
| `byte_end` | `usize` | One past the last byte |

A stretch of the analysed text in UTF-8 bytes, so `&text[span.byte_start..span.byte_end]` is the stretch. Bytes rather than characters: a caller in a language that indexes strings by character converts. The unit is in the field names because a byte span used to slice a character-indexed string, a Python `str`, is right on ASCII and wrong, with no error, after the first character outside it. `Span::new(byte_start, byte_end)` is the constructor.

## Skipped

| Field | Type | Contents |
|---|---|---|
| `kind` | `SkippedKind` | What the construct is |
| `span` | `Span` | Where it lies in the analysed text, trailing whitespace excluded |

One construct the reader saw and did not analyse. `SkippedKind` names it: `Table`, `Code`, `FrontMatter`, `Html`, `ThematicBreak`, `LinkDefinition` (`table`, `code`, `front_matter`, `html`, `thematic_break`, `link_definition` on the wire), a set a reader treats as open. Skipping is specified behaviour beside a correct result, not an error. `Skipped::new(kind, span)` is the constructor.

Together with the heading and paragraph spans, `skipped` accounts for the input: every byte of a markdown or plain-text input that is not whitespace lies in exactly one heading, paragraph or skipped span.

## DocumentError

| Field | Type | Contents |
|---|---|---|
| `path` | `Option<PathBuf>` | Where the failure occurred, `None` for input that is not a file |
| `error` | `Error` | What went wrong |

One per-document failure: the `Err` item `Engine::analyze_all` yields in the failed input's place, so documents plus errors equals inputs consumed. `Display` prints `path: error` when a path exists and the bare error otherwise; `source()` returns the wrapped `Error`. `DocumentError::new(path, error)` is the constructor. Not `Clone`, because `Error` wraps `std::io::Error`.

It serializes as its error's form plus the path, `{"path", "kind", "message", "details"}`, with `path` null for input that is not a file and written lossily where it is not UTF-8. The JSON Schema is `spec/schema/v2/error.schema.json`.

matra collects nothing across documents: a stream is the collection, and how to combine documents is the caller's decision (a count sums; `vocabulary_ttr` does not).

## Constants, aliases, and the error type

| Item | Definition | Role |
|---|---|---|
| `limits` | module of `usize` constants | Every cap and metric minimum, below |
| `SCHEMA_VERSION` | `&str`, `"2.0"` | The document schema version this build emits, as `"major.minor"`, written into `Provenance::schema`. The JSON Schema for each major is in `spec/schema/` |
| `Result<T>` | `std::result::Result<T, Error>` | Return type of every fallible library function |
| `Error` | enum, `#[non_exhaustive]` | Every failure the library returns. See [Errors](errors.md) |
| `ErrorDetails` | ordered names and values; `get(key)`, `iter()`, `len()`, `is_empty()` | What `Error::details` returns: every dynamic value in the message, by name. Serializes as an object. See [kinds and details](errors.md#kinds-and-details) |
| `DetailValue` | enum, `#[non_exhaustive]`: `Text(String)`, `Integer(u64)`, `Bool(bool)` | One value in an `ErrorDetails`. Serializes as the bare value |

## Limits

`domain::limits` holds every bound matra holds its work to, each defined once and read where it applies. They are published so you can size work before a call instead of learning a bound from a refusal: split an input under `MAX_INPUT_BYTES`, or chunk a document's sentences under the cap of the method you select, each named for that method as `SummaryMethod` and `KeyphraseMethod` name it. They are constants, not settings: no configuration changes them. They are this release's values and may change in any 0.x release, so read them from `domain::limits` rather than copying the numbers. A cap that is exceeded returns `InputTooLarge`, whose `limit` detail is the constant and whose `what` detail names the gate; a paragraph outside a metric's bounds keeps that metric `None`. Each constant's rustdoc carries the cost model behind its value.

| Constant | Value | Bounds |
|---|---|---|
| `MAX_INPUT_BYTES` | 8,388,608 (8 MiB) | Text one input may hold; the `"input"` and `"file_source"` gates |
| `MAX_CONFIG_FILE_BYTES` | 65,536 (64 KiB) | The configuration file; the `"config_file"` gate |
| `MAX_DOWNLOAD_BYTES` | 67,108,864 (64 MiB) | Any one model download; the `"udpipe_download"` and `"embedding_download"` gates |
| `MAX_TFIDF_SENTENCES` | 2,000 | Sentences a TF-IDF summary ranks; `"tfidf"` |
| `MAX_TEXTRANK_SENTENCES` | 2,000 | Sentences a TextRank summary ranks; `"textrank"` |
| `MAX_CLUSTER_SENTENCES` | `MAX_TEXTRANK_SENTENCES` | Sentences or vectors clustered; `"clusters"`. Defined as TextRank's, since both build the same quadratic graph |
| `MAX_RAKE_TOKENS` | 200,000 | Tokens RAKE reads, punctuation included; `"rake"` |
| `MAX_YAKE_TOKENS` | 200,000 | Tokens YAKE reads, punctuation included; `"yake"` |
| `MAX_COMPRESSION_PARAGRAPH_BYTES` | 262,144 (256 KiB) | The largest paragraph given a `compression_ratio` |
| `MIN_READABILITY_WORDS` | 11 | The fewest words for a `readability_grade` (more than 10) |
| `MIN_COMPRESSION_WORDS` | 51 | The fewest words for a `compression_ratio` (more than 50) |

Two caps with the same value stay two constants where their costs differ, so tuning one does not move the other: TF-IDF's 2,000 is not TextRank's, and YAKE's 200,000 is not RAKE's.

## What crosses the language boundary

The Python surface serializes values with pythonize. Fields have a serde representation and cross with their names unchanged. Methods have none, so there is nothing for them to cross with.

<svg class="mx-ffi" role="img" aria-label="Document fields cross the FFI boundary into a Python dict; Document methods stop at the boundary" viewBox="0 0 720 340" width="720" height="340" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
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
<rect class="box" x="16" y="40" width="272" height="268" rx="5"/>
<rect class="box" x="452" y="40" width="252" height="142" rx="5"/>
<text class="ty" x="30" y="64">Document</text>
<text class="ty" x="466" y="64">Document</text>
<text class="hd" x="30" y="86">stored fields</text>
<text class="hd" x="466" y="86">dict keys</text>
<text class="mem" x="30" y="104">provenance</text>
<text class="mem" x="30" y="122">sections</text>
<text class="mem" x="30" y="140">metrics</text>
<text class="mem" x="30" y="158">skipped</text>
<text class="mem" x="466" y="104">provenance</text>
<text class="mem" x="466" y="122">sections</text>
<text class="mem" x="466" y="140">metrics</text>
<text class="mem" x="466" y="158">skipped</text>
<text class="hd" x="30" y="190">methods</text>
<text class="mem" x="30" y="208">paragraphs()</text>
<text class="mem" x="30" y="226">sentences()</text>
<text class="mem" x="30" y="244">tokens()</text>
<line class="cross" x1="294" y1="100" x2="444" y2="100" marker-end="url(#mx-ffi-a)"/>
<line class="cross" x1="294" y1="118" x2="444" y2="118" marker-end="url(#mx-ffi-a)"/>
<line class="cross" x1="294" y1="136" x2="444" y2="136" marker-end="url(#mx-ffi-a)"/>
<line class="cross" x1="294" y1="154" x2="444" y2="154" marker-end="url(#mx-ffi-a)"/>
<line class="stop" x1="176" y1="204" x2="386" y2="204"/>
<line class="stop" x1="176" y1="222" x2="386" y2="222"/>
<line class="stop" x1="176" y1="240" x2="386" y2="240"/>
<line class="bar" x1="388" y1="197" x2="388" y2="211"/>
<line class="bar" x1="388" y1="215" x2="388" y2="229"/>
<line class="bar" x1="388" y1="233" x2="388" y2="247"/>
<text class="hd" x="401" y="14" text-anchor="middle">FFI boundary</text>
<text class="hd" x="401" y="26" text-anchor="middle">pythonize + serde</text>
<line class="wall" x1="398" y1="32" x2="398" y2="324"/>
<line class="wall" x1="404" y1="32" x2="404" y2="324"/>
<text class="nt" x="462" y="228">a Python caller walks these</text>
<text class="nt" x="462" y="244">in the sections it already has</text>
</svg>

`Token::feat`, `Document::sentences`, `Sentence::subtree`, and every other method in the tables above are available to Rust callers only. The two methods on `Error`, `kind` and `details`, are different: their values are computed into the error's serde form and onto the Python exception, so they cross as fields. Nothing a reader needs is method-only: every aggregate is a field of `DocumentMetrics` or `ParagraphMetrics`, and a sentence carries its `word_count`, `tree_depth` and `passive`, so each crosses with the record that holds it.

| Rust type | Python shape |
|---|---|
| `Token` | `Token` |
| `Sentence` | `Sentence` |
| `Paragraph` | `Paragraph` |
| `Section` | `Section` |
| `Document` | `matra.Document`, read-only, whose `to_dict()` is a `DocumentDict` |
| `DocumentMetrics`, `ParagraphMetrics` | `DocumentMetrics`, `ParagraphMetrics` |
| `Provenance`, `Identity`, `ProvenanceSource` | `Provenance`, `Identity`, `ProvenanceSource` |
| `ClausalComplement`, `RootMood` | `ClausalComplement`, `RootMood` |
| `Span`, `Skipped` | `Span`, `Skipped` |
| `Parameters` | `Parameters` |
| `Summary`, `SummarySentence` | `Summary`, `SummarySentence` |
| `Keyphrases`, `Keyphrase` | `Keyphrases`, `Keyphrase` |
| `Clusters`, `Cluster`, `SimilarityEdge` | `Clusters`, `Cluster`, `SimilarityEdge` |
| `DocumentError` | `DocumentError` |
| `Error` | `matra.ErrorInfo`, and the exception classes under `matra.MatraError` |
| `ErrorDetails` | `ErrorDetails` |
| `Block` | `BlockDict`, a `HeadingBlock` or a `ParagraphBlock` |
| `Format` | its name, a `FormatName` |

`Document` and `DocumentError` are the two items Python's `Engine.analyze_all` yields. A `Document` stays a Rust value behind a read-only handle, and every read of it crosses through serde like every shape above it, with one field replaced: its `provenance.source.path` is decoded with the platform's filesystem decoding (`os.fsdecode` on Unix), so a name that is not valid UTF-8 crosses exactly instead of as serde's lossy string. A `DocumentError` crosses through serde too, as `{"path", "kind", "message", "details"}`, with its `path` decoded the same way. `kind` is `Error::kind` on the domain type, pinned across bindings by `spec/tests/corpus/items.json` and exported to Python as `matra.ERROR_KINDS`; `message` is the Rust error's own `Display` text; `details` is `Error::details`. An `Error` raised rather than returned crosses as a Python exception carrying the same three, `kind`, `str(exc)` and `details`: a method is computed into a value before it crosses, as an aggregate is. A future binding takes the same serde form.

The Python shapes are `TypedDict` declarations available at runtime. They are declared in `matra.types` and re-exported from the package root, so either import works:

```python
from matra.types import DocumentDict, Keyphrases, Summary
from matra import DocumentDict, Keyphrases, Summary
```

Field names and nesting match the Rust types exactly. `Option<f64>` becomes `float | None`; `Option<String>` becomes `str | None`; `Vec<T>` becomes `list[T]`; `usize` becomes `int`.
