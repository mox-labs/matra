# I1 — Karman pipeline rename

**Status:** not-started
**Depends on:** I0 (baseline committed)
**Branch:** `i1/karman-rename` off the I0 commit

## Why this iteration exists

Names become public traits, classes, and proto schemas across three languages once 0.1.0 ships. Karman's verdict (2026-04-28) settled the four open names. K's verdict (same day) said: rename **first**, before structure moves. Renaming a stable surface is mechanical and reversible; renaming a freshly restructured surface is two changes interleaved on the same file.

This iteration is **mechanical**. No logic changes. Identifier substitutions, doctest updates, README touch-up. Reviewable in one pass.

## What lands

### Task A: rename the pipeline verbs

**Files:** `src/source/mod.rs`, `src/decompose/mod.rs`, `src/nlp/mod.rs`, `src/lib.rs`, `python/vaani/__init__.py`, `python/vaani/cli.py`, `tests/integration.rs`, `examples/basic.rs`, `README.md`.

**Why (Karman, 2026-04-28):** "Final pipeline: `ingest → decompose → parse → measure` (peer: `extract`). Module names follow."

**Steps:**

1. Trait renames:
   - `Source` (in `src/source/mod.rs`) → keep name. Karman: "`Source` already is the ingest port semantically." Rename method `read` → `ingest` if the Burner verdict (2026-04-28) prefers the verb match. Cross-check Burner's note: "rename trait verbs only if at all." Decision: keep trait `Source`; keep method `read`; the **stage name** is `ingest`, manifested in composition root function names, not in the trait method.
   - `Decomposer` → keep. Method `decompose` already matches.
   - `NlpProvider` → keep. Method `parse` already matches.
2. Stage-verb function renames in `src/lib.rs`:
   - Confirm `analyze`, `analyze_markdown`, `analyze_file`, `analyze_directory`, `parse`, `analyze_from` keep their convenience names (Ace verdict: "two-layer surface — convenience tier and power tier").
   - The composition-root *internal* helper `run_analysis` may be renamed `run_pipeline` or kept; reviewer-discretion. No external impact.
3. Module rename `decompose::markdown::parse` → `decompose::markdown::decompose` (Ace verdict: "Rename to free `parse` for NLP only"):
   - In `src/decompose/markdown.rs`, rename the public function.
   - Update call sites in `src/lib.rs` (currently lines 26 and 115 per the architecture doc).
4. Doctest updates:
   - `src/lib.rs:96-104` — verify the `parse` doctest still references `nlp.parse` correctly.
   - `src/lib.rs:112-120` — update to `decompose::markdown::decompose` if the example uses it.
5. README:
   - Update the pipeline description to use the new vocabulary: "ingest → decompose → parse → measure (+ peer extract)".
   - Update the Quick Start example if any verbs changed.

**Acceptance:**
- `rg 'markdown::parse\b' src/ tests/ examples/ python/ README.md` returns zero hits.
- `rg '\bencode\b' src/ tests/ examples/` returns zero hits as a verb (the word may appear as "encoders" historical comment; remove if so).
- All 56+ unit tests pass.
- `cargo test --doc` passes.

### Task B: confirm the stage names appear correctly in user-facing surfaces

**Files:** `README.md`, `python/vaani/__init__.py`, `python/vaani/cli.py`, `CHANGELOG.md`.

**Why:** the pipeline through-line "ingest → decompose → parse → measure (+ extract)" is the explanation users see first. If it does not appear in README, the rename's value is invisible.

**Steps:**

1. README: add or update the pipeline diagram and verb list. Cross-link to `.claude/arch/architecture.md` if appropriate (the README itself is the user-facing doc; arch is internal).
2. CHANGELOG: under `## [Unreleased]`, add:
   ```
   ### Changed

   - Pipeline vocabulary settled at `ingest → decompose → parse → measure` with `extract` as a peer stage. The `Source`, `Decomposer`, and `NlpProvider` traits keep their names; the stage verbs are reflected in the composition-root function names and documentation.
   - `decompose::markdown::parse` → `decompose::markdown::decompose` to free the verb `parse` for NLP-only use.
   ```
3. Python: confirm the public Python surface (`Vaani.analyze`, `.analyze_markdown`, `.tfidf_summarize`, etc.) does not need renames. The Python verbs match the convenience-tier names (Ace).

**Acceptance:** README and CHANGELOG reflect the rename. Python surface unchanged.

### Task C: cargo public-api diff

**Why (Ixian):** "Public API surface diff (`cargo public-api`) shows **only** identifier renames — no added or removed types."

**Steps:**

1. Install `cargo-public-api` if absent.
2. Capture the public API at HEAD of I0 (the baseline). Save to `.claude/implans/public-api-i0.txt`.
3. Capture the public API after I1 lands. Save to `.claude/implans/public-api-i1.txt`.
4. Diff. Confirm only renames appear. If any type is added or removed, that's scope creep — kick it out of I1.

**Acceptance:** the diff between I0 and I1 public APIs contains only rename hunks. No added or removed types, traits, or functions.

## Validation

- `cargo test --features udpipe`: count `≥ N₀`.
- `cargo test --no-default-features`: passes.
- `cargo test --doc`: passes.
- `cargo check --features python`: passes.
- `cargo doc --no-deps`: builds with zero broken intra-doc links.
- `bash scripts/check-boundaries.sh`: exits 0.
- `cargo public-api` diff matches the rename-only expectation (Task C).

## Acceptance gate

After I1 lands:
- The public API diff vs I0 is purely renames.
- All cross-iteration regression matrix items 1–9 pass.
- README pipeline description uses the new verbs.

## Risks

- **Risk:** a rename touches a file that also has logic changes from I0, producing an unreviewable PR.
  - **Mitigation:** I0 committed first. I1 is on a fresh branch off I0. No interleaved logic changes.

- **Risk:** the doctest at `src/lib.rs:115` references `decompose::markdown::parse` and silently fails after the rename.
  - **Mitigation:** `cargo test --doc` is part of the regression matrix.

- **Risk:** Python `__init__.py` re-exports a renamed PyO3 symbol without an alias, breaking existing imports.
  - **Mitigation:** the Python surface (`Vaani.analyze` etc.) does not change. Verify before merging.

- **Consult:** Karman if any name feels off during the rename. The verdict is settled; new ambiguities are bugs in the implan, not in the verdict.
