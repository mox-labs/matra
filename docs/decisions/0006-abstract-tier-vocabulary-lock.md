# 0006. Abstract-tier vocabulary lock

- **Status:** Accepted
- **Date:** 2026-05-24
- **Decider(s):** project maintainer (with the ontology guild: karman, burner, ace, chesterton, dijkstra, k)

## Context

matra spans three triguna tiers per the conviction settled in 2026-05-21: **record** (tokens, sentences, paragraphs, sections, POS, lemmas, dependencies; ships in 0.0.x), **abstract** (relations, schemas, modalities, speech acts, voice signatures; planned 0.2+), and **extract** (core claims, theses, principles; downstream consumer concern).

Two ontology problems were open after the 2026-05-21 conviction work:

1. **`Analysis` is structurally a misnomer.** The type holds parsed *output*: sections, paragraphs, sentences, tokens, metric slots. It is the document representation, not an analytical act. Calling it `Analysis` collapses the substrate-vs-interpreter distinction the conviction depends on. matra structures; the consumer analyzes.

2. **The abstract-tier names are not yet code, but they will be.** Relation, Schema, Modality, SpeechAct, Stylometry, the umbrella over them all (Finding, NOT Frame), the source-span pointer, the rule wrapper, the predicate function. Each name is a public-surface commitment. Settling them now (pre-publication) is cheap; settling them after 0.1.0 ships is a SemVer-major.

The risk: ship 0.0.x with the wrong names, lock downstream into them, then pay SemVer-major rename costs in 0.2 when the abstract tier lands.

## Decision

Reserve abstract-tier vocabulary now, in this ADR, without committing to shape.

### Renames executed in Phase 1 (this ADR's PR)

- **`Analysis` → `Document`.** The type that holds parsed output is the document representation; the conviction page argues this directly ("matra structures the trace"). Every reference in `src/`, `python/`, `book/`, `docs/`, `examples/`, `tests/`, `.claude/arch/`, and `CHANGELOG.md` is updated. A transitional `pub type Analysis = Document;` ships in `src/domain.rs` with a `#[deprecated]` annotation; it is scheduled for removal in 0.1.0 so in-flight branches and just-published downstream snippets keep compiling through the alpha cycle.

### Reserved names (Phase 2 ships them; this ADR forbids alternatives)

| Concept | Name | Notes |
|---|---|---|
| Relation triple (S-V-O extracted) | `Relation` | NLP term-of-art |
| Typed entity with attributes | `Schema` | Watch for JSON-Schema collision; revisit if confusion surfaces |
| Modal marker (epistemic / deontic / evidential) | `Modality` | Linguistically precise |
| Illocutionary force | `SpeechAct` | Austin / Searle term-of-art |
| Aggregate stylometric profile | `Stylometry` | "Voice" stays conviction-level; the artifact is stylometry |
| Umbrella over extraction outputs | **`Finding`** (NOT `Frame`) | `Frame` is reserved by [ADR-0002](./0002-pipeline-vocabulary.md) for Fillmore-style frame semantics |
| Source span pointer | `SourceSpan` | Struct of primitives (byte_offset, byte_length, sentence_id, token_range); FFI-safe |
| Declarative rule wrapper | `Rule` | Lives in a new `src/rules/` module |
| Predicate function over Document | `Predicate` | Lives in `src/rules/` |
| Paragraph kind discriminant (planned) | `ParagraphKind` | Variants: Body / Quote / Code / List / Caption. Replaces `Paragraph.in_blockquote` in 0.2+ |
| Orthogonal paragraph role marker (planned) | `ParagraphRole` | Distinct from `ParagraphKind` per Dijkstra's orthogonality guard |
| Per-paragraph metric grouping (planned) | `ParagraphMetrics` | Non-optional outer; partiality via inner `Option<f64>` per slot |
| Document-level metric grouping (planned) | `DocumentMetrics` | Same shape rule |

### Reserved verb

`frame` per ADR-0002 — preserved untouched.

### Finding contract (formal, mandatory when the shape lands)

If `Finding` lands as a trait, the contract is:

- **Frame-1.** Provenance mandatory. `source_span()` returns a valid byte range.
- **Frame-2.** Confidence shape mandatory, value optional. `None ≠ missing`.
- **Frame-3.** Discriminant names a structural fact, not the state of speaker / reader / world.
- **Frame-4.** FFI-safe materialization. Only primitives, `String`, `Option<primitive>`, FFI-safe structs cross the trait.
- **Frame-5.** Every concrete `*Finding` struct is `#[non_exhaustive]`.

If `Finding` lands as an enum, the same five rules apply at the variant level, plus serde adjacent-tagged + `#[non_exhaustive]` (per Ace's wire-format guard).

**The shape decision (trait vs enum) is deferred to Phase 2.** Per the burner verdict, abstraction extracts when the third concretion forces it, not the first.

## Rationale

**Why `Document` (not `Analysis`).** The type holds the output of parsing; it is the document made queryable. The consumer brings the analytical act. Calling the type `Analysis` puts the verb in the substrate, contradicting the conviction's substrate-vs-interpreter line.

**Why `Finding` (not `Frame`).** ADR-0002 reserved `Frame` for Fillmore-style semantic-frame outputs that matra may someday produce. Reusing the name for the umbrella over all extraction outputs would erase that reservation and force a confusing rename later.

**Why reserve names without committing to shape.** Trait vs enum, `#[non_exhaustive]` placement, exact field layout: these are shape decisions that depend on real consumer patterns. Names are forward-looking commitments that block competitors and let the team plan migration. Shapes are present-tense decisions that need concrete code pulling them into existence.

**Why now.** Pre-publication is the only cheap window. Once 0.1.0 ships and downstream consumers depend on the names, renames cost SemVer-major coordination.

## Consequences

### Positive

- `Document` reads correctly in three languages (Rust struct, Python TypedDict, future TS interface). "Analysis" reads as a verb in English; "document" reads as a noun.
- The abstract-tier name set is locked. Phase 2 work cannot accidentally introduce alternative names. PR review against this ADR catches drift.
- The conviction page's substrate-vs-interpreter line is structurally consistent with the type names.
- Downstream alpha consumers see one rename today (Analysis → Document), with a deprecation alias that keeps their snippets working through the 0.0.x line.

### Negative

- Existing downstream code (none yet, since matra is unpublished) that references `Analysis` needs to migrate before 0.1.0. The deprecation alias keeps the 0.0.x line working but emits compiler warnings.
- Internal documents (`.claude/arch/*.md`) referencing `Analysis` are now updated; future authors must use `Document` consistently. The type-name parity floor gate (M0) catches drift.

### Neutral

- ADR-0002's `frame` verb / `Frame` semantic-frame reservation is preserved unchanged. This ADR strengthens that reservation by routing the umbrella name through `Finding` instead.
- ADR-0004's single-crate decision is unaffected. The abstract-tier vocabulary lives in `matra::*` for now; extraction into a separate crate is a Pattern 6 decision triggered by external implementor ecosystems, not by name reservation.

## Explicit non-decisions (deferred)

- **`Paragraph.in_blockquote` → `ParagraphKind` enum.** The boolean's job (gate measure or not) is binary today. The deprecation rustdoc on the field signals the future migration; the field stays in 0.0.x and 0.1.x.
- **Metric slot grouping into `ParagraphMetrics`.** Phase 2 work, dependent on whether the cost-benefit lands. Outer `Option<ParagraphMetrics>` is **forbidden** (breaks I-P3 independent-metric-gating); inner `Option<f64>` per slot is the only shape that preserves the invariant.
- **`CorpusEntry` collapse + `RawDocument` disambiguation.** Phase 2 work, paired with the rules iteration.
- **`Finding` shape (trait vs enum).** Phase 2, decided at the first concrete consumer site.

## Validation criteria

Per the ixian validation criteria (`.claude/rhetoric/ixian-validation-criteria.md`):

- **In-PR.** `cargo check --all-targets` + `cargo check --all-targets --no-default-features` + `cargo test --features udpipe` all clean. The M0 floor gate (`just docs-floor`) passes; in particular, gate 3 (type-name parity) catches any stale `Analysis` reference in `book/src/` that did not flow through the rename.
- **Post-merge.** No new uses of `Analysis` enter the codebase (the deprecation lint catches them in CI's `-D warnings` clippy pass).
- **Pre-0.1.0.** The deprecation alias is removed from `src/domain.rs`; the type-name parity gate refuses to add `Analysis` back to the docs.

## Related

- [ADR-0001](./0001-record-architectural-decisions.md) — ADR process
- [ADR-0002](./0002-pipeline-vocabulary.md) — `frame` verb reserved; `Frame` reserved for Fillmore semantic frames
- [ADR-0003](./0003-workspace-with-rumi-nlp.md) — superseded; single-crate
- [ADR-0004](./0004-stay-single-crate.md) — single-crate decision rationale; Pattern 6 trigger conditions
- [ADR-0005](./0005-supply-chain-hardening.md) — supply-chain posture for the eventual 0.1.0 publish
