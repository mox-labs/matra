# 0004. Stay single-crate; supersede the workspace split proposal

- **Status:** Accepted
- **Date:** 2026-05-20
- **Decider(s):** project maintainer
- **Supersedes:** [0003](0003-workspace-with-rumi-nlp.md)

## Context

ADR-0003 (2026-05-01) proposed splitting vaani into a Cargo workspace with two crates: `vaani-core` (the substrate) and a sibling matcher-bridge crate. The proposal was never implemented; the working code has remained a single crate.

Between then and now, two things changed:

- **User direction (2026-05-20):** vaani's intended scope includes rule evaluation over parsed text structure *as part of vaani itself*, not as a peer crate sitting next to it in a workspace. Consumers compose against one published surface; rule evaluation lands as an internal module behind a feature flag or as a sub-API, when the design is ready.
- **rust-mastery corpus finding (M8.i3, Pattern 6):** the criterion for extracting a minimal port crate into a separately published crate is *"whether external implementors exist who need to pin the contract independently of the main crate's version churn"* — i.e., ecosystem-of-implementors size, not architectural elegance. vaani has no third-party `NlpProvider` implementor ecosystem today; extracting `vaani-nlp-api` now would be premature.

The proposal in ADR-0003 was internally coherent but no longer matches the project's direction.

## Decision

vaani stays a single Cargo crate at `mox/packages/vaani/`. The workspace split proposed in ADR-0003 is retracted.

The published surface remains `vaani` on crates.io and `vaani` on PyPI. There is no `vaani-core` or sibling crate.

## Consequences

**Positive:**
- One crate to publish, one version to bump, one CHANGELOG to maintain.
- Public surface is exactly what the working code shows; no aspirational workspace shape lurking in docs.
- The rule-evaluation capability, when it lands, lives at `src/rules/` (or similar) with the same boundary discipline the other modules follow — no cross-crate coordination required.
- The arch docs (`.claude/arch/`) describe the actual current code, not a planned workspace.

**Negative:**
- If a third-party `NlpProvider` implementor ecosystem ever emerges, the Pattern 6 criterion would fire and we'd have to extract `vaani-nlp-api` as a separate minimal crate. That's a future migration cost — but it would be driven by real demand, not preemptive structure.

**Neutral:**
- The reactor decision (deferred per `arch/evolution.md`) is unchanged. Async/streaming arrives when push-semantics or 100k+ document consumers demand it.

## Re-open conditions

This decision is reversible if any of the following becomes true:

1. **Third-party `NlpProvider` implementor crate ships.** A published Rust crate that depends on `vaani` solely for `NlpProvider` (e.g., a `vaani-stanza`, `vaani-spacy`, or `vaani-trankit` shipped by a non-vaani maintainer). At that point Pattern 6's "external implementor ecosystem" criterion fires and `vaani-nlp-api` should be extracted as a minimal port crate. `vaani` would then depend on `vaani-nlp-api` and stay the consumer-facing surface.

2. **The rule-evaluation module grows beyond a single-crate boundary.** If the future rule-evaluation capability needs its own version cadence (e.g., to track a separate query-DSL spec), it may warrant its own crate. The bar is high: same-repo workspace before separate-repo, and only if the version cadence genuinely diverges.

3. **A consumer needs `domain` types without `udpipe-rs` or `pyo3` ever appearing in their dep graph.** The `no-default-features` build already covers this; the workspace split would not improve it.

Any of these conditions, write a new ADR and supersede this one.

## Validation

This decision is correct if at 0.2.0 vaani still ships as a single crate, the rust-mastery audit (`.claude/arch/rust-mastery-audit.md`) shows no new gaps from the single-crate shape, and no third-party `NlpProvider` implementor has emerged demanding a separate port crate.

Falsified if one of the re-open conditions fires before then, in which case we extract the right boundary and supersede this ADR.

## References

- [0003](0003-workspace-with-rumi-nlp.md) — superseded predecessor.
- `.claude/arch/architecture.md` — describes the current single-crate shape.
- `.claude/arch/rust-mastery-audit.md` — Pattern 6 criterion (separate publication) and why it has not fired.
- `~/radix-workspaces/rust-mastery/frames/cross-artifact/frame__cross-artifact__m8-i3-search-tier-pattern6-substrate-stability.json` — corpus Frame establishing the Pattern 6 criterion.
