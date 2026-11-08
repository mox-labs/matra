# EPR-0006: The 0.3.0 surface

- Feature Name: `surface_0_3`
- Start Date: 2026-10-04
- Pinned at: `6e9d94b580d6cd442bf74f30a6c05ade5329ca0d` (every code citation below opens at this commit)
- Proposal PR: [#134](https://github.com/mox-labs/matra/pull/134)
- Tracking issue: [#161](https://github.com/mox-labs/matra/issues/161)
- Status: implemented

## Summary

0.3.0 settles matra's public surface once, as one design, before a TypeScript surface copies it. A caller asks for an outcome: `analyze` a document, `analyze_all` of many, then `summarize`, `keyphrases` or `clusters` of the result. Rust, Python and the command line spell those words the same way, and none of them decides anything the others do not. The method behind an outcome is optional, never in a name, and recorded on the result with every parameter that shaped it. Input is cut by decision, not by processing step: where bytes come from, which format they are, how that format is read, how blocks become sections, and how text is parsed are each one module with private adapters, so a new format or a new parser in the tree changes no published signature, and a caller can plug in a source, reader or parser of their own, checked before it reaches a result. A caller with a format matra does not read hands in headings and paragraphs directly. Every result carries its provenance, points back into the analysed text by byte span, and lists what was skipped rather than dropping it silently. The document schema becomes a governed interface with a published JSON Schema, and failures carry a kind on every channel. Every break lands in 0.3.0, at once.

The owner has answered every decision, and each ruling is recorded in its decision below. What remains is acceptance.

<pragmatics>

<ask>

Accept the proposal by merging the pull request that changes its status to `accepted`.

</ask>

<will>

- Keep the tracking issue, [#161](https://github.com/mox-labs/matra/issues/161), current with the milestones under [Milestones](#milestones).
- Start the milestones on branches once the rulings are recorded, and merge none of them before you merge the acceptance.
- Land them in order, one pull request each into the integration branch `next`, with the documentation, the skill and the CHANGELOG in the same pull request as the code they describe. `next` merges into `main` with the release, because the docsite deploys from `main` and describes only what ships.
- Leave each milestone in a state a later session can pick up cold: the records current, no half-finished refactor, no second copy of a rule.

</will>

<needs>

- Your merge of the acceptance pull request.
- A reading of the rationale with one caveat in mind. This text merges a design by Claude, an independent design and a canon review by a second Claude agent, and an architect review by a third. All are Claude agents reading one corpus, so where they agree, that is one voice, not several.

</needs>

<wont>

- Merge code that relies on this proposal while it is `proposed`.
- Change its status to `accepted`, or merge a pull request that does.
- Publish a release without your approval.

</wont>

<silence>

The proposal stays `proposed` and nothing built on it merges. Claude raises the acceptance again at the start of the next session that touches the public surface.

</silence>

</pragmatics>

<changed date="2026-11-06" since="the text of 2026-10-06">

<was>

One application port named seven operations, including the stages `annotate` and `measure` and a `feed` for text arriving over time.

</was>

<now>

The public surface names outcomes only; the stages are internal, and the feed waits for 0.4.0 as an addition. The owner settled outcomes over stages in conversation on 2026-11-02, and the architect review found that a private reader contract makes a later feed non-breaking: [decision 2](#decision-streaming-timing).

</now>

<was>

The input side was the existing `Ingest` stream, the `Decomposer` and `NlpProvider` traits were public and changed once, and a caller could hand in only text of a known format.

</was>

<now>

Input is cut by decision into fetch, detect, read, structure and parse, their adapters private; `Input` and `Inputs` replace `Ingest`; a caller hands in blocks for a format matra does not read; and the result lists what was skipped. The owner settled the cut and the caller's own section breaks on 2026-11-02, after a consumer had to disguise plain text as markdown to get sections. See [the input side](#the-input-side-cut-by-decision).

</now>

<was>

A span counted code points and bytes, on the ground that the downstream consumer asked for character offsets.

</was>

<now>

A span counts bytes of the analysed text only. That consumer's own written requirements say spans into the bytes the caller passed are enough, so the ground was wrong: [decision 3](#decision-offset-unit).

</now>

<was>

The record paired each document with its path in `CorpusEntry`, and the path also sat in `provenance.source.path`.

</was>

<now>

`CorpusEntry` goes; the path has one home, in provenance [parnas-process:c14]. See [the schema](#the-schema).

</now>

<was>

Every renamed name, flag and JSON field stayed for 0.3.x, the JSON emitted under both names, and the command line could emit the 0.2 output.

</was>

<now>

Nothing keeps its old name: 0.3.0 is one clean break in every language and on the wire, which the schema's own version already announces: [decision 8](#decision-transition-window).

</now>

<was>

Keeping the parser private kept documents trustworthy: the outcomes could rely on the parser's invariants, and a `Document` handle stopped a summary carrying provenance no matra run produced.

</was>

<now>

A document is data: the public constructors and `Deserialize` stay so a saved document reads back, so every outcome holds on a document whose parts contradict each other, and a test feeds it such documents. Provenance records how a document says it was made, and is not proof. An architect review of the built surface found the gap; the owner ruled it, and [#178](https://github.com/mox-labs/matra/pull/178) carries the test and the one fix it needed. See [reference](#reference-level-explanation).

</now>

<was>

The embedder reported its identity as a string, and its trait stated no error rule.

</was>

<now>

The embedder's `identity` returns the `Identity` the parser reports, so the embedding model's licence reaches the result, and the trait names the kinds an implementation returns. The owner ruled it after the same review; [#178](https://github.com/mox-labs/matra/pull/178) implements it. See [rationale](#keep-the-embedder-as-epr-0005-shipped-it).

</now>

<was>

`domain::limits` was published with no alternative recorded, and changing a constant was called a breaking change.

</was>

<now>

The limits are published because a caller needs them before the call, and their values may change in any 0.x release. The owner ruled it; [#178](https://github.com/mox-labs/matra/pull/178) rewrites their documentation. See [rationale](#keep-the-limits-private).

</now>

<was>

`invalid_input` covered a contract breach, bad block input, a malformed config file and a failing embedder with nothing to tell them apart, and the size gates' labels named private modules, the file source and the UDPipe adapter.

</was>

<now>

`invalid_input` stays one kind, one remedy, with a `reason` naming each check, and the labels name what the caller reads, a file or a model download. The owner ruled both; [#178](https://github.com/mox-labs/matra/pull/178) implements them. See [reference](#reference-level-explanation).

</now>

<was>

The parser, reader and source traits were crate-private, ruled "for now" on 2026-11-03, because privacy was the choice that could be undone and the parser's output was trusted unchecked.

</was>

<now>

The four ports are public: `Engine::with_parser`, `with_reader` and `with_source` plug in a caller's own, and `src/conform.rs` checks what a parser or reader returns before it reaches a `Document`, failing with `parse_failed` and a stable `reason`. The owner ruled it on 2026-11-06: before 1.0 a break costs nothing, so the case for privacy was gone and the affordance stands. [#178](https://github.com/mox-labs/matra/pull/178) implements it. See [decision 5](#decision-provider-traits).

</now>

</changed>

## Motivation

### What the owner has settled

These are inputs here, not questions. From the owner's answers of 2026-09-27 and 2026-10-05:

- **English only, for now.** A second language is not a likely change, so the language may appear in names without a module to hide it. Interfaces carry what is unlikely to change [parnas-process:c20].
- **Streaming happens**, in both senses: a stream of documents (which exists) and text arriving over time within one document (which does not).
- **matra implements the x.uma extensions itself**, the way xDS extensions live with their implementers, once the x.uma crates are published. That waits for a later proposal.

From the owner's answers in conversation on 2026-11-02:

- **Outcome names, method optional.** Functions are named for what they produce (`summarize`, `keyphrases`). The method is optional: `summarize_with(.., SummaryMethod)` in Rust, `method=` in Python, `--method` on the command line. Every result records the method and its parameters, and changing a default method is a breaking change.
- **The caller's own section breaks.** A caller can hand in plain text with section breaks of its own, without disguising it as markdown.
- **Input split by decision, not by processing step.** Fetch (file, directory, stdin, text; later a URL), detect (from the name and from the bytes), read (one reader per format, emitting canonical blocks with spans), structure (blocks into sections), parse. OCR, when it comes, is an optional adapter the PDF reader calls. No HTML, DOCX, PDF or OCR in 0.3.0.
- **The public surface names outcomes, not stages.** `annotate` and `compose` (or `measure`) leave it.
- **What a default build reads.** Sources: file, directory, stdin, text. Readers: markdown and plain text. Parser: UDPipe. Embedder: none, `Model2Vec` staying behind its feature. A format the build cannot read fails with `UnsupportedFormat`, naming the feature to enable.

### The surface reports something it does not keep

The record does not say where its text came from, and the documentation says it does.

- <claim id="verbatim-claim" basis="observed">The roadmap tells callers that `Sentence.text` and `Paragraph.text` are "verbatim by design" [ROADMAP.md:193](../../ROADMAP.md#L193 "being verbatim by design")</claim>.
- <claim id="sentence-text-rebuilt" basis="observed">Sentence text is rebuilt from token forms joined by spaces, and the type's own documentation says its whitespace "can differ from the source" [udpipe.rs:778](../../src/nlp/udpipe.rs#L778 "Reconstruct original text using SpaceAfter=No") [domain.rs:723-726](../../src/domain.rs#L723-L726 "rebuilds it from the token forms")</claim>.
- <claim id="paragraph-text-rebuilt" basis="observed">Markdown paragraph text is split into lines, rejoined and trimmed, which drops a `\r` from `\r\n` input, and blockquote text loses its `>` markers [markdown.rs:15](../../src/decompose/markdown.rs#L15 "text.lines()") [markdown.rs:29](../../src/decompose/markdown.rs#L29 "trim().to_string()") [markdown.rs:78](../../src/decompose/markdown.rs#L78 "trim_start_matches('>')")</claim>.
- <claim id="rfc0006-span" basis="observed">RFC-0006 reserved `SourceSpan` as the "Source span pointer", sketched in bytes, for exactly this job [RFC-0006:46](../legacy/rfcs/0006-abstract-tier-vocabulary-lock.md#L46 "byte_offset, byte_length")</claim>, and nothing in the domain carries an offset today.

A downstream distillation tool re-located thousands of sentences in its source by search, which worked by luck of the text rather than by contract, and an agent reading the roadmap would trust text the code does not promise. Each fact belongs in "one, and only one, place" [parnas-process:c14]; here the documentation and the code disagree about one.

### The input side drops what it cannot read

The markdown reader is a line scanner, and what it does not recognise it drops without a word.

- <claim id="tables-dropped" basis="observed">A line starting with `|` is skipped, so every table vanishes [markdown.rs:55](../../src/decompose/markdown.rs#L55 "stripped.starts_with('|')")</claim>.
- <claim id="references-truncate" basis="observed">A `## References` line ends the document: everything after it is dropped, not only the references [markdown.rs:58-61](../../src/decompose/markdown.rs#L58-L61 "## References")</claim>.
- <claim id="hashtag-heading" basis="observed">Any line starting with `#` is a heading, so a `#hashtag` opens a section [markdown.rs:62](../../src/decompose/markdown.rs#L62 "stripped.starts_with('#')")</claim>. Lists are not recognised, and inline markup stays in the text the parser reads.
- <claim id="kind-planned" basis="observed">The domain already plans a `kind` to replace the blockquote flag [domain.rs:1023](../../src/domain.rs#L1023 "kind: ParagraphKind")</claim>, which the reader cannot fill while it knows no lists.
- <claim id="binary-as-io" basis="observed">A binary file reaches `read_to_string` and fails as an I/O error, prose rather than a kind with a remedy [file.rs:51](../../src/source/file.rs#L51 "read_to_string")</claim>.

A result that silently omits part of its input deceives the caller: a handler that returns normally after dropping something fools the caller "into believing that everything is going according to plan" [neighbours-contracts-simplicity:c14], and "don't fail silently" [api-design-practice:c10].

### The three surfaces have drifted apart

| Task | Rust | Python | Command line |
|---|---|---|---|
| Summarize a README | through the pipeline | <claim id="python-plain" basis="observed">reads Markdown as plain text, so headings are summarized as prose [lib.rs:487](../../src/lib.rs#L487 "Format::PlainText")</claim> | <claim id="cli-pipeline" basis="observed">through the pipeline, for that reason [cli/mod.rs:612](../../src/cli/mod.rs#L612 "on a README would return its headings")</claim> |
| Count a document's words | a method | absent: the caller walks the tokens | <claim id="total-words-table" basis="observed">printed in the table, absent from `--json` [render.rs:45](../../src/cli/render.rs#L45 "doc.total_words()")</claim> |
| Analyze a folder | <claim basis="observed">a lazy stream [lib.rs:182](../../src/lib.rs#L182 "pub fn analyze<I>(")</claim> | an eager list | <claim basis="observed">refused [cli/mod.rs:563](../../src/cli/mod.rs#L563 "is a directory; pass a file")</claim> |
| Group sentences by meaning | <claim basis="observed">`embed_and_cluster(&doc, &embedder, threshold)` [lib.rs:373](../../src/lib.rs#L373 "pub fn embed_and_cluster(")</claim> | <claim basis="observed">`semantic_clusters(text, threshold, model)` [_core.pyi:143](../../python/matra/_core.pyi#L143 "def semantic_clusters(self, text: str, threshold: float, model: Embedder)")</claim> | no command |
| Handle an oversized input | a typed variant with `limit`, `actual`, `what` | <claim basis="observed">a bare `ValueError` [lib.rs:444](../../src/lib.rs#L444 "PyValueError::new_err(msg)")</claim> | <claim id="json-failure-prose" basis="observed">prose on stderr, nothing on stdout, exit 2 [cli/mod.rs:102](../../src/cli/mod.rs#L102 "matra: {e}")</claim> |
| How many results | <claim basis="observed">`n` or `max`, by function [textrank.rs:27](../../src/extraction/textrank.rs#L27 "n: usize") [yake.rs:30](../../src/extraction/yake.rs#L30 "max: usize")</claim> | `n` or `max_phrases` | `-n` |

<claim id="no-driving-port" basis="inferred">The cause is architectural. matra named its driven ports (`Source`, `NlpProvider`, `Embedder`) and never named what a caller can ask for</claim>, so each driving adapter assembled its own version of the application, and application rules leaked into them: the command line decides that extraction goes through the pipeline, and Python decides the opposite. A design decision with two homes is the defect information hiding exists to prevent: each module should own one decision and hide it [parnas-modules:c23]. <claim id="python-in-root" basis="observed">The Python binding also lives inside the composition root [lib.rs:406](../../src/lib.rs#L406 "mod python {")</claim>, beyond the reach of the boundary rule that holds the command line to the public surface.

### Names answer the wrong question

- <claim id="algorithm-names-today" basis="observed">The algorithm is in the function names: `tfidf_summarize` and its siblings in Rust, and the same in Python [extraction/mod.rs:14](../../src/extraction/mod.rs#L14 "pub use tfidf::summarize as tfidf_summarize") [_core.pyi:103](../../python/matra/_core.pyi#L103 "def tfidf_summarize")</claim>, and the format is in one: <claim basis="observed">`analyze_markdown` [_core.pyi:94](../../python/matra/_core.pyi#L94 "def analyze_markdown")</claim>. The ranking algorithm is a decision likely to change, and a likely change should touch no interface [parnas-process:c22].
- <claim id="compose-measures" basis="observed">The stage that runs the metric suite is called `compose` [lib.rs:250-251](../../src/lib.rs#L250-L251 "let suite = metrics::default_suite();")</claim>. A name should say what the thing does, and the same word should mean the same thing everywhere [api-design-practice:c8].
- <claim id="reporting-any-ccomp" basis="observed">`Reporting` fires on any verb with a clausal complement, including "think" and "ensure" [domain.rs:572](../../src/domain.rs#L572 "ccomp")</claim>, which breaks <claim basis="observed">matra's own rule that a derived field names a construction, "never a category" [EPR-0002:87](0002-data-model.md#L87 "never a category such as")</claim>.

### Failure has no contract outside Rust

<claim basis="observed">The JSON channel's stability promise is `format_version`, "the whole stability promise" an agent is told to pin [json.md:28](../../skills/matra/references/json.md#L28 "that is the whole stability promise")</claim>, yet a failure under `--json` writes nothing to stdout (above), so an agent reads prose. Behaviour on failure "is as much a part of the formal contract as when things go right" [api-design-practice:c23], and prose that callers parse "becomes part of the contract and cannot change" [machine-readable-contracts:c15].

### Why now, and why at once

0.2.1 is published, so every rename breaks someone. Under 0.x that is allowed, and the cheapest moment is before a TypeScript surface copies today's names into a fourth language. <claim id="migrate-once" basis="assumed">Callers would rather migrate once, in a larger release, than several times in smaller ones.</claim> Incompatibility across releases makes choosing versions a hard problem for callers [api-design-practice:c30], which favours one break over several.

## Guide-level explanation

### One API, three syntaxes

A caller asks for an outcome. The words are the same in every language.

```rust
use matra::{Engine, Input};

let engine = Engine::new()?;
let doc = engine.analyze(Input::file("README.md"))?;
let summary = matra::summarize(&doc, 3)?;
let phrases = matra::keyphrases_with(&doc, 10, matra::KeyphraseMethod::Yake)?;
println!("{}", doc.metrics.total_words);
```

```python
import matra

engine = matra.Engine()
doc = engine.analyze(file="README.md")
summary = matra.summarize(doc, n=3)
phrases = matra.keyphrases(doc, n=10, method="yake")
print(doc["metrics"]["total_words"])
```

```console
$ matra summarize README.md -n 3
$ matra keyphrases README.md -n 10 --method yake
$ matra analyze README.md --json | jq .result.metrics.total_words
$ matra analyze docs/ --json          # one envelope per line, one line per document
$ matra clusters notes.md --threshold 0.85
```

**The translation rule.** A Rust `Input` constructor is the Python keyword and the command line's input (`Input::file(p)` is `analyze(file=p)` is `matra analyze p`). A Rust `_with(.., Method)` is Python's `method=` and the command line's `--method`. Every word (`text`, `file`, `bytes`, `blocks`, `format`, `n`, `method`, `threshold`, `embedder`, and the method and format names) is spelled the same in all three. Rust has no default arguments, so Rust requires what Python defaults; the defaults are the command line's, and the effective values are recorded on every result.

### Text with its own section breaks

A caller whose text comes from somewhere matra does not read (a PDF it extracted, a CMS, a transcript) hands in headings and paragraphs, and needs no markdown:

```rust
let doc = engine.analyze(Input::blocks([
    Block::heading(1, "Field notes"),
    Block::paragraph("The river rose overnight."),
]))?;
```

```python
doc = engine.analyze(blocks=[{"kind": "heading", "level": 1, "text": "Field notes"},
                             {"kind": "paragraph", "text": "The river rose overnight."}])
```

```console
$ matra analyze notes.json --format blocks --json
```

### The record says where it came from, and what it skipped

Every top-level result carries `provenance`; every heading, paragraph, sentence, token and extracted result carries a `span` into the analysed text; and `skipped` lists what matra saw and did not analyse:

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
  "sections": [ { "heading": "Field notes", "level": 1, "span": { "byte_start": 0, "byte_end": 13 }, "paragraphs": [] } ],
  "skipped": [ { "kind": "table", "span": { "byte_start": 640, "byte_end": 812 } } ]
}
```

A summary adds what shaped it: `"method": "textrank", "parameters": { "n": 3 }`. The method strings are `tfidf`, `textrank`, `rake`, `yake` and, for clusters, `cosine`; `parameters` holds only what the method takes. A document was shaped by no method, so on a `Document` both keys are absent rather than null. A span counts UTF-8 bytes of the analysed text: for markdown and plain text that is the input itself, and for block input it is the blocks' texts joined by a blank line, in order. The text fields are normalized: a markdown paragraph's text is its inline text with the markup stripped and escapes decoded, which is what the parser reads. The span, not the text, is the authority on where a value came from.

### Errors carry a kind everywhere

```python
try:
    engine.analyze(text=huge)
except matra.InputTooLargeError as e:   # also a ValueError
    print(e.kind, e.details["limit"], e.details["actual"])
```

```json
{ "format_version": 2, "command": "analyze", "input": "big.md", "result": null,
  "error": { "kind": "input_too_large", "message": "…", "details": { "what": "input", "limit": 8388608, "actual": 9000000 } } }
```

### Migrating

The migration guide lists every rename on one page. Nothing keeps its old name: 0.3.0 is one clean break in Rust, Python, the command line and the JSON (decision 8).

## Reference-level explanation

### The public surface

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
state proposed proposed "one surface of outcomes; adapters translate; only the embedder is open"
box rust "Rust API" 2 8 114 28
box py "Python module\nrule 7, own file" 2 46 114 34
box cli "Command line" 2 90 114 28
quiet box ts "TypeScript 0.4\ntypes, viewer" 2 140 114 34
hex port "Outcomes\nanalyze\nanalyze_all\nsummarize\nkeyphrases\nclusters" 128 4 146 120
box schema "Schema 2.x\nJSON Schema" 146 140 110 34
quiet box source "Fetch, detect\nprivate" 290 4 108 34
box decomposer "Readers, private\nblocks, spans" 290 44 108 34
box parser "Parser, private\nidentity, spans" 290 84 108 34
box embedder "Embedder\npublic" 290 124 108 30
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

Today the Python box and the command-line box each carry an application rule. Proposed, those rules live behind the outcome functions, the Python module leaves the composition root, and of the driven ports only the embedder stays open to outside implementers.

| Outcome | Rust | Returns |
|---|---|---|
| analyze one | `Engine::analyze(Input)` | `Document` |
| analyze many | `Engine::analyze_all(impl IntoIterator<Item = Input>)` | a lazy stream of `Result<Document, DocumentError>`, in input order |
| summarize | `summarize(&Document, n)`, `summarize_with(&Document, n, SummaryMethod)` | `Summary` |
| keyphrases | `keyphrases(&Document, n)`, `keyphrases_with(&Document, n, KeyphraseMethod)` | `Keyphrases` |
| clusters | `clusters(&Document, &dyn Embedder, threshold: f64)` | `Clusters` |

- **Construction.** `Engine::new()` builds the defaults (the pinned parser model, the markdown and plain-text readers), locating the model by the existing resolver and reading no behaviour from a configuration file; defaults are written explicitly [api-design-systems:c38]. `Engine::from_config(&Config)` and `from_config_with_notice` stay. There is no builder: `Engine::with_parser(parser)` builds an engine around a caller's parser, and `with_reader(format, reader)` and `with_source(source)` replace one part of an engine ([decision 5](#decision-provider-traits)). <claim id="engine-new-injects" basis="observed">Today's `new` takes the parser and the reader table [lib.rs:159](../../src/lib.rs#L159 "pub fn new(nlp: Box<dyn NlpProvider>")</claim>, so the new `new` is a remove and an add [api-design-systems:c24], caught at compile time, and the migration guide says so. <claim id="with-defaults-reads-config" basis="observed">`with_defaults` reads the configuration file [lib.rs:321](../../src/lib.rs#L321 "Config::resolve()")</claim>, so it goes rather than survive with a meaning its name does not say; `Engine::from_config(&Config::resolve()?)` says the same thing explicitly. An `Engine` is `Send` without `Sync`: it moves to a worker thread but is not shared between two, because the parser mutates state as it parses. Loading a parser model file of the caller's choosing leaves both surfaces with the parser trait: the model is a pin, and the permissive model arrives in 0.4.0 as a pin chosen by id, which is an addition where keeping a file loader now would make its later removal a break.
- **Input.** `Input` is one document, opaque and lazy: `Input::text(s, Format)`, `Input::bytes(data, Option<Format>)` (detected when `None`), `Input::file(path)` (read at `analyze`, so a failed read is an item of a stream, not an abort), `Input::blocks(iter)`. `Inputs::dir(path) -> Result<Inputs>` is an iterator of `Input`. `Input::file` on a directory fails with `InvalidInput` naming `Inputs::dir`. `Ingest`, `Ingested` and `RawDocument` leave the surface. An opaque `Input` hides its representation [parnas-modules:c23], so a URL source later is a new constructor, and `Input::bytes` is what lets fetching stay private: a caller whose bytes come from elsewhere fetches them and hands them in, the cheapest extension mechanism [api-design-practice:c35].
- **Block.** `Block` is input only and opaque, with two constructors, `Block::heading(level, text)` and `Block::paragraph(text)`. Its JSON form is a published input format, `Format::Blocks`, with an input schema in `spec/schema/`; an unknown `kind` is refused, naming it. The richer vocabulary the readers emit (heading, paragraph, list item, table cell, code) is a separate, private type, so the likely change, more kinds, touches no interface [parnas-process:c22]. A shared format used as an interface becomes costly as a design evolves [parnas-modules:c25], which holds for a type callers receive or implement and not for one they only send.
- **Outcomes are free functions over a `Document`.** They need no engine and no parser, so they run in a build without UDPipe, which the TypeScript surface will need. The richer mechanism is built using the simpler one [parnas-modules:c33]. The method is a secret behind the outcome's name: the selector is an enum the caller sends, which can gain values safely, and the method recorded on the result is a string the caller receives, which readers treat as an open set (this send-versus-receive distinction is the canon synthesis's own suggestion, not a source finding). `n` is the one word for "how many" everywhere.
- **`clusters` takes an embedder in every language.** An implicit default would download a model inside an outcome call. Only the command line, the application tier, builds the default embedder from configuration.
- **A document is data.** `Document` and its records keep their public constructors and `Deserialize`, so a caller, an agent among them, saves `--json` output, whole or cut by `--depth`, and reads it back into the type the outcomes take. Desirable properties "should not be hidden" [neighbours-hoare-brooks-lampson:c37], and reading back what matra wrote is one. Nothing tells a document matra parsed from one it was handed, so the outcomes promise nothing that depends on the difference: `summarize`, `keyphrases`, `clusters` and every view on the records hold on a document whose parts contradict each other (spans past the text or inverted, heads pointing nowhere or in a cycle, empty sentences, texts that do not match their tokens), returning a result that carries the contradiction through or an error, never a panic or a loop. They correct nothing they cannot know and guess nothing. A test runs every outcome over documents broken one way each, with a deadline (own judgment: the canon does not address untrusted records).
- **The stages are internal.** `annotate` and `compose` stay as the crate's own stages, and the property that annotation is the only route from text to the parser holds for every `Input` constructor, which a law tests. <claim id="python-annotated" basis="observed">Their one caller outside the engine is the Python binding's own helper [lib.rs:488](../../src/lib.rs#L488 "self.engine.annotate(&raw)")</claim>. A public parse-only call would publish a second shape of `Document`, every metric empty, under the same name, and a reader could not tell it from a measured one, since a `null` metric already means a condition not met; "a feature which is included before it is fully understood can never be removed later" [neighbours-hoare-brooks-lampson:c13]. `--depth` also writes a smaller `Document`, and the difference is what a reader can see: it removes a level as a key, so what was not written is absent rather than empty.
- **Adapters translate and decide nothing.** The Python binding moves out of `lib.rs` into its own module and is held to boundary rule 7, the public surface only, with a semgrep rule. The command line keeps no application rule. Henning's "an API is a user interface" [api-design-practice:c21] is the grounding, with the drift above as the evidence.
- **A cross-surface parity test.** The same inputs through Rust, Python and the command line give the same `Document`, the same summary and the same error `kind`; testing finds what structure is meant to prevent [dijkstra-structure:c30].

### The input side, cut by decision

<sketch-figure id="input" seed="8" title="The input side cut by decision: each box hides one, and every adapter is private">

```text
width 400
state proposed proposed "callers enter by Input or plug in a port; the adapters below the surface are crate-private"
box text "Input::text\nInput::file\nInput::bytes" 4 4 120 44
box blocks "Input::blocks" 4 120 120 28
box fetch "Fetch: file, dir,\nstdin; guards" 140 4 120 34
box detect "Detect: name and\nbytes to a Format" 276 4 120 34
box read "Read one format:\nblocks, skipped" 276 60 120 34
box structure "Structure:\nblocks to sections" 140 120 120 34
box parse "Parse: sentences,\ntokens, spans" 276 120 120 34
box measure "Measure: metrics,\nrecord facts" 276 176 120 34
box doc "Document:\nprovenance, skipped" 140 176 120 34
edge text fetch
edge fetch detect
edge detect read
edge read structure
edge blocks structure
edge structure parse
edge parse measure
edge measure doc
```

</sketch-figure>

Each module hides one decision likely to change, and Parnas's procedure is exactly this: name the secrets, give each a module, and make the interfaces insensitive to the anticipated changes [parnas-modules:c35]. A cut by processing step, the flowchart, is "almost always incorrect" as a basis for decomposition [parnas-modules:c22] (corrected wording).

| Module | The decision it hides | Visibility |
|---|---|---|
| fetch (`source/`) | where bytes come from, and the guards on reading them (symlinks, size) | `Source` public; the file source private |
| detect | how a name and bytes map to a `Format` | private; `Format::from_path` stays public |
| read (`decompose/`) | one format's syntax | `Decomposer` public; one private reader per format |
| structure | how blocks become a section tree | private |
| parse (`nlp/`) | the parser, its model, tokenization, span alignment | `NlpProvider` public; the UDPipe adapter private |
| measure (`metrics/`, `hearst`) | detector rules and metric formulas | private |
| outcomes (`extraction/`) | which method ranks sentences or phrases | outcome functions public; method modules private |
| embed (`embed/`) | the embedding model | `Embedder` public; `Model2Vec` public behind its feature |
| domain | none: it is the published data | public |

- **Detection.** A binary signature wins over the extension (`%PDF`, a zip with `[Content_Types].xml`). Among text formats, the caller's `Format` or the extension decides. A recognised format this build cannot read fails with `UnsupportedFormat`, whose `details` name the feature that would read it when one exists (none does in 0.3.0, for PDF or DOCX); bytes that are not UTF-8 and carry no known signature fail with `UnsupportedFormat` and no feature. A text file with an unknown extension reads as plain text (decision 7).
- **The markdown reader** is rewritten on a CommonMark parser that reports source offsets, so it recognises lists, tables, code, block quotes and front matter, strips inline markup from the text the parser reads, and gives every block its span. A `## References` heading is a heading like any other. A construct it does not analyse (a table, a code block, front matter) is recorded in `skipped`, never dropped. Which parser is an implementation choice the canon does not address; pulldown-cmark is the candidate, chosen in the milestone's pull request.
- **`Paragraph.kind`** (`paragraph`, `quote`, `list_item`) arrives, since the reader can now fill it, and an enum can grow where a boolean cannot [api-design-practice:c40]. `in_blockquote` is removed in the same release (decision 8).
- **The parser** reports its own identity and, for each token, its span within the text it was given; the engine shifts those spans to absolute offsets at the one place each paragraph is parsed. The UDPipe adapter takes token positions from UDPipe's own `ranges` tokenizer option, or aligns token forms against the paragraph text, and returns `ParseFailed` naming the token when alignment fails, never a guessed span: a handler that returns normally deceives the caller [neighbours-contracts-simplicity:c14]. <claim id="parser-unvalidated" basis="observed">The parser's contract is trusted unchecked today [nlp/mod.rs:30](../../src/nlp/mod.rs#L30 "nothing validates it on the")</claim>; with the trait public, `conform::parse` checks every parse at the one place it enters a `Document` (token ids, heads, one root, no cycle, spans that match the forms), so the outcomes rely on what was checked, not on who wrote the parser.
- **Shared contract tests.** Each port gets one test every adapter must pass, stating which subtyping rule it enforces; the everyday one (an adapter may weaken a precondition and strengthen a postcondition) is Meyer's subcontracting rule [neighbours-contracts-simplicity:c12].

### The laws

The seven equivalence laws in `src/lib.rs` restate over `Input` and are run for every constructor (text, bytes, file, blocks): a stream of one is one analysis, order is preserved, and a failed read never reaches the parser. The law that analysis is annotation then measurement stays as an internal test, since both stages leave the surface but the property still holds. One law is new:

- **L8, the accounting law.** For text formats, every non-whitespace byte of the analysed text lies in exactly one of: a heading span, a paragraph span, a `skipped` span. A block's container markers (a quote's `>`, a list item's marker) fall inside its span. A conformance test over every fixture checks it, which turns "never silently" into something a test can falsify.

### The schema

```text
Document         { provenance, sections, metrics: DocumentMetrics, skipped: [Skipped] }
DocumentMetrics  { total_words, total_sentences, paragraph_count, mean_sentence_length,
                   sentence_length_std, vocabulary_ttr, nominalization_ratio, passive_ratio }
Section          { heading, level, span: Option (the heading's), paragraphs }
Paragraph        { span, text, kind, sentences, metrics: ParagraphMetrics }
ParagraphMetrics { readability_grade, lexical_density, compression_ratio }
Sentence         { span, text, tokens, word_count, tree_depth: Option, passive,
                   negations, modals, clausal_complements, root_adverbials, hearst_pairs, root_mood }
Token            { id, span, text, lemma, pos, xpos, features: map, head, dep, deps, misc, is_punct }
Skipped          { kind, span }
Span             { byte_start, byte_end } UTF-8 bytes of the analysed text
Provenance       { matra, schema, parser: Identity, format, source: { path?, bytes, sha256 }, method?, parameters? }
Parameters       { n?, threshold?, embedder?: Identity }
Identity         { name, model, sha256, license }
Summary          { provenance, sentences: [SummarySentence { index, span, text, score: f64 }] }
Keyphrases       { provenance, phrases: [Keyphrase { phrase, score: f64, occurrences: [Span] }] }
Clusters         { provenance, clusters: [Cluster { members }], edges: [SimilarityEdge { a, b, score: f64 }] }
DocumentError    { path?, kind, message, details }
```

- **Aggregates are fields.** What the command line prints and Python rebuilds becomes data: if data exists only in string form, the string becomes a de facto API [api-design-practice:c11]. `ParagraphMetrics` and `DocumentMetrics` are names RFC-0006 reserved, and no outer `Option` wraps either record, because each slot gates on its own.
- **One home for the path.** <claim id="corpus-entry-path" basis="observed">`CorpusEntry` pairs a path with a document [domain.rs:1351](../../src/domain.rs#L1351 "Self { path, analysis }")</claim>, and the path also belongs in `provenance.source.path`. `CorpusEntry` goes; there must be one, and only one, place for every fact [parnas-process:c14]. `Corpus` leaves the surface too, with its aggregate methods: methods do not cross into Python, averaging across documents is the caller's decision (defect 12 below), and nothing in the tree needed a collector.
- **Skipped kinds** name the construct (`table`, `code`, `front_matter`, `html`, `thematic_break`, `link_definition`, later `page_image`), never a judgment, following matra's rule that a derived field names a construction (EPR-0002). `skipped` is not an error and carries no error code: skipping a construct is specified behaviour reported beside a correct result, so AIP-193's rule against partial errors [api-design-systems:c30] does not bind it.
- **CoNLL-U is converted.** `features` is a map. `is_punct` is a field the parse derives, which every word count and metric reads, so a reader counts words as matra counted them without knowing which tags it treats as punctuation; it is a second home for a fact the parse already carries [parnas-process:c14], kept because the counts depend on it. **Sentinels become `Option`**: `tree_depth` is `None` on a cycle instead of `usize::MAX`, and the visited-set walk is unchanged. **Scalars cross as `f64`.**
- **Record facts are named for what the detector finds.** `ClausalComplement` and `clausal_complements` replace `Reporting` and `reportings`; `root_mood` (the mood feature of the root, with the root's token id) replaces the `bare_assertion` flag. Name a thing by what it is [dijkstra-systems:c2]; `root_mood` is a retype, and its values are an open set.
- **One format vocabulary**: `markdown`, `plain_text`, `blocks`, in the JSON, in Python and on the command line.
- **Where spans sit**: section headings, paragraphs, `skipped` entries, sentences, tokens, summary sentences and keyphrase occurrences. Cluster members stay sentence indices, which already resolve through the document.

### Versioning and the consumer rules

<claim id="format-version-ships" basis="observed">The command line already carries `format_version: 1`, which "increments on any change to the envelope or to the meaning of a field inside `result`" [cli/mod.rs:39](../../src/cli/mod.rs#L39 "const FORMAT_VERSION: u32 = 1;") [json.md:23](../../skills/matra/references/json.md#L23 "It increments on any change to the envelope or to the meaning of a field inside")</claim>. A second, weaker number beside it would weaken what agents were already told to rely on.

- **One schema version, in one place.** `provenance.schema` is `"major.minor"` and lives nowhere else [parnas-process:c14]. 0.2's implicit schema is 1; this release is `2.0`.
- **What moves which number.** A minor adds optional fields only, invisible to old readers [machine-readable-contracts:c18]. A major is anything else: a field removed, renamed or retyped, or a field whose meaning changes, including a changed default, since defaults are behaviour [api-design-systems:c25] and semantic compatibility is compatibility [api-design-systems:c21].
- **`format_version` keeps its rule** and moves whenever the envelope changes or the schema major changes. A minor changes no existing field's meaning, so it moves neither.
- **Rules for consumers.** Ignore fields you do not recognize; treat every string set you receive as open (`kind`, `method`, `root_mood`, `pos`, `dep`, the `skipped` kinds); reject a major you do not know. That is Terraform's stated policy for its JSON output [machine-readable-contracts:c35], and Kubernetes found that "adding a new enum value breaks backward compatibility" for enums clients receive [api-design-systems:c34].
- **A published schema.** `spec/schema/` holds a JSON Schema per major, validated against every conformance output in every binding, and printed by `matra --schema`. A contract published as a machine-readable artifact lets programs discover what a tool returns without its source [machine-readable-contracts:c36]. It is hand-written, or written by a dev-only test, because deriving it on the domain types would add a dependency to `domain.rs`, which boundary rule 1 forbids.
- **Calibration.** Strict compatibility rules assume many callers whose upgrades you do not control; interfaces whose callers are co-owned "should set their own requirements" [machine-readable-contracts:c19]. These rules govern the wire from 2.0 on; the crate stays 0.x.

### Provenance

`provenance` sits on `Document`, `Summary`, `Keyphrases` and `Clusters`. Derived results copy the document's provenance and add their own `method` and `parameters` (`n`, `threshold`, the embedder's identity). The parser and the embedder report themselves in one shape, `Identity { name, model, sha256, license }`, so the embedding model's name, digest and licence reach `parameters.embedder` as the parsing model's reach `parser`, and one reader reads both; the same word means the same thing [api-design-practice:c8]. The rule the embedder already carried applies to both: <claim id="embedder-identity" basis="observed">two embedders "that can disagree must not share an identity" [embed/mod.rs:41](../../src/embed/mod.rs#L41 "embedders that can disagree must not share an identity")</claim>. The model is the decision most likely to change (a permissively licensed model is planned), so a consumer must see which model produced a parse rather than discover it as a shift in the numbers; an interface states "the meaning and limits of information exchanged" [parnas-process:c46]. The input digest is computed once, at annotation, with `sha2`, which becomes a required dependency of the crate, not of `domain`. There is no timestamp: matra's outputs are deterministic, conformance compares them, and a timestamp would make two identical analyses differ (own judgment).

Provenance is a record, not proof. It says how a result says it was made: a document read back from JSON, or built with `Provenance::new`, carries whatever provenance it was given, and an outcome derived from it copies that on. What a reader can check it against is `source.sha256`, the digest of the analysed text every span points into. In Python a `Document` comes only from `analyze`, so a Python outcome carries the provenance an analysis recorded; that is a property of the Python surface, not of documents.

### Errors on every channel

- **Rust.** `Error` keeps its variants and `kind()`, and gains `details()`, the values a caller acts on as structured data, and a serde form `{kind, message, details}`. `DocumentError` serializes as that plus `path`, so the hand-built projection in the Python binding is deleted. Every dynamic value in an error message must also be present as metadata [machine-readable-contracts:c13]. `UnsupportedFormat`'s details name the feature that would read the format, when one exists.
- **Labels in the caller's terms.** `InputTooLarge.what` names the gate by what the caller reads or selects, never by a private module: `input`; `file_read` and `config_file_read`; `parser_model_download` and `embedding_model_download`; and for an outcome, the method the caller selects (`tfidf`, `textrank`, `rake`, `yake`) or `clusters`. Error messages "must not assume that the user will know anything about" the implementation [api-design-systems:c29], and a label naming the `Source` adapter or the UDPipe adapter asks exactly that.
- **One kind for one remedy, and a reason for each check.** `invalid_input` stays one kind, because its cases share a remedy: change the call, the configuration, the data or the embedder, since making the call again as it is fails the same way [api-design-systems:c28]. Its cases still call for different actions (fix a config file, fix an embedder, pass a directory to `Inputs::dir`), so `InvalidInput { reason, message }` carries `reason`, a stable snake_case name for the check that failed (`threshold_not_finite`, `embedder_vector_count`, `config_malformed`, `is_a_directory`, `usage`, ...), in `details.reason` on every channel. The errors reference lists every reason, and a test holds the list to the source. Here the design departs from [machine-readable-contracts:c13] for this one kind: an `invalid_input` message adds, for a person, values its details do not repeat, because `reason` is what a caller acts on.
- **Python.** `matra.MatraError` with `.kind` and `.details`, and one subclass per kind that also inherits today's builtin class (`InputTooLargeError(MatraError, ValueError)`), so a caller who catches the builtin class catches it too [api-design-practice:c39]. The class names end in `Error`, as Python names its exceptions; the name shared across languages is the `kind` string. `IoError` routes on the kind it wraps: `IoNotFoundError` (also a `FileNotFoundError`), `IoPermissionError` (also a `PermissionError`), else `IoError` (an `OSError`). An I/O failure on a path matra was given names that path, in the message and in `details.path`.
- **Command line.** Under `--json`, a failure writes the envelope with `"result": null` and an `error` object to stdout, writes `matra: <message>` to stderr, and exits 2. On success `"error": null`; both keys are always present. The command line's own refusals use the existing kind whose remedy matches: error identity is defined by what the caller would do about it [api-design-systems:c28].
- **Exit codes** stay 0 (found), 1 (nothing found), 2 (failed). In `analyze DIR --json`, each document is one envelope per line; a failed document is a line with its `error`, the walk continues, and the exit code is 2 if any document failed. Per-item failure in a bulk operation is the one sanctioned partial success [api-design-systems:c30].
- **Retry.** Download failures carry `details.retryable: true`. Whether a retry is safe depends on both the operation and the failure [machine-readable-contracts:c16].

### Limits and defaults

The input cap, the sentence caps and the metric minimums move into one module as published constants, `domain::limits`, each defined once and referenced where it applies. They are published because a caller needs them before the call: to split an input under `MAX_INPUT_BYTES`, or to chunk a document's sentences or tokens under the cap of the method it selects, where the error's `limit` detail reports a cap only after the call has been refused. Desirable properties "should not be hidden" [neighbours-hoare-brooks-lampson:c37]. Each method cap is named for the method the caller selects (`MAX_TFIDF_SENTENCES` for `SummaryMethod::TfIdf`), as the selectors are, so the names follow the caller's choice. The values are this release's and may change in any 0.x release; each constant's documentation carries the cost model behind its value, and a caller reads the constant rather than copying the number. Raising a cap is left out of this release: the canon does not cover resilience bounds, and the resilience floor keeps safe defaults until its own reviewer rules. Library defaults are written in the signatures; configuration applies at the command line and in the `from_config` constructors, in Python as in Rust: `Engine()` and `Model2Vec.potion_base_8m()` read no configuration file, and `Engine.from_config` and `Model2Vec.from_config` read one. Defaults "should be written explicitly" [api-design-systems:c38], and a changed default is a changed behaviour [api-design-systems:c25]. The effective parameters are recorded in `provenance`.

### Signatures

```rust
impl Engine {
    pub fn new() -> Result<Engine>;
    pub fn from_config(cfg: &Config) -> Result<Engine>;
    pub fn from_config_with_notice(cfg: &Config, notice: impl FnMut(&ProvisionNotice)) -> Result<Engine>;
    pub fn analyze(&self, input: Input) -> Result<Document>;
    pub fn analyze_all<I: IntoIterator<Item = Input>>(&self, inputs: I)
        -> impl Iterator<Item = std::result::Result<Document, DocumentError>> + '_;
}
impl Input {
    pub fn text(text: impl Into<String>, format: Format) -> Input;
    pub fn bytes(data: impl Into<Vec<u8>>, format: Option<Format>) -> Input;
    pub fn file(path: impl AsRef<Path>) -> Input;
    pub fn file_as(path: impl AsRef<Path>, format: Format) -> Input;   // a binary signature still outranks it
    pub fn blocks(blocks: impl IntoIterator<Item = Block>) -> Input;
}
impl Inputs { pub fn dir(path: impl AsRef<Path>) -> Result<Inputs>; }   // Iterator<Item = Input>
impl Block {
    pub fn heading(level: u8, text: impl Into<String>) -> Block;
    pub fn paragraph(text: impl Into<String>) -> Block;
}
pub fn summarize(doc: &Document, n: usize) -> Result<Summary>;
pub fn summarize_with(doc: &Document, n: usize, method: SummaryMethod) -> Result<Summary>;
pub fn keyphrases(doc: &Document, n: usize) -> Result<Keyphrases>;
pub fn keyphrases_with(doc: &Document, n: usize, method: KeyphraseMethod) -> Result<Keyphrases>;
pub fn clusters(doc: &Document, embedder: &dyn Embedder, threshold: f64) -> Result<Clusters>;
#[non_exhaustive] pub enum SummaryMethod { TfIdf, TextRank }
#[non_exhaustive] pub enum KeyphraseMethod { Rake, Yake }
#[non_exhaustive] pub enum Format { Markdown, PlainText, Blocks, Pdf, Docx }   // Pdf, Docx recognised, not read
pub trait Embedder: Send {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Embedding>>;   // ModelInvalid, Io or InputTooLarge on its own failure
    fn identity(&self) -> Identity;                               // the shape the parser reports
}
// Error::InvalidInput { reason: &'static str, message: String }, reason the check that failed
```

```python
class Engine:
    def __init__(self) -> None: ...
    @staticmethod
    def from_config(config_file: os.PathLike | None = None) -> Engine: ...
    def analyze(self, text: str | None = None, *, file: os.PathLike | None = None,
                bytes: builtins.bytes | None = None, blocks: list[BlockDict] | None = None,
                format: Literal["markdown", "plain_text", "blocks"] | None = None) -> Document: ...
    def analyze_all(self, *, dir: os.PathLike | None = None, texts: Iterable[str] | None = None,
                    format: Literal["markdown", "plain_text", "blocks"] | None = None
                    ) -> Iterator[Document | DocumentError]: ...

def summarize(doc: Document, n: int = 3, *, method: Literal["tfidf", "textrank"] | None = None) -> Summary: ...
def keyphrases(doc: Document, n: int = 10, *, method: Literal["rake", "yake"] | None = None) -> Keyphrases: ...
def clusters(doc: Document, embedder: Embedder, threshold: float = 0.85) -> Clusters: ...

class Document: ...   # read-only; doc["metrics"]["total_words"]; doc.to_dict()
class MatraError(Exception): kind: str; details: dict

class Model2Vec:      # an Embedder: embed(texts) and identity() -> Identity
    @staticmethod
    def from_dir(dir: os.PathLike) -> Model2Vec: ...
    @staticmethod
    def potion_base_8m(dir: os.PathLike | None = None) -> Model2Vec: ...      # reads no config file
    @staticmethod
    def from_config(config_file: os.PathLike | None = None) -> Model2Vec: ...
```

Exactly one of `text`, `file`, `bytes` and `blocks` (and of `dir` and `texts`) is accepted; any other count is a `TypeError` at the call. A `str` is always text, never a path, and reads as `plain_text` when no `format` is given. Python's vectors-in `semantic_clusters` goes with the rest: it has no Rust counterpart, and `clusters` takes an embedder, which can return vectors the caller already holds.

```
matra analyze    PATH|DIR|-  [--format markdown|plain_text|blocks] [--json] [--depth document|paragraph|sentence|token]
matra summarize  PATH|-      [-n 3]  [--method tfidf|textrank] [--format F] [--json]
matra keyphrases PATH|-      [-n 10] [--method rake|yake]      [--format F] [--json]
matra clusters   PATH|-      [--threshold 0.85]                [--format F] [--json]
matra config show | init [--force]        matra completions SHELL
matra --schema [NAME]                     matra --skill [-r NAME]
```

`-` reads stdin as bytes and detects the format. `--depth` lets an agent choose how much of the parse it pays for, since every token shown costs the caller [agent-computer-interfaces:c2] (vendor guidance). It requires `--json`, defaults to `token` (the whole parse), and removes the levels it cuts rather than leaving them empty. A cut document reads back into Rust's `Document`, the removed levels empty, so what the command line writes composes with what the library takes. `--format` names the format of a file or stdin and is refused with a directory. In `analyze DIR`, a file that is neither text nor a recognised binary format (an image) is an `unsupported_format` line, so a walk over a mixed directory exits 2 (decision 7). Under `--json` a usage error is an envelope too.

### Names

| 0.2 | 0.3 | Kind |
|---|---|---|
| `Engine::new(nlp, decomposers)` | `Engine::new()` | constructor (re-signed) |
| `Engine::analyze(Ingest)`, `analyze_one` | `analyze(Input)`, `analyze_all(inputs)` | methods |
| `Engine::annotate`, `Engine::compose` | internal | methods |
| `Ingest`, `Ingested`, `RawDocument` | `Input`, `Inputs` | types |
| `tfidf_summarize`, `textrank_summarize` | `summarize`, `summarize_with` | functions |
| `rake_keyphrases`, `yake_keyphrases` | `keyphrases`, `keyphrases_with` | functions |
| `embed_and_cluster`, `semantic_clusters` | `clusters` | function |
| `Reporting`, `Sentence.reportings` | `ClausalComplement`, `clausal_complements` | type, field |
| `Sentence.bare_assertion` | `Sentence.root_mood` | field (retyped) |
| `Paragraph.in_blockquote` | `Paragraph.kind` | field (retyped) |
| `SemanticClusters.model_hash` | `Clusters.provenance.parameters.embedder`, an `Identity` whose `sha256` is the old hash | field (moved, retyped) |
| `Embedder::identity() -> &str` | `Embedder::identity() -> Identity` | trait method (retyped) |
| `InputTooLarge.what`: `file_source`, `config_file`, `udpipe_download`, `embedding_download` | `file_read`, `config_file_read`, `parser_model_download`, `embedding_model_download` | error labels |
| `Error::InvalidInput(String)` | `Error::InvalidInput { reason, message }` | variant (reshaped) |
| `CorpusEntry` | `provenance.source.path` | type removed |
| `Paragraph.readability_grade` and siblings | `Paragraph.metrics.*` | fields (moved) |
| `Document.vocabulary_ttr` and siblings | `Document.metrics.*` | fields (moved) |
| `Source`, `Decomposer`, `NlpProvider` | the same names, public, plugged in by `Engine::with_source`, `with_reader` and `with_parser` (decision 5) | traits (reshaped) |
| `metrics`, `hearst` | crate-private (decision 4) | modules |
| `Corpus`, `CorpusResult` | none: the caller collects | types removed |
| Python `Matra.from_path` | none: the model is a pin | constructor removed |
| Python `Matra`, `analyze_markdown` | `Engine`, `analyze(format=)` | class, method |
| config `[summarize]`, `semantic.*`, `algorithm`, `models.udpipe` | `[summary]`, `[keyphrases]`, `[clusters] threshold`, `method`, removed | keys |

The traits keep their names; renaming them would move files the boundary checks scope by path.

### Stability, by surface

| Surface | Promise |
|---|---|
| Rust items | 0.x: breaking changes only in a minor bump |
| Python names and exception classes | the same as Rust |
| CLI commands, flags, exit codes | the same |
| JSON schema | the major and minor rule above |
| CLI envelope | `format_version`, moving with the schema major |
| Skill text (`matra --skill`) | versioned with the binary; it changes in the same pull request as the surface it describes, and a changed incantation is listed in the CHANGELOG |

<claim id="semver-pr-only" basis="observed">`cargo-semver-checks` runs on pull requests only [ci.yml:318](../../.github/workflows/ci.yml#L318 "if: github.event_name == 'pull_request'")</claim>. It will also run on pushes to `main`, so the Rust promise has a mechanical check.

### Release split

| Release | Contents |
|---|---|
| 0.3.0 | Every break: the outcome surface, `Input` and `Block`, the private input side with the new markdown reader, spans, `skipped` and L8, schema 2.0 and its JSON Schema, provenance, errors on every channel, the Python `Document`, the parity test, the stability table and the semver gate |
| 0.3.x | Fixes |
| 0.4.0 | The feed for text arriving over time, in all three syntaxes at once; caller extensions (predicates and findings) or x.uma extensions in their own proposal; schema-typed TypeScript and a web component that renders a parse; the result of a UDPipe-in-WebAssembly spike; the permissive parser model as a pin chosen by id |

### Milestones

Each is one pull request into `next`, with its documentation, skill text and CHANGELOG entry in the same pull request, and each leaves `next` building with every check green except the semver gate, which binds on the release pull request from `next` into `main`.

1. **The schema.** The metric records, provenance with the parser's identity, the record-fact renames, `features` as a map, `tree_depth` as an `Option`, the format vocabulary; the JSON Schema in `spec/schema/` and its conformance fixtures.
2. **The input side.** `Span`, `Skipped` and `Paragraph.kind` with their schema entries; `Input`, `Inputs` and `Block`, with `Engine::analyze(Input)` and `analyze_all` replacing `Ingest` and `analyze_one`, and `CorpusEntry` removed; detection; the CommonMark reader with spans and `skipped`; the plain-text reader with spans; structure from blocks; the parser's token spans; L8 and the laws restated over every constructor.
3. **The outcomes.** `Engine::new`; the outcome functions and their method enums with the method and parameters recorded; the traits, `metrics`, `hearst` and the method modules made private; `Corpus` removed.
4. **Errors on every channel.** `details()` and the serde form in Rust; the Python exception classes; `domain::limits`.
5. **Python.** The module out of `lib.rs` under boundary rule 7, `Engine`, the read-only `Document`, the outcome functions, the stub file; the parity test.
6. **The command line.** Directories, `clusters`, `--format`, `--json` errors, `--depth`, `--schema`, the configuration sections; the skill and its references.
7. **The release.** The migration guide, the docsite and the roadmap, the semver gate on pushes, a newcomer pass, the version bump, the pull request from `next` into `main`, and the 0.3.0 release, which publishes only on the owner's approval.

## Drawbacks

- Every caller migrates, and moving metrics into records has no both-name form in Python dicts: a reader of `paragraph["readability_grade"]` must read `paragraph["metrics"]["readability_grade"]`.
- Spans and provenance make `analyze --json` larger; `--depth` mitigates it for agents.
- Closing the parser and reader traits makes a WebAssembly parser matra's own work rather than anyone's, and <claim id="roadmap-wasm-promise" basis="observed">the roadmap's promise that the parser trait needs no change for WebAssembly [ROADMAP.md:56](../../ROADMAP.md#L56 "requires no change to support a WASM implementation")</claim> must be rewritten with the code.
- One release carries a lot of change, so its review is long. Lehman reports that what a release can carry is limited by how fast people absorb its changes [card:lehman-1996-laws-revisited] (unverified reading), which is why streaming moved out.

## Rationale and alternatives

The design rests on one diagnosis: the three surfaces drifted because no module owned what a caller can ask for. So one surface of outcomes owns it, and each adapter only translates. The input side is cut so that each likely change (a format, a parser, a source) lands in one private module. The record carries its provenance, its spans and its `skipped` list, so a caller can check a result without trusting the prose around it.

Each section below is a design that could reasonably have been chosen instead. It says what a caller would see, what the design would gain and cost, and which canon principles argue for it and against it. Parnas's group made "a policy of recording all of the design alternatives that we considered and rejected", with the reason for each [parnas-process:c16]; this section is that record for 0.3.0.

The canon was checked against these decisions after they were made. That makes it easy to find support after the fact, so each citation below names the alternative it rules out, and where the canon pulls the other way, the section says so. The canon's own review warns that it holds no sources gathered from its authors' critics, so its disagreements are only those among the authors it chose. Where the canon says nothing, the section says that too.

### Do nothing

0.2 stays as it is. A Python caller summarizes a string, not a document, and the headings of a README are summarized as prose (defect 3):

```python
m = matra.Matra.english()
m.tfidf_summarize(open("README.md").read(), 3)
```

**For.** No caller migrates. Tulach ranks functional compatibility, the same results from the same calls, highest: "in the end it is the most important one" [api-design-practice:c29].

**Against.** The defects in [Motivation](#motivation) stay, and a TypeScript surface would copy them into a fourth language. A program in use "undergoes continual change or becomes progressively less useful" [neighbours-systems-evolution:c36]. When versions of one system have drifted apart, Parnas's remedy is to hide their differences behind documented interfaces and "collapse the versions into a single system" [parnas-process:c41]; here the drifted versions are the three surfaces. Callers cannot fix it themselves. A caller can wrap the old names, but a wrapper "ignores the cost of doing so" [api-design-practice:c16], and spans, provenance and `skipped` need facts that exist only inside matra.

**Outcome.** Rejected. The drift comes from the structure, so it outlives any fix that keeps the structure.

### Several proposals instead of one

The names, the input side, the schema and the errors would each get a proposal, accepted and shipped on its own schedule.

**For.** Each review is shorter. Lehman reports that what a release can carry is limited by how fast people absorb its changes [card:lehman-1996-laws-revisited] (unverified reading), and this proposal already moved streaming out on that ground (Drawbacks).

**Against.** The parts constrain each other. The outcome functions take a `Document`, so their signatures depend on the schema. Spans depend on the reader. The error kinds depend on what the input side can refuse. Split apart, each part is decided with the others guessed at. Brooks: it is better for a system to "reflect one set of design ideas, than to have one that contains many good but independent and uncoordinated ideas" [neighbours-hoare-brooks-lampson:c19]. Separate proposals would also mean separate breaks (the next section).

**Outcome.** Rejected. The design is reviewed as one, and the [milestones](#milestones) still land as seven pull requests.

### Spread the renames across releases

Each rename would land when its area is ready: the outcome names in 0.3.0, the metric records in 0.4.0, the Python `Document` in 0.5.0.

**For.** Each upgrade is small, and a caller can stop at any step.

**Against.** Every release breaks again. Tulach shows that incompatible versions make choosing versions an NP-complete problem, while with compatible ones the answer is "just take the newest version of each" [api-design-practice:c30]. Three breaks give a caller three places to get stuck between. The case against this alternative also rests on an assumption the owner is asked to confirm: that [callers would rather migrate once](#claim-migrate-once). If callers prefer small steps, this alternative gains.

**Outcome.** Rejected, on an assumed preference.

### Keep algorithm-named functions

```rust
let top = matra::extraction::tfidf_summarize(&sentences, 3)?;
let phrases = matra::extraction::yake_keyphrases(&sentences, 10)?;
```

**For.** The name says exactly what runs, so a caller reproducing a published result sees the method at the call site. Lampson: "The purpose of abstractions is to conceal undesirable properties; desirable ones should not be hidden" [neighbours-hoare-brooks-lampson:c37].

**Against.** The method is the decision most likely to change behind each outcome, and "it should be possible to make likely changes without changing any module interfaces" [parnas-process:c22]. With the method in the name, a new method is a new function in three languages. The word for "a summary" also differs by algorithm, where the same word should mean the same thing [api-design-practice:c8].

**Outcome.** Rejected by both choices of [decision 1](#decision-algorithm-choice). What it would have kept, a visible method, survives as the optional `_with` and as the `method` recorded on every result.

### Outcomes only, with no choice of method

This is decision 1's choice (a). `summarize(&doc, n)` is the only form; matra picks the method and names it on the result; `--method` and the `algorithm` keys go. The case for it is the decision's own Against.

**For.** It is the smallest surface. "You can always add, but you can never remove" [api-design-practice:c6], and a typed selector can be added later without a break. It hides the method completely, which is Parnas's rule read strictly: an interface "chosen to reveal as little as possible" [parnas-modules:c23].

**Against.** It hides a property callers need. RAKE and YAKE [score on unrelated scales](#claim-score-scales), so a caller who compares scores across runs, or who replicates a paper, must be able to name the method. Ousterhout: "When the details are important, it is better to make them explicit", and hiding them "just creates obscurity" [api-design-systems:c9]. Henning gives the general form: "General-purpose APIs should be 'policy-free'" [api-design-practice:c20], and which ranking to use is a policy. It would also remove a flag that [the command line ships today](#claim-cli-method-flag), so (a) is the break and (b) is not.

**Outcome.** Rejected; the owner chose (b) on 2026-11-02. The cost of (b) is weight: each outcome is two functions in Rust (the next section).

### An options struct, or an `Option` argument, in Rust

Inside choice (b), Rust could spell the optional method two other ways:

```rust
let s = matra::summarize(&doc, 3, None)?;                                // an Option argument
let s = matra::summarize(&doc, SummaryOptions::new(3).method(SummaryMethod::TextRank))?;   // an options struct
```

**For.** Either gives one function per outcome instead of two, so Rust has three outcome functions rather than five. An options struct also grows by a field where `_with` would grow by another function.

**Against.** An `Option` argument makes every common call write `None`. Henning asks for an API that is minimal "without imposing undue inconvenience on the caller" [api-design-practice:c17]. An options struct adds a type per outcome, and because matra's public structs with public fields are `#[non_exhaustive]`, a caller outside the crate cannot build one with a struct literal, so it needs a constructor and setters: a new concept per outcome. Bloch weighs an API by its concepts, "Conceptual weight more important than bulk" [api-design-practice:c6], and `_with` adds bulk, a second name, but no concept. It also maps one to one onto Python's `method=` and the command line's `--method`, so the translation rule stays one sentence.

**Outcome.** The proposal takes `_with`. The owner ruled on decision 1 as a whole, not on this spelling.

### Keep the stages public

This is the surface RFC-0007 shipped: `annotate` parses without measuring, and `compose` measures.

```rust
let mut doc = engine.annotate(&raw)?;   // every metric slot None
engine.compose(&mut doc);
```

**For.** A caller who needs only the parse could skip the metric suite. Parnas observes that "the 'more powerful' mechanism could have been built separately from, but using, simpler mechanisms" [parnas-modules:c33], and the point of building it that way is that the simpler one stays available. Read for what it protects, that claim argues for this alternative. This page cites it for outcome functions that need no engine. The design does not dispute the claim; it answers with the cost below. Lampson's "higher levels should not bury this power" [neighbours-hoare-brooks-lampson:c37] points the same way, but only if measuring costs enough to matter, and nobody has measured it.

**Against.** A public parse-only call publishes a second shape of `Document` under the same name, every metric empty, and a reader of the JSON cannot tell an unmeasured document from one with no sentences, because a `null` metric already means a condition not met. `matra analyze --depth` writes a smaller `Document` under the same schema too, and what separates the two is what a reader can see: `--depth` removes a level as a key, so what was not written is absent, never present and empty, and the schema says which keys a cut may remove. "A feature which is included before it is fully understood can never be removed later" [neighbours-hoare-brooks-lampson:c13]. No use case asks for a parse without its measures; the [one caller outside the engine](#claim-python-annotated) is matra's own Python helper. Agent-facing guidance also warns against tools that "merely wrap existing software functionality" [agent-computer-interfaces:c6] (vendor guidance), which is what a stage name is. What RFC-0007 wanted from the stages survives: they stay inside the crate, and a law tests that annotation is the only route from text to the parser.

**Outcome.** Rejected; the owner settled outcomes over stages on 2026-11-02.

### Keep `Ingest` as the one stream type

RFC-0007 made a string a stream of one and a directory a stream of many. In 0.2, one document is the first item of a stream:

```rust
let entry = engine.analyze(Ingest::text(s, Format::Markdown)).next().expect("a stream of one")?;
let doc = entry.analysis;
```

**For.** One concept covers every source, and a single document runs the same code path as a corpus. `Input` and `Inputs` are two types where `Ingest` is one.

**Against.** Every driving adapter that wants one document unwraps the stream, so the unification served the implementation, not the caller. Henning: "What should drive the design of APIs is not the needs of the implementer" [api-design-practice:c21]. `RawDocument` also hands the caller the representation the readers consume, which an opaque `Input` hides [parnas-modules:c23]. The property RFC-0007 wanted is kept by a law instead of a type: `analyze_all` over one `Input` equals `analyze` of it.

**Outcome.** Rejected.

### A public `Normalizer` or `Block` shared by readers and callers

The readers' block vocabulary (headings, paragraphs, list items, table cells, code) would be public, and an outside reader would implement a trait that emits it:

```rust
impl matra::Normalizer for MyPdfReader {
    fn blocks(&self, bytes: &[u8]) -> matra::Result<Vec<matra::Block>> { /* ... */ }
}
```

**For.** An outside format becomes a first-class reader, and one block type serves both directions. That is extension by others, which the owner has asked for (decision 5, Against).

**Against.** Two readers exist, below the three Bloch asks for before a plug-in interface ships: "If you write three, it will work fine" [api-design-practice:c4]. A type that readers emit and callers implement against serves two roles that evolve under opposite rules [api-design-practice:c31]. More block kinds is the change most likely to come, and each would change a type outside code is built on; shared formats used as interfaces are costly because "design evolution forces frequent changes" on them [parnas-modules:c25]. `Input::blocks` meets the same need with a two-constructor input type that callers only send, so it can grow. "Normalize" also already means Unicode and case normalization in an NLP library [api-design-practice:c8].

**Outcome.** Rejected. A caller keeps a way in for any format; what stays private is the reader contract.

### Open the parser, reader and source traits behind an `unstable` feature

This is [decision 5](#decision-provider-traits)'s choice (b):

```toml
matra = { version = "0.3", features = ["unstable"] }
```

```rust
impl matra::nlp::NlpProvider for MyTransformerParser { /* ... */ }
```

**For.** A WebAssembly or transformer parser written outside matra becomes possible now, and the owner has asked for matra to be extensible by others. The canon has a direct argument for it. The *Framework Design Guidelines* say "Sealing a class because you cannot think of an extensibility scenario is not a good reason" [api-design-practice:c38], and the recommendation's ground, that no outside implementer is named, is close to that reason.

**Against.** The traits change in this very release (spans, the parser's identity), and a provider interface takes "No new method additions" once someone implements it [api-design-practice:c32]. Two readers and one parser fall short of the "several concrete implementations" an abstraction should be tested by before it ships [api-design-practice:c36]. A feature flag does not stop dependence: with enough users, "all observable behaviors of your system will be depended on by somebody" [api-design-systems:c17]. Lampson warns that extensibility "needs a lot of care to keep from exposing secrets of the code that you might want to change" [neighbours-hoare-brooks-lampson:c49], and the [parser's contract is trusted unchecked](#claim-parser-unvalidated). Bloch's default runs opposite to the guidelines' ("Conservative policy: all concrete classes final" [api-design-practice:c12]), so the canon disagrees with itself here. The design takes the side that can be undone: opening a trait later is an addition, and closing one is a break.

**Outcome.** Rejected in this form. The owner first ruled the traits private "for now", then on 2026-11-06 opened them outright, with no feature flag: before 1.0 a break costs nothing, so the side that can be undone bought nothing, and the unchecked contract is answered by checking it at the boundary ([decision 5](#decision-provider-traits)).

### Change a provider contract now and implement it later

0.3.0 would publish a contract no code uses yet, such as the feed's settled-offset reader contract or the caller extensions RFC-0006 named, and implement it in a later release.

**For.** Parnas's A-7E team "chose to specify both mechanisms in every case, but plan to implement only the ones that are actually needed" [parnas-process:c50], so that a later need means implementing an interface, not changing it.

**Against.** The A-7E team specified interfaces inside one project that owned every implementation. A contract published to outside implementers freezes before anything has tested it: "DO NOT provide abstractions unless they are tested by developing several concrete implementations and APIs consuming the abstractions" [api-design-practice:c36], and "it would be rash to standardize before we have the models" [neighbours-systems-evolution:c27].

**Outcome.** Rejected for published contracts, and kept in Parnas's form for private ones. The reader already emits ordered blocks with spans, which a later settled offset needs, and a test pins that without publishing it ([decision 2](#decision-streaming-timing)). RFC-0006 reserved names the same way: a name held, no contract frozen.

### Leave the metric suite public

This is [decision 4](#decision-metric-suite)'s choice (b). `metrics`, `hearst` and the algorithm-named functions stay public, documented as a suite a caller may run:

```rust
let mut suite = matra::metrics::default_suite();
suite.push(Box::new(|doc: &mut matra::domain::Document| { /* may write any field */ }));
matra::metrics::run_suite(&mut doc, &suite);
```

**For.** It is shipped surface, and "Once module has users, can't change API at will" [api-design-practice:c1]. Nobody has shown that no caller uses it.

**Against.** It extends nothing a loop could not: [`compose` takes no suite from the caller](#claim-compose-ignores-suite), and [a `Metric` may write any field](#claim-metric-type), a provider interface with no stated contract about what it owns. "Make classes and members as private as possible" [api-design-practice:c7], and a plain function over `&Document` is the cheaper extension mechanism [api-design-practice:c35].

**Outcome.** Rejected; the owner widened the decision to `hearst` and the method modules.

### A narrow feed in 0.3.0

This is [decision 2](#decision-streaming-timing)'s choice (a), in Rust and the command line, with Python in 0.4.0:

```rust
let mut feed = engine.feed(Format::Markdown);
for chunk in chunks {
    for event in feed.push(chunk)? { /* a block that can no longer change */ }
}
let doc = feed.finish()?;
```

```console
$ tail -f notes.md | matra analyze - --stream --json
```

**For.** The owner settled that streaming happens, and callers could stream within a document one release sooner. The original reason was that the reader contract should break once, not twice.

**Against.** That reason holds only for a published provider interface [api-design-practice:c32]. With the reader private, a settled offset added later breaks nobody, and `Engine::feed` is an addition in the client-facing space that can grow [api-design-practice:c31]. No consumer of text arriving within one document is named, and the canon freezes an interface only after implementations and consumers have used it [api-design-practice:c36]. Parnas's procedure is to find "the minimal subset that might conceivably perform a useful service" and then "a set of minimal increments" [parnas-modules:c34]; the feed is an increment. Two syntaxes of three would also break the translation rule.

**Outcome.** Rejected; the feed lands in 0.4.0, in all three syntaxes, as an addition.

### Code points beside bytes

This is [decision 3](#decision-offset-unit)'s choice (b): `start` and `end` in code points, plus `byte_start` and `byte_end`.

```python
p = doc["sections"][0]["paragraphs"][0]
text[p["span"]["start"]:p["span"]["end"]]     # right under (b)
matra.span_text(text, p["span"])              # what (a) asks of a Python caller
```

**For.** Python slices a `str` by code point, and the W3C text position selector counts code points; under (a) every Python and TypeScript caller converts. Meyer's rule for that case: when every client repeats the same handling, "The contract should be renegotiated and made broader" [neighbours-contracts-simplicity:c8]. Gabriel's story shows the cost of the opposite trade, an implementation kept simple by pushing a step onto every caller [neighbours-contracts-simplicity:c25]. The strongest point is how (a) fails when misused. A Python caller who slices a `str` with a byte span gets the right text while the input is ASCII, and the wrong text, with no error, once a character outside ASCII comes earlier. Agent-facing guidance asks for parameters redesigned so the common mistake cannot be made [agent-computer-interfaces:c18] (vendor guidance), and under (a) it can be.

**Against.** Two encodings of one fact on every token cost every reader, agents included, for each token shown [agent-computer-interfaces:c2], and one unit is the smaller surface [api-design-practice:c6]. Bytes are the one unit identical in every binding, checkable against the input's digest, and what the input cap measures (own judgment; the canon is silent on offset units). JavaScript indexes strings by UTF-16 code unit, so (b) would not spare TypeScript callers a conversion either. The helper is the broader contract Meyer asks for, placed in one function per binding instead of in every field.

**Outcome.** Rejected; the owner accepted bytes only, with a slicing helper per binding. The quiet wrong slice is the price, and the field names (`byte_start` and `byte_end`, so the unit shows wherever a span is read), the helper and the guides are where it is answered.

### Make the text fields verbatim

The roadmap's promise would become true: `Paragraph.text` would be the exact source slice, markup, `>` markers and `\r\n` included.

**For.** A caller sees what a paragraph said without keeping the input, and the documentation stops overclaiming without a rewrite.

**Against.** The parser must read text without markup, so a verbatim field would not be what was analysed. Either the parser reads markup, or the record carries a second, normalized text, and there "must be one, and only one, place for every fact" [parnas-process:c14]. With the span as the authority, the verbatim text is one slice away for any caller who keeps the input, and the normalized text is what the metrics describe. Block input has no source beyond its blocks.

**Outcome.** Rejected. The text is documented as normalized, and the roadmap's claim is rewritten (defect 1).

### Refuse, or warn about, what a reader cannot analyse

Two alternatives to `skipped`: fail the whole analysis when a table appears, or drop the table and print a warning on stderr.

**For.** Refusing fails fast and never silently [api-design-practice:c10]. RFC 9413 argues that "Tolerating unexpected input instead conceals problems" [card:rfc9413-robust-protocols] (unverified reading).

**Against.** A table in a README is nobody's error; refusing would make most real markdown unanalysable. A warning on stderr is prose, which callers then parse until it "is implicitly part of the API contract" [machine-readable-contracts:c15], and a library caller does not read stderr at all. `skipped` follows the line RFC 9413 draws instead (the same unverified reading): the handling is specified in advance, and each case is reported to the caller rather than tolerated in silence. AIP-193's rule against partial errors [api-design-systems:c30] does not bind it, because nothing failed. L8 makes the accounting a test.

**Outcome.** Rejected; `skipped` lists each construct by kind and span.

### Fail on every unrecognised extension

This is [decision 7](#decision-unknown-extension)'s choice (b):

```console
$ matra analyze server.log --json      # unsupported_format, unless --format plain_text
```

**For.** A log, a CSV or a source file would no longer analyse as prose without complaint. For an agent walking a directory, a refusal is a signal, and a plausible analysis of a CSV is not. Bloch's fail-fast rule [api-design-practice:c10] and RFC 9413's case against tolerating unexpected input [card:rfc9413-robust-protocols] (unverified reading) both point this way.

**Against.** Plain text is documented as ["the decomposer that reads anything"](#claim-plain-reads-anything), and "Changing the default value is considered breaking" [api-design-systems:c25]. The reading is also not silent: every result's `provenance.format` says `plain_text`, so a caller who cares can check. That is the line RFC 9413 draws between flexibility with defined handling and guessing at intent (the same unverified reading). Bytes that are not UTF-8 still fail, with a kind and a remedy [api-design-systems:c28].

**Outcome.** Rejected; unknown text reads as plain text when it is valid UTF-8. The tolerance argument (an unverified reading) stands against the choice, and the answer is that the tolerance is specified and recorded, not inferred.

### Return a plain dict from Python

This is [decision 6](#decision-python-document)'s choice (b):

```python
doc = engine.analyze(file="README.md")      # a dict
doc["metrics"]["total_words"] = 0            # allowed
matra.summarize(doc, n=3)                    # rebuilds and re-checks the whole parse
```

**For.** A dict is what a Python caller expects from a JSON-shaped result, and code that tests `isinstance(doc, dict)` keeps working. "An API is a user interface" [api-design-practice:c21], and this is the interface Python callers know.

**Against.** Every outcome call would convert the whole document from Python objects into the Rust value again, where the handle hands the outcome the value the analysis made, so a text crosses the binding once however many outcomes are asked of it. A wrong argument that is a type error fails at the "first bad method invocation" [api-design-practice:c10], and agent-facing guidance says the same ("Use enums and object structure to prevent invalid states" [agent-computer-interfaces:c19], vendor guidance). The handle keeps the dict reading (`doc["metrics"]["total_words"]`), and `to_dict()` gives a real dict. Two reasons an earlier text gave do not hold. A dict would need no re-check of the parser's invariants, because the outcomes rely on none of them: they hold on any document, since Rust reads documents back from JSON ([the public surface](#reference-level-explanation)). And the handle does not make provenance proof: in Rust a document read from JSON carries whatever provenance it was given, so provenance is a record in every language ([Provenance](#reference-level-explanation)).

**Outcome.** Rejected. spaCy took the same path (Prior art).

### A builder for the engine

```rust
let engine = Engine::builder().parser(my_parser).reader(Format::Markdown, my_reader).build()?;
```

**For.** It is the usual Rust shape for an object with optional parts, and it would let a caller inject a parser or a reader.

**Against.** A builder type is one more thing to learn for at most three optional parts. `Engine::with_parser` and the two `with_` methods on an engine set the same parts with no new type, and "When in doubt leave it out" [api-design-practice:c6].

**Outcome.** Rejected. When [decision 5](#decision-provider-traits) opened the traits, injection came through `with_parser`, `with_reader` and `with_source`.

### A default embedder inside `clusters`

```python
matra.clusters(doc)      # would download an embedding model on first use
```

**For.** It matches `summarize(doc)`, and it is the shortest call.

**Against.** A call that looks like a pure function over a document would reach the network and the disk, and the result would depend on a model the call does not show. Henning's split fits: general-purpose APIs "should be 'policy-free'", and special-purpose ones "policy-rich" [api-design-practice:c20]. The library is the general-purpose part, so it takes the embedder as an argument, and the command line, the special-purpose part, builds the default from configuration.

**Outcome.** Rejected.

### Keep the `Embedder` as EPR-0005 shipped it

```rust
pub trait Embedder: Send {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Embedding>>;
    fn identity(&self) -> &str;   // for Model2Vec, the digest over its artifacts
}
```

**For.** A string is the least an implementer has to produce, and the one implementation in the tree needed no more. "When in doubt leave it out" [api-design-practice:c6].

**Against.** This is the port a caller implements, and an implementer writes against what it says [api-design-practice:c32] [api-design-practice:c37]. As shipped it said too little in two places. "Who produced this" had two shapes: the parser reports an `Identity` with its model's name, digest and licence, the embedder a string, so the embedding model's licence reached the download notice and never the result, and a reader learnt two shapes for one fact [api-design-practice:c8]. And it said nothing about failure, though the "expected behavior when undesired events occur" is part of an interface [parnas-process:c46]: `Model2Vec` returned `ModelInvalid`, the Python bridge `InvalidInput`, and an implementer had no rule to follow. Bloch's count bears on the shape: an interface with one implementation "probably won't support another" [api-design-practice:c4], and an abstraction should be tested by "several concrete implementations" before it ships [api-design-practice:c36] (corrected wording). Test doubles by the port's author do not count, by the canon's own critique, and neither does the Python bridge, so the count is one. The reshaped `identity` takes the parser's shape, the nearest second voice the tree has; a second, independent embedder written against the trait (one that calls a remote service, say) is what would show whether the shape fits.

**Outcome.** Rejected. `identity` returns `Identity`, the shape the parser reports, and the trait names the kinds an implementation returns for its own failure: `ModelInvalid` when its model cannot embed an input, `Io` when something it reaches fails, and `InputTooLarge` past a cap it holds. `InvalidInput` is what `clusters` returns when the vectors break the contract, and what the Python bridge returns for an object that raised, since it cannot tell which of the three an arbitrary exception is. No method is added for growth (a `dimensions`, which `Model2Vec` already has): no caller asks for one [api-design-practice:c6].

### Keep the limits private

```rust
pub(crate) const MAX_TFIDF_SENTENCES: usize = 2000;   // reported only as the error's `limit`
```

**For.** An interface carries only the assumptions "considered unlikely to change" [parnas-process:c20], and each module hides a decision likely to change [parnas-modules:c23]. A per-method memory bound is such a decision: its documentation justifies its value by a cost model, which is a fact about the implementation. The error already reports the cap that fired as data, in its `limit` detail [machine-readable-contracts:c13].

**Against.** A caller needs the bound before the call, not after a refusal. An agent splitting a long input under `MAX_INPUT_BYTES`, or chunking a document's sentences under the cap of the method it selects, has nothing to size from if the number arrives only inside an error; desirable properties "should not be hidden" [neighbours-hoare-brooks-lampson:c37]. The names follow the selector the caller sends (`MAX_TFIDF_SENTENCES` for `SummaryMethod::TfIdf`), so a published cap reveals the method the caller chose and nothing behind it. Parnas's concern is the value, and the design answers it without hiding the name: each constant is documented as this release's value, which may change in any 0.x release, and a caller reads it from `domain::limits` rather than copying the number.

**Outcome.** Rejected: the limits are published, because callers need them. Python and the command line state the same caps in their documentation and in the error's `limit` detail; giving Python the constants themselves is not part of 0.3.0.

### Derive the JSON Schema from the domain types

```rust
#[derive(Serialize, schemars::JsonSchema)]
pub struct Document { /* ... */ }
```

**For.** The types and the schema could not drift, because one would be generated from the other. Parnas: "There must be one, and only one, place for every fact" [parnas-process:c14]. A hand-written schema is a second home for the document's shape, so here the canon pulls against the design as made.

**Against.** It adds a dependency to `domain.rs`, which boundary rule 1 forbids without an enhancement proposal of its own. The design closes the drift by a test instead: every conformance output in every binding is validated against the schema, so a field that changes in the types and not in the schema fails the build. A schema written by a dev-only test is also allowed.

**Outcome.** Rejected for 0.3.0 on boundary rule 1. The test is weaker than construction, because it catches only the shapes the fixtures exercise.

### A second version number with a weaker rule

An earlier text of this proposal added a `schema_version` that moved only on a removal, a rename or a retype, beside the shipped `format_version`:

```json
{ "format_version": 1, "schema_version": 2, "result": { } }
```

**For.** It separates the envelope from the document, so a change to one need not move the other.

**Against.** [`format_version` already promises](#claim-format-version-ships) to move on "any change to the envelope or to the meaning of a field inside `result`", and agents were told it is the whole stability promise. A second number with a weaker rule, one that ignores a changed meaning, would let the two disagree about whether a change breaks, and an agent could not tell which to trust. Semantic compatibility is compatibility: "Code written against a previous version must continue to receive what most reasonable developers would expect" [api-design-systems:c21]. The design does keep two fields, but under one rule: `provenance.schema` carries the schema version in one place [parnas-process:c14], and `format_version` moves with its major.

**Outcome.** Rejected.

### Deprecated aliases for renamed functions

This is [decision 8](#decision-transition-window)'s choice (a), which Claude recommended:

```rust
#[deprecated(note = "use matra::summarize_with(doc, n, SummaryMethod::TfIdf)")]
pub fn tfidf_summarize(/* ... */) -> Result<Summary> { /* ... */ }
```

**For.** Of all the rejected alternatives, this is the one the canon favours most. AIP-180: "Renaming a component is semantically equivalent to 'remove and add'", and a service "must not remove the existing one" [api-design-systems:c24]. Lampson asks for interfaces "that are stable for years", with a compatibility package when one must change [neighbours-hoare-brooks-lampson:c39]. Tulach ranks functional compatibility first [api-design-practice:c29]. An alias costs one line, the compiler and the interpreter both announce it, and a deprecation that names its replacement is a correction an agent can act on [agent-computer-interfaces:c16].

**Against.** AIP-180's rule binds within a major version [api-design-systems:c20], and Cargo treats 0.2 to 0.3 as a change of major, so 0.3.0 is the place a break is allowed. AIP-180 itself says an API whose callers are few, or whose updates can be enforced, "should carefully consider its own compatibility requirements" [machine-readable-contracts:c19], and the callers of 0.2.1 are few and named. Henning describes the long-run cost: an API that stays compatible "accumulates crud that, eventually, does more damage than the good it ever did" [api-design-practice:c18]. Every later session would read past the aliases until 0.4.0 removed them.

**Outcome.** Rejected by the owner's redirect to (c): "we don't have to worry about backwards compatibility". Here the canon argues against the decision as made. AIP-180, Lampson and Tulach favour (a); what answers them is the 0.x rule, the small set of callers, and the owner's ruling.

### The full transition window

This is [decision 8](#decision-transition-window)'s choice (b): choice (a), plus every renamed JSON field emitted under both names, hidden aliases for renamed flags, and the 0.2 output on request:

```console
$ matra analyze README.md --json --format-version 1     # the 0.2 output, through 0.3.x
```

**For.** An agent pinned to `format_version: 1` keeps working and migrates on its own schedule. AIP-185 asks that "Different versions of the same API must be able to work at the same time within a single client application for a reasonable transition period" [machine-readable-contracts:c20], and Lampson's compatibility package for a changed interface is the same kind of thing [neighbours-hoare-brooks-lampson:c39]. `cargo metadata --format-version` is the precedent (Prior art).

**Against.** A both-name emitter and a 0.2 projection are a second implementation of the output, kept for one minor and then removed, and every later session must read past them until then. The wire already tells a reader which side it is on: `format_version` moves to 2, every result carries `provenance.schema`, and the consumer rule is to "Reject any input which reports an unsupported major version" [machine-readable-contracts:c35]. A pinned agent therefore fails loudly at once rather than misreading a result.

**Outcome.** Rejected by both the recommendation and the ruling.

## Prior art

Each entry says what the source did, what matra takes from it, and where matra departs and why.

- **The owner's software-design canon** (462 claims from primary texts, each checked against a fresh copy of its source; cited here as `[cluster:cN]`), and its canon-0010 reading cards (cited as `[card:...]`, unverified readings). It supplies the grounds for every structural choice on this page. matra departs from it where it is silent: offset units, resilience bounds and the choice of Markdown parser are marked as own judgment. Its own review warns that "Zero refutations is evidence that the quotes are faithful. It is not evidence that the canon is balanced", and that no part of it was gathered from its authors' critics. That is why the rationale above names what each citation rules out and where the canon pulls the other way.
- **J. Bloch, "How to Design a Good API and Why it Matters", 2006.** Heuristics drawn from the Java libraries: leave it out when in doubt, keep everything as private as possible, fail fast, give programmatic access to data available as strings, and write three plug-ins before shipping a plug-in interface. matra takes most of the defaults: aggregates as fields, typed errors, private method modules. It departs on plug-ins: all four ports are public with one or two implementations each in the tree, short of the three, and `Embedder` is the one a 0.3.0 use case asks for. Test doubles and the Python bridge to caller objects are not independent implementations, so the count is one. The `Embedder` stays public because it is what a caller with vectors of its own (from a service, a cache, a library matra does not ship) hands to `clusters`, the one extension point a 0.3.0 use case asks for. The one-implementation risk is the port's shape, and [Reshape the `Embedder`](#keep-the-embedder-as-epr-0005-shipped-it) records how that was answered.
- **M. Henning, "API Design Matters", 2007.** An API is a user interface for programmers, and a flaw in it is paid for at every call site. matra takes the caller-first stance behind the outcome surface, and Henning's split between policy-free general APIs and policy-rich special ones is why the command line, not the library, builds a default embedder. It departs on timing: Henning says the worst time to document "is after implementation" [api-design-practice:c23]. This proposal was written before the code, and it was then revised as each milestone settled a detail against the code. That is iteration, which Brooks argues for when he writes that the building metaphor "has outlived its usefulness" [neighbours-hoare-brooks-lampson:c33].
- **AIP-180 and AIP-193 (Google's API improvement proposals for compatibility and errors).** AIP-180 treats a rename as a removal, a changed default as a break, and semantic compatibility as compatibility. AIP-193 gives each error a machine-readable identity defined by the caller's remedy, puts every dynamic value in metadata, and forbids partial success except in bulk operations. matra takes all of these: the Names table lists every rename as a break, a changed default method is a schema-major change, `details` carries every dynamic value, and `analyze DIR` is the one bulk operation with per-item failure. It departs twice. On `invalid_input`, `details.reason` names the check that failed and the message adds values for a person that the details do not repeat ([Errors on every channel](#reference-level-explanation)). And on keeping old names. AIP-180 keeps them within a major version; 0.3.0 starts a new one under Cargo's rules for 0.x, and it keeps none ([decision 8](#decision-transition-window)).
- **The Kubernetes API conventions.** Defaults are written explicitly, and phase enums were deprecated because "adding new enum values breaks backward compatibility". matra takes explicit defaults and the rule that every string set a consumer receives is open. It departs by keeping such sets rather than replacing them with conditions. matra's Rust enums are `#[non_exhaustive]`, which forces a wildcard arm, and JSON readers are told to treat every set they receive as open. The send-versus-receive distinction this relies on is the canon synthesis's suggestion, not Kubernetes's.
- **D. L. Parnas, "On the Criteria To Be Used in Decomposing Systems into Modules", 1972, and the 1985 module guide.** The 1972 paper decomposes one small program two ways and shows that the decomposition by hidden decisions survives change better than the one by processing steps. The 1985 guide describes each module of a large system by its secret. matra takes both: the input-side table names each module's secret. One thing deserves saying plainly, because the figure looks like a pipeline. Parnas's own second decomposition still has an input module, a circular shifter, an alphabetizer and an output module (read in the 1972 text, Modularization 2). What separates it from the flowchart is what each module hides, not the order the modules run in, and the same test applies here: a new format or parser touches one box. matra departs on scale. The A-7E guide is a tree with restricted and hidden interfaces, because hiding without a guide "could backfire" at hundreds of modules [parnas-process:c23]; matra has nine modules and a flat table. It also reveals more than Parnas's "even the presence or absence of a component should be hidden" [parnas-modules:c37]: an `UnsupportedFormat` names the feature that would read the format, because the caller needs the remedy, the kind of restriction the A-7E team revealed when "there is no loss in revealing" it [parnas-process:c49].
- **D. D. Woods, "The theory of graceful extensibility", 2018**, outside the verified canon. Brittleness is how fast performance falls near a unit's boundary, and neighbouring units extend it by monitoring its signals. matra takes the idea of a signal at the boundary: `skipped` is that signal. It departs in scope: Woods writes about adaptive units in sociotechnical systems, and matra borrows one idea, not the theory. The canon claims cited for `skipped` carry the decision alone.
- **A. Cockburn, "The Hexagonal (Ports and Adapters) Architecture", 2005.** An application is driven through ports named for the purpose of a conversation, adapters translate each technology to a port, and business logic that leaks into an adapter is caught by driving every function through the API in tests [card:cockburn-hexagonal-current] (unverified reading). matra takes the diagnosis (application rules leaked into the Python binding and the command line) and the remedy: the outcome surface is the driving port, and the parity test drives all three adapters through it. It follows him on the driven side too: the parser, reader, source and embedder ports are open, so a test or a new technology is swapped in from outside the crate, and what crosses a parser or reader port is checked before it reaches a result ([decision 5](#decision-provider-traits)).
- **Terraform's JSON output format.** Its `format_version` is a major and a minor; a consumer ignores properties it does not recognise and rejects a major it does not know [machine-readable-contracts:c35]. matra takes those consumer rules unchanged and the major-and-minor form for `provenance.schema`. It departs by keeping the shipped integer `format_version` on the command-line envelope as well, moving with the schema major, because agents already pin it.
- **`cargo metadata --format-version`.** One integer version, chosen by a flag; within a version, Cargo does not count new fields or new values of enum-like fields as incompatible, and it recommends passing the flag to future-proof a caller. matra takes the integer and the rule that new fields and values are compatible. It departs by offering no flag to select an older output, because 0.3.0 keeps no 0.2 projection ([decision 8](#decision-transition-window)).
- **The W3C Web Annotation Data Model.** Its text position selector has `start` and `end` and counts "in terms of unicode code points", not code units. matra takes the shape and departs on the unit: its spans count UTF-8 bytes, and the fields say so, `byte_start` and `byte_end` ([decision 3](#decision-offset-unit)). The standard's names with a different unit would be a trap for a caller who knows the standard, which is why the names carry the unit and the slicing helper exists.
- **CommonMark, and pulldown-cmark.** CommonMark is a specification of Markdown with a conformance test suite, and pulldown-cmark is a pull parser for it whose offset iterator pairs every event with its range in the source. matra takes the specification instead of its own line scanner, which is what removes the dropped tables and the `## References` cut-off, and the source ranges, which give every block its span. The parser crate is chosen in milestone 2's pull request; the canon does not address it.
- **Universal Dependencies and CoNLL-U.** UD supplies the tag vocabularies matra adopts (the universal part-of-speech tags, the dependency relations, the morphological features), and CoNLL-U is the ten-column text encoding UDPipe emits, its columns `ID`, `FORM`, `LEMMA`, `UPOS`, `XPOS`, `FEATS`, `HEAD`, `DEPREL`, `DEPS` and `MISC`, with the features packed as `Name=Value|...` strings ([the CoNLL-U format](https://universaldependencies.org/format.html)). matra takes the tag values as is and departs from the encoding: `features` becomes a map. It also departs on the column names, which this page does not argue for: `pos` holds `UPOS` and `dep` holds `DEPREL`, while `xpos`, `deps` and `misc` keep CoNLL-U's names, so a reader fluent in CoNLL-U meets a mapping, and `dep` sits one letter from `deps`. UDPipe's own `ranges` option stores each token's range "counted in Unicode characters". matra's spans count bytes, so whichever way the adapter finds token positions (that option, or aligning token forms against the text), it converts them at the one place each paragraph is parsed.
- **spaCy.** A `Language` object (`nlp`) is the one entry point, and `nlp(text)` returns a `Doc` whose values are attributes, with `Doc.to_json()` for a plain-data view and, added later (in 3.3.1), a `Doc.from_json` that can validate its input. matra takes the shape for Python: one `Engine`, a read-only `Document` handle with `to_dict()`, and a from-JSON entry left for later ([decision 6](#decision-python-document)). It departs twice. spaCy's offsets (`Token.idx`) count characters, and matra's count bytes. spaCy's pipeline is open, so a caller can add a component with `Language.add_pipe`; matra keeps its stages private and opens only the parser, reader and source, each replacing one part.
- **Earlier records here.** RFC-0006 reserved `SourceSpan`, sketched in bytes as an offset and a length, and the names `ParagraphMetrics` and `DocumentMetrics`; this proposal takes the bytes and the record names, and departs on the span's shape, `Span { byte_start, byte_end }`, whose names carry its unit ([decision 3](#decision-offset-unit)). RFC-0007 made the surface one pipeline with public stages; this proposal keeps its laws and takes the stages off the surface (above). EPR-0002's rule that a derived field names a construction, "never a category", is why `Reporting` becomes `ClausalComplement` and why `skipped` kinds name constructs.

## Unresolved questions

<decision id="algorithm-choice" title="1. Does the caller keep a choice of method?" reversible="costly" grounds="algorithm-names-today cli-method-flag score-scales default-method use-cases-no-algorithm">

<choice key="a" title="No: outcomes only, the method recorded">

`summarize(&doc, n)` and `keyphrases(&doc, n)` with no way to choose. matra picks the method; every result names the one that ran. `--method` and the `algorithm` config keys are removed. A typed selector can be added later without breaking anyone.

</choice>

<choice key="b" title="Yes, optionally: outcomes first, a typed method on request">

The same outcome calls are the common form. An optional typed method selects explicitly: `summarize_with(&doc, n, SummaryMethod::TextRank)` in Rust, `method="textrank"` in Python (a closed set of literals), `--method` on the command line. Every result records the method that ran. No function name carries an algorithm.

</choice>

<recommendation choice="b">

On names, the canon agrees with the owner's question "why are the methods implementation specific? that breaks apis", and both choices take the names out: a likely change should touch no interface [parnas-process:c22]. On removing the choice, it does not. The canon card's fifth rule is to hide undesirable properties, not desirable ones: "desirable ones should not be hidden" [neighbours-hoare-brooks-lampson:c37]. Which method ran decides what a score means: <claim id="score-scales" basis="observed">the skill tells agents the RAKE and YAKE scales are unrelated [SKILL.md:105](../../skills/matra/SKILL.md#L105 "The two scales are unrelated")</claim>, so a caller comparing scores, or replicating a published result, needs to name the method. <claim id="cli-method-flag" basis="observed">The command line already offers `--method` [cli/mod.rs:180](../../src/cli/mod.rs#L180 "method: Option<SummaryMethod>")</claim>, so keeping a choice adds nothing; removing it is the break.

</recommendation>

<against>

Leave it out when in doubt, because "you can always add, but you can never remove" [api-design-practice:c6]. The ranking algorithm is a legitimate secret [card:cunningham-2019-modular-design-notes] (unverified reading). <claim id="use-cases-no-algorithm" basis="assumed">None of the 26 use cases in the API audit behind this proposal asks for an algorithm; they ask for a summary or for keyphrases.</claim> Under (a), changing the method behind the default is still a schema-major change, so a caller is told when results move: <claim id="default-method" basis="observed">today's default summary method is TF-IDF [default.toml:8](../../config/default.toml#L8 "algorithm")</claim>.

</against>

<ruling response="accept" date="2026-11-02">

The owner chose (b) in conversation: outcome-named functions, the method optional and recorded with its parameters, and a change of default method treated as breaking.

</ruling>

</decision>

<decision id="streaming-timing" title="2. When does text that arrives over time get its implementation?" reversible="yes" depends="provider-traits" grounds="decomposer-today paragraph-unit">

<choice key="a" title="0.3.0: a narrow feed in Rust and the command line">

The readers return blocks with a settled offset, the point after which appending text cannot change a block. `Engine::feed(format)` returns a synchronous `Feed` whose `push(chunk)` returns the events for every block that became settled and whose `finish()` returns the whole `Document`; `matra analyze - --stream --json` writes one line per event. Python's feed follows in 0.4.0.

</choice>

<choice key="b" title="0.4.0: the feed in all three syntaxes, as an addition">

0.3.0 ships no feed. The 0.3.0 markdown reader already emits blocks in order with spans, the precondition a settled offset needs, and a test pins that without publishing it. In 0.4.0 the feed arrives in Rust, Python and the command line together, as new items beside the existing ones.

</choice>

<recommendation choice="b">

The reason for 0.3.0 was that the reader contract had to break once, not twice. That holds only for a published provider interface, which cannot grow [api-design-practice:c32]; with the reader trait private (decision 5), a settled offset added later breaks nobody, and what remains public is an addition, `Engine::feed` and a `Feed` type, in the client-facing space that can grow [api-design-practice:c31]. No consumer of text arriving within one document is named; the one written streaming request is for many documents, which `analyze_all` meets in 0.3.0. The canon freezes an interface only after implementations and consumers have used it [api-design-practice:c36]. Shipping two syntaxes of three would also break "one API, three syntaxes". <claim id="decomposer-today" basis="observed">Today the reader returns a tree of copied strings [decompose/mod.rs:17](../../src/decompose/mod.rs#L17 "fn decompose(&self, text: &str) -> Vec<Section>;")</claim>, and <claim id="paragraph-unit" basis="observed">the paragraph is already the unit the parser runs on [lib.rs:239](../../src/lib.rs#L239 "para.sentences = self.nlp.parse(&para.text)?;")</claim>, so the paragraph stays the unit of finality when the feed lands.

</recommendation>

<against>

The owner settled that streaming happens, and an x.uma extension that feeds body chunks would be a consumer the moment it exists; another release cycle passes before any caller can stream within a document.

</against>

<ruling response="accept" date="2026-11-03">

The owner accepted Claude's recommendations in conversation: streaming lands in 0.4.0, additively, in all three syntaxes.

</ruling>

</decision>

<decision id="offset-unit" title="3. What unit does a span count in?" reversible="costly" grounds="rfc0006-span sentence-text-rebuilt">

<choice key="a" title="UTF-8 bytes only">

`span: { byte_start, byte_end }` in UTF-8 bytes of the analysed text. Each binding offers a helper (`matra.span_text(source, span)` in Python, the same in TypeScript) to slice by it. The unit is in the field names because a Python `str` sliced with a byte span is right on ASCII and wrong, with no error, after the first character outside it; the name puts the unit at every call site.

</choice>

<choice key="b" title="Code points and bytes">

`span: { start, end }` in Unicode code points, as the W3C Web Annotation Data Model's text position selector counts them, plus `byte_start` and `byte_end` in UTF-8 bytes, both computed once in Rust.

</choice>

<recommendation choice="a">

One unit is the smaller surface [api-design-practice:c6], and two encodings of one fact on every token cost every reader, agents included, for each token shown [agent-computer-interfaces:c2]. RFC-0006 sketched `SourceSpan` in bytes; bytes are the one unit identical in every binding, cheap to check against the input's digest, and what the input cap already measures (own judgment; the canon is silent on offset units). The earlier recommendation of (b) rested on the belief that the downstream consumer asked for character offsets; its own written requirements say spans into the bytes the caller passed are enough. A helper per binding is one line.

</recommendation>

<against>

Python slices strings by code point and the one standard for text selection counts code points, so under (a) every Python and TypeScript caller converts; Meyer's rule for that case is to widen the contract when every client repeats the same handling [neighbours-contracts-simplicity:c8]. JavaScript indexes by UTF-16 unit, which neither choice serves.

</against>

<ruling response="accept" date="2026-11-03">

The owner accepted Claude's recommendations in conversation: bytes only, with a slicing helper per binding.

</ruling>

</decision>

<decision id="metric-suite" title="4. Does the public metric suite become crate-private?" reversible="costly" grounds="compose-ignores-suite metric-type">

<choice key="a" title="Yes, in 0.3.0, with hearst and the method modules">

`metrics` (including the text-taking readability helper), `hearst`, and the algorithm-named functions in `extraction` become private to the crate. The measures stay public as fields of the document. A caller who wants another measure writes a function over `&Document`. Caller extensions are designed in their own proposal.

</choice>

<choice key="b" title="No; leave them public">

The items stay, documented as a suite the caller may run themselves.

</choice>

<recommendation choice="a">

<claim id="compose-ignores-suite" basis="observed">`compose` always runs the default suite and takes none from the caller [lib.rs:251](../../src/lib.rs#L251 "let suite = metrics::default_suite();")</claim>, so the public suite extends nothing a caller could not do with a loop. <claim id="metric-type" basis="observed">A metric is a closure that may write any field of the document [metrics/mod.rs:31](../../src/metrics/mod.rs#L31 "pub type Metric = Box<dyn Fn(&mut Document)>;")</claim>: a provider interface with no stated contract about what it owns. "Make classes and members as private as possible" [api-design-practice:c7]; keep what clients call apart from what providers implement [api-design-practice:c31]; a plain function over published types is the cheapest extension mechanism [api-design-practice:c35].

</recommendation>

<against>

It is shipped public surface, and with enough users every observable behaviour is depended on [api-design-systems:c17]. No caller has been shown to use it, and none shown not to.

</against>

<ruling response="accept" date="2026-11-03">

The owner accepted Claude's recommendations in conversation, which included this one widened to `hearst` and the method modules.

</ruling>

</decision>

<decision id="provider-traits" title="5. Are the parser, reader and source traits open to outside implementers in 0.3.0?" reversible="yes" grounds="parser-unvalidated roadmap-wasm-promise">

<choice key="a" title="Private to the crate">

`Source`, `Decomposer` and `NlpProvider` become `pub(crate)`, keeping their names and boundary rules inside the crate. Outside code extends matra through `Input::bytes` (its own fetching) and `Input::blocks` (its own formats). `Embedder` stays public and open.

</choice>

<choice key="b" title="Public behind an unstable feature">

The traits stay reachable behind a Cargo feature named `unstable`, outside the semver promise, until a third implementation of each exists.

</choice>

<choice key="c" title="Public, with their output checked">

`Source`, `Decomposer` and `NlpProvider` are public. `Engine::with_parser`, `with_reader` and `with_source` plug in a caller's own, and one module checks what a parser or reader returns before any of it reaches a `Document`.

</choice>

<recommendation choice="a">

The owner's stated reason for clean architecture is that later sessions, which extend matra from inside the crate, can change it safely. In-tree extension needs no public trait: a new reader is a file and a registration line. A public trait serves only outside implementers, and none is named. Privacy is also the reversible choice: a private trait can become public later as an addition, and a public one cannot go back. What clients call and what providers implement evolve under opposite rules [api-design-practice:c31]; a provider interface takes "no new method additions" once someone implements it [api-design-practice:c32], and these contracts change in this very release. The parser's contract is trusted unchecked, so publishing it would let outside code break invariants the tree walks rely on. An `unstable` feature still publishes the trait to anyone who enables it, and a feature that exists invites dependence on it [api-design-systems:c17].

</recommendation>

<against>

The owner has asked for matra to be extensible by others, and a WebAssembly or transformer parser written outside matra would need a public parser trait. An `unstable` feature is the Rust community's usual way to offer that without a promise.

</against>

<ruling response="redirect" date="2026-11-06">

The owner accepted (a) on 2026-11-03, "for now", then redirected to (c) on 2026-11-06: "there is no issue with breaking changes pre 1.0 okay, we need to enable the affordances". Before 1.0 a break costs nothing, so privacy's one advantage, that it can be undone, bought nothing, and the unchecked contract is a reason to check at the boundary, not to hide the port. `src/conform.rs` checks the parser's tokens (ids, heads, one root, no cycle, spans against forms) and the reader's blocks (spans, order, heading levels, paragraph text against its span), each failure `parse_failed` with a stable `reason`; [#178](https://github.com/mox-labs/matra/pull/178) implements it. One cost moves: [decision 2](#decision-streaming-timing)'s reason that a later feed breaks nobody now holds for callers of the outcomes but not for outside readers, and before 1.0 the owner does not count it.

</ruling>

</decision>

<decision id="python-document" title="6. What does Python's analyze return?" reversible="costly" grounds="python-annotated">

<choice key="a" title="A read-only Document handle">

`matra.Document` holds the Rust value, with mapping access that mirrors the JSON (`doc["metrics"]["total_words"]` keeps working) and `to_dict()`. The outcome functions accept only that type in 0.3.0.

</choice>

<choice key="b" title="A plain dict, as today">

`analyze` returns a dict, and every outcome call rebuilds the Rust value from it.

</choice>

<recommendation choice="a">

Under (b), every `matra.summarize(doc)` converts the whole document from Python objects into the Rust value again; under (a) the outcome takes the value the analysis made, so a text crosses the binding once however many outcomes are asked of it. Making the wrong argument unrepresentable fails at the first bad call [api-design-practice:c10]. A from-dict entry belongs with the TypeScript from-JSON entry, designed once.

Two reasons this recommendation first gave do not hold, and the choice stands without them. The outcomes rely on none of the parser's invariants, since Rust reads documents back from JSON and every outcome holds on a document whose parts contradict each other, so a dict would need no re-check. And neither choice makes provenance proof: it records how a document says it was made, in every language ([Provenance](#reference-level-explanation)).

</recommendation>

<against>

Code that tests `isinstance(doc, dict)` or mutates the result breaks, and a dict is what Python callers expect from a JSON-shaped result.

</against>

<ruling response="accept" date="2026-11-03">

The owner accepted Claude's recommendations in conversation: a read-only `Document` with `to_dict()`.

</ruling>

</decision>

<decision id="unknown-extension" title="7. Does a text file with an unknown extension still read as plain text?" reversible="costly" grounds="plain-reads-anything binary-as-io">

<choice key="a" title="Yes, when its bytes are UTF-8">

An unknown extension, or none, reads as plain text, as today. Only a recognised format the build cannot read, or bytes that are not UTF-8 and carry no known signature, fail with `UnsupportedFormat`.

</choice>

<choice key="b" title="No: every unrecognised extension fails">

`UnsupportedFormat` for anything without a known extension, unless the caller names a format.

</choice>

<recommendation choice="a">

<claim id="plain-reads-anything" basis="observed">The documented rule is that plain text is "the decomposer that reads anything" [domain.rs:1301](../../src/domain.rs#L1301 "plain text is the decomposer that reads anything")</claim>, and changing a default is a behaviour change [api-design-systems:c25]. The owner's "unknown formats fail" is read as recognised-but-unread formats and undecodable bytes, which today fail as I/O prose and will fail as a kind with a remedy [api-design-systems:c28].

</recommendation>

<against>

A log, a CSV or a source file then analyses as prose without complaint, and a caller learns nothing of it.

</against>

<ruling response="accept" date="2026-11-03">

The owner accepted Claude's recommendations in conversation: unknown text reads as plain text when it is valid UTF-8.

</ruling>

</decision>

<decision id="transition-window" title="8. What stays callable under its old name for 0.3.x?" reversible="costly" grounds="format-version-ships migrate-once">

<choice key="a" title="Renamed functions only; one break on the wire">

Renamed Rust functions and methods (`tfidf_summarize`, `Engine::with_defaults` and the like) and renamed Python methods stay as deprecated aliases that name their replacement. Types and modules made private go at once. The JSON carries schema 2.0 only, with `format_version: 2`, and no 0.2 projection.

</choice>

<choice key="b" title="The full window">

Choice (a), plus every renamed JSON field emitted under both names for 0.3.x, renamed CLI flags kept as hidden aliases, and `--format-version 1` emitting the 0.2 output.

</choice>

<choice key="c" title="Nothing: one clean break everywhere">

No deprecated aliases in Rust or Python, no hidden CLI flags, no both-name JSON and no 0.2 projection. The migration guide lists every rename.

</choice>

<recommendation choice="a">

The wire already tells a reader which side it is on: the envelope's `format_version` moves to 2 and every result carries `provenance.schema`, and the consumer rule is to reject a major you do not know. A both-name emitter and a 0.2 projection are a second implementation of the output, maintained for one minor and then removed, and every later session must read past them until then. A deprecated alias for a renamed function is one line that the compiler and the interpreter both announce, and a deprecation that names its replacement is an instruction an agent can follow [agent-computer-interfaces:c16]. The callers of 0.2.1 are few and named, so the full window buys little.

</recommendation>

<against>

Old and new should coexist for a transition period [machine-readable-contracts:c20], and an agent pinned to `format_version: 1` fails at once rather than migrating on its own schedule; the price of not breaking pinned callers is a compatibility projection [neighbours-hoare-brooks-lampson:c39].

</against>

<ruling response="redirect" date="2026-11-03">

The owner redirected to (c) in conversation: "we don't have to worry about backwards compatibility". 0.3.0 keeps no old name anywhere; the migration guide carries the renames.

</ruling>

</decision>

What the owner should confirm or strike, collected from the claims above that stand on no grounds yet:

<assumptions />

## Future possibilities

- **The feed** for text arriving over time, in 0.4.0, in all three syntaxes (decision 2).
- **Caller extensions**: predicates producing findings, the names RFC-0006 reserved, in their own proposal, on this schema. Freeze that provider interface only after three uses [api-design-practice:c4].
- **x.uma extensions**, implemented in matra behind a feature named for the dependency, as a driving adapter over the public surface, once the x.uma crates are published.
- **More formats**: HTML, DOCX and PDF, each a reader file and a registration line, with OCR as an optional adapter the PDF reader calls and `Document.text` (an optional field, a schema minor) carrying the analysed text when it is not the input.
- **A URL source**, as `Input::url` behind a feature.
- **TypeScript**, in three steps, each usable alone: types generated from the published schema and a web component that renders a parse from matra's JSON; then the pure-Rust core in WebAssembly; and a spike, with a go or no-go, on compiling UDPipe's C++ to WebAssembly.
- **A permissively licensed default parser** trained on UD English EWT, as a second pinned model chosen by id, visible in `provenance.parser`.

## Defects in what ships

Found while writing this proposal. Those the milestones remove are marked; the rest are fixed in a separate pull request.

| # | Defect | Evidence |
|---|---|---|
| 1 | The roadmap says sentence and paragraph text are verbatim; the code rebuilds both (milestone 2 documents the text as normalized and adds spans) | <claim basis="observed">[ROADMAP.md:193](../../ROADMAP.md#L193 "being verbatim by design") against [udpipe.rs:778](../../src/nlp/udpipe.rs#L778 "Reconstruct original text using SpaceAfter=No")</claim> |
| 2 | Markdown paragraph text drops `\r` from `\r\n` input and strips `>` from blockquotes (milestone 2) | <claim basis="observed">[markdown.rs:15](../../src/decompose/markdown.rs#L15 "text.lines()") [markdown.rs:78](../../src/decompose/markdown.rs#L78 "trim_start_matches('>')")</claim> |
| 3 | Python's extractors read markdown as plain text, so a README's headings are summarized; the command line does not (milestone 5) | <claim basis="observed">[lib.rs:487](../../src/lib.rs#L487 "Format::PlainText") against [cli/mod.rs:612](../../src/cli/mod.rs#L612 "on a README would return its headings")</claim> |
| 4 | `total_words` and the other aggregates are printed in the table and absent from `--json` (milestone 1) | <claim basis="observed">[render.rs:45](../../src/cli/render.rs#L45 "doc.total_words()")</claim> |
| 5 | A failure under `--json` writes nothing to stdout (milestone 6) | <claim basis="observed">[cli/mod.rs:102](../../src/cli/mod.rs#L102 "matra: {e}")</claim> |
| 6 | `Document::passive_ratio()` returns `0.0` on a document with no sentences, while the field stays `None` | <claim basis="observed">[domain.rs:1185](../../src/domain.rs#L1185 "pub fn passive_ratio(&self) -> f64") [document.rs:19](../../src/metrics/document.rs#L19 "if analysis.total_sentences() > 0")</claim> |
| 7 | `cargo-semver-checks` runs on pull requests only, though EPR-0003 says every push (milestone 7) | <claim basis="observed">[ci.yml:318](../../.github/workflows/ci.yml#L318 "if: github.event_name == 'pull_request'")</claim> |
| 8 | A missing file reaches Python as `OSError`, not `FileNotFoundError` (milestone 4) | <claim basis="observed">[lib.rs:446](../../src/lib.rs#L446 "Io(_) => PyOSError::new_err(msg)")</claim> |
| 9 | The main size gate's message reads "input input too large" | <claim basis="observed">[domain.rs:166](../../src/domain.rs#L166 "input too large: {actual} > limit {limit}") [lib.rs:34](../../src/lib.rs#L34 "what: \"input\"")</claim> |
| 10 | The roadmap names `analyze_directory`, which does not exist | <claim basis="observed">[ROADMAP.md:118](../../ROADMAP.md#L118 "analyze_directory")</claim> |
| 11 | The embedding model's pin carries no licence, unlike the parsing model's, and its first download is silent | <claim basis="observed">[model2vec.rs:66](../../src/embed/model2vec.rs#L66 "const POTION_BASE_8M_URLS") against [udpipe.rs:40](../../src/nlp/udpipe.rs#L40 "CC BY-NC-SA 4.0 (non-commercial)")</claim> |
| 12 | `vocabulary_ttr` is unsound across documents of different length, and nothing on the type says so | <claim basis="observed">[ROADMAP.md:118](../../ROADMAP.md#L118 "As a cross-document feature it is currently unsound")</claim> |
| 13 | matra's ACES skill pairs the counter-forces with the decay forces differently from the ACES synthesis it comes from | <claim basis="observed">[aces SKILL.md:43](../../.claude/skills/aces/SKILL.md#L43 "Counters drag.")</claim> |
| 14 | The markdown reader drops tables, ends the document at a `## References` line, and reads `#hashtag` as a heading (milestone 2) | <claim basis="observed">[markdown.rs:55](../../src/decompose/markdown.rs#L55 "if in_code_block") [markdown.rs:58](../../src/decompose/markdown.rs#L58 "## References") [markdown.rs:62](../../src/decompose/markdown.rs#L62 "stripped.starts_with('#')")</claim> |
