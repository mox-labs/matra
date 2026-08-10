# 0002. Pipeline vocabulary: ingest / decompose / parse / measure (+ peer extract)

- **Status:** accepted
- **Date:** 2026-04-28
- **Decider(s):** project maintainer; ontology review by the architecture guild (Karman)

## Context

matra's pipeline produces structured analysis from text in stages. The
stage names become the public vocabulary: trait method names appear in
docs, trait names in code, free function names in `lib.rs`, and (via
PyO3) Python class methods. Once 0.1.0 ships these names appear on
crates.io and PyPI; rename costs amplify with every consumer.

The original v2 plan used: `Source -> Decompose -> Annotate (NLP) ->
Encode -> Extract`. A user-floated alternative was: `arrange ->
decompose -> frame -> compose`. Both have ergonomic and semantic
issues that needed settling before structural work.

## Options considered

### Option A: Keep `Source / Decompose / Annotate / Encode / Extract`

The original v2 vocabulary.

**Pros:** familiar to anyone reading the original plan; no churn.
**Cons:** "Encode" is implementation-vocabulary, not domain-vocabulary
(Karman's prior objection); "Annotate" is sterile and does not name
what NLP gives us (linguistic structure with edges, not just labels).

### Option B: Adopt `arrange / decompose / frame / compose`

The user's floated alternative.

**Pros:** more lyrical; "compose" suggests output assembly.
**Cons:** "arrange" is ambiguous between "ingest and dispatch" and
"corpus-level ordering"; "frame" collides with Fillmore frame-semantics
that future consumers building FrameNet-style analysis will need;
"compose" is filler — the stage produces measurements (scalars), not
assemblies.

### Option C: `ingest / decompose / parse / measure` (+ peer `extract`)

Karman's verdict after rejecting Option B's three problematic names:

- `arrange` -> `ingest`: unambiguous; describes loading bytes from
  a path or text into a `RawDocument`.
- `decompose`: kept (the only name from B that survived).
- `frame` -> `parse`: NLP-correct; reuses the established term of
  art; preserves "frame" for the future semantic-frame consumer.
- `compose` -> `measure`: honest about what the stage produces
  (scalars per paragraph and per document).
- `extract`: peer to `measure`, not nested. Selections (top sentences,
  keyphrases) are ontologically distinct from aggregations (metrics).

**Pros:** every name is honest about what its stage does; no collisions
with downstream vocabulary; "measure" and "extract" cleanly separate
aggregation from selection.
**Cons:** requires renames in `lib.rs`, doc comments, README, CHANGELOG.

## Decision

We adopt **Option C**. Pipeline vocabulary: `ingest -> decompose ->
parse -> measure` with `extract` as a peer.

The `Source`, `Decomposer`, and `NlpProvider` traits keep their
existing names — they semantically match the new verbs already
(`Source`'s job is ingestion; `NlpProvider::parse` is the parse stage's
mechanism). The renamed verbs appear in stage descriptions, doc
comments, and composition-root function names.

One concrete code change: removed the free function
`decompose::markdown::parse`. Markdown decomposition now goes through
`MarkdownDecomposer.decompose(text)` only. This frees the verb
`parse` for NLP-only use across the codebase.

## Consequences

**Positive:**
- Vocabulary is honest about what each stage does.
- "frame" is preserved for a future semantic-frame analysis layer
  (a downstream consumer building on top of `parse` output).
- `measure` vs `extract` distinction makes the public surface easier
  to teach.

**Negative:**
- One-time rename pass (landed in I1).
- Anyone reading old v2 plan documents needs to translate.

## Validation

Right if the names hold up against being read in three languages
(Rust, Python, TypeScript-via-WASM). Falsified if a downstream
consumer requests a name we have to break to give them.

## References

- I1 plan: `book/src/plans/i1-rename.md`.
- Architecture: `.claude/arch/architecture.md` (pipeline through-line).
- I1 PR (merged 2026-05-01): https://github.com/mox-labs/matra/pull/1.
