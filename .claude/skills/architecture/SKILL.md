---
name: architecture
description: Vaani's architectural disciplines — hex boundary rules, port design, composition root, adapter pattern, applying canonical patterns from the rust-mastery corpus (Pattern 5 deployment-shape, Pattern 6 trait-substrate-stability, Pattern 10 orthogonal-dispatch). Use when adding modules, creating adapters, or making structural changes.
---

# architecture

Architectural disciplines for vaani. This skill codifies the hex layout, the boundary rules, and the canonical patterns from the rust-mastery corpus that apply at vaani's scale.

## When to invoke

- Adding a new module.
- Creating a new adapter for an existing port.
- Designing a new port.
- Evaluating whether a port should be extracted into its own crate.
- Auditing a structural change for boundary compliance.

## The non-negotiable foundation — ACES

Before any architectural decision, run the boundary test from `.claude/skills/aces/SKILL.md`. ACES (Adaptable, Composable, Extensible) is the design philosophy vaani is built on. It is non-negotiable. The three questions:

1. Does the change make the system more adaptable, or less?
2. More composable, or less?
3. More extensible, or less?

A change that's good engineering but violates ACES is not good for vaani. The hex layout below *is* the ACES discipline made concrete; understand the philosophy before applying the structure.

## The hex layout

```
src/
├── lib.rs              # composition root (the only file that knows the whole)
├── domain.rs           # types — serde + thiserror + std only
├── source/             # Source port + FileSource + DirectorySource adapters
├── decompose/          # Decomposer port + MarkdownDecomposer + PlainTextDecomposer
├── nlp/                # NlpProvider port + Udpipe adapter
├── metrics/            # metric functions — depend only on domain + stopwords
├── extraction/         # extractor functions — depend only on domain + stopwords
└── stopwords.rs        # shared utility
```

Dependencies point inward. Adapters know about `domain` and the port they implement. Ports know only about `domain`. Domain knows nothing. Composition root knows everything.

## The seven boundary rules

These are enforced and non-negotiable:

1. `domain.rs` depends only on `serde`, `thiserror`, `std`. Any further dep requires an ADR.
2. Port modules (`source/mod.rs`, `decompose/mod.rs`, `nlp/mod.rs`) import only from `domain`.
3. No port module imports another port module.
4. `nlp/udpipe.rs` is the only file that imports `udpipe_rs`.
5. `metrics/` and `extraction/` import only from `domain` and `stopwords`.
6. `cargo check --no-default-features` must compile.
7. The composition root (`lib.rs`) is the only place that knows all adapters and ports.

`scripts/check-boundaries.sh` enforces rules 2, 3, 4 in CI. Rules 1, 5, 6, 7 are enforced by the type system + `cargo check`.

When you break a rule, you're either:

- Fixing a bug in the rules (write an ADR explaining why),
- Or making a structural mistake (fix the structure, not the rule).

## Adding a new adapter

The recipe:

1. **Pick the port.** Source, Decomposer, or NlpProvider — usually obvious from the verb.
2. **Create the file** in the port's module: `src/source/<name>.rs` or equivalent.
3. **Implement the port trait** for your new adapter struct.
4. **Don't import other adapters.** Other adapters live in sibling files; do not reach across.
5. **Translate external errors** to `domain::Error` variants. Never propagate external crate errors through the boundary.
6. **Document contract overrides** inline. If your adapter's behavior differs from the port's documented postconditions (e.g., DirectorySource's sort order), document the override at the impl site.
7. **Add unit tests** for the adapter's contract: what it accepts, what it rejects, what it returns under each input class.
8. **Wire it into the composition root** if it should be available via the convenience API; otherwise leave it as a manually-composed building block.

## Adding a new port

The bar is high. From `portsmith.md`:

1. Real adapter need (not anticipated need).
2. Trait is small (one or two methods).
3. Imports only `domain`.
4. There's at least one consumer in the composition root.

Three ports today is the minimum that preserves the boundary discipline. Adding more raises coordination cost without adding composability.

## Applying canonical patterns from the corpus

The rust-mastery corpus closed at 2026-05-14 with four corpus-canonical patterns. Each has a specific application criterion at vaani's scale:

### Pattern 5 — deployment-shape via feature flags

Vaani is library-shaped, capability-composition flavor. Two feature flags (`udpipe`, `python`), each adds a capability without changing the core library. Adding a third flag is fine if it adds a new capability axis (e.g., `wasm` for the future WASM crust, `pdf` for a hypothetical PDF decomposer).

Watch for the lancedb leakage failure mode: external dep's default features pulling in unwanted backends (issues #2865, #2567 in lancedb). Vaani's deps are small enough today that this isn't an active risk, but audit `default-features = false` discipline on any new dep that could transitively pull in optional backends.

### Pattern 6 — trait-substrate-stability via separately published minimal crate

Criterion (`frames/cross-artifact/m8-i3-search-tier-pattern6-substrate-stability.json`): separate a port trait into its own minimal crate IFF an **external implementor ecosystem** exists who needs to pin the contract independently.

Today, vaani has no external `NlpProvider` implementor crates. Keep `NlpProvider` in-crate.

If a third-party `vaani-stanza`, `vaani-spacy`, etc. emerges, extract `vaani-nlp-api` as a minimal crate (domain types + the trait, no other deps), and rewrite `vaani` to depend on `vaani-nlp-api`. Write an ADR superseding `0004-stay-single-crate.md` at that point.

### Pattern 10 — orthogonal-dispatch axes

Vaani uses **one** runtime dispatch axis: `&dyn NlpProvider`. The Source and Decomposer choices are made statically at the composition root (format enum → MarkdownDecomposer vs PlainTextDecomposer; path vs directory → FileSource vs DirectorySource).

This is appropriate at vaani's scale. The corpus shows N=4 axes only at search-engine scale (tantivy). Resist the temptation to runtime-dispatch the other ports until there's a concrete need.

### Pattern 11 — incremental computation

Not applicable. Vaani is a one-shot pipeline (parse → analyze → return), not an incremental system. Memoizing parse results across calls is fine at the consumer level (and is what `parse-once-use-many` via the public `parse` function enables), but the substrate is stateless.

## Cross-language considerations

Every type in `domain.rs` appears in Rust today, in Python via `pythonize`, and in TypeScript via `serde-wasm-bindgen` when the WASM crust lands. Architectural implications:

- **Methods do not cross FFI. Only fields do.** Aggregate methods (e.g., `Analysis::passive_ratio()`) are not visible to Python consumers in serialized output. If you need them visible, materialize as a field on a derived summary type.
- **Names cross.** Every public type/field/enum-variant name is a contract across at least two languages. When picking a name, ask: does it read clearly as a Python dict key? As a TypeScript interface field?
- **`#[non_exhaustive]` on every public type.** Forward compatibility across all three languages — additive changes are minor-version, not major.

## When you reach for the corpus

- `frames/cross-artifact/vaani-readiness.json` — the integrating M1 Frame; the complete architectural prescription.
- `frames/cross-artifact/cross-iteration-pattern-consolidation.json` — the four corpus-canonical patterns and their adoption-discipline regimes.
- `frames/cross-artifact/m8-i3-search-tier-pattern6-substrate-stability.json` — Pattern 6 criterion (when to extract a port to its own crate).
- `frames/cross-artifact/m8-i2-vector-db-deployment-shape-asymmetry.json` — Pattern 5 (library-shaped vs server-shaped) with the lancedb leakage failure mode.

## What this skill won't tell you

- Specific Rust patterns (which combinator, which iterator method) — case-by-case.
- FFI surface choices — `ffi-surface` skill.
- Failure mode design — `resilience-floor` skill.
- Documentation sync — `docs-lockstep` skill.
