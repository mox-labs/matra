# Evolution

Architecture is a sequence of decisions across iterations. This file is the change history of the boundary, not the change history of the code (that's in git).

## Iterations

| Iteration | Boundary | What lands | Status |
|---|---|---|---|
| I0 | none | Stabilize the post-recovery baseline | planned |
| I1 | none | Karman pipeline rename: `ingest → decompose → parse → measure` + peer `extract` | planned |
| I2 | resilience floor | The 10 antifragile fixes (Taleb + Knuth + Vector + Lamport) | planned |
| I3 | **MVP** | Error restructure (Dijkstra split) + tracing PR1 (Wolf) | planned |
| I4 | **MLP** | Streaming iterator + Engine struct + CorpusResult wrapper | planned |
| I5 | post-publish | OTel feature, PDF/DOCX adapters, possibly the reactor | post-0.1 |

**MVP** = "is correct, is bounded, has a recovery contract." A consumer can use it and recover from errors. Not yet streaming.

**MLP** = MVP plus "scales to corpus-sized work without OOM." The streaming iterator is what turns a working library into a deployable substrate.

## What got rejected and why

### `arrange / decompose / frame / compose` (the original Karman proposal)

The user-floated quartet had two false names. Karman rejected them:

- **`arrange`** — ambiguous between "ingest and dispatch" and "order within a corpus." A name that requires disambiguation is a placeholder. Replaced with `ingest`.
- **`frame`** — collides with Fillmore frame-semantics. A future consumer building FrameNet-style semantic role analysis will want exactly that vocabulary. Burning the word for "tokens + POS + dep parse" forecloses a future contract. Replaced with `parse`.
- **`compose`** — vague. The stage produces measurements (scalars), not assemblies. Replaced with `measure`.

`decompose` survived because it accurately names what the stage does.

### A four-port model with a separate `Ingest` trait

Tested: would adding a fourth port (`Ingest`) clarify the boundary between "open the file" and "convert to RawDocument"? Burner: no. `Source` already is the ingest port. A separate `Ingest` duplicates surface.

### The reactor

Tested: does the user's "leverage the reactor pattern" goal justify async/event machinery now? Erlang and K both said no:

- UDPipe is `#[pyclass(unsendable)]` — there is no parallel sink to feed, so a reactor is "a bigger queue, not more throughput."
- The hot path for all three downstream consumers is `analyze(doc)` on a single document. The reactor optimizes a workload that nobody runs.
- ~140 lines of sync glue does not justify an async dependency tree imposed on every consumer.

The streaming iterator (`analyze_directory_iter`) is the right primitive for now. It eliminates the OOM-at-10k-docs failure of the buffered `analyze_directory` and provides the boundary where a channel slots in if the reactor ever lands.

#### Reactor triggers (when to revisit)

The reactor comes back when **any one** of these is observable:

1. A consumer needs incremental re-analysis on file change (push semantics, not pull).
2. A corpus consumer reports more than 100k documents in regular use.
3. A second `Source` arrives that is inherently push (websocket, filesystem watch, message queue).

Until at least one trigger fires, the reactor does not ship and that is the correct answer. If none of them fire across the lifetime of the project, the reactor never ships.

The triggers are named here so a future iteration can check them without re-litigating the decision.

## Public surface evolution

| Surface element | I0 | I1 | I2 | I3 (MVP) | I4 (MLP) | I5+ |
|---|---|---|---|---|---|---|
| Pipeline verbs | old | renamed | renamed | renamed | renamed | renamed |
| `Error` variants | `Io`, `ParseFailed(String)`, ... | same | same | split + structured | same | same |
| `is_skip_doc` / `is_fatal` | absent | absent | absent | present | present | present |
| `analyze_directory` | buffered | buffered | buffered | buffered | deprecated | deprecated |
| `analyze_directory_iter` | absent | absent | absent | absent | present | present |
| `Engine` struct | absent | absent | absent | absent | present | present |
| `tracing` dep | absent | absent | absent | always-on | always-on | always-on |
| `otel` feature | absent | absent | absent | absent | absent | available |
| PDF/DOCX | `UnsupportedFormat` | same | same | same | same | gated feature |

`#[non_exhaustive]` on every public type is locked from I0 forward. This is the v2 commitment that survives every restructure (Chesterton fence 2).

## What never gets undone

These are commitments, not preferences. Once locked, they hold:

- **Domain purity.** `domain.rs` will never import a non-`std`/non-`serde` crate. Not even `tracing`.
- **Single UDPipe importer.** Only `nlp/udpipe.rs` imports `udpipe_rs`. Adding a second site is a boundary failure.
- **`#[non_exhaustive]` on every public type.** Forward compatibility is non-negotiable; vaani is a substrate.
- **Hex layout.** Adapters do not import each other. Ports do not import each other. The composition root is the only file that knows the whole pipeline.
- **No publish without explicit approval.** `cargo publish` and `maturin publish` always run with `--dry-run` first; explicit per-publish approval per the project memory.

## What is allowed to change

- Which adapters exist. Adding `PdfDecomposer`, a non-English UDPipe variant, a pure-Rust NLP backend, etc. — all permitted as long as they implement existing ports without polluting them.
- Which features are default. `udpipe = ["udpipe-rs", "sha2"]` is default for ergonomic reasons; a future release could flip it if a lighter backend exists.
- The number of metrics in the default suite. Adding readability variants, gating expensive metrics behind a feature flag.
- Internal data structures. `SentenceIndex` caches, memoized depth tables, intern pools — none of these touch the public surface.

## Origin notes

Two memory entries shape every iteration boundary:

1. **Ontology-first.** Karman runs before structural work. Names settle first, code moves second. This is why I1 (rename) precedes I2 (resilience) and not the other way: renaming a stable surface is cheaper than renaming a freshly restructured one.

2. **Never publish without approval.** Each iteration's "ship" gate stops at `cargo publish --dry-run`. Explicit per-publish approval is required. One approval authorizes one publish; do not reuse across versions or packages.

These are not negotiable per-iteration; they hold across the whole project.

## How to add a future iteration

1. Identify the trigger. What changed in the world that requires a new boundary? (Consumer report, performance ceiling, new requirement.)
2. Write a short proposal in `.claude/implans/` named `iN-<topic>.md`. Use the same structure as the existing implans: what, why, validation, acceptance.
3. Convene the relevant guild members for review. Boundary changes go through Burner and Karman at minimum. Resilience changes go through Taleb. Flow changes go through Erlang.
4. Update this file with the new row in the iterations table and the surface evolution matrix.
5. Land. Validate. Document the post-ship measurement (the loop closure pattern from Ixian).

The iteration table is append-only. Past iterations don't get rewritten when a new one lands; they get a `superseded by` note in the row instead.
