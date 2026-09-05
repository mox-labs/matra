---
name: aces
description: ACES — Adaptable, Composable, Extensible. The non-negotiable design philosophy for matra. Every system decays through three endogenous forces (stasis, drag, opacity); ACE is the discipline that resists them. Use when making any structural decision, designing an interface, or evaluating whether a change preserves long-term substrate value.
---

# aces

ACES is the design philosophy for matra. It is **non-negotiable**. Every design decision is checked against it.

## The cycle

Every system decays through three endogenous forces. Left unchecked, they feed each other:

- **Stasis** — the system stops evolving. Decisions harden. New requirements fight the architecture instead of fitting into it.
- **Drag** — complexity accumulates. Dependencies tangle. Simple changes take weeks.
- **Opacity** — understanding fades. Workarounds compound. Nobody knows why something works (or whether it does).

**Opacity feeds stasis feeds drag.** A codebase that nobody fully understands cannot evolve safely; an unevolving codebase forces workarounds that pile on as drag; the drag obscures what's still load-bearing, deepening the opacity. This is the cycle matra is built to resist.

## The counter-forces — ACE

Three disciplines, each countering one decay mode:

### Adaptable — design for change

**Counters stasis.** Configuration over hardcoding. Feature flags additive and orthogonal. Public types `#[non_exhaustive]` so future variants don't break consumers. Boundary rules stated explicitly so future maintainers know what they can move and what they can't.

Matra's adaptable surface:

- `default_suite` in `metrics/mod.rs` configures the default metric set; consumers can compose a different suite.
- `Format` enum is `#[non_exhaustive]` so adding PDF/DOCX support later is additive.
- Feature flags (`udpipe`, `python`) gate optional capabilities without changing the core.
- Every public type carries `#[non_exhaustive]` for additive evolution.

### Composable — discrete components, clear boundaries, swappable parts

**Counters drag.** Four ports (Source, Decomposer, NlpProvider, Embedder). Multiple adapters per port. One composition root. Each piece has a single responsibility and a single boundary; replacing one piece does not require rewriting any other.

Matra's composable surface:

- The hex layout (domain → ports → adapters → composition root).
- `&dyn NlpProvider` runtime dispatch so the NLP backend is replaceable without recompiling the rest.
- Per-paragraph parse so paragraph-level changes don't cascade into document-level rewrites.
- The eight boundary rules, stated with motivation in `book/src/reference/boundary-rules.md`. Enforcement is mostly review: `scripts/check-boundaries.sh` greps rules 3, 4 and 8 from `just check` and the opt-in pre-commit hook, and rule 6 is the only rule with a CI gate.

### Extensible — clear interfaces that invite contribution without requiring full comprehension

**Counters opacity.** A new contributor should be able to add a new `Source`, `Decomposer`, or `NlpProvider` adapter by reading only the port trait and one existing adapter — not by reading the whole codebase. Every public surface has rustdoc; every boundary rule is stated explicitly.

Matra's extensible surface:

- Four port traits, each minimal (one or two methods).
- Adapter constraints documented in `book/src/architecture/design.md`.
- Rustdoc on every public type, with examples on every public function.

## The boundary test

For any proposed change, ask three questions:

### 1. Does this make the system more adaptable, or less?

- More: the change makes a hardcoded constant configurable, makes a public type `#[non_exhaustive]`, gates a new capability behind a feature flag.
- Less: the change locks in a specific format/backend, removes `#[non_exhaustive]`, couples two previously-independent features.

A "less" answer needs a strong justification (an ADR, a falsifiable prediction, a measured constraint).

### 2. Does this make the system more composable, or less?

- More: the change adds a port adapter that implements an existing port, splits a god-module into smaller files with clear responsibilities, factors a shared utility into `stopwords.rs`-style shared module.
- Less: the change adds a cross-adapter import, blurs a boundary between ports, makes the composition root know less by making adapters know more.

The hex layout is the canonical composable shape. Test every change against it.

### 3. Does this make the system more extensible, or less?

- More: the change adds rustdoc to a previously-undocumented surface, writes an ADR for a non-obvious decision, simplifies a trait signature so external implementors don't need to reason about lifetimes.
- Less: the change removes rustdoc, hides a public type, adds a workaround without explaining why.

The extension test: would a new contributor, reading only this PR + the touched module + the related ADR, be able to add the next adapter on top of this change?

## The cycle map

Each ACE force prevents the cycle from spinning:

```
   Stasis ──fed by──> Drag ──fed by──> Opacity ──fed by──> Stasis
     │                  │                   │
     │                  │                   │
     ▼                  ▼                   ▼
  Adaptable        Composable          Extensible
```

When a change makes any one of the three forces worse, all three counter-disciplines drift together. The reviewer's job is to catch this early.

## ACES at matra's scale today

Where matra is doing well:

- **A**daptable: every public type is `#[non_exhaustive]`; feature flags are additive; the boundary rules are explicit.
- **C**omposable: hex layout intact; four ports with adapters behind them; clear composition root.
- **E**xtensible: rustdoc on every public surface; ADRs for substantive decisions.

Where to keep watch:

- **A**daptable: when a new feature wants to imply other features (e.g., `python` implying `udpipe`), the implication is a coupling — push back unless there's a measured reason.
- **C**omposable: when an adapter wants to import another adapter ("just this once"), the boundary is being crossed — find a different shape.
- **E**xtensible: when rustdoc or ADRs lag behind code, opacity builds. The `archivist` agent's lockstep contract exists to prevent this.

## Inversion mechanisms — what to reach for when the cycle starts spinning

From the antifragile lens:

- **Stasis-first symptoms** (the platform stops evolving, decisions harden): inversion is **Adaptability**. Make the hardcoded thing configurable. Add the missing feature flag. Make the closed enum `#[non_exhaustive]`.
- **Drag-first symptoms** (the platform team is a bottleneck, simple changes take quarters): inversion is **Extensibility**. Open the extension point so contributors don't need platform-team approval to add new adapters.
- **Opacity-first symptoms** (nobody knows why something works, workarounds pile on): inversion is **Composability**. Decompose the god-module into discrete components with stated boundaries; what was opaque becomes a set of clearly-bounded pieces.

matra is small enough today that the cycle isn't actively spinning. The disciplines exist so that when scale arrives, the cycle never gets started.


## What this skill won't tell you

- Specific Rust patterns (which combinator, which trait shape) — those are `rust-craft`.
- Implementation mechanics — those are the other skills.
- Whether a specific change is "ACES-compliant" — that's a judgment call; this skill gives you the questions to ask.

## The non-negotiable part

ACES is the philosophy matra is built on. A change that's good engineering but violates ACES is not good for matra. Specifically:

- A change that makes the system harder to evolve (removes `#[non_exhaustive]`, locks in a backend, hardcodes a value that should be configurable) — **reject** unless there's an ADR justifying the trade.
- A change that blurs boundaries (adapter imports another adapter, port imports another port, domain.rs grows a new dep beyond serde/thiserror/std) — **reject** unless there's an ADR.
- A change that grows opacity (removes rustdoc, adds a workaround without explaining, deletes an ADR's context) — **reject** in review.

These rejections are not personal; they are the substrate's self-defense.
