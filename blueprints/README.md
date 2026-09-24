# Blueprints

matra's design record. Two kinds of document live here, and the process
around them follows the Rust RFC process. [RFC-0019](rfcs/0019-rfc-and-ep-process.md)
introduced it.

| Kind | Where | Cited as | What it records |
|---|---|---|---|
| RFC | `rfcs/NNNN-name.md` | `RFC-NNNN` | A design-level change: architecture, systems, the framework, the toolchain. What and why. |
| EP | `eps/NNNN-name.md` | `EP-NNNN` | An enhancement plan: how an accepted RFC gets from decision to shipping. Iterations, milestones, test plan, ship criteria, status. |

Records cited as `ADR-NNNN` before 2026-09-24 are the RFC of the same
number: `ADR-0008` is [RFC-0008](rfcs/0008-structural-primitives-are-fields.md).
Iteration plans cited as `iN` or `IN` are the EP of the same number: `i9` is
[EP-0009](eps/0009-embeddings-adapter.md).

## The process

**An RFC is proposed as a pull request.** Copy
[`rfcs/0000-template.md`](rfcs/0000-template.md) to the next free number,
fill every section, and open a pull request with the RFC alone or beside the
code it binds. An unaccepted RFC lives only as that open pull request.
Discussion happens there.

**Merging the pull request accepts the RFC.** The file reaches `main` with
its status set to `accepted`. Closing the pull request without merging
declines it, and nothing lands here.

**An accepted RFC gets an EP when its implementation spans more than one
pull request.** Copy [`eps/0000-template.md`](eps/0000-template.md) to the
next free EP number and fill `Implements`. Work that one
pull request delivers needs no EP; the RFC and the pull request are the
record.

**An RFC is not rewritten after acceptance.** Two edits are allowed: the
status line, and a dated note directly under the header saying what changed
and why. A change of mind is a new RFC that supersedes the old one, and the
old one's status then reads `superseded by RFC-NNNN`. The lineage stays
readable because nothing in it is overwritten.

**An EP is a living plan until it ships.** If a milestone turns out to be
ambiguous, the plan is the bug: edit the plan first, then the code. Every
change of status adds a dated line to its status log. Once it ships or is dropped, it is kept as the record of
how the work went.

### Status

| Kind | Status | Meaning |
|---|---|---|
| RFC | `accepted` | Merged; the decision is in effect. |
| RFC | `implemented` | Accepted, and the CHANGELOG records it shipping. |
| RFC | `superseded by RFC-NNNN` | A later RFC replaced it. Kept unchanged apart from the status line and a dated note. |
| EP | `planned` | Written, not started. |
| EP | `in progress` | At least one milestone has landed. |
| EP | `shipped in X.Y.Z` | Every milestone landed, and release X.Y.Z carries the work. |
| EP | `dropped` | Will not be carried out as written. The status log says why. |

### Numbering

Numbers are four digits, zero-padded, and never reused. RFC numbers carried
over from the decision records keep their number. A number reserved for an
open pull request is listed below as `open, reserved` so that no other record
takes it.

### Checks

`scripts/check-blueprint-refs.sh` runs from `just check` and in the
`Docsite floor` CI job. It fails when an `RFC-NNNN` or `EP-NNNN` cited
anywhere in the tracked tree resolves to no file here and to no reserved
row below, and when a file in `rfcs/` or `eps/` is missing from these
tables. The docsite floor's em-dash gate covers this directory too.

## RFCs

| RFC | Title | Status | EP |
|---|---|---|---|
| [RFC-0001](rfcs/0001-record-architectural-decisions.md) | Record architectural decisions | superseded by RFC-0019 | none |
| [RFC-0002](rfcs/0002-pipeline-vocabulary.md) | Pipeline vocabulary: ingest / decompose / parse / measure (+ peer extract) | superseded by RFC-0007 | none |
| [RFC-0003](rfcs/0003-workspace-with-rumi-nlp.md) | Cargo workspace with `matra-core` and `rumi-nlp` | superseded by RFC-0004 | none |
| [RFC-0004](rfcs/0004-stay-single-crate.md) | Stay single-crate; supersede the workspace split proposal | accepted | none |
| [RFC-0005](rfcs/0005-supply-chain-hardening.md) | Supply-chain hardening posture | accepted | none |
| [RFC-0006](rfcs/0006-abstract-tier-vocabulary-lock.md) | Abstract-tier vocabulary lock | accepted | none |
| [RFC-0007](rfcs/0007-one-pipeline.md) | One pipeline: ingest -> decompose -> compose, with abstract reserved | implemented | [EP-0008](eps/0008-pipeline-surface.md) |
| [RFC-0008](rfcs/0008-structural-primitives-are-fields.md) | Structural primitives are fields | implemented | [EP-0007](eps/0007-structural-primitives.md) |
| [RFC-0009](rfcs/0009-feats-lookup-accessor.md) | Feats lookup accessor, Rust-only | implemented | [EP-0007](eps/0007-structural-primitives.md) |
| [RFC-0010](rfcs/0010-embeddings-adapter.md) | Embeddings: a Tier-2 channel behind an Embedder port, static adapter first | implemented | [EP-0009](eps/0009-embeddings-adapter.md) |
| [RFC-0011](rfcs/0011-out-of-the-box.md) | Out of the box: configuration, paths, and one CLI | implemented | [EP-0010](eps/0010-foundations.md) |
| [RFC-0012](rfcs/0012-agent-surface.md) | The agent surface: a skill the binary prints | implemented | [EP-0011](eps/0011-agent-surface.md) |
| [RFC-0013](rfcs/0013-attribution-and-citation.md) | Attribution and Citation | implemented | none |
| [RFC-0014](rfcs/0014-distribution-matrix.md) | The Distribution Matrix | implemented | none |
| [RFC-0015](rfcs/0015-provisioning-failures.md) | Provisioning is matra's own, and a failure to fetch is not an invalid model | implemented | none |
| RFC-0016 | Release automation | open, reserved | none |
| RFC-0017 | Harness layers | open, reserved | none |
| RFC-0018 | The docsite on SvelteKit | open, reserved | none |
| [RFC-0019](rfcs/0019-rfc-and-ep-process.md) | The RFC and EP process | accepted | none |
| [RFC-0000](rfcs/0000-template.md) | The template | not a record | none |

## EPs

EP numbers follow the iteration plans they replace, so there is no EP-0001,
0002 or 0004: the i0, i1 and i2 plans were retired once their work landed
and their commits are in the history, and the i4 workspace plan was retracted
with RFC-0003.

| EP | Title | Status | Implements |
|---|---|---|---|
| [EP-0003](eps/0003-error-tracing.md) | Error restructure + tracing PR1 | planned | none recorded |
| [EP-0005](eps/0005-streaming.md) | Streaming iterator + Engine + CorpusResult | dropped | none recorded |
| [EP-0006](eps/0006-post-publish.md) | Post-publish: OTel, PDF/DOCX, `rumi-nlp` patterns, possibly the reactor | planned | none recorded |
| [EP-0007](eps/0007-structural-primitives.md) | Five structural primitives | shipped in 0.1.0 | RFC-0008, RFC-0009 |
| [EP-0008](eps/0008-pipeline-surface.md) | One pipeline, not six entry points | shipped in 0.1.0 | RFC-0007 |
| [EP-0009](eps/0009-embeddings-adapter.md) | Embeddings as a specialist adapter | shipped in 0.2.0 | RFC-0010 |
| [EP-0010](eps/0010-foundations.md) | Out of the box | shipped in 0.2.0 | RFC-0011 |
| [EP-0011](eps/0011-agent-surface.md) | The agent surface | shipped in 0.2.0 | RFC-0012 |
| [EP-0000](eps/0000-template.md) | The template | not a record | none |

## Carried over from the plans index

The index of the iteration plans, `book/src/plans/README.md` until
2026-09-24, held three things that belong to the plans rather than to any
one of them. They are kept here as they were written; the EPs refer to the
regression matrix, and the 0.1.0 predicate is the record of what that
release was held to.

### Iterations and their boundaries

| Plan | Boundary | Title |
|---|---|---|
| the i0 stabilization work | none | Commit the post-recovery baseline; capture N₀ and noise floor |
| the i1 rename work | none | Karman pipeline rename |
| the i2 resilience work | resilience floor | Ten antifragile fixes |
| [EP-0003](eps/0003-error-tracing.md) | **MVP** | Error restructure + tracing PR1 + cdylib feature-gating |
| the retracted workspace plan | structural | Workspace conversion + `rumi-nlp` skeleton |
| [EP-0005](eps/0005-streaming.md) | **MLP** | Streaming iterator + Engine + CorpusResult |
| [EP-0006](eps/0006-post-publish.md) | post-publish | OTel feature, PDF/DOCX, `rumi-nlp` patterns, deferred reactor |
| [EP-0007](eps/0007-structural-primitives.md) | rule-substrate | Negation, typed feats, modality, evidentiality, Hearst patterns |
| [EP-0008](eps/0008-pipeline-surface.md) | **pre-publish surface freeze** | One pipeline replacing six entry points |
| [EP-0009](eps/0009-embeddings-adapter.md) | post-publish, additive only | Embedder port, model2vec adapter, semantic clusters |
| [EP-0010](eps/0010-foundations.md) | post-publish; additive to the library surface, and the `--json` payload becomes an envelope | Config and paths, default constructors, one CLI with two launchers, pinned embedding download, Python extension points |
| [EP-0011](eps/0011-agent-surface.md) | additive | The `--skill` flag and its references, the executed-incantation test, `llms.txt`, `AGENTS.md`, the plugin layout |

**Strict ordering.** No iteration starts until the previous one has met its acceptance gate. K's strategic verdict on this is non-negotiable: rename a stable surface before structure moves; install the resilience floor before the error contract; ship the error contract before the streaming surface that consumes it.

### The cross-iteration regression matrix

At every iteration landing (I1, I2, I3, I4, I5, I6), all of these must hold:

1. `cargo test` count `≥ N₀` (PR0 baseline). New iterations add tests; none silently delete.
2. `cargo test --no-default-features` passes (CLAUDE.md rule 6).
3. `cargo test --features udpipe` passes.
4. `cargo test --doc` passes (doctests track API renames).
5. `cargo check --features python` passes (PyO3 surface tracks renames).
6. `cargo clippy -- -D warnings` clean.
7. `cargo public-api` diff reviewed in PR description; deltas match stated scope.
8. README example block compiles via `cargo test --doc`.
9. **Boundary check** (`scripts/check-boundaries.sh`):
   - `rg 'use udpipe_rs' src/` returns hits **only** in `src/nlp/udpipe.rs` (rule 4).
   - `rg '^use tracing|tracing::' src/domain.rs src/source/mod.rs src/decompose/mod.rs src/nlp/mod.rs` returns empty (rule 8).
   - `rg 'use crate::source|use crate::decompose|use crate::nlp' src/source/mod.rs src/decompose/mod.rs src/nlp/mod.rs` returns empty (rule 3).

If any matrix item fails at iteration landing, the iteration is rolled back, not patched forward. False foundations compound.

### The 0.1.0 ship predicate

matra 0.1.0 is publishable if and only if **all** of the following are true at HEAD on the release commit:

- [ ] Cross-iteration regression matrix items 1-9 pass.
- [ ] `cargo publish --dry-run` succeeds.
- [ ] Fault-injection corpus passes (see the i2 resilience work Validation):
  - 25-depth and 1000-depth chains return correct depths.
  - oversized inputs to TF-IDF, RAKE, and YAKE each return three distinct `InputTooLarge` errors with three distinct `what:` labels.
  - panic-injecting NLP fixture returns `ParseFailed`, never aborts.
  - directory with permission-denied file yields error and continues.
  - corrupt model returns `ModelInvalid { recoverable: false }`.
  - concurrent `Udpipe::english(same_dir)` calls both succeed; final hash matches.
  - 1GB file returns `InputTooLarge`, not OOM.
- [ ] `cargo test --test integration -- --ignored` green with UDPipe model. Wall time recorded against PR0 N₀.
- [ ] `cargo publish --dry-run` succeeds with no warnings.
- [ ] `maturin build --release` produces a wheel; `pip install <wheel>` plus `python -c "import matra"` succeeds in a clean venv.
- [ ] Chesterton matrix v2 (Fence 7): zero contradictions vs post-restructure surface.
- [ ] `CHANGELOG.md` documents every public-API delta vs the pre-recovery state.

Noise floor: `cargo test` wall time vs PR0 baseline within ±2σ of the 5-run median.

**Rollback trigger.** Any item false → do not run `cargo publish` or `maturin upload`. Stop at `--dry-run`.

**Publish authorization.** One explicit approval per publish event. The user authorizes; the plan does not. (Memory rule, non-negotiable.)

### The post-ship loop closure

Within 2 weeks of 0.1.0 publish, write `scratch/post-ship-0.1.0.md` capturing:

- crates.io download count.
- GitHub issues with labels `panic`, `crash`, `oom`, `hang`.
- Any consumer-side issue that traces back to matra.
- Whether any of the deferred-reactor triggers fired (file-change push, >100k corpora, push-source request).

The file is not optional. Without it, "we shipped resilience" is just a claim. With it, we know whether the iteration plan held.
