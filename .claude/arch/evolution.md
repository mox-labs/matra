# Evolution

Architecture is a sequence of decisions across iterations. This file is the change history of the boundary, not the change history of the code (that's in git and CHANGELOG.md).

## Iterations shipped

| Iteration | What it locked | Status |
|---|---|---|
| I0 | Hex layout, three ports, single-crate shape, `#[non_exhaustive]` everywhere | shipped |
| I1 | Pipeline verbs (`ingest → decompose → parse → measure` + peer `extract`) | shipped |
| I8 | Three stages (`ingest -> decompose -> compose`), `abstract` reserved, the six entry points deleted for `Ingest` + `Engine`. ADR-0007 supersedes ADR-0002 | shipped 2026-08-21 |
| I2 | Resilience floor (size caps, symlink rejection, atomic download, TOCTOU closure, `catch_unwind` panic boundary, O(n) tree_depth, parse-per-paragraph) | shipped |

Future iterations are tracked in `book/src/plans/`. Past iterations are not rewritten; commitments only get superseded by new ADRs, never edited out.

## What never gets undone

These are commitments, not preferences. Once locked, they hold:

- **Domain purity.** `domain.rs` imports only `serde`, `thiserror`, and `std`. No further crates without an ADR.
- **Single UDPipe importer.** Only `nlp/udpipe.rs` imports `udpipe_rs`. Adding a second site is a boundary failure. Enforced by `scripts/check-boundaries.sh`.
- **`#[non_exhaustive]` on every public enum and every public struct with public fields.** Forward compatibility is non-negotiable; matra is a substrate.
- **Hex layout.** Adapters do not import each other. Ports do not import each other. The composition root is the only file that knows the whole pipeline.
- **No publish without explicit approval.** `cargo publish` and `maturin publish` always run with `--dry-run` first; explicit per-publish approval per the project memory.
- **Conventional commits.** Commit messages follow the conventional-commit grammar so the CHANGELOG generator works without per-commit editing.

## What is allowed to change

- Which adapters exist. Adding `PdfDecomposer`, a non-English UDPipe variant, a pure-Rust NLP backend — all permitted as long as they implement existing ports without polluting them.
- Which features are default. `udpipe = ["udpipe-rs", "sha2"]` is default for ergonomic reasons; a future release could flip it if a lighter backend exists.
- The number of metrics in the default suite. Adding readability variants, gating expensive metrics behind a feature flag.
- Internal data structures. Sentence index caches, memoized depth tables, intern pools — none of these touch the public surface.
- Adding a new sub-module within matra for a planned capability (e.g., rule evaluation over parsed text structure).

## Decisions previously considered and rejected

### Workspace split (`matra-core` + sibling matcher-bridge crate)

Proposed in ADR-0003. The proposal was to split matra into a substrate crate plus a sibling crate for rule-based pattern matching over parsed sentences. Superseded by ADR-0004 (2026-05-20) on the grounds that the rule-evaluation capability is part of matra's own surface, not a peer crate, and the workspace-split criterion (Pattern 6 from the rust-mastery corpus: separately publish a minimal port crate when an external implementor ecosystem exists) has not fired.

If and when external NLP backends emerge as published crates (`matra-stanza`, `matra-spacy`, etc.), extract `matra-nlp-api` as a minimal port crate and keep `matra` as the consumer-facing crate. Until then, single-crate is correct.

### Built-in extractors for specific patterns (SVO, copular, prepositional, passive, nominal modifier)

Considered as part of matra-core's surface. Rejected by user direction earlier in the project. Pattern extractors are opinions; matra is a substrate that provides parse trees and aggregate metrics, not opinions about which patterns matter. Pattern extraction lands as consumer code or as a separate sub-module behind a clear "opinionated" boundary; it does not enter the default surface.

### A four-port model with a separate `Ingest` trait

Tested: would adding a fourth port (`Ingest`) clarify the boundary between "open the file" and "convert to RawDocument"? No. `Source` already is the ingest port. A separate `Ingest` duplicates surface.

### Async pipeline

Tested: would async/event machinery help? No, for two reasons:

- UDPipe is `!Send` — there is no parallel sink to feed.
- The hot path is `analyze(doc)` on a single document. Async would optimize a workload that nobody runs.

The pipeline stays synchronous. If a push-semantics consumer arrives (websocket, filesystem watch, message queue), revisit.

### Streaming reactor with channels

Tested: leverage the reactor pattern. Same reasoning as async pipeline. The pipeline stays synchronous; a streaming iterator API arrives if and when corpus-sized consumers need it.

Triggers to revisit:

1. A consumer needs incremental re-analysis on file change (push semantics, not pull).
2. A corpus consumer reports more than 100k documents in regular use.
3. A second `Source` arrives that is inherently push (websocket, filesystem watch, message queue).

Until at least one trigger fires, the reactor does not ship and that is the correct answer.

## Future direction

Planned capabilities, not yet shipped. Each carries its trigger condition:

- **Rule evaluation over parsed text structure.** A sub-module inside matra that lets consumers query parse trees with rule-like predicates (matchers over POS sequences, dep relations, lemma sets, subtrees). Lands when the surface design is settled and an internal or external consumer commits.
- **WASM/TS crust.** Same Rust core, second crust via `wasm-bindgen` + `serde-wasm-bindgen`. Lands when a TypeScript consumer commits.
- **Pdf/Docx adapters.** Behind feature flags, when a consumer needs them. Half-shipping a PDF adapter would lock a bad shape into the public surface; the gap is deliberate.

## Origin notes

Two memory entries shape every iteration boundary:

1. **Ontology-first.** Names settle first, code moves second. This is why renames precede structural work: renaming a stable surface is cheaper than renaming a freshly restructured one.

2. **Never publish without approval.** Each iteration's "ship" gate stops at `cargo publish --dry-run`. Explicit per-publish approval is required. One approval authorizes one publish; do not reuse across versions or packages.

These hold across the whole project, not per-iteration.

## How to add a future iteration

1. Identify the trigger. What changed in the world that requires a new boundary? (Consumer report, performance ceiling, new requirement.)
2. Write a short proposal in `book/src/plans/` named `iN-<topic>.md`. Use the same structure as the existing plans.
3. If the change touches the public surface or a boundary rule, write an ADR under `docs/decisions/` and link it from the plan.
4. Land. Validate. Update CHANGELOG.md.

The iteration table above is append-only. Past iterations don't get rewritten when a new one lands; they get a superseded-by note in the row instead.
