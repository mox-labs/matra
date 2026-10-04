# EPR-0006: The 0.3.0 surface

- Feature Name: `surface_0_3`
- Start Date: 2026-10-04
- Pinned at: `4fcfb4adc85524243c5e392becd4f490e3e42252` (every code citation below opens at this commit)
- Proposal PR: [#134](https://github.com/mox-labs/matra/pull/134)
- Tracking issue: (opened before the owner accepts)
- Status: proposed

## Summary

0.3.0 settles matra's public surface once, as one design, before it grows a fourth language. One application port names what a caller can ask for (analyze, measure, summarize, keyphrases, clusters); Rust, Python and the command line become three syntaxes of that one port and decide nothing on their own. Names say what a thing is, never which algorithm produced it; the algorithm that ran is recorded as data. The document schema becomes a governed interface with a version. The parser port is named for its purpose and converts its file format into matra's terms. Every breaking change lands in this one release, with deprecated aliases for one minor version where the language allows them.

<pragmatics>

<ask>

Accept the 0.3.0 surface as one design, and settle the five decisions under [Unresolved questions](#unresolved-questions). Each can be answered on its own.

</ask>

<will>

- Open the tracking issue with the six milestones below, before you decide.
- Once you accept, land the milestones in order, one pull request each, starting with the schema and its fixture, so a reader sees the contract before the code that meets it.
- Keep every renamed Rust item and Python method as a deprecated alias for 0.3.x.

</will>

<needs>

- An answer to each of the five decisions: accept, accept with a reservation, object or redirect.
- For each assumption listed under Unresolved questions: confirm it or strike it.
- Whether the release split in decision 5 matches when you want streaming.

</needs>

<wont>

- Write code that relies on this proposal while it is `proposed`.
- Change its status to `accepted`, or merge a pull request that does.
- Remove a deprecated alias before 0.4.0, or publish a release without your approval.

</wont>

<silence>

The proposal stays `proposed` and nothing is built on it. Claude lists the open decisions again at the start of the next session that touches the public surface.

</silence>

</pragmatics>

<changed date="2026-10-04" since="the draft of 2026-10-03">

<was>

Old config keys are accepted with a warning, as an enhancement proposal numbered 0020 "already does for two of them".

</was>

<now>

That precedent is RFC-0020, a legacy record; the EPR series starts at this baseline and has no 0020. [RFC-0020](../legacy/rfcs/0020-deprecate-unread-config-keys.md#L12-L17 "the keys are accepted, deprecated, and ignored")

</now>

<was>

RFC-0002 ruled the stage `measure` at lines 105 to 108.

</was>

<now>

The ruling is lines 107 and 108; lines 105 and 106 rename `frame` to `parse`. [RFC-0002](../legacy/rfcs/0002-pipeline-vocabulary.md#L107-L108 "honest about what the stage produces")

</now>

<was>

The renamed parser port would newly "state its contract": ids, heads, one root, no cycles.

</was>

<now>

`NlpProvider` states that contract today. What this proposal adds is text order, `identity()`, and a contract test every adapter must pass, since nothing checks the contract now. [nlp/mod.rs](../../src/nlp/mod.rs#L14-L30 "The trait cannot enforce any of this and nothing validates it")

</now>

<was>

"matra's own conformance test" walks every token to count words.

</was>

<now>

The Python conformance test does; the Rust one calls `total_words()`. [test_conformance.py](../../python/tests/test_conformance.py#L77-L78 "if not t[\"is_punct\"]") [conformance.rs](../../tests/conformance.rs#L190 "doc.total_words()")

</now>

<was>

"Both independent reviews of this design" dissented on algorithm choice.

</was>

<now>

Both were reviews by Claude agents, so they share an instrument with each other and with this proposal's author, and count as one dissent, not two independent ones. The project already says this of its own model review. [pr-review](../../.claude/skills/pr-review/SKILL.md#L14-L17 "it is not independent verification")

</now>

</changed>

## Motivation

### The three surfaces have drifted apart

The same task gives different answers depending on which language asks.

| Task | Rust | Python | Command line |
|---|---|---|---|
| Summarize a README | routes through the pipeline | <claim basis="observed">reads Markdown as plain text, so headings are summarized as prose [lib.rs:486-488](../../src/lib.rs#L486-L488 "Format::PlainText")</claim> | <claim basis="observed">routes through the pipeline, for that reason [cli/mod.rs:608-614](../../src/cli/mod.rs#L608-L614 "on a README would return its headings")</claim> |
| Count a document's words | <claim basis="observed">`Document::total_words()`, a method [domain.rs:1174](../../src/domain.rs#L1174 "pub fn total_words(&self)")</claim> | <claim basis="observed">absent: the caller walks every token, and matra's Python conformance test does too [test_conformance.py:77](../../python/tests/test_conformance.py#L77 "if not t[\"is_punct\"]")</claim> | <claim basis="inferred">printed in the table, absent from `--json`, which serializes the fields and has none for it [render.rs:45](../../src/cli/render.rs#L45 "doc.total_words()") [domain.rs:1103](../../src/domain.rs#L1103 "pub struct Document")</claim> |
| Analyze a folder | <claim basis="observed">a lazy stream [lib.rs:182-187](../../src/lib.rs#L182-L187 "impl Iterator")</claim> | <claim basis="observed">an eager list [_core.pyi:167](../../python/matra/_core.pyi#L167 "list[CorpusItem]")</claim> | <claim basis="observed">refused [cli/mod.rs:561-564](../../src/cli/mod.rs#L561-L564 "is a directory; pass a file")</claim> |
| Group sentences by meaning | <claim basis="observed">`embed_and_cluster(&doc, &embedder, 0.85)` [lib.rs:373-377](../../src/lib.rs#L373-L377 "embedder: &dyn embed::Embedder")</claim> | <claim basis="observed">`semantic_clusters(text, 0.85, model)`, another argument order [_core.pyi:143](../../python/matra/_core.pyi#L143 "def semantic_clusters(self, text: str, threshold: float, model: Embedder)")</claim> | no command |
| Handle an oversized input | <claim basis="observed">`Error::InputTooLarge { limit, actual, what }` [domain.rs:167-175](../../src/domain.rs#L167-L175 "InputTooLarge {")</claim> | <claim basis="observed">a bare `ValueError` whose message is the only detail [lib.rs:443-445](../../src/lib.rs#L443-L445 "PyValueError::new_err(msg)")</claim> | <claim basis="observed">prose on stderr, exit 2 [cli/mod.rs:102-103](../../src/cli/mod.rs#L102-L103 "matra: {e}")</claim> |
| How many results | <claim basis="observed">`n`, `max` or `max_phrases`, by function [textrank.rs:27](../../src/extraction/textrank.rs#L27 "n: usize") [yake.rs:30](../../src/extraction/yake.rs#L30 "max: usize")</claim> | `max_phrases` | <claim basis="observed">`-n`, and the config says `algorithm` where the flag says `--method` [cli/mod.rs:176-181](../../src/cli/mod.rs#L176-L181 "method: Option<SummaryMethod>")</claim> |

<claim basis="inferred">The cause is architectural, not careless. matra named its driven ports carefully (`Source`, `NlpProvider`, `Embedder`) and never named the driving one.</claim> What a caller can ask for is spread across `Engine` methods, free functions in `extraction` and `embed_and_cluster`, so each driving adapter assembled its own version of the application, and application rules leaked into them: the command line decides that extraction goes through the pipeline, and Python decides the opposite. The figure under [One API, three syntaxes](#one-api-three-syntaxes) draws both.

### Names answer the wrong question

- **Algorithms are in the API.** `tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`, the `--method` flag and the `summarize.algorithm` key make the algorithm part of the contract. <claim basis="inferred" likelihood="almost certain">Replacing TextRank would break every caller who asked for a summary by its name.</claim>
- **The stage that measures is called `compose`.** <claim basis="observed">The method is `Engine::compose`, and it runs the metric suite [lib.rs:250](../../src/lib.rs#L250 "pub fn compose(&self, doc: &mut Document)")</claim>. <claim basis="observed">RFC-0002 had ruled it `measure`, "honest about what the stage produces" [RFC-0002:107-108](../legacy/rfcs/0002-pipeline-vocabulary.md#L107-L108 "honest about what the stage produces")</claim>; <claim basis="observed">RFC-0007 reverted that with "None recorded" under its rationale [RFC-0007:133](../legacy/rfcs/0007-one-pipeline.md#L133 "None recorded when this was decided.")</claim>. <claim basis="observed">Until #127 the domain's own prose still called it the measure stage; the documentation was corrected to match the name, not the other way round [#127](https://github.com/mox-labs/matra/pull/127)</claim>.
- **Two record-tier facts are named for readings they do not make.** <claim basis="observed">`Reporting` fires on any verb with a clausal complement, including "think" and "ensure": the detector's only tests are the `ccomp` relation and a verb head [domain.rs:568-574](../../src/domain.rs#L568-L574 "filter(|c| c.dep == \"ccomp\")")</claim>. <claim basis="observed">`bare_assertion` names a speech act, which RFC-0006 reserves for the abstract tier [RFC-0006:43](../legacy/rfcs/0006-abstract-tier-vocabulary-lock.md#L43 "SpeechAct")</claim>.
- **One identity has two names.** <claim basis="observed">The embedding adapter's `identity()` returns its `model_hash()` [model2vec.rs:603-605](../../src/embed/model2vec.rs#L603-L605 "self.model_hash()")</claim>, and `SemanticClusters` carries it as `model_hash`.
- **The port is named after a technology.** <claim basis="observed">`NlpProvider` names the field and a generic supplier; its only act is `parse` [nlp/mod.rs:47-51](../../src/nlp/mod.rs#L47-L51 "fn parse(&self, text: &str)")</claim>.

### The parser's file format reaches the schema

UDPipe speaks CoNLL-U, and its encodings arrive in matra's domain unconverted. <claim basis="observed">`feats` is the string `"Mood=Ind|Tense=Pres"`, and `deps` and `misc` are raw strings [domain.rs:246-256](../../src/domain.rs#L246-L256 "pub feats: String,")</claim>. <claim basis="observed">A Rust-only accessor reads `feats`; Python and JSON callers parse it themselves [domain.rs:299-301](../../src/domain.rs#L299-L301 "Rust-only by design")</claim>. Universal Dependencies' vocabulary is matra's language; CoNLL-U's encoding of it is the parser's technology, and converting it is the adapter's job.

### Why now, and why at once

<claim basis="observed">0.2.1 is published, on crates.io and PyPI [Cargo.toml:3](../../Cargo.toml#L3 "version = \"0.2.1\"") [crates.io](https://crates.io/crates/matra/0.2.1)</claim>, so every rename breaks someone. Under 0.x that is allowed, and the cheapest moment is the one before a TypeScript surface copies the current names into a fourth language. <claim basis="inferred" likelihood="likely">Without a named driving port, a fourth surface would assemble its own application as the other three did, and drift the same way.</claim> Doing it in one release means callers migrate once and the schema version moves once. <claim basis="assumed">Callers would rather migrate once, in a larger release, than several times in smaller ones.</claim>

## Guide-level explanation

### One API, three syntaxes

A caller asks for an outcome. The words are the same in every language:

```rust
let engine = matra::Engine::with_defaults()?;
let doc = engine.analyze_file("README.md")?;
let summary = matra::summarize(&doc, 3)?;
let phrases = matra::keyphrases(&doc, 10)?;
println!("{}", doc.metrics.total_words);
```

```python
m = matra.Engine()
doc = m.analyze(open("README.md").read(), format="markdown")
summary = m.summarize(doc, n=3)
phrases = m.keyphrases(doc, n=10)
print(doc["metrics"]["total_words"])
```

```console
$ matra summarize README.md -n 3
$ matra keyphrases README.md -n 10
$ matra analyze README.md --json | jq .metrics.total_words
$ matra analyze docs/ --json        # one JSON line per document
$ matra clusters notes.md --threshold 0.85
```

The hexagon, today and as proposed. Today each driving adapter reaches the surfaces it needs on its own; proposed, all three reach one application port, and the parser port carries its new name.

<sketch-figure id="hexagon" seed="6" title="The driving side, today and proposed">

```text
width 400
state today shipped "the surface at 4fcfb4a: three adapters, three surfaces, no driving port"
box rust "Rust caller" 4 20 96 34
box py "Python binding\n(in lib.rs)" 4 96 96 34
box cli "Command line" 4 172 96 34
box engine "Engine methods" 146 20 120 34
box extraction "extraction::*" 146 96 120 34
box clusters "embed_and_cluster" 146 172 120 34
quiet box source "Source" 300 8 96 30
quiet box decomposer "Decomposer" 300 62 96 30
quiet box nlp "NlpProvider" 300 116 96 30
quiet box embedder "Embedder" 300 170 96 30
edge rust engine
edge rust extraction
edge rust clusters
edge py engine
edge py extraction
edge py clusters
edge cli engine
edge cli extraction
edge engine source
edge engine decomposer
edge engine nlp
edge clusters embedder
state proposed proposed "EPR-0006: one application port, three adapters that translate"
box rust "Rust API" 4 20 96 34
box py "Python module\n(rule 7)" 4 96 96 34
box cli "Command line" 4 172 96 34
hex port "Application port\nanalyze measure\nsummarize\nkeyphrases\nclusters" 128 40 150 150
quiet box source "Source" 300 8 96 30
quiet box decomposer "Decomposer" 300 62 96 30
quiet box parser "Parser\n(NlpProvider)" 300 116 96 34
quiet box embedder "Embedder" 300 174 96 30
edge rust port
edge py port
edge cli port
edge port source
edge port decomposer
edge port parser
edge port embedder
```

</sketch-figure>

### Outcomes, not algorithms

`summarize` returns the sentences that best represent the text; `keyphrases` the phrases that best characterize it. matra chooses how. Which method ran is recorded in the result's provenance, so a result stays attributable and citable:

```json
{ "sentences": [], "provenance": { "method": "textrank", "matra_version": "0.3.0" } }
```

### The schema is the contract

Everything a caller reads is a field, in every language. Aggregates move into `metrics` records:

```json
{
  "schema_version": 1,
  "provenance": { "matra_version": "0.3.0", "parser": { "name": "udpipe", "model": "...", "sha256": "..." } },
  "metrics": { "total_words": 412, "total_sentences": 19, "mean_sentence_length": 21.7, "passive_ratio": 0.11 },
  "sections": []
}
```

`schema_version` changes only when a field is removed, renamed or retyped. Adding a field does not change it.

### Errors carry a kind everywhere

```python
try:
    m.analyze(huge)
except matra.MatraError as e:          # still a ValueError, so old except clauses keep working
    if e.kind == "input_too_large":
        print(e.limit, e.actual)
```

The command line's `--json` failures carry the same `kind`. <claim basis="observed">The kind already exists in Rust, as a string per variant [domain.rs:211](../../src/domain.rs#L211 "\"input_too_large\"")</claim>; Python and the command line do not carry it.

### Migrating

Every renamed Rust item and Python method keeps its old name for 0.3.x as a deprecated alias (`#[deprecated]` in Rust, a `DeprecationWarning` in Python). Old config keys are accepted with a warning, <claim basis="observed">as RFC-0020 already does for two of them [RFC-0020:12-17](../legacy/rfcs/0020-deprecate-unread-config-keys.md#L12-L17 "the keys are accepted, deprecated, and ignored")</claim>. The migration guide lists every rename on one page. Removal happens in 0.4.0.

## Reference-level explanation

### The application port

| Operation | Rust | Returns |
|---|---|---|
| analyze | `Engine::analyze(Ingest)`, `analyze_text(&str, Format)`, `analyze_file(path)` | a stream of `CorpusEntry`, or one `Document` |
| annotate | `Engine::annotate(&RawDocument)` | a `Document` with structure and record facts |
| measure | `Engine::measure(&mut Document)` (was `compose`) | fills `metrics` |
| summarize | `summarize(&Document, n)` | `Summary` |
| keyphrases | `keyphrases(&Document, n)` | `Keyphrases` |
| clusters | `clusters(&Document, &dyn Embedder, threshold: f64)` | `Clusters` |

Extractors take a `&Document`, not a cloned `&[Sentence]`. Python's methods accept text or a `Document`, take keyword defaults read from configuration, and `analyze` takes `format=`.

### Adapters translate and decide nothing

- The Python binding moves out of `lib.rs` into its own module and is held to the same boundary rule as the command line (rule 7): the public surface only. A semgrep rule enforces it.
- The command line keeps no application rule. Routing extraction through the pipeline moves into the port, which is where Python then gets the same answer.
- `matra.Engine()` has a constructor; `Matra` stays as an alias. <claim basis="observed">Today `Matra` has none: it is built by `from_path` or `english` [_core.pyi:46-56](../../python/matra/_core.pyi#L46-L56 "def from_path(model_path")</claim>.

### The schema

```text
Document         { sections, metrics: DocumentMetrics, provenance: Provenance }
DocumentMetrics  { total_words, total_sentences, paragraph_count, mean_sentence_length,
                   sentence_length_std, vocabulary_ttr, nominalization_ratio, passive_ratio }
Paragraph        { text, sentences, metrics: ParagraphMetrics }
ParagraphMetrics { readability_grade, lexical_density, compression_ratio }
Sentence         { text, tokens, word_count, tree_depth: Option<usize>, passive,
                   negations, modals, clausal_complements, root_adverbials, hearst_pairs, root_mood }
Token            { id, text, lemma, pos, xpos, features: map, feats, head, dep, deps, misc }
Provenance       { matra_version, schema_version, parser: Identity, decomposer, method }
Identity         { name, model, sha256 }
Summary          { sentences: [SummarySentence { index, text, score: f64 }], provenance }
CorpusEntry      { path, document }
```

<claim basis="observed">`ParagraphMetrics` and `DocumentMetrics` are names RFC-0006 reserved [RFC-0006:51-52](../legacy/rfcs/0006-abstract-tier-vocabulary-lock.md#L51-L52 "`DocumentMetrics`")</claim>. Scalars cross every boundary as `f64`. Sentinels become `Option`: <claim basis="observed">`tree_depth` returns `usize::MAX` on a cycle today [domain.rs:897](../../src/domain.rs#L897 "usize::MAX is the cycle sentinel")</claim>, and returns `None` instead.

### Ports

- **`Parser`** (was `NlpProvider`) gains `identity()`, keeps the contract `NlpProvider` states today, and adds one clause to it: sentences in text order. Its adapter converts CoNLL-U: `features` as a map beside the raw `feats`.
- **`Decomposer`** can find paragraph boundaries over a stream, so text within one document can be analyzed as it arrives. The contract changes now; the streaming implementation can follow.
- **Model acquisition** becomes one internal module used by both model adapters, instead of two copies.

Every port gets one shared contract test that each adapter must pass.

### Names

| 0.2 | 0.3 | Kind |
|---|---|---|
| `NlpProvider` | `Parser` | trait |
| `Engine::compose` | `Engine::measure` | method |
| `tfidf_summarize`, `textrank_summarize` | `summarize` | function |
| `rake_keyphrases`, `yake_keyphrases` | `keyphrases` | function |
| `embed_and_cluster`, `semantic_clusters` | `clusters` | function |
| `Reporting`, `Sentence.reportings` | `ClausalComplement`, `clausal_complements` | type, field |
| `Sentence.bare_assertion` | `Sentence.root_mood` (points at the root token) | field |
| `SemanticClusters.model_hash` | `identity` | field |
| `CorpusEntry.analysis` | `CorpusEntry.document` | field |
| Python `Matra`, `analyze_markdown` | `Engine`, `analyze(format=)` | class, method |
| CLI `--method`; config `summarize.algorithm`, `keyphrases.algorithm`, `semantic.*` | removed; `[summary] n`, `[keyphrases] n`, `[clusters] threshold` | flag, keys |

### Limits

The input cap, the sentence caps and the metric minimums move into one module as published constants. Safe defaults stay; a caller can raise them explicitly.

### Invariants and checks

- A schema fixture pins `schema_version` and every field; a breaking change fails it.
- The conformance fixtures in `spec/tests/` pass unchanged in meaning, with updated names.
- Each port's contract test runs against every adapter.
- The Python stub parity check covers every renamed method.
- A semgrep rule holds the Python module to boundary rule 7.

### Milestones

Tracked in this proposal's tracking issue:

1. Schema, metrics records and names, with aliases.
2. The application port; adapters reduced to translation.
3. The parser contract and CoNLL-U conversion; model acquisition as one module.
4. Python: `Engine`, keyword defaults, `MatraError`, the module moved out of `lib.rs`.
5. The command line: directories, `clusters`, `--json` errors, config sections.
6. The migration guide, the docsite updated, and the 0.3.0 release.

## Drawbacks

- Every caller migrates. Aliases soften it for one minor version, but the schema change in JSON and Python has no alias: a reader of `paragraph["readability_grade"]` must read `paragraph["metrics"]["readability_grade"]`.
- Algorithm choice leaves the public surface. A caller who wanted YAKE specifically can no longer ask for it (decision 1).
- One release carries a lot of change, so its review is long.

## Rationale and alternatives

- **One proposal, not several.** The Rust process scopes a proposal to one feature, but an API is designed as a whole: the names, the port and the schema constrain each other, and splitting them would move the schema version several times and make callers migrate repeatedly.
- **Keep algorithm choice.** This is decision 1, with the case for it set out there.
- **Spread the renames across releases.** Each release would break again.
- **Do nothing.** The drift is structural; a fourth surface would copy it.

## Prior art

- J. Bloch, "How to Design a Good API and Why it Matters", OOPSLA 2006: keep implementation out of the API; when in doubt leave it out; provide programmatic access to data in string form.
- A. Cockburn, "The Hexagonal (Ports & Adapters) Architecture", 2005: name ports by purpose; one driving port; adapters translate.
- D. L. Parnas, "On the Criteria To Be Used in Decomposing Systems into Modules", 1972: an exposed representation must be governed as an interface.
- B. Liskov and J. Wing, "A Behavioral Notion of Subtyping", 1994: port contracts.
- The Rust API Guidelines, PEP 8 and the PyO3 guide, and clig.dev: each syntax's own conventions.
- Universal Dependencies and CoNLL-U: the vocabulary matra adopts, and the encoding it converts.
- spaCy's `Doc` attributes as fields, and its `Language` object as the single entry point.

## Unresolved questions

For the owner to settle in this proposal's review. Each is a decision of its own.

<decision id="algorithm-choice" title="1. Does algorithm choice leave the public surface?">

<choice key="a" title="Outcomes only; the method is recorded in provenance">

`summarize` and `keyphrases` with no method argument. matra picks the method, and every result names the one that ran. A typed selector can be added later without breaking anyone.

</choice>

<choice key="b" title="Keep the choice, as a typed selector">

`summarize(&doc, n, Method::TextRank)`, with an enum in place of today's function names and strings, so the algorithm stays part of the contract but is typed.

</choice>

<recommendation choice="a">

<claim basis="assumed">None of the 26 use cases in the API audit behind this proposal asks for an algorithm; they ask for a summary or for keyphrases.</claim> Citation is met by recording the method in provenance. Adding a selector later is additive; removing one later breaks everyone who used it.

</recommendation>

<against>

TextRank, RAKE and YAKE are cited methods, and a caller replicating a published result needs a specific one. <claim basis="observed">The command line already offers `--method`, so someone may depend on it today [cli/mod.rs:178-180](../../src/cli/mod.rs#L178-L180 "method: Option<SummaryMethod>")</claim>. Both earlier reviews of this design argued for keeping the choice; they were reviews by Claude agents, so they count as one voice, but it is the only dissent on record.

</against>

</decision>

<decision id="port-names" title="2. Does compose become measure, and NlpProvider become Parser?">

<choice key="a" title="Rename both">

`Engine::measure` and the `Parser` trait, each with a deprecated alias for 0.3.x.

</choice>

<choice key="b" title="Rename the stage only">

`Engine::measure`, and the trait keeps its name, so no third-party adapter breaks.

</choice>

<choice key="c" title="Keep both names">

Document what each does, and change nothing.

</choice>

<recommendation choice="a">

Each name should say what the thing does: the stage measures, and the port parses. RFC-0002 already ruled `measure` once, and the reversal recorded no reason.

</recommendation>

<against>

`compose` is the third word of the pipeline's vocabulary (ingest, decompose, compose), which RFC-0007 restored deliberately, and `measure` breaks the triad. A trait rename breaks every adapter written outside matra, and a Rust trait alias cannot be implemented, so the deprecation path is weaker than for a method.

</against>

</decision>

<decision id="record-facts" title="3. Do Reporting and bare_assertion take names for what they detect?">

<choice key="a" title="Rename to ClausalComplement and root_mood">

`ClausalComplement` and `clausal_complements` for what the detector finds, and a `root_mood` fact that points at the root token in place of the `bare_assertion` flag.

</choice>

<choice key="b" title="Keep the names, sharpen the documentation">

The names stay; their documentation says plainly which readings they do not make.

</choice>

<recommendation choice="a">

Today's names assert readings the detectors do not make, and a name is read far more often than its documentation.

</recommendation>

<against>

<claim basis="observed">The field's documentation already disclaims the reading: it "reports the surface form only" [domain.rs:750-753](../../src/domain.rs#L750-L753 "only; what the assertion commits its speaker to is the")</claim>. A renamed field breaks every JSON reader with no alias, since an alias in serde helps reading, not writing.

</against>

</decision>

<decision id="caller-extensions" title="4. Do caller extensions get their own proposal?">

<choice key="a" title="A separate proposal, for 0.4.0">

Predicates producing Findings, the names RFC-0006 reserved, are designed in their own proposal once this surface is settled.

</choice>

<choice key="b" title="A slot in this proposal">

This proposal reserves the extension point in the 0.3.0 schema now, without the predicates themselves.

</choice>

<recommendation choice="a">

The extension point depends on the schema and the port this proposal settles; designing it on top of them is cheaper than designing all three at once.

</recommendation>

<against>

<claim basis="inferred" likelihood="roughly even chance">If the extension point needs a field in the schema, 0.4.0 moves `schema_version` again, which is the repeated migration this proposal exists to avoid.</claim>

</against>

</decision>

<decision id="release-split" title="5. Is the release split right?">

<choice key="a" title="Breaking shape in 0.3.0, the rest in 0.4.0">

0.3.0 carries every breaking change. Source spans, predicates and the streaming implementation follow in 0.4.0, as additions.

</choice>

<choice key="b" title="Everything in 0.3.0">

One release carries the shape, the spans, the predicates and streaming.

</choice>

<recommendation choice="a">

The breaking changes land together and callers migrate once; what follows is additive and can ship when it is ready.

</recommendation>

<against>

A caller waiting for streaming waits a release longer, though its contract changes now. <claim basis="assumed">The owner's answers of 2026-09-27 were: English only for now; streaming happens, with its contract change in this release; and matra's own extensions come in a later proposal.</claim> If that timing has moved, this split should move with it.

</against>

</decision>

What the owner should confirm or strike, collected from the claims above that stand on no grounds yet:

<assumptions />

## Future possibilities

Proposals that build on this surface: source spans and provenance; Predicates and Findings; a default parser model trained on UD 2.18 English EWT, whose licence permits commercial use; and the WebAssembly and TypeScript surface, with a shareable component that renders a parse.
