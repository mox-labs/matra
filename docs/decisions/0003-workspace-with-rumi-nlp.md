# 0003. Cargo workspace with `matra-core` and `rumi-nlp`

- **Status:** Superseded by [0004](0004-stay-single-crate.md) on 2026-05-20
- **Date:** 2026-05-01
- **Decider(s):** project maintainer

> **Superseded.** This proposal was never implemented; matra stayed single-crate. The successor ADR ([0004](0004-stay-single-crate.md)) formalizes the single-crate decision and explains the conditions that would re-open the question. Read this ADR for historical context only.

## Context

matra is the substrate: parsing, structure, metrics, summarization,
keyphrase extraction. Some consumers want rule-based pattern matching
over the parsed dependency tree (SVO triples, copular constructions,
stance classification, etc.). The matcher engine for this exists
externally (the `rumi-core` matcher engine, an xDS Unified Matcher
API implementation). What does not exist is the bridge between
matra's parsed `Sentence` and the matcher engine's `DataInput<Ctx>`
trait.

The bridge ("rumi-nlp") needs to live somewhere. Two homes are
plausible:

- Inside the matcher engine's workspace, alongside other domain
  extensions (HTTP routing, hook policies).
- Inside matra's workspace, alongside `matra-core`.

## Options considered

### Option A: rumi-nlp lives in the matcher engine's workspace

The bridge crate sits next to `rumi-http` and `rumi-claude` in the
matcher engine's repo, depending on `matra` for `Sentence` and
`Token`.

**Pros:** consistent with where other rumi-* extensions live;
matcher engine team controls the bridge release cadence.
**Cons:** forces the matcher engine to know about NLP terminology
(token, dependency relation, lemma); violates its "matcher engine,
not policy engine" stance — rumi-nlp would import matra-core into a
workspace whose stated discipline is to avoid domain knowledge.

### Option B: rumi-nlp lives in matra's workspace

matra becomes a Cargo workspace with two crates:
`matra-core` (the substrate, today's `matra` crate) and `rumi-nlp`
(the matcher bridge). `rumi-nlp` depends on `matra-core` (for the
domain types) and on `rumi-core` (for the matcher engine).
`matra-core` does not depend on `rumi-nlp` or `rumi-core`.

**Pros:** the project that owns the *domain* owns the matcher-bridge
crate for that domain. Matcher engine stays domain-agnostic. Domain
knowledge stays colocated with the substrate that produces it. New
NLP-specific matcher extensions land in matra's workspace, not
across two repos.
**Cons:** matra repo grows beyond a single crate; release flow has
to handle two crates; consumers who want only `matra-core` must
opt out of `rumi-nlp` (separate crates handle this naturally).

### Option C: Defer the decision; put rumi-nlp in a third repo

A separate repository whose only role is the bridge.

**Pros:** maximally decoupled.
**Cons:** discovery problem (who finds it?); versioning across three
repos is painful; the bridge is small enough that a separate repo is
overhead, not modularity.

## Decision

We choose **Option B**. matra becomes a Cargo workspace at I4:

```
matra/                                # workspace root
├── crates/
│   ├── matra-core/                   # substrate (today's matra crate)
│   └── rumi-nlp/                     # matcher bridge (depends on matra-core + rumi-core)
```

`matra-core` keeps the published name `matra` on crates.io (the
canonical name stays). `rumi-nlp` publishes as a separate crate.
At 0.1.0, `rumi-nlp` ships with primitives only (one
`DataInput<Sentence>` implementation as a smoke test); domain-specific
patterns (SVO, copular, stance) land incrementally post-publish,
driven by real consumer needs.

## Consequences

**Positive:**
- Matcher engine's "no domain knowledge in core" discipline holds.
- matra consumers who want only parsing/metrics pay nothing for
  matcher infrastructure.
- A natural home exists for future NLP-specific matcher extensions
  (e.g., `matra-stance` later) within matra's workspace.

**Negative:**
- Two-crate publish flow at release time.
- Workspace conversion is structural work (planned in I4).

**Neutral:**
- The shape mirrors how the matcher engine's own workspace handles
  HTTP and Claude-hook extensions, so contributors familiar with
  either side find the other recognizable.

## Validation

Right if at 0.2.0 we can land NLP pattern content in `rumi-nlp`
without touching `matra-core` (and consumer code that uses only
`matra-core` is unaffected). Falsified if we end up cross-importing,
or if the matcher engine workspace turns out to be the better home
because its release tooling was already there.

The reactor pattern (deferred to ≥0.3) is a related but separate
question; this ADR does not bind it.

## References

- I4 plan: `book/src/plans/i4-workspace.md`.
- Architecture: `.claude/arch/architecture.md` (workspace section).
- Evolution rationale: `.claude/arch/evolution.md` ("rumi-nlp in
  the matcher-engine's workspace" — rejected option).
