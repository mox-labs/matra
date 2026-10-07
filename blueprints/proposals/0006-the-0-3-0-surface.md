# EPR-0006: The 0.3.0 surface

- Feature Name: `surface_0_3`
- Start Date: 2026-10-04
- Pinned at: `6e9d94b580d6cd442bf74f30a6c05ade5329ca0d` (every code citation below opens at this commit)
- Proposal PR: [#134](https://github.com/mox-labs/matra/pull/134)
- Tracking issue: (opened before the owner accepts)
- Status: proposed

## Summary

0.3.0 settles matra's public surface once, as one design, before a TypeScript surface copies it. One application port names what a caller can ask for (analyze, annotate, measure, summarize, keyphrases, clusters, and a feed for text that arrives over time); Rust, Python and the command line become syntaxes of that one port and translate without deciding. Names say what a thing does; the method that ran, and every parameter that shaped a result, are recorded on the result. The document schema becomes a governed interface: one version in one place, rules for consumers, and a published JSON Schema. Every result carries its provenance and points back to its source by span. Failures carry a kind on every channel. The parser and decomposer contracts change once, in this release, each with its implementation. Every break lands in 0.3.0, with a transition window for 0.3.x: deprecated aliases, renamed JSON fields emitted under both names, and the 0.2 output on request from the command line.

Four decisions remain open for the owner. The first goes against a position the owner has stated, and says so.

<pragmatics>

<ask>

Accept the 0.3.0 surface as one design, and settle the four decisions under [Unresolved questions](#unresolved-questions). Each can be answered on its own.

</ask>

<will>

- Open the tracking issue with the milestones under Reference-level explanation, before you decide.
- Once you accept, land the milestones in order, one pull request each, starting with the schema, its JSON Schema and its fixtures, so a reader sees the contract before the code that meets it.
- Fix the shipped defects listed in [Defects in what ships](#defects-in-what-ships) in a separate pull request, since none of them waits on this design.
- Keep every renamed Rust item, Python method, CLI flag and JSON field for 0.3.x, as the transition window describes, and remove them in 0.4.0 only.

</will>

<needs>

- An answer to each of the four decisions: accept, accept with a reservation, object or redirect.
- For each assumption listed under Unresolved questions: confirm it or strike it.
- A reading of the rationale with one caveat in mind. This text merges a design by Claude, an independent design by a second Claude agent working from the owner's software-design canon, and a canon review by a third. All three are Claude agents reading one corpus, so where they agree, that is one voice, not three.

</needs>

<wont>

- Write code that relies on this proposal while it is `proposed`.
- Change its status to `accepted`, or merge a pull request that does.
- Remove a deprecated name before 0.4.0, or publish a release without your approval.

</wont>

<silence>

The proposal stays `proposed` and nothing is built on it. Claude lists the open decisions again at the start of the next session that touches the public surface.

</silence>

</pragmatics>

<changed date="2026-10-06" since="the text of 2026-10-04">

<was>

The schema carried a new `schema_version`, at the top level in one example and inside `Provenance` in another, bumped only on removal, rename or retype, and the shipped `format_version` went unmentioned.

</was>

<now>

One version, in `provenance` only, with rules for consumers; the shipped `format_version` keeps its promise and moves with the schema major. [json.md:23](../../skills/matra/references/json.md#L23 "It increments on any change to the envelope or to the meaning of a field inside")

</now>

<was>

Algorithm choice left the surface, recommended without a stated case for the other side beyond one dissent.

</was>

<now>

The choice is open, with both cases grounded in the canon; the recommendation keeps an optional typed method and says plainly that this goes against the owner's stated position. [#145](https://github.com/mox-labs/matra/pull/145)

</now>

<was>

The `Decomposer` contract changed in 0.3.0 and the streaming implementation followed in 0.4.0.

</was>

<now>

A provider contract changes only together with an implementation; streaming timing is an open decision between a narrow feed in 0.3.0 and a new trait in 0.4.0. [#145](https://github.com/mox-labs/matra/pull/145)

</now>

<was>

Source spans waited for 0.4.0, and JSON readers had no transition window.

</was>

<now>

Span postconditions join the parser and decomposer contracts in 0.3.0, since both contracts change here anyway; the offset unit is an open decision. Renamed JSON fields are emitted under both names for 0.3.x, and the command line can emit the 0.2 output. [#145](https://github.com/mox-labs/matra/pull/145)

</now>

<was>

Python took keyword defaults read from configuration.

</was>

<now>

Library defaults are written in the signatures; configuration applies at the command line and in the `from_config` constructors; every effective parameter is recorded on the result. [#145](https://github.com/mox-labs/matra/pull/145)

</now>

</changed>

## Motivation

### What the owner has settled

These are settled, by the owner's answers of 2026-09-27 and 2026-10-05, and are inputs here rather than questions:

- **English only, for now.** A second language is not a likely change, so the language may appear in names (`Engine.english()`) without a module to hide it. Interfaces carry what is unlikely to change [parnas-process:c20].
- **Streaming happens**, in both senses: a stream of documents (which exists) and text arriving over time within one document (which does not).
- **matra implements the x.uma extensions itself**, the way xDS extensions live with their implementers, once the x.uma crates are published. That waits for a later proposal.

### The surface reports something it does not keep

The record does not say where its text came from, and the documentation says it does.

- <claim id="verbatim-claim" basis="observed">The roadmap tells callers that `Sentence.text` and `Paragraph.text` are "verbatim by design" [ROADMAP.md:193](../../ROADMAP.md#L193 "being verbatim by design")</claim>.
- <claim id="sentence-text-rebuilt" basis="observed">Sentence text is rebuilt from token forms joined by spaces, and the type's own documentation says its whitespace "can differ from the source" [udpipe.rs:778](../../src/nlp/udpipe.rs#L778 "Reconstruct original text using SpaceAfter=No") [domain.rs:723-726](../../src/domain.rs#L723-L726 "rebuilds it from the token forms")</claim>.
- <claim id="paragraph-text-rebuilt" basis="observed">Markdown paragraph text is split into lines, rejoined and trimmed, which drops a `\r` from `\r\n` input, and blockquote text loses its `>` markers [markdown.rs:15](../../src/decompose/markdown.rs#L15 "text.lines()") [markdown.rs:29](../../src/decompose/markdown.rs#L29 "trim().to_string()") [markdown.rs:78](../../src/decompose/markdown.rs#L78 "trim_start_matches('>')")</claim>.
- <claim id="rfc0006-span" basis="observed">RFC-0006 reserved `SourceSpan` as the "Source span pointer", sketched in bytes, for exactly this job [RFC-0006:46](../legacy/rfcs/0006-abstract-tier-vocabulary-lock.md#L46 "byte_offset, byte_length")</claim>, and nothing in the domain carries an offset today.

A downstream distillation tool needs character offsets into the source, a provenance stamp and a versioned schema, and an agent reading the roadmap would trust text the code does not promise. Each fact belongs in "one, and only one, place" [parnas-process:c14]; here the documentation and the code disagree about one.

### The three surfaces have drifted apart

| Task | Rust | Python | Command line |
|---|---|---|---|
| Summarize a README | through the pipeline | <claim id="python-plain" basis="observed">reads Markdown as plain text, so headings are summarized as prose [lib.rs:487](../../src/lib.rs#L487 "Format::PlainText")</claim> | <claim id="cli-pipeline" basis="observed">through the pipeline, for that reason [cli/mod.rs:612](../../src/cli/mod.rs#L612 "on a README would return its headings")</claim> |
| Count a document's words | a method | absent: the caller walks the tokens | <claim id="total-words-table" basis="observed">printed in the table, absent from `--json` [render.rs:45](../../src/cli/render.rs#L45 "doc.total_words()")</claim> |
| Analyze a folder | <claim basis="observed">a lazy stream [lib.rs:182](../../src/lib.rs#L182 "pub fn analyze<I>(")</claim> | an eager list | <claim basis="observed">refused [cli/mod.rs:563](../../src/cli/mod.rs#L563 "is a directory; pass a file")</claim> |
| Group sentences by meaning | <claim basis="observed">`embed_and_cluster(&doc, &embedder, threshold)` [lib.rs:373](../../src/lib.rs#L373 "pub fn embed_and_cluster(")</claim> | <claim basis="observed">`semantic_clusters(text, threshold, model)` [_core.pyi:143](../../python/matra/_core.pyi#L143 "def semantic_clusters(self, text: str, threshold: float, model: Embedder)")</claim> | no command |
| Handle an oversized input | a typed variant with `limit`, `actual`, `what` | <claim basis="observed">a bare `ValueError` [lib.rs:444](../../src/lib.rs#L444 "PyValueError::new_err(msg)")</claim> | <claim id="json-failure-prose" basis="observed">prose on stderr, nothing on stdout, exit 2 [cli/mod.rs:102](../../src/cli/mod.rs#L102 "matra: {e}")</claim> |
| How many results | <claim basis="observed">`n` or `max`, by function [textrank.rs:27](../../src/extraction/textrank.rs#L27 "n: usize") [yake.rs:30](../../src/extraction/yake.rs#L30 "max: usize")</claim> | `n` or `max_phrases` | `-n` |

<claim id="no-driving-port" basis="inferred">The cause is architectural. matra named its driven ports (`Source`, `NlpProvider`, `Embedder`) and never named the driving one</claim>, so each driving adapter assembled its own version of the application, and application rules leaked into them: the command line decides that extraction goes through the pipeline, and Python decides the opposite. A design decision with two homes is the defect information hiding exists to prevent: each module should own one decision and hide it [parnas-modules:c23]. <claim id="python-in-root" basis="observed">The Python binding also lives inside the composition root [lib.rs:406](../../src/lib.rs#L406 "mod python {")</claim>, beyond the reach of the boundary rule that holds the command line to the public surface.

### Names answer the wrong question

- <claim id="algorithm-names-today" basis="observed">The algorithm is in the function names: `tfidf_summarize` and its siblings in Rust, and the same in Python [extraction/mod.rs:14](../../src/extraction/mod.rs#L14 "pub use tfidf::summarize as tfidf_summarize") [_core.pyi:103](../../python/matra/_core.pyi#L103 "def tfidf_summarize")</claim>, and the format is in one: <claim basis="observed">`analyze_markdown` [_core.pyi:94](../../python/matra/_core.pyi#L94 "def analyze_markdown")</claim>. The ranking algorithm is a decision likely to change, and a likely change should touch no interface [parnas-process:c22].
- <claim id="compose-measures" basis="observed">The stage that runs the metric suite is called `compose` [lib.rs:250-251](../../src/lib.rs#L250-L251 "let suite = metrics::default_suite();")</claim>. A name should say what the thing does, and the same word should mean the same thing everywhere [api-design-practice:c8].
- <claim id="reporting-any-ccomp" basis="observed">`Reporting` fires on any verb with a clausal complement, including "think" and "ensure" [domain.rs:572](../../src/domain.rs#L572 "ccomp")</claim>, which breaks <claim basis="observed">matra's own rule that a derived field names a construction, "never a category" [EPR-0002:87](0002-data-model.md#L87 "never a category such as")</claim>.
- <claim id="parser-port-name" basis="observed">The parser port is named for a field and a supplier, `NlpProvider`; its only act is to parse [nlp/mod.rs:47](../../src/nlp/mod.rs#L47 "pub trait NlpProvider: Send")</claim>.

### Failure has no contract outside Rust

<claim basis="observed">The JSON channel's stability promise is `format_version`, "the whole stability promise" an agent is told to pin [json.md:28](../../skills/matra/references/json.md#L28 "that is the whole stability promise")</claim>, yet a failure under `--json` writes nothing to stdout (above), so an agent reads prose. Behaviour on failure "is as much a part of the formal contract as when things go right" [api-design-practice:c23], and prose that callers parse "becomes part of the contract and cannot change" [machine-readable-contracts:c15].

### Why now, and why at once

0.2.1 is published, so every rename breaks someone. Under 0.x that is allowed, and the cheapest moment is before a TypeScript surface copies today's names into a fourth language. <claim id="migrate-once" basis="assumed">Callers would rather migrate once, in a larger release, than several times in smaller ones.</claim> Incompatibility across releases makes choosing versions a hard problem for callers [api-design-practice:c30], which favours one break over several.

## Guide-level explanation

### One API, three syntaxes

A caller asks for an outcome. The words are the same in every language.

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
$ matra analyze README.md --json | jq .result.metrics.total_words
$ matra analyze docs/ --json          # one envelope per line, one line per document
$ matra clusters notes.md --threshold 0.85
```

How the method is chosen, if at all, is decision 1. Every result records the method that ran and the `n` or threshold it ran with, whatever decision 1 says.

<sketch-figure id="hexagon" seed="6" title="The driving side and the driven ports, today and proposed">

```text
width 400
state today shipped "three adapters reach three surfaces, and two of them decide"
box rust "Rust caller" 2 10 118 30
box py "Python (lib.rs):\nmarkdown as plain" 2 56 118 34
box cli "Command line:\nown pipeline rule" 2 106 118 34
box clusters "embed_and_cluster" 140 10 120 30
box engine "Engine methods" 140 58 120 30
box extract "extraction::*\nalgorithm names" 140 104 120 34
quiet box embedder "Embedder" 286 4 112 26
quiet box source "Source" 286 40 112 26
quiet box decomposer "Decomposer" 286 76 112 26
quiet box nlp "NlpProvider\nCoNLL-U strings" 286 112 112 34
edge rust clusters
edge rust engine
edge rust extract
edge py clusters
edge py engine
edge py extract
edge cli engine
edge cli extract
edge clusters embedder
edge engine source
edge engine decomposer
edge engine nlp
state proposed proposed "one application port; adapters translate; TypeScript reads the schema"
box rust "Rust API" 2 8 114 28
box py "Python module\nrule 7, own file" 2 46 114 34
box cli "Command line" 2 90 114 28
quiet box ts "TypeScript 0.4\ntypes, viewer" 2 140 114 34
hex port "Application port\nanalyze annotate\nmeasure summarize\nkeyphrases\nclusters feed" 128 4 146 120
box schema "Schema 2.x\nJSON Schema" 146 140 110 34
quiet box source "Source" 290 4 108 26
box decomposer "Decomposer\nblocks, spans" 290 38 108 34
box parser "Parser\nidentity, spans" 290 80 108 34
quiet box embedder "Embedder" 290 122 108 26
edge rust port
edge py port
edge cli port
edge ts schema
edge port schema
edge port source
edge port decomposer
edge port parser
edge port embedder
```

</sketch-figure>

Today the Python box and the command-line box each carry an application rule, and the parser port hands CoNLL-U's string encodings straight into the domain. Proposed, those rules move into the port, the Python module leaves the composition root, and the two driven ports whose contracts change in this release are drawn at full weight.

### The record says where it came from

Every top-level result carries `provenance`, and every value that names text carries a `span` into the input:

```json
{
  "provenance": {
    "matra": "0.3.0",
    "schema": "2.0",
    "parser": { "name": "udpipe", "model": "english-ewt-ud-2.5-191206", "sha256": "…", "license": "CC BY-NC-SA 4.0" },
    "format": "markdown",
    "source": { "path": "notes.md", "bytes": 1832, "sha256": "…" }
  },
  "metrics": { "total_words": 412, "total_sentences": 19, "passive_ratio": 0.11 },
  "sections": [ { "heading": "Field notes", "span": { "…": "…" }, "paragraphs": [] } ]
}
```

A summary adds what shaped it: `"method": "textrank", "n": 3`. The text fields keep their 0.2 meaning, now documented as normalized; the span, not the text, is the authority on which source characters a value came from. Which unit a span counts in is decision 3.

### Errors carry a kind everywhere

```python
try:
    m.analyze(huge)
except matra.InputTooLarge as e:     # still a ValueError, so old except clauses keep working
    print(e.kind, e.details["limit"], e.details["actual"])
```

```json
{ "format_version": 2, "command": "analyze", "input": "big.md", "result": null,
  "error": { "kind": "input_too_large", "message": "…", "details": { "what": "input", "limit": 8388608, "actual": 9000000 } } }
```

### Migrating

For 0.3.x, every renamed Rust item and Python method keeps its old name as a deprecated alias (`#[deprecated]` in Rust, a `DeprecationWarning` in Python that names the replacement call); every renamed JSON field is emitted under both its old and new names; renamed CLI flags stay as hidden aliases; and `matra ... --json --format-version 1` emits the 0.2 output. A reader detects which side it is on by `provenance.schema`, which is absent before 0.3.0. Removal happens in 0.4.0. The migration guide lists every rename on one page.

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
| feed | `Engine::feed(Format) -> Feed` (decision 2) | events, then the `Document` |

Extractors take a `&Document`, not a cloned `&[Sentence]`, so a parse is reused and its provenance travels; the richer mechanism is built using the simpler one [parnas-modules:c33]. Routing extraction through the pipeline is the port's policy, owned there once. A mechanism-only layer is never wholly free of policy [card:levin-1975-policy-mechanism] (unverified reading), so the port names the policies it owns rather than claiming to have none.

**Adapters translate and decide nothing.** The Python binding moves out of `lib.rs` into its own module and is held to boundary rule 7, the public surface only, with a semgrep rule. The command line keeps no application rule. `matra.Engine()` gains a constructor; `Matra` stays as an alias. Cockburn's ports are purposeful conversations, driving or driven, and adapters translate across them [card:cockburn-hexagonal-current] (unverified reading; Cockburn is in the canon-0010 cards, not in the verified claims). The verified grounding is the observed drift above and Henning's "an API is a user interface" [api-design-practice:c21].

**A cross-surface parity test.** The same inputs through Rust, Python and the command line must give the same `Document`, the same summary and the same error `kind`. A promise that nothing leaks across a boundary holds only with "a mechanism to detect when a violation of that promise occurs" [card:cockburn-hexagonal-current] (unverified reading), and testing finds what structure is meant to prevent [dijkstra-structure:c30].

### The schema

```text
Document         { provenance, sections, metrics: DocumentMetrics }
DocumentMetrics  { total_words, total_sentences, paragraph_count, mean_sentence_length,
                   sentence_length_std, vocabulary_ttr, nominalization_ratio, passive_ratio }
Section          { heading, level, span, paragraphs }
Paragraph        { span, text, in_blockquote, sentences, metrics: ParagraphMetrics }
ParagraphMetrics { readability_grade, lexical_density, compression_ratio }
Sentence         { span, text, tokens, word_count, tree_depth: Option, passive,
                   negations, modals, clausal_complements, root_adverbials, hearst_pairs, root_mood }
Token            { id, span, text, lemma, pos, xpos, features: map, head, dep, deps, misc }
Provenance       { matra, schema, parser: Identity, format, source, method?, parameters? }
Identity         { name, model, sha256, license }
Summary          { provenance, sentences: [SummarySentence { index, span, text, score: f64 }] }
Keyphrases       { provenance, phrases: [Keyphrase { phrase, score: f64, occurrences: [span] }] }
CorpusEntry      { path, document }
```

- **Aggregates are fields.** What the command line prints and Python rebuilds becomes data: if data exists only in string form, the string becomes a de facto API [api-design-practice:c11]. `ParagraphMetrics` and `DocumentMetrics` are names RFC-0006 reserved, and no outer `Option` wraps either record, because each slot gates on its own.
- **CoNLL-U is converted.** `features` is a map; the raw `feats` string is emitted beside it for 0.3.x and removed in 0.4.0, so the fact has one home after the window [parnas-process:c14].
- **Sentinels become `Option`.** `tree_depth` is `None` on a cycle instead of `usize::MAX`; the visited-set walk is unchanged.
- **Scalars cross as `f64`.**
- **Record facts are named for what the detector finds.** `ClausalComplement` and `clausal_complements` replace `Reporting` and `reportings`; `root_mood` (the mood feature of the root, with the root's token id) replaces the `bare_assertion` flag. Name a thing by what it is [dijkstra-systems:c2]; what must stay static is "the semantics of the mapping" [api-design-systems:c42]. `root_mood` is a retype, and its values are an open set.

### Versioning and the consumer rules

<claim id="format-version-ships" basis="observed">The command line already carries `format_version: 1`, which "increments on any change to the envelope or to the meaning of a field inside `result`" [cli/mod.rs:39](../../src/cli/mod.rs#L39 "const FORMAT_VERSION: u32 = 1;") [json.md:23](../../skills/matra/references/json.md#L23 "It increments on any change to the envelope or to the meaning of a field inside")</claim>. A second, weaker number beside it would weaken what agents were already told to rely on.

- **One schema version, in one place.** `provenance.schema` is `"major.minor"` and lives nowhere else [parnas-process:c14]. 0.2's implicit schema is 1; this release is `2.0`.
- **What moves which number.** A minor adds optional fields only, invisible to old readers [machine-readable-contracts:c18]. A major is anything else: a field removed, renamed or retyped, or a field whose meaning changes, including a changed default, since defaults are behaviour [api-design-systems:c25] and semantic compatibility is compatibility [api-design-systems:c21].
- **`format_version` keeps its rule** and moves whenever the envelope changes or the schema major changes. A minor changes no existing field's meaning, so it moves neither, as today.
- **Rules for consumers.** Ignore fields you do not recognize; treat every string set you receive as open (`kind`, `method`, `root_mood`, `pos`, `dep`); reject a major you do not know. That is Terraform's stated policy for its JSON output [machine-readable-contracts:c35], and Kubernetes found that "adding a new enum value breaks backward compatibility" for enums clients receive [api-design-systems:c34]. Without these rules, "adding a field does not change the version" would hold for Rust callers (whose types are `#[non_exhaustive]`) and not for Python or JSON readers.
- **A published schema.** `spec/schema/` holds a JSON Schema per major, validated against every conformance output in every binding, and printed by `matra --schema`. A contract published as a machine-readable artifact lets programs discover what a tool returns without its source [machine-readable-contracts:c36]. It is hand-written, or written by a dev-only test, because deriving it on the domain types would add a dependency to `domain.rs`, which boundary rule 1 forbids without an RFC.
- **Calibration.** Strict compatibility rules assume many callers whose upgrades you do not control; interfaces whose callers are co-owned "should set their own requirements" [machine-readable-contracts:c19]. These rules govern the wire from 2.0 on; the crate stays 0.x.

<sketch-figure id="versioning" seed="7" title="One schema version, the envelope number that moves with it, and the consumer rules">

```text
width 400
state today shipped "one integer on the CLI envelope; the Python dict and the result carry none"
box env "CLI envelope\nformat_version 1" 4 10 120 34
box result "result: Document\nno version" 150 10 120 34
box py "Python dict\nno version" 150 64 120 34
edge env result
state proposed proposed "one version in provenance; the envelope moves with its major"
box env "CLI envelope\nformat_version 2" 4 10 112 34
box py "Python dict" 4 112 112 30
box doc "Document\nprovenance.schema\n2.0" 140 54 120 44
box js "JSON Schema\nper major" 280 6 118 34
box minor "minor: optional\nfields added" 280 58 118 34
box major "major: removed,\nrenamed, retyped,\nmeaning changed" 280 106 118 44
box consumer "Consumer: ignore\nunknown fields, open\nsets, reject new major" 120 162 160 44
edge env doc
edge py doc
edge doc js
edge minor doc
edge major doc
edge major env
edge doc consumer
```

</sketch-figure>

### Provenance

`provenance` sits on `Document`, `Summary`, `Keyphrases` and `Clusters`. Derived results copy the document's provenance and add their own `method` and `parameters` (`n`, `threshold`, the embedder's identity). The parser reports its own identity, as the embedder already does: <claim id="embedder-identity" basis="observed">two embedders "that can disagree must not share an identity" [embed/mod.rs:41](../../src/embed/mod.rs#L41 "embedders that can disagree must not share an identity")</claim>. The model is the decision most likely to change (a permissively licensed model is planned), so a consumer must see which model produced a parse, rather than discover it as a shift in the numbers; an interface states "the meaning and limits of information exchanged" [parnas-process:c46]. The input digest is computed in `annotate` with `sha2`, which becomes a required dependency of the crate, not of `domain`. There is no timestamp (own judgment: matra's outputs are deterministic, conformance compares them, and a timestamp would make two identical analyses differ).

### Ports and their contracts

- **`Parser`** (was `NlpProvider`) gains `identity()`, keeps the contract `NlpProvider` states today, and adds two clauses: sentences in text order, and each token's span within the text it was given. The composition root shifts those spans to absolute offsets in `annotate`, the one choke point (<claim basis="observed">each paragraph is parsed there [lib.rs:239](../../src/lib.rs#L239 "para.sentences = self.nlp.parse(&para.text)?;")</claim>). The UDPipe adapter takes token positions from UDPipe's own `ranges` tokenizer option, which needs a one-line change in udpipe-rs; until then it aligns token forms against the paragraph text and returns `ParseFailed` naming the token when alignment fails, never a guessed span. A handler that returns normally deceives the caller [neighbours-contracts-simplicity:c14].
- **`Decomposer`** returns blocks (headings and paragraphs) with their spans, and a settled offset: the point after which appending text cannot change a block. The composition root builds the section tree from the blocks and slices paragraph text from the source. When this lands depends on decision 2.
- **Shared contract tests.** Each port gets one test that every adapter must pass. The test states which subtyping rule it enforces: the everyday one (an adapter may weaken a precondition and strengthen a postcondition) is Meyer's subcontracting rule [neighbours-contracts-simplicity:c12]; Liskov and Wing's strict extension-map rule requires equal preconditions [card:liskov-wing-1994-subtyping] (unverified reading).
- **No migration path for outside implementors.** A blanket `impl<T: NlpProvider> Parser for T` would need a default `identity()`, and the embedder rule above makes any default dishonest. <claim id="no-outside-implementors" basis="assumed">No parser or decomposer is implemented outside matra.</claim> Provider interfaces cannot grow without breaking every implementor [api-design-practice:c32], which is why each changes once, here, together with its implementation, and not again in 0.4.0.
- **Model acquisition** stays two copies, as RFC-0015 decided; copying a small component is "usually the right thing" [neighbours-hoare-brooks-lampson:c47]. The second pinned parser model is the moment to revisit it.

### Errors on every channel

- **Rust.** `Error` keeps its seven variants and `kind()`, and gains `details()`, every dynamic value in the message as structured data, and a serde form `{kind, message, details}`. `DocumentError` serializes as that plus `path`, so the hand-built projection in the Python binding is deleted. Every dynamic value in an error message must also be present as metadata [machine-readable-contracts:c13].
- **Python.** `matra.MatraError` with `.kind` and `.details`, and one subclass per kind that also inherits today's builtin class (`InputTooLarge(MatraError, ValueError)`), so every documented exception still catches [api-design-practice:c39]. `Io` routes on the kind it wraps: `FileNotFoundError`, `PermissionError`, else `OSError`.
- **Command line.** Under `--json`, a failure writes the envelope with `"result": null` and an `error` object to stdout, writes `matra: <message>` to stderr, and exits 2. On success `"error": null`; both keys are always present. The command line's own refusals use the existing kind whose remedy matches: a bad flag and a contract violation are both "fix the call", so both are `invalid_input`. Error identity is defined by what the caller would do about it [api-design-systems:c28].
- **Exit codes** stay 0 (found), 1 (nothing found), 2 (failed). In `analyze DIR --json`, each document is one envelope per line; a failed document is a line with its `error`, the walk continues, and the exit code is 2 if any document failed. Per-item failure in a bulk operation is the one sanctioned partial success [api-design-systems:c30].
- **Retry.** Download failures carry `details.retryable: true`. Whether a retry is safe depends on both the operation and the failure [machine-readable-contracts:c16].

### Limits

The input cap, the sentence caps and the metric minimums move into one module as published constants. Raising a cap is left out of this release: the canon does not cover resilience bounds, and the resilience floor keeps safe defaults until its own reviewer rules.

### Defaults

Library defaults are written in the signatures. Configuration applies at the command line and in the explicit `from_config` constructors. Defaults "should be written explicitly", and changing one is breaking [api-design-systems:c38] [api-design-systems:c25]; hidden state is a source of agent error [agent-computer-interfaces:c18]. The effective parameters are recorded in `provenance`.

### Names

| 0.2 | 0.3 | Kind |
|---|---|---|
| `NlpProvider` | `Parser` | trait |
| `Engine::compose` | `Engine::measure` | method |
| `tfidf_summarize`, `textrank_summarize` | `summarize` | function |
| `rake_keyphrases`, `yake_keyphrases` | `keyphrases` | function |
| `embed_and_cluster`, `semantic_clusters` | `clusters` | function |
| `Reporting`, `Sentence.reportings` | `ClausalComplement`, `clausal_complements` | type, field |
| `Sentence.bare_assertion` | `Sentence.root_mood` | field (retyped) |
| `SemanticClusters.model_hash` | `identity` | field |
| `CorpusEntry.analysis` | `CorpusEntry.document` | field |
| `Paragraph.readability_grade` and siblings | `Paragraph.metrics.*` | fields (moved) |
| `Document.vocabulary_ttr` and siblings | `Document.metrics.*` | fields (moved) |
| Python `Matra`, `analyze_markdown` | `Engine`, `analyze(format=)` | class, method |
| config `[summarize]`, `semantic.*` | `[summary]`, `[clusters] threshold` | keys |

A rename removes the old name and adds a new one [api-design-systems:c24], which is why each keeps its old name for 0.3.x. The renames of `Parser` and `measure` cost outside implementors nothing extra: the trait breaks in this release anyway, because it gains `identity()`. A Rust trait alias cannot be implemented, so the old trait name is kept as a deprecated re-export only.

### Stability, by surface

| Surface | Promise |
|---|---|
| Rust items | 0.x: breaking changes only in a minor bump, with deprecated aliases for one minor |
| Python names and exception classes | the same as Rust |
| CLI commands, flags, exit codes | the same; removed flags stay as hidden aliases for one minor |
| JSON schema | the major and minor rule above |
| CLI envelope | `format_version`, moving with the schema major |
| Skill text (`matra --skill`) | versioned with the binary; a changed incantation is listed in the CHANGELOG, and the skill changes in the same pull request as the surface it describes |

<claim id="semver-pr-only" basis="observed">`cargo-semver-checks` runs on pull requests only [ci.yml:318](../../.github/workflows/ci.yml#L318 "if: github.event_name == 'pull_request'")</claim>. It is pinned and also runs on pushes to `main`, so the Rust promise above has a mechanical check.

### Agents

The skill and its references change in the same pull request as each surface change. A model reproduces whatever surface it last read, and the printed skill is the current documentation at call time. Deprecation warnings name the replacement, since error text is an instruction to an agent [agent-computer-interfaces:c16]. `analyze --json` gains `--depth document|paragraph|sentence|token`, default `token` (today's output), because every token shown costs the caller [agent-computer-interfaces:c2] and the caller should choose the detail [agent-computer-interfaces:c11]. This rests on vendor guidance; no source measures CLI design for agents.

### Release split

| Release | Contents |
|---|---|
| 0.3.0 | Every break: the port, the names, the schema 2.0 and its JSON Schema, provenance, spans and the parser and decomposer contracts with their implementations, errors on every channel, the transition window, the parity test, the stability table, the semver gate; and, if decision 2 says so, the narrow feed |
| 0.3.x | The transition window open |
| 0.4.0 | The window closes; Python's feed; caller extensions (Predicates and Findings) or x.uma extensions in their own proposal; schema-typed TypeScript and a web component that renders a parse; the result of a UDPipe-in-WebAssembly spike; the permissive parser model as a pin chosen by id |

Lehman reports that what a release can carry is limited by how fast people absorb its changes [card:lehman-1996-laws-revisited] (unverified reading). That is a caution on 0.3.0's size, and why the narrow feed is the first item to move if 0.3.0 grows.

### Milestones

1. The schema: JSON Schema, fixtures, metric records, names, both-name emission.
2. The application port; adapters reduced to translation; Python out of `lib.rs`; the parity test.
3. The parser contract, CoNLL-U conversion and spans; the decomposer contract with both decomposers.
4. Errors on every channel; Python's `Engine` and exception classes.
5. The command line: directories, `clusters`, `--json` errors, `--format-version 1`, `--depth`, config sections; the skill.
6. If decision 2 says so, the feed and `--stream`.
7. The migration guide, the docsite, the semver gate, and the 0.3.0 release.

## Drawbacks

- Every caller migrates. The window softens it, but moving metrics into records has no both-name form in Python dicts: a reader of `paragraph["readability_grade"]` must read `paragraph["metrics"]["readability_grade"]` by 0.4.0.
- Spans and provenance make `analyze --json` larger; `--depth` mitigates it for agents.
- `--format-version 1` is a compatibility projection maintained for one minor, the price of not breaking pinned agents [neighbours-hoare-brooks-lampson:c39].
- One release carries a lot of change, so its review is long.

## Rationale and alternatives

- **One proposal, not several.** An API is designed as a whole: the names, the port and the schema constrain each other.
- **Keep algorithm-named functions.** Rejected by both sides of decision 1; the open question is whether the choice survives as a parameter.
- **A second version number with a weaker rule.** Rejected: it weakens the promise already printed to agents.
- **Change a provider contract now and implement it later.** Rejected: an abstraction should not ship before implementations have tested it [api-design-practice:c36], and "it would be rash to standardize before we have the models" [neighbours-systems-evolution:c27]. Parnas's A-7E team specified mechanisms ahead and implemented only what was needed [parnas-process:c50], which supports reserving names, as RFC-0006 does, not freezing a provider trait.
- **A hard JSON break with no window.** Rejected: old and new should coexist for a transition period [machine-readable-contracts:c20].
- **Spread the renames across releases.** Each release would break again.
- **Do nothing.** The drift is structural; a fourth surface would copy it.

## Prior art

- The owner's software-design canon (462 verified claims; cited here as `[cluster:cN]`) and its canon-0010 reading cards (cited as `[card:...]`, unverified readings).
- J. Bloch, "How to Design a Good API and Why it Matters", 2006; M. Henning, "API Design Matters", 2007; AIP-180 and AIP-193; the Kubernetes API conventions.
- D. L. Parnas, "On the Criteria To Be Used in Decomposing Systems into Modules", 1972, and the 1985 module guide.
- A. Cockburn, "The Hexagonal (Ports and Adapters) Architecture", 2005.
- Terraform's JSON output format and its `format_version`; `cargo metadata --format-version`.
- The W3C Web Annotation Data Model, whose text position selector counts Unicode code points.
- RFC 9413, "Maintaining Robust Protocols", 2023.
- Universal Dependencies and CoNLL-U: the vocabulary matra adopts, and the encoding it converts.
- spaCy's `Doc` attributes as fields, and its `Language` object as one entry point.

## Unresolved questions

<decision id="algorithm-choice" title="1. Does the caller keep a choice of method?" reversible="costly" grounds="algorithm-names-today cli-method-flag score-scales default-method use-cases-no-algorithm">

<choice key="a" title="No: outcomes only, the method recorded">

`summarize(&doc, n)` and `keyphrases(&doc, n)` with no way to choose. matra picks the method; every result names the one that ran. `--method` and the `algorithm` config keys are removed (as hidden aliases for 0.3.x). A typed selector can be added later without breaking anyone.

</choice>

<choice key="b" title="Yes, optionally: outcomes first, a typed method on request">

The same outcome calls are the common form. An optional typed method selects explicitly: `summarize_with(&doc, n, SummaryMethod::TextRank)` in Rust, `method="textrank"` in Python (a closed set of literals), `--method` on the command line. Every result records the method that ran. No function name carries an algorithm.

</choice>

<recommendation choice="b">

**This goes against the owner's stated position.** The owner asked "why are the methods implementation specific? that breaks apis". On names, the canon agrees with the owner, and both choices take the names out: a likely change should touch no interface [parnas-process:c22]. On removing the choice, it does not. The canon card's fifth rule is to hide undesirable properties, not desirable ones: "desirable ones should not be hidden" [neighbours-hoare-brooks-lampson:c37]. Which method ran decides what a score means: <claim id="score-scales" basis="observed">the skill tells agents the RAKE and YAKE scales are unrelated [SKILL.md:105](../../skills/matra/SKILL.md#L105 "The two scales are unrelated")</claim>, so a caller comparing scores, or replicating a published result, needs to name the method. <claim id="cli-method-flag" basis="observed">The command line already offers `--method` [cli/mod.rs:180](../../src/cli/mod.rs#L180 "method: Option<SummaryMethod>")</claim>, so keeping a choice adds nothing; removing it is the break. A method the caller *sends* can gain values without breaking anyone, which is why the selector is an input and the recorded method is an open string (this send-versus-receive distinction is the canon synthesis's own suggestion, not a source finding). Choice (b) keeps the outcome call as the common form, so the owner's API is the one most callers write.

</recommendation>

<against>

Leave it out when in doubt, because "you can always add, but you can never remove" [api-design-practice:c6], and a feature included before it is fully understood "can never be removed later" [neighbours-hoare-brooks-lampson:c13]. The ranking algorithm is a legitimate secret [card:cunningham-2019-modular-design-notes] (unverified reading, secondary via Cunningham). <claim id="use-cases-no-algorithm" basis="assumed">None of the 26 use cases in the API audit behind this proposal asks for an algorithm; they ask for a summary or for keyphrases.</claim> Under (a), changing the method behind the default is still a schema-major change, so a caller is told when results move: <claim id="default-method" basis="observed">today's default summary method is TF-IDF [default.toml:8](../../config/default.toml#L8 "algorithm")</claim>. Choice (a) is the smaller surface, and the canon's review of the earlier text found that the owner's position is the canon's majority reading on names.

</against>

</decision>

<decision id="streaming-timing" title="2. When does text that arrives over time get its implementation?" reversible="costly" grounds="decomposer-today sync-load-bearing paragraph-unit">

<choice key="a" title="0.3.0: the contract with a narrow feed">

`Decomposer` returns blocks with spans and a settled offset, implemented for markdown and plain text. `Engine::feed(format)` returns a synchronous `Feed` whose `push(chunk)` returns the events for every block that became settled and whose `finish()` returns the rest and the whole `Document`. `matra analyze - --stream --json` writes one envelope per event line. A new law, L8, joins the seven in `src/lib.rs`: for any text and any split of it into chunks, the feed's `Document` equals `analyze_one` of the whole text. Python's feed follows in 0.4.0.

</choice>

<choice key="b" title="0.4.0: a new trait beside Decomposer">

0.3.0 leaves streaming out entirely. In 0.4.0 a new `StreamingDecomposer` trait (or a settled-offset method on a new trait) arrives beside `Decomposer`, which is additive, with the feed in Rust, Python and the command line together.

</choice>

<recommendation choice="a">

The owner settled that streaming happens and placed its contract in the 0.3.0 window. A contract without an implementation is the one outcome the canon rules out [api-design-practice:c36] [neighbours-systems-evolution:c27], so the contract arrives with one. <claim id="decomposer-today" basis="observed">Today `Decomposer` returns a tree of copied strings [decompose/mod.rs:17](../../src/decompose/mod.rs#L17 "fn decompose(&self, text: &str) -> Vec<Section>;")</claim>, and it must change in 0.3.0 for spans anyway; folding the settled offset into the same change breaks this provider interface once, not twice [api-design-practice:c32]. The feed stays synchronous: <claim id="sync-load-bearing" basis="observed">RFC-0007 records that staying synchronous is "load-bearing for this design" [RFC-0007:109](../legacy/rfcs/0007-one-pipeline.md#L109 "load-bearing for this design")</claim>, and a `push` that runs each settled paragraph to completion keeps that property. <claim id="paragraph-unit" basis="observed">The paragraph is already the unit the parser runs on [lib.rs:239](../../src/lib.rs#L239 "para.sentences = self.nlp.parse(&para.text)?;")</claim>, so it is the unit of finality: a sentence boundary can still move while its paragraph grows (own judgment, from how the adapter groups words by sentence id). Shipping only Rust and the command line keeps the frozen surface to the two places that cost least to change, since one consumer is not several [api-design-practice:c4]. If 0.3.0 grows too large, this is the first item to move, and it moves as choice (b).

</recommendation>

<against>

No streaming consumer is named yet, and the canon would freeze an interface only after several implementations and consumers have used it [api-design-practice:c36]. A new trait in 0.4.0 is additive and leaves `Decomposer`'s 0.3.0 change to spans alone. Choice (b) also ships all three syntaxes of the feed at once, which is what "one API, three syntaxes" asks for, rather than two of three.

</against>

</decision>

<sketch-figure id="feed" seed="9" title="Text arriving over time: blocks emitted once settled, and the law that ties the feed to analyze_one">

```text
width 400
state feed proposed "chunks in, settled paragraphs out, the whole document at finish (decision 2a)"
box c1 "chunk 1" 4 4 90 26
box c2 "chunk 2" 110 4 90 26
box c3 "chunk 3" 216 4 90 26
box buffer "Feed: text from the\nlast settled offset" 100 48 200 34
box decomposer "Decomposer: blocks\nwith spans, settled" 4 104 180 34
box parser "Parser: each settled\nparagraph, once" 216 104 180 34
box events "Paragraph events\none NDJSON line each" 4 162 180 34
box end "finish: Document\nL8: equals analyze_one" 216 162 180 34
edge c1 buffer
edge c2 buffer
edge c3 buffer
edge buffer decomposer
edge decomposer parser
edge parser events
edge events end
```

</sketch-figure>

The cumulative input of a feed is capped at the same 8 MiB as a document, with the same `what` label, and the unsettled tail is capped too, so a stream that never settles a block is refused rather than buffered without bound.

<decision id="offset-unit" title="3. What unit does a span count in?" reversible="costly" grounds="rfc0006-span sentence-text-rebuilt radix-asks-characters">

<choice key="a" title="UTF-8 bytes only">

`span: { start, end }` in UTF-8 bytes of the input. Each binding offers a helper (`matra.span_text(source, span)` in Python, the same in TypeScript) to slice by it.

</choice>

<choice key="b" title="Code points and bytes">

`span: { start, end }` in Unicode code points, as the W3C Web Annotation Data Model's text position selector counts them, plus `byte_start` and `byte_end` in UTF-8 bytes. Both are computed once, in Rust, from one source.

</choice>

<recommendation choice="b">

<claim id="radix-asks-characters" basis="assumed">The distillation tool downstream of matra asked for character offsets.</claim> Python slices strings by code point, and the one standard for text selection counts code points; Rust slices, and digests verify, by byte. Under (a), every Python and TypeScript caller writes the same conversion, and Meyer's rule for that case is to widen the contract: if every client repeats the same handling, the supplier should take it on [neighbours-contracts-simplicity:c8]. Exposing code points also avoids calling byte offsets `start` and `end`, names the standard gives another meaning. Giving callers both is "programmatic access" to what they would otherwise compute themselves [api-design-practice:c11]. JavaScript indexes by UTF-16 unit, which neither choice serves; the TypeScript helper converts from code points.

</recommendation>

<against>

One unit is the smaller surface [api-design-practice:c6], and two encodings of one fact on every token cost every reader, agents included, for each token shown [agent-computer-interfaces:c2]. RFC-0006 sketched `SourceSpan` in bytes, and bytes are the one unit that is identical in every binding, cheap to check against the input's digest, and what the input cap already measures (own judgment; the canon is silent on offset units). A helper per binding is one line, and the conversion lives in matra either way.

</against>

</decision>

<decision id="metric-suite" title="4. Does the public metric suite become crate-private?" reversible="costly" grounds="compose-ignores-suite metric-type">

<choice key="a" title="Yes, in 0.3.0">

`metrics::Metric`, `default_suite` and `run_suite` become private to the crate. A caller who wants another measure writes a function over `&Document`. Caller extensions are designed in their own proposal (0.4.0).

</choice>

<choice key="b" title="No; leave them public">

The three items stay, documented as a suite the caller may run themselves.

</choice>

<recommendation choice="a">

<claim id="compose-ignores-suite" basis="observed">`compose` always runs the default suite and takes none from the caller [lib.rs:251](../../src/lib.rs#L251 "let suite = metrics::default_suite();")</claim>, so the public suite extends nothing a caller could not do with a loop. <claim id="metric-type" basis="observed">A metric is a closure that may write any field of the document [metrics/mod.rs:31](../../src/metrics/mod.rs#L31 "pub type Metric = Box<dyn Fn(&mut Document)>;")</claim>: a provider interface with no stated contract about what it owns. What clients call and what providers implement should be kept apart [api-design-practice:c31]; the cheapest extension mechanism that meets the need is a plain function over published types [api-design-practice:c35]. Removing it inside the 0.3.0 window costs one migration; removing it later costs another.

</recommendation>

<against>

It is shipped public surface, and with enough users every observable behaviour is depended on [api-design-systems:c17]. No caller has been shown to use it, and none shown not to. matra's own ACES skill lists `default_suite` as part of its adaptable surface, and caller extensions are deferred to 0.4.0 anyway, where the suite could be redesigned with them.

</against>

</decision>

What the owner should confirm or strike, collected from the claims above that stand on no grounds yet:

<assumptions />

## Future possibilities

- **Caller extensions**: Predicates producing Findings, the names RFC-0006 reserved, in their own proposal, on this schema and port.
- **x.uma extensions**, implemented in matra behind a feature named for the dependency, as a driving adapter over the public surface with typed configs per extension, once the x.uma crates are published. Typed extension configs are the mechanism the ACES synthesis names for letting contributors add capability without platform mediation [aces:aces-synthesis.md§10].
- **TypeScript**, in three steps, each usable alone: types generated from the published schema and a web component that renders a parse from matra's JSON (no WebAssembly needed); then the pure-Rust core in WebAssembly; and a spike, with a go or no-go, on compiling UDPipe's C++ with exceptions to WebAssembly. Start from the minimal subset that performs a useful service [parnas-modules:c34].
- **A permissively licensed default parser** trained on UD English EWT, as a second pinned model chosen by id, never by path or URL, visible in `provenance.parser`.
- **Python's feed**, if decision 2 lands the feed in 0.3.0.

## Defects in what ships

Found while writing this proposal. Each stands apart from the design, and they are fixed in a separate pull request.

| # | Defect | Evidence |
|---|---|---|
| 1 | The roadmap says sentence and paragraph text are verbatim; the code rebuilds both | <claim basis="observed">[ROADMAP.md:193](../../ROADMAP.md#L193 "being verbatim by design") against [udpipe.rs:778](../../src/nlp/udpipe.rs#L778 "Reconstruct original text using SpaceAfter=No")</claim> |
| 2 | Markdown paragraph text drops `\r` from `\r\n` input and strips `>` from blockquotes | <claim basis="observed">[markdown.rs:15](../../src/decompose/markdown.rs#L15 "text.lines()") [markdown.rs:78](../../src/decompose/markdown.rs#L78 "trim_start_matches('>')")</claim> |
| 3 | Python's extractors read markdown as plain text, so a README's headings are summarized; the command line does not | <claim basis="observed">[lib.rs:487](../../src/lib.rs#L487 "Format::PlainText") against [cli/mod.rs:612](../../src/cli/mod.rs#L612 "on a README would return its headings")</claim> |
| 4 | `total_words` and the other aggregates are printed in the table and absent from `--json` | <claim basis="observed">[render.rs:45](../../src/cli/render.rs#L45 "doc.total_words()")</claim> |
| 5 | A failure under `--json` writes nothing to stdout | <claim basis="observed">[cli/mod.rs:102](../../src/cli/mod.rs#L102 "matra: {e}")</claim> |
| 6 | `Document::passive_ratio()` returns `0.0` on a document with no sentences, while the field stays `None` | <claim basis="observed">[domain.rs:1185](../../src/domain.rs#L1185 "pub fn passive_ratio(&self) -> f64") [document.rs:19](../../src/metrics/document.rs#L19 "if analysis.total_sentences() > 0")</claim> |
| 7 | `cargo-semver-checks` runs on pull requests only, though EPR-0003 says every push | <claim basis="observed">[ci.yml:318](../../.github/workflows/ci.yml#L318 "if: github.event_name == 'pull_request'")</claim> |
| 8 | A missing file reaches Python as `OSError`, not `FileNotFoundError` | <claim basis="observed">[lib.rs:446](../../src/lib.rs#L446 "Io(_) => PyOSError::new_err(msg)")</claim> |
| 9 | The main size gate's message reads "input input too large" | <claim basis="observed">[domain.rs:166](../../src/domain.rs#L166 "input too large: {actual} > limit {limit}") [lib.rs:34](../../src/lib.rs#L34 "what: \"input\"")</claim> |
| 10 | The roadmap names `analyze_directory`, which does not exist | <claim basis="observed">[ROADMAP.md:118](../../ROADMAP.md#L118 "analyze_directory")</claim> |
| 11 | The embedding model's pin carries no licence, unlike the parsing model's, and its first download is silent | <claim basis="observed">[model2vec.rs:66](../../src/embed/model2vec.rs#L66 "const POTION_BASE_8M_URLS") against [udpipe.rs:40](../../src/nlp/udpipe.rs#L40 "CC BY-NC-SA 4.0 (non-commercial)")</claim> |
| 12 | `vocabulary_ttr` is unsound across documents of different length, and nothing on the type says so | <claim basis="observed">[ROADMAP.md:118](../../ROADMAP.md#L118 "As a cross-document feature it is currently unsound")</claim> |
| 13 | matra's ACES skill pairs the counter-forces with the decay forces differently from the ACES synthesis it comes from (Composable against drag here; Composability against opacity, and Extensibility against drag, in the synthesis's framework table) | <claim basis="observed">[aces SKILL.md:43](../../.claude/skills/aces/SKILL.md#L43 "Counters drag.")</claim> |
| 14 | The previous text of this proposal placed `schema_version` in two places and left the shipped `format_version` out | <claim basis="observed">[0006 at 6e9d94b:244](0006-the-0-3-0-surface.md#L244 "schema_version") [0006 at 6e9d94b:301](0006-the-0-3-0-surface.md#L301 "Provenance")</claim> |
