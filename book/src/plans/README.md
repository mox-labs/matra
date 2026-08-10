# Plans

The [roadmap](../roadmap.md) says what matra does not do yet and what condition would change that. A plan exists only once that condition has fired, and it says how the work gets done and how anyone knows it is finished. So the roadmap is the register and the plans are the execution; neither restates the other.

Each iteration has one plan. The plans are agent-legible: every task names the file, the line range, the reviewer who asked for it, the acceptance predicate, and the validation test.

## Iterations and their boundaries

| Plan | Boundary | Title |
|---|---|---|
| the i0 stabilization work | none | Commit the post-recovery baseline; capture N₀ and noise floor |
| the i1 rename work | none | Karman pipeline rename |
| the i2 resilience work | resilience floor | Ten antifragile fixes |
| [i3-error-tracing.md](i3-error-tracing.md) | **MVP** | Error restructure + tracing PR1 + cdylib feature-gating |
| the retracted workspace plan | structural | Workspace conversion + `rumi-nlp` skeleton |
| [i5-streaming.md](i5-streaming.md) | **MLP** | Streaming iterator + Engine + CorpusResult |
| [i6-post-publish.md](i6-post-publish.md) | post-publish | OTel feature, PDF/DOCX, `rumi-nlp` patterns, deferred reactor |
| [i7-structural-primitives.md](i7-structural-primitives.md) | rule-substrate | Negation, typed feats, modality, evidentiality, Hearst patterns |

**Strict ordering.** No iteration starts until the previous one has met its acceptance gate. K's strategic verdict on this is non-negotiable: rename a stable surface before structure moves; install the resilience floor before the error contract; ship the error contract before the streaming surface that consumes it.

## How to read an plan

Each plan has the same skeleton:

- **Why this iteration exists**. the ground-truth conviction, with reviewer attribution.
- **What lands**. task list. Each task has:
  - File(s) and line range.
  - Why (reviewer quote, with file or session attribution).
  - Steps.
  - Acceptance (an observable predicate that says the task is done).
- **Validation**. Ixian's tests. Falsification scenarios. Tie-breaker experiments where there were disputes.
- **Acceptance gate**. the single predicate that says the iteration is done. If this is false, the plan is not done.
- **Risks**. what could go wrong, who to consult.

If a task is ambiguous in the plan, the plan is the bug. Edit the plan first, then the code.

## The cross-iteration regression matrix

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

## The 0.1.0 ship predicate

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

## The post-ship loop closure

Within 2 weeks of 0.1.0 publish, write `scratch/post-ship-0.1.0.md` capturing:

- crates.io download count.
- GitHub issues with labels `panic`, `crash`, `oom`, `hang`.
- Any consumer-side issue that traces back to matra.
- Whether any of the deferred-reactor triggers fired (file-change push, >100k corpora, push-source request).

The file is not optional. Without it, "we shipped resilience" is just a claim. With it, we know whether the iteration plan held.
