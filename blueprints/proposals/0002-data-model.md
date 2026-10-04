# EPR-0002: The data model

- Feature Name: `data_model`
- Start Date: 2026-10-04
- Proposal PR: (this proposal's pull request)
- Tracking issue: none (a baseline proposal describes code that already ships, so there is no milestone to track)
- Status: proposed (only the owner changes it to `accepted`)
- Pinned at: `4fcfb4adc85524243c5e392becd4f490e3e42252`

## Summary

Everything matra hands back is a type in `src/domain.rs`, which depends on
`serde`, `thiserror` and `std` and nothing else. A parsed document is a tree:
`Document` holds `Section`s, which hold `Paragraph`s, which hold
`Sentence`s, which hold CoNLL-U `Token`s. Structural facts derived from the
parse (negation cues, modal auxiliaries, the bare-assertion flag, reporting
constructions, root adverbials, Hearst pairs) and the document-level passive
ratio are stored as fields, so they reach Python and the JSON the command
line emits as data. A failure is a `domain::Error` variant with a stable
kind string. Names reserved for the rule-evaluation tier stay reserved.

This is a baseline proposal. It describes the code at the pinned commit and
carries forward the reasons that still hold from RFC-0002, RFC-0006,
RFC-0008, RFC-0009 and EP-0007, with the error-routing points RFC-0011 and
RFC-0015 settled. A later proposal that changes any of it says so. Unmarked
statements about the code are observed at the pinned commit through the link
beside them; a claim that is inferred or assumed says so.

## Motivation

The data model is the contract that crosses languages. Rust callers read the
types; Python callers read the dicts `pythonize` builds from their `serde`
form ([`to_dict`][to-dict]); command-line callers read the same form inside
the JSON envelope. Three decisions make that contract hold.

**Fields cross the language boundary, methods do not.** The Python surface
is the `Serialize` derive and nothing else, so a Rust method on a domain
type does not exist for Python or the JSON. Before
[RFC-0008](../legacy/rfcs/0008-structural-primitives-are-fields.md), the
Python command line re-implemented passive detection over raw tokens because
`Sentence::is_passive` was a method, and it disagreed with the Rust one.
The rule settled then: derivations of the parse are stored as fields,
computed once in Rust; views over data already crossing stay methods.

**The record is structure, not interpretation.** Every derived field names a
construction in the parse (a cue and its arc, an auxiliary and its head, a
pattern and its spans), never a category such as "hedged" or "deontic".
[EP-0007](../legacy/eps/0007-structural-primitives.md) held this line for
all five primitives, because the moment matra decides `must` is deontic it
has taken a position on meaning the caller is better placed to take.

**A small, pure domain.** Keeping `domain.rs` to three dependencies means a
caller can depend on the types without pulling the parser, the embedding
stack or Python, and it keeps the wire form exactly the struct form.

## Guide-level explanation

A `Document` is the parse made queryable. In Rust:

```rust
for sentence in doc.sentences() {
    for neg in &sentence.negations {
        // neg.cue_id, neg.cue_lemma, neg.head_id: token ids in this sentence
    }
    let verb = sentence.tokens.iter().find(|t| t.pos == "VERB");
    let mood = verb.and_then(|t| t.feat("Mood")); // Some("Ind"), say
}
let ratio = doc.passive_ratio; // Option<f64>, filled by compose
```

In Python the same document is a dict with the same keys
(`doc["sections"][0]["paragraphs"][0]["sentences"][0]["negations"]`), typed
by `TypedDict`s in `matra.types` ([`types.py`][py-types]).

What a caller should hold on to:

- **Metric slots are `Option`.** `None` means `compose` has not run or the
  metric does not apply (a paragraph with no words, a blockquote). There is
  no separate "not measured" marker.
- **Ids are sentence-scoped.** Every derived fact points into
  `sentence.tokens` by token id, so provenance is checkable against the
  parse.
- **Lexicons are the caller's.** Reporting verbs and evidential adverbs are
  open classes, so matra reports every construction and lets the caller
  filter: `sentence.reportings_in(&["claim", "report"])` in Rust, a filter on
  `verb_lemma` elsewhere ([`reportings_in`][reportings-in]).
- **Errors are matched, not read.** A caller branches on the `Error`
  variant in Rust, on the exception class in Python, and on `kind` wherever
  an error is materialized as data.

## Reference-level explanation

### The tree

| Type | Fields | Filled by |
|---|---|---|
| [`Document`][document] | `sections`, `vocabulary_ttr`, `nominalization_ratio`, `passive_ratio` | sections by `annotate`; the three ratios by `compose` |
| [`Section`][section] | `heading`, `level`, `paragraphs` | the decomposer |
| [`Paragraph`][paragraph] | `text`, `in_blockquote`, `sentences`, `readability_grade`, `lexical_density`, `compression_ratio` | text and flag by the decomposer; sentences by `annotate`; slots by `compose` |
| [`Sentence`][sentence] | `text`, `tokens`, `negations`, `modals`, `bare_assertion`, `reportings`, `root_adverbials`, `hearst_pairs` | the provider, through `Sentence::new`; `hearst_pairs` by `annotate` |
| [`Token`][token] | the ten CoNLL-U columns plus `is_punct` | the provider |

`Sentence` documents four invariants (ids sorted, one root, heads in range,
no cycle) and does not validate them; the `NlpProvider` contract states them
for providers. The tree walks stay safe on input that breaks the last one:
[`tree_depth`][tree-depth] returns `usize::MAX` on a cycle, using a visited
set rather than a depth ceiling, and `subtree` carries a visited set too.

### The structural primitives

| Field | Reports | Detector |
|---|---|---|
| `negations` | cue (`not`, `never`, `no`, `neither`, `nor`) on `advmod`, `det` or `cc`, and its head | [`detect_negations`][negation] |
| `modals` | an auxiliary from the closed ten-lemma class, on `aux` or tagged `AUX`, and its head; never its reading | [`detect_modals`][modal] |
| `bare_assertion` | root clause finite indicative with no modal governing it | [`detect_bare_assertion`][bare] |
| `reportings` | each verb governing a `ccomp`, with its subject when parsed | [`detect_reportings`][reporting] |
| `root_adverbials` | each `advmod` attached to the root | [`detect_root_adverbials`][root-adv] |
| `hearst_pairs` | candidate hypernym and hyponym spans from the six Hearst (1992) patterns, with the pattern tag | `hearst::hypernymy_pairs` ([types][hearst-types]) |

Five are computed in [`Sentence::new`][sentence-new]. `hearst_pairs` is filled
at the annotate stage ([`annotate`][annotate-hearst]) because its detector
lives in `src/hearst.rs`, outside the domain, which imports only domain
types ([`hearst.rs`][hearst-imports]). That is the amendment
[RFC-0008](../legacy/rfcs/0008-structural-primitives-are-fields.md)
recorded for EP-0007's fifth milestone: the field still crosses as data;
only the point that fills it moved. A hand-built `Sentence` carries an empty
`hearst_pairs` until the caller runs the detector.

Every derived field is `#[serde(default)]`, so a document serialized before
the field existed still deserializes. Derived fields reflect `tokens` as
passed to `Sentence::new`; mutating `tokens` afterwards does not recompute
them, and keeping them consistent is the caller's side of the documented
contract.

### Views stay methods

[`Token::feat`][feat] looks up one key in the `feats` string by a linear
scan with no allocation, and returns the raw value, so `Case=Nom,Acc` comes
back unsplit. It is Rust-only by design
([RFC-0009](../legacy/rfcs/0009-feats-lookup-accessor.md)): `feats` already
crosses as a string, so the lookup adds nothing to the wire.
`reportings_in`, `root_adverbials_in`, `is_passive`, `tree_depth`,
`subtree`, the `Document` iterators and the `Corpus` aggregates are views of
the same kind.

`Document::passive_ratio()` the method and `Document::passive_ratio` the
field both exist ([method][passive-method], [field][document]). The method
computes the ratio on any document; the metric suite stores it in the field
so it crosses ([`document::compute`][doc-metrics]).

### Errors

[`Error`][error] has seven variants, each with a stable kind string from
[`Error::kind`][error-kind], an exhaustive match with no wildcard so a new
variant cannot inherit another's key. The vocabulary is pinned for every
binding in [`spec/tests/corpus/items.json`][items-kinds] and mirrored in
Python as `ERROR_KINDS` ([`types.py`][py-kinds]). `InputTooLarge` carries a
`what` label naming which gate fired.

In Python each variant maps to one exception class through an exhaustive
match ([`From<MatraError> for PyErr`][pyerr]): `ModelNotFound` to
`FileNotFoundError`; `InputTooLarge`, `UnsupportedFormat` and
`InvalidInput` to `ValueError`; `Io` to `OSError`; `ModelInvalid` and
`ParseFailed` to `RuntimeError`. A new variant fails to compile until it is
routed.

`DocumentError` and `CorpusResult` are not `Serialize`, because `Error`
wraps `std::io::Error` ([corpus types][corpus-result]). Where a per-document
failure crosses, it is materialized as `{kind, message}` by hand
([`document_error_dict`][doc-error-dict]).

### Forward compatibility

Every public struct with public fields and every public enum in the library
is `#[non_exhaustive]`, observed by listing them at the pinned commit. The
one exception is `Embedding`, a tuple struct whose constructor external
`Embedder` implementors must call; [RFC-0010](../legacy/rfcs/0010-embeddings-adapter.md)
records why ([EPR-0005](0005-embeddings-and-semantic-clusters.md)).
`Token::builder` exists because the attribute forbids struct literals
outside the crate ([`TokenBuilder`][token-builder]).

### Reserved vocabulary still in force

[RFC-0006](../legacy/rfcs/0006-abstract-tier-vocabulary-lock.md) reserved
names for the rule-evaluation tier before any of it exists, so that tier
cannot arrive under other names. None of these is a type in `src/` today.

| Concept | Reserved name |
|---|---|
| Declarative rule wrapper; predicate over `Document` | `Rule`, `Predicate` |
| Umbrella over rule output | `Finding` (not `Frame`) |
| Pointer back to the source bytes | `SourceSpan` |
| Relation triple; typed entity; modal marker; illocutionary force; stylometric profile | `Relation`, `Schema`, `Modality`, `SpeechAct`, `Stylometry` |
| Paragraph kind; orthogonal paragraph role | `ParagraphKind`, `ParagraphRole` |
| Grouped metric slots | `ParagraphMetrics`, `DocumentMetrics` |

`Frame` and the verb `frame` stay reserved for frame semantics, as
[RFC-0002](../legacy/rfcs/0002-pipeline-vocabulary.md) first set and
RFC-0006 kept. The five `Finding` rules RFC-0006 states (provenance
mandatory, confidence shape mandatory, a discriminant that names a
structural fact, only FFI-safe types, `#[non_exhaustive]`) bind whichever
shape lands. If metric slots are ever grouped, an outer
`Option<ParagraphMetrics>` is ruled out, because each slot gates
independently. `ROADMAP.md` describes the rule-evaluation entry against this
vocabulary ([roadmap][roadmap-rules]).

### No longer in force

- **The `Analysis` alias.** [RFC-0006](../legacy/rfcs/0006-abstract-tier-vocabulary-lock.md)
  renamed `Analysis` to `Document` with a deprecated alias due for removal
  before 0.1.0. The alias is gone: no type named `Analysis` exists in
  `src/`.
- **`in_blockquote` as a deprecation signal.** RFC-0006 says the field's
  rustdoc signals its replacement by `ParagraphKind`. The rustdoc now says
  the replacement is planned and that nothing is deprecated, because the
  field's job is still binary ([`Paragraph`][paragraph-blockquote]).
- **The `measure` and `extract` split as a type distinction.**
  [RFC-0002](../legacy/rfcs/0002-pipeline-vocabulary.md) called extraction
  a peer of measurement; [RFC-0007](../legacy/rfcs/0007-one-pipeline.md)
  superseded it. Extraction output (`ScoredSentence`, `Keyphrase`) remains
  plain domain types; the stage vocabulary is
  [EPR-0001](0001-pipeline-and-ports.md)'s.

## Drawbacks

- **Every crossing field is a schema commitment** in Rust, Python and the
  JSON. A rename is a breaking change in all three
  ([RFC-0008](../legacy/rfcs/0008-structural-primitives-are-fields.md)).
- **Everyone pays for every primitive.** Each sentence stores its derived
  facts whether or not the caller reads them. RFC-0008 judged this small
  beside the tokens already on the wire; assumed, because no benchmark at
  the pinned commit measures it.
- **Derived fields go stale under mutation** of `tokens`, by the documented
  contract rather than by a check.
- **Two spellings of one value.** `passive_ratio` is both a method and a
  field on `Document`, and they disagree on a document with no sentences:
  the method returns `0.0` ([method][passive-method]) and the field stays
  `None` even after `compose` ([`document::compute`][doc-metrics]).

## Rationale and alternatives

- **Methods only**: no schema lock-in and no storage, but invisible to every
  binding, so each binding re-implements each primitive with nothing
  checking they agree. Rejected in
  [RFC-0008](../legacy/rfcs/0008-structural-primitives-are-fields.md),
  with the Python passive fold as the live case.
- **Methods with a hand-written `Serialize`**: the same wire at a worse
  price, a wire that no longer equals the struct and graph walks inside
  serialization, in the most protected file. Rejected in RFC-0008.
- **Exhaustive typed enums for `feats`**: the Universal Dependencies feature
  inventory is large, language-dependent and grows by release. Rejected in
  [RFC-0009](../legacy/rfcs/0009-feats-lookup-accessor.md), as was a
  per-token `HashMap`, for an allocation on the hot path with no benchmark
  asking for it.
- **Resolving modal readings or shipping evidential word lists**: rejected
  in [EP-0007](../legacy/eps/0007-structural-primitives.md), because a
  reading is interpretation and an incomplete open-class list that looks
  authoritative is worse than none.
- **Routing primitives through `Finding`**: would make the primitives wait
  on a shape RFC-0006 defers until rule evaluation exists. Rejected in
  RFC-0008.

## Prior art

- CoNLL-U, the Universal Dependencies format, is the token shape: one field
  per column.
- Hearst (1992), "Automatic acquisition of hyponyms from large text
  corpora", is the source of the six patterns, implemented here as
  dependency-arc patterns rather than surface regexes.
- [RFC-0006](../legacy/rfcs/0006-abstract-tier-vocabulary-lock.md),
  [RFC-0008](../legacy/rfcs/0008-structural-primitives-are-fields.md),
  [RFC-0009](../legacy/rfcs/0009-feats-lookup-accessor.md) and
  [EP-0007](../legacy/eps/0007-structural-primitives.md) hold the full
  history.

## Unresolved questions

- **`CorpusEntry.analysis` still carries the rejected name.** RFC-0006's
  deferred list, extended by RFC-0010, records that the field should be
  renamed, and that the rename became a breaking change once 0.1.0 shipped
  ([`CorpusEntry`][corpus-entry]). When, and folded into what, is open.
- **`Error::Io` routes to `OSError` whatever its kind.** A missing directory
  arrives in Python as `OSError`, not `FileNotFoundError`, although the
  wrapped `io::ErrorKind` is `NotFound`
  ([RFC-0011](../legacy/rfcs/0011-out-of-the-box.md), unresolved questions;
  [routing][pyerr]). Routing on the wrapped kind would change a shipped
  mapping.

## Future possibilities

- The rule-evaluation tier, using the reserved names, reading these fields
  rather than walking the raw parse. Its `Finding` shape (trait or enum) is
  decided when the first concrete consumer appears.
- `ParagraphKind` replacing `in_blockquote` once a paragraph needs more than
  parse-or-skip.

[to-dict]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L452-L455
[pyerr]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L417-L450
[doc-error-dict]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L1036-L1055
[annotate-hearst]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L235-L243
[error]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L151-L191
[error-kind]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L193-L217
[token]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L226-L259
[feat]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L287-L308
[token-builder]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L261-L285
[negation]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L373-L418
[modal]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L420-L481
[bare]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L483-L514
[reporting]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L516-L587
[root-adv]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L589-L636
[hearst-types]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L638-L704
[sentence]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L706-L790
[sentence-new]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L792-L820
[reportings-in]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L822-L851
[tree-depth]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L875-L956
[paragraph]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L1008-L1042
[paragraph-blockquote]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L1019-L1029
[section]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L1070-L1082
[document]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L1095-L1120
[passive-method]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L1178-L1191
[corpus-entry]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L1338-L1346
[corpus-result]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L1448-L1464
[doc-metrics]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/metrics/document.rs#L12-L22
[hearst-imports]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/hearst.rs#L28
[items-kinds]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/spec/tests/corpus/items.json#L4-L12
[py-types]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/python/matra/types.py#L155-L201
[py-kinds]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/python/matra/types.py#L15-L31
[roadmap-rules]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/ROADMAP.md#L24-L48
