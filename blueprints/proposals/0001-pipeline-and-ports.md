# EPR-0001: The pipeline and its ports

- Feature Name: `pipeline_and_ports`
- Start Date: 2026-10-04
- Proposal PR: (this proposal's pull request)
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
the six with one pipeline so each invariant has one home. Today the size
check sits in [`check_input_size`][size-fn], called first thing in
[`Engine::annotate`][annotate], and the metric suite takes the document
alone ([`Metric`][metric]), so the sentence set exists once.

**Per-paragraph parsing.** An earlier pipeline joined paragraphs, parsed
once, and wired sentences back to paragraphs by prefix match. Two paragraphs
sharing their first 30 characters had their sentences silently reassigned
(the failure the project calls FM1). `annotate` parses each paragraph on its
own ([lines 235 to 243][annotate-loop]), and a regression test pins the
case ([`parse_per_paragraph_scopes_sentences_to_originating_paragraph`][fm1-test]).

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
crate's version churn, and no such implementors exist. Observed: one
`[package]` in [`Cargo.toml`][cargo-package] and no workspace.

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

`analyze` is lazy: nothing is read or parsed until the iterator is pulled,
and each pull runs one document to completion ([`Engine::analyze`][analyze]).
So `Ingest::path(dir)?` returning `Ok` means the directory listed, not that
every file read. Per-file failures arrive as `DocumentError` items carrying
the path ([`Ingest::path`][ingest-path]), and collecting into
`CorpusResult` partitions successes from failures.

The two stages are public. `annotate` returns a `Document` with structure
and sentences and every metric slot `None`; `compose` fills the slots and
cannot fail ([`annotate` and `compose`][annotate]). `analyze_one` is exactly
`annotate` then `compose` ([`analyze_one`][analyze-one]).

Text over 8 MiB is refused with `Error::InputTooLarge { what: "input" }`
before any parse ([`MAX_INPUT_BYTES`][max-input]). Blockquote paragraphs are
kept in the structure but never parsed or measured.

The Python `Matra` class wraps one `Engine` and routes every method through
it ([`Matra`][py-matra]), so the cap and the per-paragraph parse hold there
too.

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
Observed: no module, type or function in `src/` occupies it.

### The ports

| Port | Trait | Contract, as written on the trait | Adapters |
|---|---|---|---|
| source | [`Source`][source-port] (`Send`) | `read` a path into documents; `accepts` without reading contents | `FileSource`, `DirectorySource` |
| decompose | [`Decomposer`][decompose-port] | `decompose` is total; malformed input is read as best it can be | `MarkdownDecomposer`, `PlainTextDecomposer` |
| nlp | [`NlpProvider`][nlp-port] (`Send`) | sentences satisfy the domain's tree invariants; failure is an `Err`, never a panic; no size limit of its own | `Udpipe`, behind `udpipe` |
| embed | [`Embedder`][embed-port] (`Send`) | one vector per text, uniform dimension; a stable `identity` | `Model2Vec`, behind `model2vec` ([EPR-0005](0005-embeddings-and-semantic-clusters.md)) |

The format table is data. [`Decomposers`][decomposers] is a `Vec` keyed on
`Format`; `with` replaces on a duplicate key; lookup is the partial step and
each decomposer stays total. The composition root fills it through
[`default_decomposer`][default-decomposer], a match over `Format` with no
wildcard, so a new `Format` variant fails to compile until someone decides
whether it has a decomposer. `Pdf` and `Docx` are reserved variants with
none.

### The composition root

[`src/lib.rs`][lib-mods] declares every module, builds `Engine`, builds the
standard decomposer table, and holds the one function that pairs a
`Document` with an `Embedder` ([`embed_and_cluster`][embed-and-cluster]). The
`udpipe`-gated constructors `Engine::from_config`,
`Engine::from_config_with_notice` and `Engine::with_defaults` wire the
standard pipeline ([constructors][engine-config]). The PyO3 module lives in
the same file behind `python`.

### The size gate

[`MAX_INPUT_BYTES`][max-input] is a constant (8 MiB), not a setting. Two
places enforce it: [`Engine::annotate`][annotate] for text
(`what = "input"`), and [`FileSource`][file-source] on a file's metadata
size before reading (`what = "file_source"`). The domain documents the gap
honestly: a caller who invokes `NlpProvider::parse` directly bypasses the
bound.

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

L1 to L3 say a single document is a collection of one. L7 is provable
rather than empirical because `annotate` is the only caller of the parser
in the pipeline.

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

Rule 6 is compiled by the `rust` and `msrv` jobs in CI
([`ci.yml`][ci-msrv]). Rules 1, 2, 3, 4, 5, the `src/cli/` part of 7, and 8
are semgrep rules under `.semgrep/`, each tested against a fixture of the
forms it claims, run by `scripts/check-boundaries.sh` from `just check` and
the `Boundary check` job ([`ci.yml`][ci-boundaries]).
[EP-0014](../legacy/eps/0014-architecture-guardrails.md) records what each
rule catches and what it leaves to review: whether a trait's shape leaks an
adapter, whether a function takes text where it should take structure, and
whether a file other than `src/lib.rs` wires adapters together.

The panic boundary that rule 4 protects is
[`catch_parse_panic`][udpipe-panic], which wraps `Model::parse` and turns a
panic into `Error::ParseFailed`.

### Features

`udpipe` is the default; `model2vec`, `cli` and `python` are opt-in
([features][cargo-features]). `cli` enables `udpipe`; `python` enables
`cli`. Disabling defaults removes the UDPipe adapter and leaves the domain,
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
  states it for the whole surface. The Python class still has
  `analyze_markdown` and `analyze_path` ([Python methods][py-methods]),
  each a thin call into the one `Engine`. The invariants still live in one
  place; the names do not.
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
- **The result stream is not `Send`.** It borrows the engine, and
  `NlpProvider` is `Send` without `Sync` ([`analyze`][analyze]).
- **One parse call per paragraph.** Inferred, not measured: a document of
  many short paragraphs pays the per-call overhead many times. The
  correctness argument (FM1) outranks it until a measurement says
  otherwise.
- **The cap is bypassable below the pipeline.** Calling
  `NlpProvider::parse` directly skips it; the domain documents this rather
  than enforcing it ([`MAX_INPUT_BYTES`][max-input]).

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

- **An adapter imports a sibling adapter.** `src/source/directory.rs`
  imports `FileSource` from `src/source/file.rs` and reads each listed file
  through it ([`DirectorySource`][directory-source]). The boundary rules do
  not name adapter-to-adapter imports, and
  [EP-0014](../legacy/eps/0014-architecture-guardrails.md) leaves "one
  adapter importing another" to review. Is an adapter composing another
  adapter of the same port acceptable, or should the composition root do
  it, as `Ingest::path` already does for the streaming route?
- **`Decomposer` has no `Send` bound.** `Source`, `NlpProvider` and
  `Embedder` require `Send`; [`Decomposer`][decompose-port] does not. Inferred
  from the types, not tested: `Decomposers` holds `Box<dyn Decomposer>`, so
  `Engine` is not `Send` either. Nothing in matra needs it today (the Python
  class is `unsendable`), but the asymmetry is unrecorded. Is it intended?

## Future possibilities

- Rule evaluation in the `abstract` tier, as `ROADMAP.md` describes it,
  reading the fields the pipeline materializes.
- PDF and DOCX decomposers as new table entries; the `Format` variants are
  already reserved.
- A separate `matra-nlp-api` crate if one of
  [RFC-0004](../legacy/rfcs/0004-stay-single-crate.md)'s re-open conditions
  fires, chiefly a third-party `NlpProvider` crate.

[size-fn]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L23-L38
[ingest]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L59-L141
[ingest-path]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L89-L113
[analyze]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L163-L192
[analyze-one]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L194-L210
[annotate]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L212-L253
[annotate-loop]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L235-L243
[compose]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L247-L253
[engine-config]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L256-L323
[default-decomposer]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L325-L358
[embed-and-cluster]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L360-L399
[lib-mods]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L1-L21
[py-matra]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L457-L496
[py-methods]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L534-L548
[laws]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L1163-L1296
[fm1-test]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L1475-L1496
[max-input]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L12-L28
[metric]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/metrics/mod.rs#L22-L57
[source-port]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/source/mod.rs#L13-L28
[decompose-port]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/decompose/mod.rs#L11-L18
[decomposers]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/decompose/mod.rs#L20-L63
[nlp-port]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/mod.rs#L11-L51
[embed-port]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/mod.rs#L18-L43
[file-source]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/source/file.rs#L9-L58
[directory-source]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/source/directory.rs#L1-L62
[udpipe-panic]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L704-L734
[cargo-package]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/Cargo.toml#L1-L14
[cargo-features]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/Cargo.toml#L92-L97
[ci-msrv]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/ci.yml#L18-L100
[ci-boundaries]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/ci.yml#L122-L153
[boundary-rules]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/site/content/reference/boundary-rules.md
