# EPR-0001: The pipeline and its ports

- Feature Name: `pipeline_and_ports`
- Start Date: 2026-10-04
- Proposal PR: [#133](https://github.com/mox-labs/matra/pull/133)
- Tracking issue: none (a baseline proposal describes code that already ships, so there is no milestone to track)
- Status: proposed (only the owner changes it to `accepted`)
- Pinned at: `4fcfb4adc85524243c5e392becd4f490e3e42252`

## Summary

matra is one crate whose surface is one pipeline. A caller turns input into
an `Ingest` stream and hands it to an `Engine`, which runs each document
through two stages: `annotate` (decompose the text into sections and
paragraphs, then parse each paragraph) and `compose` (run the metric suite).
`annotate` is the only route from text to the parser, so the input size cap
is a property of the pipeline rather than of each entry point. Four ports
(`Source`, `Decomposer`, `NlpProvider`, `Embedder`) declare what the pipeline
needs from the outside; adapters implement them; `src/lib.rs` is the
composition root that wires them; eight boundary rules keep the dependency
arrows pointing inward.

This is a baseline proposal. It describes the code at the pinned commit and
carries forward the reasons that still hold from RFC-0002, RFC-0003,
RFC-0004, RFC-0007, EP-0008 and EP-0014. A later proposal that changes any of
it says so. Unmarked statements about the code are observed at the pinned
commit through the link beside them; a claim that is inferred or assumed
says so.

<pragmatics>

<ask>

Accept EPR-0001 as the record of matra's pipeline and ports as they stand at `4fcfb4a`, and settle the two decisions under [Unresolved questions](#unresolved-questions). Each can be answered on its own.

</ask>

<will>

- Cite this proposal, rather than the legacy records it carries forward, when a change touches the pipeline, its ports or the boundary rules; a change to what it describes goes in a new proposal that says so.
- Carry out each decision as you rule it, through the record the process names for that change.
- If the cited code moves before you decide, move the `Pinned at` line forward in a revision, so every citation is checked again, and say what changed.

</will>

<needs>

- An answer to the proposal and to each of the two decisions: accept, accept with a reservation, object or redirect.

</needs>

<wont>

- Change the status to `accepted`, or merge a pull request that does.
- Change the code either decision names before you rule on it.
- Edit a legacy record, or edit this proposal to describe a later change.

</wont>

<silence>

The proposal stays `proposed`. The code it describes ships as it is, since a baseline decides nothing new, and Claude lists the open decisions again at the start of the next session that touches the pipeline or its ports.

</silence>

</pragmatics>

## Motivation

Why this is the foundation, rather than any other shape, comes down to four
reasons, each paid for by a defect or a near miss.

**One implementation of each invariant.** Before 0.1.0 the public Rust
surface was six free functions, each a partial application of the same
chain, and each restated the invariants itself. Two live defects came from
that shape: the input size cap held on four Python methods and not on the
other four, and `analyze_from` returned documents whose paragraph metrics
were always `None` because the sentence set travelled twice with nothing
enforcing agreement ([EP-0008](../legacy/eps/0008-pipeline-surface.md),
"The two defects"). [RFC-0007](../legacy/rfcs/0007-one-pipeline.md) replaced
the six with one pipeline so each invariant has one home.
<claim basis="observed">Today the size check sits in `check_input_size`, called first thing in `Engine::annotate`, and the metric suite takes the document alone (`Metric`), so the sentence set exists once [lib.rs:23-38][size-fn] [lib.rs:212-253][annotate] [metrics/mod.rs:22-57][metric]</claim>.

**Per-paragraph parsing.** An earlier pipeline joined paragraphs, parsed
once, and wired sentences back to paragraphs by prefix match. Two paragraphs
sharing their first 30 characters had their sentences silently reassigned
(the failure the project calls FM1).
<claim basis="observed">`annotate` parses each paragraph on its own, and a regression test, `parse_per_paragraph_scopes_sentences_to_originating_paragraph`, pins the case [lib.rs:235-243][annotate-loop] [lib.rs:1475-1496][fm1-test]</claim>.

**Ports factored by dependency and failure mode.** A port exists where the
outside world differs in what it depends on and how it fails: reading bytes
(`Source`), cutting text into structure (`Decomposer`, total), parsing
(`NlpProvider`, native code and a panic boundary), embedding (`Embedder`,
model output). [RFC-0007](../legacy/rfcs/0007-one-pipeline.md) kept the
trait names [RFC-0002](../legacy/rfcs/0002-pipeline-vocabulary.md) chose
for this reason: they are ports, not stages.

**One crate.** [RFC-0004](../legacy/rfcs/0004-stay-single-crate.md) kept
matra a single crate because the test for splitting out a port crate is
whether external implementors need to pin the contract apart from the main
crate's version churn, and no such implementors exist.
<claim basis="observed">There is one `[package]` in `Cargo.toml` and no workspace [Cargo.toml:1-14][cargo-package]</claim>.

## Guide-level explanation

### For a caller

You hold an `Engine` and feed it documents. A string is a stream of one, a
file is a stream of one, a directory is a stream of many; all three are an
`Ingest`, one concrete type ([`Ingest`][ingest]).

```rust
use matra::domain::{CorpusResult, Format};
use matra::{Engine, Ingest};

let engine = Engine::with_defaults()?;

// One document in memory: a stream of one.
let one: CorpusResult = engine
    .analyze(Ingest::text("The report was filed.", Format::PlainText))
    .collect();

// A directory: each file is read when the stream is pulled, and a file
// that fails becomes an error item rather than ending the walk.
let many: CorpusResult = engine.analyze(Ingest::path("docs/")?).collect();
for failure in &many.errors { /* failure.path, failure.error */ }
```

<claim basis="observed">`analyze` is lazy: nothing is read or parsed until the iterator is pulled, and each pull runs one document to completion [lib.rs:163-192][analyze]</claim>.
So `Ingest::path(dir)?` returning `Ok` means the directory listed, not that
every file read.
<claim basis="observed">Per-file failures arrive as `DocumentError` items carrying the path [lib.rs:89-113][ingest-path]</claim>,
and collecting into `CorpusResult` partitions successes from failures.

The two stages are public.
<claim basis="observed">`annotate` returns a `Document` with structure and sentences and every metric slot `None`; `compose` fills the slots and cannot fail [lib.rs:212-253][annotate]</claim>.
<claim basis="observed">`analyze_one` is exactly `annotate` then `compose` [lib.rs:194-210][analyze-one]</claim>.

<claim basis="observed">Text over 8 MiB is refused with `Error::InputTooLarge { what: "input" }` before any parse [domain.rs:12-28][max-input] [lib.rs:23-38][size-fn]</claim>.
Blockquote paragraphs are kept in the structure but never parsed or
measured.

<claim basis="observed">The Python `Matra` class wraps one `Engine` and routes every method through it [lib.rs:457-496][py-matra]</claim>,
so the cap and the per-paragraph parse hold there too.

### For a contributor

The tree has four layers, and imports only point inward:

1. `src/domain.rs`: every type the library hands back. Depends on `serde`,
   `thiserror` and `std`.
2. The ports: `src/source/mod.rs`, `src/decompose/mod.rs`,
   `src/nlp/mod.rs`, `src/embed/mod.rs`. Each declares one trait and
   imports only `domain`.
3. The adapters, one port each, beside their port. `src/nlp/udpipe.rs` is
   the only file that names `udpipe_rs`, because the panic boundary lives
   there. `metrics/`, `extraction/` and `hearst.rs` are plain functions over
   domain types and touch no port.
4. `src/lib.rs`, the composition root, the only file that knows every port
   and adapter. `src/config.rs` sits beside it; the command line in
   `src/cli/` sits above it and uses only the public surface
   ([EPR-0004](0004-command-line-and-configuration.md)).

Adding a format is a table entry: a `Decomposer` adapter plus one arm in
the composition root's exhaustive match. Adding a metric is a function
appended to the suite. Neither adds an entry point.

## Reference-level explanation

<sketch-figure id="pipeline" seed="1" title="The pipeline and its ports, as they ship">

```text
width 400
state shipped shipped "the pipeline at 4fcfb4a; each arrow is a call, and each port box names its adapters"
box caller "Caller" 4 78 72 40
box engine "Engine\nanalyze\nannotate, compose" 96 4 140 56
box ingest "Ingest\n(text, path)" 96 92 140 40
box cluster "embed_and_cluster" 96 156 140 40
box decomposer "Decomposer\nMarkdown, PlainText" 256 4 140 40
box nlp "NlpProvider\nUdpipe" 256 52 140 40
box source "Source\nFileSource,\nDirectorySource" 256 100 140 48
box embedder "Embedder\nModel2Vec" 256 156 140 40
edge caller engine
edge caller cluster
edge engine ingest
edge engine decomposer
edge engine nlp
edge ingest source
edge cluster embedder
```

</sketch-figure>

### The stages

| Stage | Where | What it does |
|---|---|---|
| ingest | [`Ingest`][ingest] | Concrete iterator of `Result<RawDocument, DocumentError>`. `Ingest::text` never fails; `Ingest::path` lists a directory eagerly and reads each file when pulled, through `FileSource`. |
| decompose | inside [`annotate`][annotate] | Looks up the document's `Format` in the `Decomposers` table; a missing entry is `Error::UnsupportedFormat`. |
| parse | inside [`annotate`][annotate-loop] | One `NlpProvider::parse` call per non-blockquote paragraph; the Hearst detector then fills each sentence's `hearst_pairs`. |
| compose | [`compose`][compose] | Runs [`default_suite()`][metric]: readability, lexical density, document-level ratios, compression. Total. |

`abstract` is reserved by [RFC-0007](../legacy/rfcs/0007-one-pipeline.md) as
the empty tier between structure and purpose-fitted output, where rule
evaluation lands. It is a Rust keyword, so it names the tier and never code.
<claim basis="observed">No module, type or function in `src/` occupies it [lib.rs:1-21][lib-mods]</claim>.

### The ports

| Port | Trait | Contract, as written on the trait | Adapters |
|---|---|---|---|
| source | [`Source`][source-port] (`Send`) | `read` a path into documents; `accepts` without reading contents | `FileSource`, `DirectorySource` |
| decompose | [`Decomposer`][decompose-port] | `decompose` is total; malformed input is read as best it can be | `MarkdownDecomposer`, `PlainTextDecomposer` |
| nlp | [`NlpProvider`][nlp-port] (`Send`) | sentences satisfy the domain's tree invariants; failure is an `Err`, never a panic; no size limit of its own | `Udpipe`, behind `udpipe` |
| embed | [`Embedder`][embed-port] (`Send`) | one vector per text, uniform dimension; a stable `identity` | `Model2Vec`, behind `model2vec` ([EPR-0005](0005-embeddings-and-semantic-clusters.md)) |

The format table is data. [`Decomposers`][decomposers] is a `Vec` keyed on
`Format`; `with` replaces on a duplicate key; lookup is the partial step and
each decomposer stays total.
<claim basis="observed">The composition root fills it through `default_decomposer`, a match over `Format` with no wildcard, so a new `Format` variant fails to compile until someone decides whether it has a decomposer [lib.rs:325-358][default-decomposer]</claim>.
`Pdf` and `Docx` are reserved variants with none.

### The composition root

[`src/lib.rs`][lib-mods] declares every module, builds `Engine`, builds the
standard decomposer table, and holds the one function that pairs a
`Document` with an `Embedder` ([`embed_and_cluster`][embed-and-cluster]). The
`udpipe`-gated constructors `Engine::from_config`,
`Engine::from_config_with_notice` and `Engine::with_defaults` wire the
standard pipeline ([constructors][engine-config]). The PyO3 module lives in
the same file behind `python`.

### The size gate

[`MAX_INPUT_BYTES`][max-input] is a constant (8 MiB), not a setting.
<claim basis="observed">Two places enforce it: `Engine::annotate` for text (`what = "input"`), and `FileSource` on a file's metadata size before reading (`what = "file_source"`) [lib.rs:212-253][annotate] [source/file.rs:9-58][file-source]</claim>.
<claim basis="observed">The domain documents the gap honestly: a caller who invokes `NlpProvider::parse` directly bypasses the bound [domain.rs:12-28][max-input-bypass]</claim>.

### The laws

Seven equivalence laws from [RFC-0007](../legacy/rfcs/0007-one-pipeline.md)
pin the grains together, each a test in `src/lib.rs`
([`law_l1` through `law_l7`][laws]):

```
L1  analyze(a.chain(b))        = analyze(a).chain(analyze(b))
L2  analyze(empty())           = empty()
L3  analyze(once(Ok(raw)))     = once(analyze_one(raw))
L4  analyze_one(r).analysis    = { let mut d = annotate(&r)?; compose(&mut d); d }
L5  |entries| + |errors|       = |input|
L6  Err input item             => identical Err output, analyze_one not called
L7  no text over MAX_INPUT_BYTES reaches NlpProvider::parse
```

L1 to L3 say a single document is a collection of one.
<claim basis="inferred">L7 is provable rather than empirical because `annotate` is the only caller of the parser in the pipeline [lib.rs:212-253][annotate-only]</claim>.

### The boundary rules and how they are checked

The eight rules are canonical in
[`site/content/reference/boundary-rules.md`][boundary-rules], with the
motivation for each. In summary: the domain depends only on `serde`,
`thiserror`, `std` (1); ports import only `domain` (2) and never another
port (3); only `nlp/udpipe.rs` names `udpipe_rs` (4); `metrics/` and
`extraction/` import only `domain` and `stopwords` (5);
`cargo check --no-default-features` compiles (6); the composition root alone
knows every adapter and port, and `src/cli/` uses only the public surface
(7); no `tracing` in the domain or the ports (8).

<claim basis="observed">Rule 6 is compiled by the `rust` and `msrv` jobs in CI [ci.yml:18-100][ci-msrv]</claim>.
<claim basis="observed">Rules 1, 2, 3, 4, 5, the `src/cli/` part of 7, and 8 are semgrep rules under `.semgrep/`, each tested against a fixture of the forms it claims, run by `scripts/check-boundaries.sh` from `just check` and the `Boundary check` job [ci.yml:122-153][ci-boundaries]</claim>.
[EP-0014](../legacy/eps/0014-architecture-guardrails.md) records what each
rule catches and what it leaves to review: whether a trait's shape leaks an
adapter, whether a function takes text where it should take structure, and
whether a file other than `src/lib.rs` wires adapters together.

<claim basis="observed">The panic boundary that rule 4 protects is `catch_parse_panic`, which wraps `Model::parse` and turns a panic into `Error::ParseFailed` [nlp/udpipe.rs:704-734][udpipe-panic]</claim>.

### Features

<claim basis="observed">`udpipe` is the default and `model2vec`, `cli` and `python` are opt-in; `cli` enables `udpipe`, and `python` enables `cli` [Cargo.toml:92-97][cargo-features]</claim>.
Disabling defaults removes the UDPipe adapter and leaves the domain,
the ports, the pipeline, the metrics and the extractors.

### No longer in force

- **The five-verb vocabulary.** [RFC-0002](../legacy/rfcs/0002-pipeline-vocabulary.md)'s
  `ingest / decompose / parse / measure` with `extract` as a peer was
  superseded by [RFC-0007](../legacy/rfcs/0007-one-pipeline.md): the verbs
  named calling conventions, not transformations. Its kept trait names
  remain.
- **The workspace split.** [RFC-0003](../legacy/rfcs/0003-workspace-with-rumi-nlp.md)
  was never implemented and is superseded by
  [RFC-0004](../legacy/rfcs/0004-stay-single-crate.md); there is one
  `[package]` and no `[workspace]` in [`Cargo.toml`][cargo-package].
- **"No function name mentions a format or a source kind" holds for the
  Rust surface only.** [RFC-0007](../legacy/rfcs/0007-one-pipeline.md)
  states it for the whole surface.
  <claim basis="observed">The Python class still has `analyze_markdown` and `analyze_path`, each a thin call into the one `Engine` [lib.rs:534-548][py-methods] [lib.rs:658][py-analyze-path]</claim>.
  The invariants still live in one place; the names do not.
- **Boundary enforcement by grep and review alone.** Records before
  2026-09-26 say rules 1, 2, 5 and 7 have no mechanical check and that 3, 4
  and 8 are a ripgrep over literal imports.
  [EP-0014](../legacy/eps/0014-architecture-guardrails.md) replaced that
  with the semgrep rules above.

## Drawbacks

- **Not fewer names.** [RFC-0007](../legacy/rfcs/0007-one-pipeline.md)
  bought one implementation per invariant, not a smaller namespace.
- **Callers hold an `Engine`.** Someone owns the decomposer table and the
  provider; free functions could not.
- **Work moves to consumption time.** A directory that lists is not a
  directory that reads; failures surface as items.
- **The result stream is not `Send`.**
  <claim basis="observed">It borrows the engine, and `NlpProvider` is `Send` without `Sync` [lib.rs:163-192][analyze-send]</claim>.
- **One parse call per paragraph.** Not measured:
  <claim basis="inferred">a document of many short paragraphs pays the per-call overhead many times</claim>.
  The correctness argument (FM1) outranks it until a measurement says
  otherwise.
- **The cap is bypassable below the pipeline.**
  <claim basis="observed">Calling `NlpProvider::parse` directly skips it; the domain documents this rather than enforcing it [domain.rs:12-28][max-input-bypass]</claim>.

## Rationale and alternatives

These are the alternatives the legacy records weighed and rejected.

- **Six entry points with a parity test** (the 0.1.0 pre-release surface):
  rejected because the test catches drift after it happens and the
  compiler checks none of the restatements
  ([EP-0008](../legacy/eps/0008-pipeline-surface.md)).
- **`ingest / decompose / abstract / compose` as four stages**: rejected
  because `Decomposer::decompose` and `NlpProvider::parse` share one shape
  (string in, latent structure out) and differ by dependency and failure
  mode, which is what ports are factored by; and because `abstract` cannot
  name code in Rust ([EP-0008](../legacy/eps/0008-pipeline-surface.md)).
- **A typestate split of `Document`** (structure, annotated, measured):
  declined because the `Option` slots also encode inapplicability, so the
  split would not remove them, and law L4 answers "has compose run" more
  cheaply ([EP-0008](../legacy/eps/0008-pipeline-surface.md), costs 5).
- **Join paragraphs, parse once, match sentences back**: rejected after
  FM1 (above).
- **A Cargo workspace with a matcher-bridge crate**: rejected by
  [RFC-0004](../legacy/rfcs/0004-stay-single-crate.md); rule evaluation is
  to land inside matra, and no external `NlpProvider` implementors exist.
- **An async or reactor-driven pipeline**: deferred.
  [RFC-0007](../legacy/rfcs/0007-one-pipeline.md) records that the lazy
  stream is safe only because `analyze_one` runs to completion inside one
  `next()`, so staying synchronous is load-bearing, not merely simpler.
- **Semgrep's Rust AST patterns for the boundary checks**: rejected after
  measurement showed they miss brace groups, deep paths and type positions
  silently ([EP-0014](../legacy/eps/0014-architecture-guardrails.md)).

Not doing this would leave each caller and each binding to restate the cap,
the paragraph wiring and the format dispatch, which is the defect class the
current shape removed.

## Prior art

- Ports and adapters (Alistair Cockburn's hexagonal architecture) is the
  layering this follows: the domain at the centre, ports as the interfaces
  it needs, adapters at the edge, one place that wires them.
- Streaming document pipelines in NLP libraries, such as spaCy's
  `Language.pipe`, take an iterable of texts and yield processed documents
  lazily. Context, not a reason: matra's laws and its per-document error
  items are its own.
- [RFC-0002](../legacy/rfcs/0002-pipeline-vocabulary.md),
  [RFC-0004](../legacy/rfcs/0004-stay-single-crate.md),
  [RFC-0007](../legacy/rfcs/0007-one-pipeline.md),
  [EP-0008](../legacy/eps/0008-pipeline-surface.md) and
  [EP-0014](../legacy/eps/0014-architecture-guardrails.md) hold the full
  history.

## Unresolved questions

For the owner to settle in this proposal's review. Each is a decision of its
own.

**An adapter imports a sibling adapter.**
<claim basis="observed">`src/source/directory.rs` imports `FileSource` from `src/source/file.rs` and reads each listed file through it [source/directory.rs:1-62][directory-source]</claim>.
The boundary rules do not name adapter-to-adapter imports, and
[EP-0014](../legacy/eps/0014-architecture-guardrails.md) leaves "one
adapter importing another" to review.

<decision id="sibling-adapter" title="1. May an adapter compose another adapter of its own port?">

<choice key="a" title="Yes, within one port">

`DirectorySource` keeps reading each file through `FileSource`, and the boundary rules page records that an adapter may compose another adapter of the same port.

</choice>

<choice key="b" title="No; the composition root does it">

`DirectorySource` is given the `Source` it reads each file through, and `src/lib.rs` passes it `FileSource`, as the composition root already does its own wiring for the streaming route in `Ingest::path`.

</choice>

<recommendation choice="a">

Both adapters implement one port, and the symlink refusal and the size cap live once, in `FileSource`; reading through it is what keeps them in one place. An import inside one port does not cross a layer, which is what the boundary rules exist to hold. Recording the case turns an unrecorded exception into a reviewed one.

</recommendation>

<against>

Rule 7 makes the composition root the only place adapters are wired together, and an import inside one port looks the same to a reviewer as one across ports, so an accepted exception rests on review alone. `Ingest::path` shows the composition root can do this wiring.

</against>

</decision>

**`Decomposer` has no `Send` bound.**
<claim basis="observed">`Source`, `NlpProvider` and `Embedder` require `Send`; `Decomposer` does not [decompose/mod.rs:11-18][decompose-port] [source/mod.rs:13-28][source-port] [nlp/mod.rs:11-51][nlp-port] [embed/mod.rs:18-43][embed-port]</claim>.
<claim basis="observed">`Decomposers` holds `Box<dyn Decomposer>`, so `Engine` is not `Send` either: observed by compiling a `Send` assertion on `Engine` at the pinned commit, which fails on `dyn Decomposer` [decompose/mod.rs:20-63][decomposers]</claim>.
<claim basis="observed">Nothing in matra needs it today (the Python class is `unsendable`) [lib.rs:457-496][py-matra]</claim>,
but the asymmetry is unrecorded.

<decision id="decomposer-send" title="2. Is Engine meant to be single-threaded?">

<choice key="a" title="No: Decomposer requires Send">

`Decomposer` gains `Send` as a supertrait, as the other three ports have, so an `Engine` can move to another thread.

</choice>

<choice key="b" title="Yes: record the asymmetry as intended">

`Decomposer` stays as it is, and its documentation and the ports table say that `Engine` is not `Send` and why.

</choice>

<recommendation choice="a">

The other three ports require `Send`, which pays off only if an `Engine` holding them can move between threads, and `Decomposer` is the one bound that stops it. The two decomposers matra ships are unit structs with no state to make them unsendable.

</recommendation>

<against>

A new supertrait breaks any decomposer written outside matra that holds a non-`Send` value, and nothing in matra needs `Send` today. A bound added without a caller who needs it is a constraint chosen in advance.

</against>

</decision>

<assumptions />

## Future possibilities

- Rule evaluation in the `abstract` tier, as `ROADMAP.md` describes it,
  reading the fields the pipeline materializes.
- PDF and DOCX decomposers as new table entries; the `Format` variants are
  already reserved.
- A separate `matra-nlp-api` crate if one of
  [RFC-0004](../legacy/rfcs/0004-stay-single-crate.md)'s re-open conditions
  fires, chiefly a third-party `NlpProvider` crate.

[size-fn]: ../../src/lib.rs#L23-L38 "fn check_input_size(text: &str)"
[ingest]: ../../src/lib.rs#L59-L141 "pub struct Ingest {"
[ingest-path]: ../../src/lib.rs#L89-L113 "so one bad file cannot abort a directory walk"
[analyze]: ../../src/lib.rs#L163-L192 "Lazy: nothing is parsed until the returned iterator is pulled"
[analyze-send]: ../../src/lib.rs#L163-L192 "is `Send` without `Sync`"
[analyze-one]: ../../src/lib.rs#L194-L210 "Analyze one document: annotate, then compose."
[annotate]: ../../src/lib.rs#L212-L253 "check_input_size(&raw.text)?;"
[annotate-only]: ../../src/lib.rs#L212-L253 "This is the only route from text to the parser"
[annotate-loop]: ../../src/lib.rs#L235-L243 "para.sentences = self.nlp.parse(&para.text)?;"
[compose]: ../../src/lib.rs#L247-L253 "pub fn compose(&self, doc: &mut Document)"
[engine-config]: ../../src/lib.rs#L256-L323 "pub fn with_defaults() -> domain::Result<Engine>"
[default-decomposer]: ../../src/lib.rs#L325-L358 "deliberately exhaustive with no wildcard"
[embed-and-cluster]: ../../src/lib.rs#L360-L399 "pub fn embed_and_cluster("
[lib-mods]: ../../src/lib.rs#L1-L21 "pub mod source;"
[py-matra]: ../../src/lib.rs#L457-L496 "#[pyclass(unsendable)]"
[py-methods]: ../../src/lib.rs#L534-L548 "fn analyze_markdown<'py>("
[py-analyze-path]: ../../src/lib.rs#L658 "fn analyze_path<'py>"
[laws]: ../../src/lib.rs#L1163-L1296 "fn law_l7_no_oversized_text_reaches_the_parser()"
[fm1-test]: ../../src/lib.rs#L1475-L1496 "fn parse_per_paragraph_scopes_sentences_to_originating_paragraph()"
[max-input]: ../../src/domain.rs#L12-L28 "pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;"
[max-input-bypass]: ../../src/domain.rs#L12-L28 "bypasses this bound"
[metric]: ../../src/metrics/mod.rs#L22-L57 "pub type Metric = Box<dyn Fn(&mut Document)>;"
[source-port]: ../../src/source/mod.rs#L13-L28 "pub trait Source: Send {"
[decompose-port]: ../../src/decompose/mod.rs#L11-L18 "pub trait Decomposer {"
[decomposers]: ../../src/decompose/mod.rs#L20-L63 "entries: Vec<(Format, Box<dyn Decomposer>)>,"
[nlp-port]: ../../src/nlp/mod.rs#L11-L51 "pub trait NlpProvider: Send {"
[embed-port]: ../../src/embed/mod.rs#L18-L43 "pub trait Embedder: Send {"
[file-source]: ../../src/source/file.rs#L9-L58 'what: "file_source",'
[directory-source]: ../../src/source/directory.rs#L1-L62 "use super::file::FileSource;"
[udpipe-panic]: ../../src/nlp/udpipe.rs#L704-L734 "fn catch_parse_panic<F, T>(f: F)"
[cargo-package]: ../../Cargo.toml#L1-L14 "[package]"
[cargo-features]: ../../Cargo.toml#L92-L97 'python = ["dep:pyo3", "dep:pythonize", "cli"]'
[ci-msrv]: ../../.github/workflows/ci.yml#L18-L100 "cargo check --all-targets --no-default-features"
[ci-boundaries]: ../../.github/workflows/ci.yml#L122-L153 "run: bash scripts/check-boundaries.sh"
[boundary-rules]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/site/content/reference/boundary-rules.md
