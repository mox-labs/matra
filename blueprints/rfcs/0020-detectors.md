# RFC-0020: Detectors, and the first extension point

- Feature Name: `detectors`
- Start Date: 2026-09-26
- RFC PR: (this pull request)
- Tracking EP: (assigned when the EP opens)
- Status: accepted (the file reaches `main` only by merging, which is what accepts it)

## Summary

Name the layer that finds structure in a parsed sentence, give it a home, and open it to
code outside matra. The built-in detectors (negation, modals, bare assertion, reporting,
root adverbials, Hearst pairs) move out of `domain.rs` and `hearst.rs` into `src/detect/`,
where they are plain functions like `metrics/`. A `Detector` trait lets a caller add their
own, in Rust or as a Python callable. A caller's detector returns `Finding`s, the umbrella
type RFC-0006 reserved, each pointing at the tokens and the source span it is about, and
they arrive on `Sentence.findings`. Built-in results stay typed fields.

## Motivation

**Adding a detector today means editing the core.** Five of the six detectors run inside
`Sentence::new` in `src/domain.rs`, so the module that is meant to hold types does work.
The sixth, Hearst pairs, lives at the crate root in `src/hearst.rs` and is wired by hand
into `Engine::annotate` (`src/lib.rs`) because the domain cannot import it. Each new
detector has had to pick one of these two places, and each needed a new field on
`Sentence`, a Python stub, and a docs entry. None of that is open to someone outside the
repository.

**The detectors people ask for are not ones matra should ship.** Hedging vocabularies,
house-style rules, domain terms, anaphora heuristics, a team's own checks: these are
open-class or opinionated, which is the reason `Sentence::reportings_in` takes a
caller's lexicon rather than shipping one ("an incomplete list that looks authoritative is
worse than none"). The right answer for them is not a field on `Sentence`; it is a place
for the caller's code to run inside the pipeline, over the same parse, with its results
carried in the same output.

**Callers already work around the gap.** A Python caller who wants a custom check today
runs matra, walks the returned dict, and re-implements dependency navigation in Python,
which is the duplication RFC-0008 was written to remove.

**Rules need it.** RFC-0006 reserves `Rule`, `Predicate` and `Finding`. A rule is a
detector whose logic is data. Without a detector layer there is nowhere for rule
evaluation to plug in; with one, a rule engine is one more `Detector`.

## Guide-level explanation

### Writing a detector in Rust

```rust
use matra::detect::Detector;
use matra::domain::{Finding, Sentence};

struct Hedges { words: Vec<String> }

impl Detector for Hedges {
    fn name(&self) -> &str { "example.hedges" }

    fn detect(&self, sentence: &Sentence) -> matra::domain::Result<Vec<Finding>> {
        Ok(sentence.tokens.iter()
            .filter(|t| self.words.iter().any(|w| *w == t.lemma))
            .map(|t| Finding::new(self.name(), "hedge", vec![t.id]))
            .collect())
    }
}

let engine = Engine::with_defaults()?.with_detector(Box::new(Hedges { words }));
```

`Finding::new` takes the detector's name, a kind, and the token ids it is about. The
engine fills in the finding's `span` from those tokens (RFC-0018), so a detector never
computes offsets and cannot get them wrong.

### Writing a detector in Python

```python
def hedges(sentence):
    return [
        {"kind": "hedge", "tokens": [t["id"]]}
        for t in sentence["tokens"] if t["lemma"] in {"perhaps", "arguably", "seem"}
    ]

m = matra.Matra.from_path(model).with_detector("example.hedges", hedges)
doc = m.analyze(text)
doc["sections"][0]["paragraphs"][0]["sentences"][0]["findings"]
# [{"detector": "example.hedges", "kind": "hedge", "tokens": [3],
#   "span": {...}, "attributes": {}, "confidence": null}]
```

The callable receives the sentence exactly as Python sees it everywhere else (the same
dict), and returns a list of dicts with `kind` and `tokens`, plus optional `attributes`
and `confidence`.

### What a finding is

| Field | Meaning |
|---|---|
| `detector` | The name of the detector that produced it, as the detector reports it. |
| `kind` | What was found, in the detector's own vocabulary (`"hedge"`). |
| `tokens` | The ids of the tokens it is about, within its sentence. Never empty. |
| `span` | The source range covering those tokens, filled by the engine (RFC-0018). |
| `attributes` | Extra detail: a map from names to booleans, integers, floats, strings or lists of them. |
| `confidence` | Optional. `None` means the detector makes no confidence claim. |

A finding names a structural fact about the text ("this token is in the caller's hedge
list"), not a judgement about the writer or the world. That is RFC-0006's Frame-3; matra
cannot enforce it on someone else's code, so the documentation states it as the contract.

### What changes for existing callers

Nothing in the output. `negations`, `modals`, `bare_assertion`, `reportings`,
`root_adverbials` and `hearst_pairs` stay typed fields with the same values. They are
computed at the annotate stage now rather than inside `Sentence::new`, so a Rust caller
who builds a `Sentence` by hand gets those fields empty until they run
`matra::detect::builtin(&mut sentence)`. That is the one behaviour change, and it is named
in the CHANGELOG.

## Reference-level explanation

### Layout

```
src/detect/mod.rs        the Detector trait (a port: imports only domain)
src/detect/negation.rs   built-in detectors: plain functions over &[Token],
src/detect/modal.rs      importing only domain and stopwords (rule 5's class)
src/detect/reporting.rs
src/detect/adverbial.rs
src/detect/hearst.rs     moved from src/hearst.rs
```

`detect::builtin(&mut Sentence)` runs the built-ins in a fixed order and fills their
fields. `Engine::annotate` calls it for every parsed sentence, then runs the engine's
extension detectors in registration order, appending their findings to
`Sentence.findings`.

### The trait

```rust
pub trait Detector: Send {
    fn name(&self) -> &str;
    fn detect(&self, sentence: &Sentence) -> domain::Result<Vec<Finding>>;
}
```

`&Sentence`, not `&mut`: a detector reads the parse and the built-in fields and returns
findings; it cannot alter the parse or another detector's output. `Send` without `Sync`,
matching `NlpProvider`, because the engine is already `Send`-only.

### Types (domain)

```rust
#[non_exhaustive]
pub struct Finding { pub detector: String, pub kind: String, pub tokens: Vec<usize>,
                     pub span: Option<SourceSpan>,
                     pub attributes: BTreeMap<String, AttributeValue>,
                     pub confidence: Option<f64> }

#[non_exhaustive]
#[serde(untagged)]
pub enum AttributeValue { Bool(bool), Int(i64), Float(f64), Text(String),
                          List(Vec<AttributeValue>) }
```

`Sentence` gains `findings: Vec<Finding>`. `AttributeValue` is a closed, domain-owned value
type rather than `serde_json::Value`, so boundary rule 1 holds (the domain depends on
`serde`, `thiserror` and `std` only), and it serialises to plain JSON values so Python
sees ordinary dicts and lists.

RFC-0006 deferred `Finding`'s shape (trait or enum) until a third concretion forced it.
This RFC settles it as a struct, because the concretions are callers' detectors, whose
variety matra cannot enumerate: an enum would have to be closed, and a trait object would
not serialise. Its five rules map as follows: Frame-1 (provenance) is `detector` plus
`span`; Frame-2 (confidence shape mandatory, value optional) is `confidence:
Option<f64>`; Frame-3 is the documented contract above; Frame-4 (FFI-safe) holds because
every field is a primitive, a string, or a container of them; Frame-5 is
`#[non_exhaustive]`.

### Invariants and checks

- **Findings are well-formed.** The engine rejects a finding whose `tokens` is empty or
  names an id not in the sentence, and a detector whose `name` is empty or starts with
  `matra.` (reserved for built-ins later). The document fails with
  `Error::Detector { name, message }`; a malformed finding is never dropped silently.
- **A detector's failure is the document's failure.** An `Err` from `detect`, or a Python
  exception from a callable, becomes `Error::Detector` for that document, which the
  corpus stream reports as a `DocumentError` like any other. A Rust detector that panics
  is caught at the call with `catch_unwind` and reported the same way, so a caller's bug
  does not abort the host process.
- **`Error::Detector` routes to a specific Python exception.** `From<MatraError> for
  PyErr` has no wildcard, so the new variant fails to compile until it is wired.
- **Built-in output is unchanged.** The conformance fixtures in `spec/tests/` (negation,
  modal, hearst and the rest) pass unmodified across the move.
- **Boundaries.** New semgrep rules in `.semgrep/` (EP-0014) hold `src/detect/mod.rs` to
  rule 2 (a port imports only `domain`) and the built-in files to rule 5's import set, and
  forbid `detect` from importing `nlp`, `source`, `decompose` or `embed`. Each rule ships
  with its fixture.

### Python

`Matra.with_detector(name, callable)` returns a new `Matra` whose engine carries a
`PyDetector` adapter: it holds the callable, converts the sentence with the same
`pythonize` path the rest of the surface uses, calls it under the GIL (the `Matra` class is
already `unsendable`), and converts the returned list back into `Finding`s, rejecting
malformed entries as above. The adapter lives in the Python module in `src/lib.rs`, beside
the other bindings.

## Drawbacks

- **`Sentence::new` stops filling derived fields.** A Rust caller constructing sentences by
  hand must call `detect::builtin`. Pre-1.0 and named in the CHANGELOG; the alternative
  (the domain calling into `detect`) breaks boundary rule 1.
- **A second channel for results.** Built-in facts are typed fields; extension facts are
  `Finding`s. A reader has to know which is which. The split is deliberate (below), and
  the docs present it once, in the reference.
- **Python detectors are slow.** One Python call per sentence per detector. Acceptable for
  the use (a caller's own checks), and a caller who needs speed writes the detector in
  Rust.

## Rationale and alternatives

- **Everything as findings, built-ins included.** One channel, but it throws away the
  typed fields RFC-0008 established and every consumer relies on, and turns
  `sentence.negations` into a filter over strings. Built-ins are matra's contract; typed
  fields are the stronger contract.
- **An open attribute bag on `Sentence` that detectors write into.** No provenance (who
  wrote this key?), no span, and detectors could overwrite each other.
- **`&mut Sentence` detectors.** Simpler to write, and lets any detector corrupt the parse
  or the built-in fields for every detector after it.
- **Name-based registry and config file selection** (`[[detectors]] type = "..."`, as
  x.uma's `TypedExtensionConfig` does for inputs, matchers and actions). Needed when a
  detector must be chosen by name from outside the program: the CLI enabling an optional
  detector, or a rule set loaded from a file. No such case exists in 0.3.0, so it is left
  for the RFC that introduces the first one; the `Detector` trait and `name()` are the
  shape a registry would build on, so adding one later changes nothing here.
- **Extension points for metrics and extractors too.** Same argument: no concrete need
  yet. A metric is a number over a scope, which a detector can already report as a
  finding with an attribute; if that proves awkward in practice, that is the trigger.
- **Leave it to callers.** Today's state; it duplicates dependency walking in every
  consumer and gives their results no span or place in the output.

## Prior art

- spaCy: pipeline components added with `nlp.add_pipe`, custom attributes registered on
  `Doc`, `Span` and `Token` through `set_extension`. The open-attribute design is the one
  rejected above for lack of provenance; the component model is close to this one.
- Stanza: processors registered by name, with variants.
- semgrep and ESLint: rules as data, each finding carrying the rule id and a source range;
  the model for `detector` plus `span`.
- Envoy and x.uma: typed extension configs resolved by a factory registry; the model for
  the deferred registry.
- RFC-0006 (the reserved `Finding` and its contract), RFC-0008 (fields cross FFI),
  RFC-0018 (spans), EP-0014 (the semgrep rules this extends).

## Unresolved questions

- To resolve in review: whether `with_detector` builds a new engine (as proposed) or
  mutates in place.
- To resolve in the EP: whether `detect::builtin` is public (proposed) or only the engine
  runs built-ins; the name of the Python exception for `Error::Detector`.
- Out of scope: the name-based registry and config selection; rule evaluation as a
  detector; exposing findings to x.uma as a matcher input.

## Future possibilities

Rule evaluation lands as a `Detector` whose logic is loaded from data, which is where
`Rule` and `Predicate` (RFC-0006) take their shape. A registry lets the CLI and config
files choose detectors by name. A bridge exposes findings to x.uma as a data input, so a
matcher can select on "sentences with a hedge finding" without matra knowing about
matchers.
