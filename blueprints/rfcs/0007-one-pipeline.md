# 0007. One pipeline: ingest -> decompose -> compose, with abstract reserved

- **Status:** Accepted. Supersedes [ADR-0002](0002-pipeline-vocabulary.md).
- **Date:** 2026-08-21
- **Decider(s):** project maintainer; formal review opened by a maintainer question ("why do we have so many entry points?")

## Context

The public Rust surface was six free functions: `analyze`,
`analyze_markdown`, `analyze_file`, `analyze_directory`, `parse`,
`analyze_from`. Each was a partial application of one chain, with the
source kind and the format enumerated as function names. The project
already had abstractions for exactly those axes, the `Source` and
`Decomposer` ports, and no entry point accepted them.

Taking the stage types seriously surfaced two live defects with one
shape: N entry points means N restatements of each invariant, and the
compiler checks none of them. The input size cap was bypassable from
Python because four methods restated the gate and four did not, and
`analyze_from` returned a half-populated `Document` because the metric
suite carried the sentence set twice, flattened in a slice and attached
to paragraphs, with nothing enforcing agreement.

ADR-0002's five verbs (`ingest / decompose / parse / measure` + peer
`extract`) named that surface. On review, they enumerate calling
conventions rather than transformations: `measure` mutates and returns
unit while extractors return values, and that projection difference is
the only thing that made `extract` a peer. `Decomposer::decompose` and
`NlpProvider::parse` share one representational shape (string in,
latent structure out) and differ by dependency and failure mode, which
is what ports are factored by, not what stages are.

This was a pre-publish boundary: deleting public functions is free
before 0.1.0 and a SemVer-major after it.

## Decision

**The surface is one pipeline.** `Ingest` carries the source
variation as data (a string is a stream of one, a file is a stream of
one, a directory is a stream of many); the `Decomposers` table carries
the format variation as data; `Engine` runs the chain. No function
name mentions a format or a source kind.

**The stage vocabulary is `ingest -> decompose -> compose`,** exposed
on `Engine` as `analyze` (the whole chain over a stream), `analyze_one`
(the singleton view), `annotate` (decompose plus parse, producing an
unmeasured `Document`), and `compose` (the metric suite, total).
`annotate` is the sole route from text to `NlpProvider::parse`, which
makes the input size cap a property of the pipeline rather than of
each entry point.

**`abstract` is reserved as the named empty seam** between structure
and purpose-fitted output. It is where rule evaluation over parsed
structure lands (`Document -> Vec<Finding>`, per the vocabulary locked
in [ADR-0006](0006-abstract-tier-vocabulary-lock.md)), and it is
unoccupied at 0.1.0. `abstract` is a reserved keyword in Rust and can
never name code; it names the tier, not a function. We do not name a
stage that has no code, and we do not simulate a capability the
substrate does not have: until the seam is filled, matra's output stops
at deterministic, verifiable structure.

**The trait names stay.** `Source`, `Decomposer`, `NlpProvider` are
ports, factored by dependency and failure mode, and ADR-0002's decision
to keep them is carried forward unchanged.

## The laws are the contract

Seven equivalence laws pin the surface in `src/lib.rs` tests, so the
grains cannot drift apart silently:

```
L1  analyze(a.chain(b))        = analyze(a).chain(analyze(b))
L2  analyze(empty())           = empty()
L3  analyze(once(Ok(raw)))     = once(analyze_one(raw))
L4  analyze_one(r).analysis    = { let mut d = annotate(&r)?; compose(&mut d); d }
L5  |entries| + |errors|       = |input|
L6  Err input item             => identical Err output, analyze_one not called
L7  no text over MAX_INPUT_BYTES reaches NlpProvider::parse
```

L1 to L3 are the formal content of "a single document is a collection
of one": `once` is the singleton injection and the pipeline commutes
with it, so n=0, n=1 and n=N are one function at three lengths.

## Consequences

**Positive:**
- One implementation of every invariant, checked at one choke point.
- Closure under format growth: a PDF decomposer is a table entry, not
  a new function family.
- Streaming by default: a directory holds one document's allocations
  at a time, and per-file failures travel as `DocumentError` items.

**Negative, accepted knowingly:**
- Not fewer names (roughly nine against six). What is bought is one
  implementation, not a smaller namespace.
- Callers hold an `Engine`; someone must own the decomposer table.
- Work moves to consumption time: `Ingest::path(dir)?` reads nothing
  until pulled, so "it returned Ok, therefore every file was read"
  stops holding.
- The result stream is not `Send`: it borrows the `Engine`, and
  `NlpProvider` is `Send` without `Sync`.

**Load-bearing dependency:** the laziness is safe only because
`analyze_one` runs to completion inside a single `next()` call, which
is guaranteed by matra having no reactor. Two lazy streams interleaved
on one thread cannot interleave inside a document. ADR-0004's decision
to stay synchronous is therefore load-bearing for this design, not
merely a simplification; revisiting it requires revisiting this ADR.

## Validation

Right if the laws stay green and format growth lands as table entries.
Falsified if a consumer needs an entry point that cannot be expressed
as an `Ingest` constructor plus the pipeline, or if the no-reactor
guarantee is dropped without this surface being redesigned.

## References

- Plan and defect record: `book/src/plans/i8-pipeline-surface.md`.
- Vocabulary tier lock: [ADR-0006](0006-abstract-tier-vocabulary-lock.md).
- Single-crate and no-reactor context: [ADR-0004](0004-stay-single-crate.md).
