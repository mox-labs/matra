# Architecture Decision Records

This directory holds **Architecture Decision Records (ADRs)** for matra.
An ADR captures a single architectural decision: the context, the options
considered, the choice made, and the consequences.

## Why we use ADRs

Code shows *what*. Commit messages show *what changed*. Neither shows
*why this option and not another*. ADRs are the audit trail for
non-obvious calls, written at the time the decision is made rather than
reconstructed after.

A reader six months from now should be able to ask "why did we do X
instead of Y?" and find the answer in `docs/decisions/`.

## When to write one

Write an ADR when:

- The decision binds future code (changes the API, shape, or boundaries).
- Multiple plausible options were considered.
- The reason for the choice is not obvious from the code itself.
- A future contributor would benefit from understanding the tradeoff.

Do *not* write an ADR for:

- Cosmetic choices (formatting, naming convention details).
- Bug fixes (the commit message is enough).
- Decisions that are already self-evident from the code shape.

## How to write one

Copy `template.md` to `NNNN-short-name.md` (next sequence number,
zero-padded to 4 digits). Fill in each section. Open a PR with the
ADR alongside any code changes it binds.

## Status lifecycle

| Status | Meaning |
|---|---|
| `proposed` | Under discussion. Linked to a `decision`-type issue. |
| `accepted` | Decided and in effect. The default for ADRs that ship. |
| `deprecated` | No longer in effect; superseded by a later ADR. |
| `superseded by ADR-NNNN` | Same as deprecated, with a link forward. |

ADRs are **append-only**. We do not edit history; we add new ADRs that
supersede old ones. The full lineage is preserved.

## Index

| ID | Title | Status |
|---|---|---|
| [0001](0001-record-architectural-decisions.md) | Record architectural decisions | accepted |
| [0002](0002-pipeline-vocabulary.md) | Pipeline vocabulary: ingest / decompose / parse / measure | superseded by ADR-0007 |
| [0003](0003-workspace-with-rumi-nlp.md) | Cargo workspace with `matra-core` and `rumi-nlp` | superseded by ADR-0004 |
| [0004](0004-stay-single-crate.md) | Stay single-crate; supersede the workspace split | accepted |
| [0005](0005-supply-chain-hardening.md) | Supply-chain hardening | accepted |
| [0006](0006-abstract-tier-vocabulary-lock.md) | Abstract-tier vocabulary lock | accepted |
| [0007](0007-one-pipeline.md) | One pipeline: ingest -> decompose -> compose, with abstract reserved | accepted |
| [0008](0008-structural-primitives-are-fields.md) | Structural primitives are fields | accepted |
| [0009](0009-feats-lookup-accessor.md) | Feats lookup accessor, Rust-only | accepted |
| [0010](0010-embeddings-adapter.md) | Embeddings: a Tier-2 channel behind an Embedder port, static adapter first | accepted |
