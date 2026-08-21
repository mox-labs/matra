# 0008. Structural primitives are fields

- **Status:** Accepted
- **Date:** 2026-08-21
- **Decider(s):** project maintainer; question raised by I7 M1 ("are structural primitives fields or methods?")

## Context

`Sentence::is_passive` is a method. Methods do not cross FFI: the Python
surface in `src/lib.rs` is `pythonize::pythonize(py, &Document)` over the
`Serialize` derive, with no `#[pymethods]` on domain types, so the only
channel to a non-Rust consumer is the serialized field set. A future
WASM/TS crust built on serde inherits the same property. "Method"
therefore means "does not exist for Python, TypeScript, or the CLI's JSON
output."

The cost is already live: `python/matra/cli.py` re-implemented passive
detection over raw tokens, in Python, against the same parse the Rust
method had already read. matra's own crust duplicated matra's own
primitive. Every consumer in every language does the same today and will
do the same for negation, modality and evidentiality unless the channel
question is settled. I7 exists because of this duplication, and M1
settles the question with the first real primitive (negation) in hand.
M2 through M5 inherit the answer, and M5's rubric has already committed
its endgame: span pairs cross as data, with a fixture in `spec/tests/`.

## Options considered

### Option A: methods

Each primitive is a Rust method beside `is_passive`. Zero storage, zero
parse-time cost, zero schema commitment.

**Pros:**
- No schema lock-in; the wire shape stays exactly the parse.
- Consumers pay nothing for primitives they do not read.

**Cons:**
- Invisible to every non-Rust consumer by construction. Each crust
  re-implements each primitive: N crusts times M primitives, with no
  checker that the re-implementations agree. This is ADR-0007's
  diagnosis one layer up (N restatements of an invariant, compiler
  checks none of them), and `cli.py`'s passive fold is the live case.
- Plausible for a three-string fold; absurd for M5's six multi-arc
  Hearst patterns re-derived per crust against a fixture.
- The escape hatch (route crossing through `Finding`) inverts I7's
  sequencing: I7 is rule-substrate work that lands before any
  `src/rules/` or `Finding` shape design, and ADR-0006 defers Finding's
  shape to Phase 2. Primitives cannot wait on a shape that is deferred
  until after the primitives exist.

### Option B: fields

Each primitive materializes as stored, serde-visible data on the domain
type, computed once by a single Rust implementation at a pipeline choke
point. The codebase already has the choke points: `annotate` builds
structure (Sentence construction), `compose` fills metric slots.

**Pros:**
- One implementation, one choke point, every crust and the CLI's JSON
  see the identical keys at zero binding code. One conformance fixture
  describes all crusts.
- Inherits ADR-0007's consolidation instead of undoing it.
- M5's committed shape (span pairs as data) falls out of the same
  convention rather than needing a second mechanism.

**Cons (accepted knowingly):**
- Schema lock-in. Mitigated by pre-publish being the cheap reshaping
  window, `#[non_exhaustive]` on every crossing struct, and a
  deliberately minimal shape: ids and lemmas, no nested token copies.
- Staleness if `tokens` is mutated after construction. Mitigated by
  extending the existing documented contract on `Sentence::new` (caller
  upholds the invariants) to cover derived fields.
- Everyone pays storage whether or not they want it. This is grams next
  to the UDPipe parse: roughly 1 to 2 KB of tokens per sentence already
  cross the wire.

### Option C: hybrid (hand-written Serialize)

Keep primitives as methods; hand-write `Serialize` for `Sentence` and
`Document` so the methods' results appear on the wire anyway.

**Pros:**
- Same wire shape as Option B without stored state.

**Cons:**
- Buys the identical wire at a worse price: a hand-written `Serialize`
  impl in the most protected file (`domain.rs`), threaded through by
  every future primitive; a wire shape that no longer equals the struct
  (opacity); graph walks executing inside `Serialize::serialize`; and a
  second materialization mechanism alongside the compose-stage one that
  ADR-0007 just consolidated.

## Decision

We choose Option B. Derived structural facts cross FFI as serde-visible
fields with a single Rust implementation. Structure materializes at the
annotate stage: `Sentence` construction computes sentence-level
primitives (M1: `negations: Vec<Negation>`) from its tokens.
Document-level aggregates materialize as `Option` slots filled by
`compose` (M1: `Document.passive_ratio`, exactly like `vocabulary_ttr`
and `nominalization_ratio`). Zero-information accessors over data
already on the wire (M2's `feat` lookup on the `feats` string) stay
Rust-only methods: they derive nothing, so there is nothing to cross.

The criterion, stated once: derivations cross as fields; views over
data already crossing stay methods.

**Amendment (I7 M5, 2026-08-21):** one sentence-level primitive is not
computed by `Sentence::new`. `Sentence.hearst_pairs` is filled by
`Engine::annotate`, because its detector lives in `matra::hearst`,
outside the domain (the M5 boundary rubric requires a new module
importing only `domain`, and `domain.rs` cannot import it back). The
field still crosses as data per this ADR; only the choke point moved
from construction to the annotate stage. A hand-built `Sentence`
carries an empty `hearst_pairs` until the caller runs the detector.

This does NOT pre-empt ADR-0006's deferred `Finding` shape. These are
record-tier structural facts on record-tier types. The abstract tier
(`Finding`, `Rule`, `Predicate`) still lands separately in Phase 2, and
its trait-vs-enum decision remains open. The field shapes here satisfy
ADR-0006 Frame-4 (FFI-safe materialization: primitives, `String`,
`Option<primitive>`, FFI-safe structs) so the two tiers stay consistent.

Substrate discipline binds every field this ADR licenses: a primitive
reports structure (the cue, the construction, the arc), never an
interpretive category. Field names name what is in the parse.

## Consequences

- Positive: Python, the CLI's JSON, and any future TS/WASM crust gain
  each primitive with zero binding code. `python/matra/cli.py` deletes
  its passive fold and reads `result["passive_ratio"]`. One
  `spec/tests/` fixture per crossing primitive proves all crusts agree.
- Negative: every crossing field is a public schema commitment across
  three languages; renames after 0.1.0 are SemVer-major. Serialized
  output grows by the size of the materialized facts.
- Neutral: `python/matra/types.py` and the conformance harnesses grow
  in lockstep with each crossing field, per the existing docs-lockstep
  discipline.

## Validation

Falsified if any of the following is observed, each recorded with its
escape hatch at decision time:

- A benchmark at M5 shows materializing all five primitives degrades
  analyze throughput or serialized size beyond low single-digit percent
  on book-length input. Escape: per-primitive compose-stage gating.
- A real consumer class needs pay-only-for-what-you-use sentence
  output. Escape: an opt-in projection entry point.
- Mutation-after-parse workflows emerge that make stale derived fields
  a recurring bug class. Escape: recompute-on-write or projection.
- The crusts stop being thin serde projections (a crust adds logic that
  diverges from the wire). That would break the premise, not the
  mechanism.

None is in evidence today. Each escape is additive rather than
breaking.

## References

- Plan: `book/src/plans/i7-structural-primitives.md` (M1 decides; M2
  through M5 inherit; M5 rubric already commits span pairs as data).
- [ADR-0007](0007-one-pipeline.md): one implementation of every
  invariant at one choke point; `annotate` and `compose` are the choke
  points this ADR reuses.
- [ADR-0006](0006-abstract-tier-vocabulary-lock.md): Frame-4 FFI-safe
  materialization; `Finding` shape deferred to Phase 2, untouched here.
- Live duplication: `python/matra/cli.py` passive fold over raw tokens
  (deleted by M1).
- Wire architecture: `to_dict` in `src/lib.rs` is
  `pythonize::pythonize(py, &Document)` over the `Serialize` derive.
