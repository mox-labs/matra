# 0003. Cargo workspace with `vaani-core` and `rumi-nlp`

- **Status:** proposed
- **Date:** 2026-05-01
- **Decider(s):** project maintainer

## Context

vaani is the substrate: parsing, structure, metrics, summarization,
keyphrase extraction. Some consumers want rule-based pattern matching
over the parsed dependency tree (SVO triples, copular constructions,
stance classification, etc.). The matcher engine for this exists
externally (the `rumi-core` matcher engine, an xDS Unified Matcher
API implementation). What does not exist is the bridge between
vaani's parsed `Sentence` and the matcher engine's `DataInput<Ctx>`
trait.

The bridge ("rumi-nlp") needs to live somewhere. Two homes are
plausible:

- Inside the matcher engine's workspace, alongside other domain
  extensions (HTTP routing, hook policies).
- Inside vaani's workspace, alongside `vaani-core`.

## Options considered

### Option A: rumi-nlp lives in the matcher engine's workspace

The bridge crate sits next to `rumi-http` and `rumi-claude` in the
matcher engine's repo, depending on `vaani` for `Sentence` and
`Token`.

**Pros:** consistent with where other rumi-* extensions live;
matcher engine team controls the bridge release cadence.
**Cons:** forces the matcher engine to know about NLP terminology
(token, dependency relation, lemma); violates its "matcher engine,
not policy engine" stance — rumi-nlp would import vaani-core into a
workspace whose stated discipline is to avoid domain knowledge.

### Option B: rumi-nlp lives in vaani's workspace

vaani becomes a Cargo workspace with two crates:
`vaani-core` (the substrate, today's `vaani` crate) and `rumi-nlp`
(the matcher bridge). `rumi-nlp` depends on `vaani-core` (for the
domain types) and on `rumi-core` (for the matcher engine).
`vaani-core` does not depend on `rumi-nlp` or `rumi-core`.

**Pros:** the project that owns the *domain* owns the matcher-bridge
crate for that domain. Matcher engine stays domain-agnostic. Domain
knowledge stays colocated with the substrate that produces it. New
NLP-specific matcher extensions land in vaani's workspace, not
across two repos.
**Cons:** vaani repo grows beyond a single crate; release flow has
to handle two crates; consumers who want only `vaani-core` must
opt out of `rumi-nlp` (separate crates handle this naturally).

### Option C: Defer the decision; put rumi-nlp in a third repo

A separate repository whose only role is the bridge.

**Pros:** maximally decoupled.
**Cons:** discovery problem (who finds it?); versioning across three
repos is painful; the bridge is small enough that a separate repo is
overhead, not modularity.

## Decision

We choose **Option B**. vaani becomes a Cargo workspace at I4:

```
vaani/                                # workspace root
├── crates/
│   ├── vaani-core/                   # substrate (today's vaani crate)
│   └── rumi-nlp/                     # matcher bridge (depends on vaani-core + rumi-core)
```

`vaani-core` keeps the published name `vaani` on crates.io (the
canonical name stays). `rumi-nlp` publishes as a separate crate.
At 0.1.0, `rumi-nlp` ships with primitives only (one
`DataInput<Sentence>` implementation as a smoke test); domain-specific
patterns (SVO, copular, stance) land incrementally post-publish,
driven by real consumer needs.

## Consequences

**Positive:**
- Matcher engine's "no domain knowledge in core" discipline holds.
- vaani consumers who want only parsing/metrics pay nothing for
  matcher infrastructure.
- A natural home exists for future NLP-specific matcher extensions
  (e.g., `vaani-stance` later) within vaani's workspace.

**Negative:**
- Two-crate publish flow at release time.
- Workspace conversion is structural work (planned in I4).

**Neutral:**
- The shape mirrors how the matcher engine's own workspace handles
  HTTP and Claude-hook extensions, so contributors familiar with
  either side find the other recognizable.

## Validation

Right if at 0.2.0 we can land NLP pattern content in `rumi-nlp`
without touching `vaani-core` (and consumer code that uses only
`vaani-core` is unaffected). Falsified if we end up cross-importing,
or if the matcher engine workspace turns out to be the better home
because its release tooling was already there.

The reactor pattern (deferred to ≥0.3) is a related but separate
question; this ADR does not bind it.

## References

- I4 implan: `.claude/implans/i4-workspace.md`.
- Architecture: `.claude/arch/architecture.md` (workspace section).
- Evolution rationale: `.claude/arch/evolution.md` ("rumi-nlp in
  the matcher-engine's workspace" — rejected option).
