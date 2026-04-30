# I0 — Stabilize the post-recovery baseline

**Status:** not-started
**Depends on:** none
**Branch:** `restore/jsonl-recovery-2026-04-09` (continue on existing branch)

## Why this iteration exists

The working tree on `restore/jsonl-recovery-2026-04-09` carries 12 modified files plus untracked recovery artifacts (jsonl mining, scratch notes, `scripts/`, `src/metrics/`). Until that lands as a commit, every subsequent iteration is built on a foundation that mixes "what was already in flight" with "what we decided after twelve guild verdicts." That is the false-foundation failure mode.

K's strategic call (2026-04-28): "**One move in 24h: commit the dirty tree.** Not the rename, not the resilience fixes — the recovery baseline. Every other workstream is built on top of an unstable foundation until that lands."

This iteration is also the **measurement baseline**. Before any change that affects performance (notably the parse-per-paragraph rewrite in I2), capture the test count and wall-time noise floor so signal can be told apart from infrastructure variance later.

## What lands

### Task A: commit the modified files

**Files:** all 12 listed in `git status` plus the untracked working artifacts that belong in source control.

**Why (K, 2026-04-28):** "Folding 12 modified files into a hardening PR mixes 'what was already in flight' with 'what we decided after four guild verdicts' — that's the false-foundation failure mode."

**Steps:**

1. Run `git status` and `git diff --stat HEAD`. Confirm the 12 modified file list matches the session-start snapshot:
   - `CLAUDE.md`, `Cargo.lock`, `Cargo.toml`, `README.md`
   - `src/domain.rs`, `src/extraction/mod.rs`, `src/extraction/textrank.rs`, `src/lib.rs`
   - `src/nlp/udpipe.rs`, `src/source/directory.rs`
   - Plus deletions: `src/encoders.rs`, `src/markdown.rs`
2. Add the deliberate untracked content: `.github/`, `CHANGELOG.md`, `scripts/`, `src/metrics/` (the post-recovery decomposition of `encoders.rs`).
3. **Do not** add: the `scratch/agent-*.jsonl` raw mining transcripts, the `scratch/4492d686-*.jsonl` file. These are recovery artifacts; they should remain untracked or be added to `.gitignore`. Confirm with the user before deciding their fate.
4. Commit message:
   ```
   restore: post-recovery baseline

   Lands the post-recovery v2 implementation on top of the jsonl
   mining recovery. metrics/ replaces the deleted encoders.rs;
   decompose/markdown.rs replaces the deleted markdown.rs;
   extraction/ adds textrank and yake; source/directory.rs gains
   symlink skip; nlp/udpipe.rs gains the SHA-256 verify path.

   Working tree was carried across the rm -rf incident on 2026-04-09;
   this commit is the boundary between "recovered as-was" and
   "everything we ship next."
   ```

**Acceptance:** `git status` is clean (no modified, no untracked except the two scratch families). `git log --oneline` shows exactly one new commit on top of `d51d142`.

### Task B: capture the regression baseline N₀

**Why (Ixian, 2026-04-28):** "Run `cargo test` 5x on a quiet machine; record min/median/max wall time. This is the CI noise floor for the perf-relevant changes in PR2j (parse-per-paragraph). Without N₀ timing we cannot tell signal from infrastructure variance — that is the 6pp illusion."

**Steps:**

1. Run `cargo test --features udpipe` 5 times consecutively. Record wall time for each run.
2. Run `cargo test --no-default-features` once. Record wall time. Confirm zero failures.
3. Record the **test count** at HEAD as `N₀`. Today this is 56; verify by reading the `cargo test` output `test result: ok. <N> passed`.
4. Write the baseline to `.claude/implans/baselines.md` (new file). Format:
   ```
   ## I0 baseline — captured <date> on <hostname/CPU>

   N₀ (with udpipe): 56 tests
   N₀ (no-default-features): <count>

   cargo test --features udpipe wall times (5 runs):
     min: <s>, median: <s>, max: <s>
     2σ noise floor: ±<s>

   cargo test --no-default-features wall time:
     <s>
   ```

**Acceptance:** `.claude/implans/baselines.md` exists and contains the captured numbers. Future iterations check perf deltas against this file.

### Task C: install the boundary check script

**Files:** `scripts/check-boundaries.sh` (new).

**Why (Ixian + Burner):** boundary rules 3, 4, and 8 must be machine-verifiable so a future iteration cannot silently violate them.

**Steps:**

1. Create `scripts/check-boundaries.sh` with executable permission. Content:
   ```sh
   #!/usr/bin/env bash
   # Verifies CLAUDE.md boundary rules. Runs in CI.
   set -euo pipefail

   fail=0

   # Rule 4: only nlp/udpipe.rs imports udpipe_rs.
   if rg -q 'use udpipe_rs' src/ --glob '!src/nlp/udpipe.rs'; then
       echo "FAIL: udpipe_rs imported outside src/nlp/udpipe.rs (rule 4)"
       rg 'use udpipe_rs' src/ --glob '!src/nlp/udpipe.rs'
       fail=1
   fi

   # Rule 8: tracing forbidden in domain.rs and port modules.
   if rg -q '^use tracing|tracing::' src/domain.rs src/source/mod.rs src/decompose/mod.rs src/nlp/mod.rs 2>/dev/null; then
       echo "FAIL: tracing imported in domain.rs or a port module (rule 8)"
       rg '^use tracing|tracing::' src/domain.rs src/source/mod.rs src/decompose/mod.rs src/nlp/mod.rs
       fail=1
   fi

   # Rule 3: ports do not import each other.
   if rg -q 'use crate::source|use crate::decompose|use crate::nlp' src/source/mod.rs src/decompose/mod.rs src/nlp/mod.rs 2>/dev/null; then
       echo "FAIL: cross-port import detected (rule 3)"
       fail=1
   fi

   if [ $fail -eq 0 ]; then
       echo "boundary checks pass"
   fi
   exit $fail
   ```
2. Run the script locally. Confirm exit 0 at HEAD.

**Acceptance:** `bash scripts/check-boundaries.sh` exits 0.

## Validation

- `git status` clean post-commit.
- `cargo check`, `cargo check --no-default-features`, `cargo test --features udpipe` all green.
- `N₀` recorded in `.claude/implans/baselines.md`.
- `bash scripts/check-boundaries.sh` exits 0.
- The 14 `scratch/agent-*.jsonl` files remain untracked (or are added to `.gitignore`, per user direction).

## Acceptance gate

`git log -1 --pretty=%s` returns "restore: post-recovery baseline". `cargo test --features udpipe` count equals N₀ recorded in `baselines.md`. Boundary script exits 0.

## Risks

- **Risk:** committing one of the `scratch/agent-*.jsonl` raw transcripts by accident. Recovery artifacts should not enter source control.
  - **Mitigation:** explicit `git add` per file; do not use `git add .`.
  - **Consult:** if unsure about an artifact, ask the user.

- **Risk:** N₀ wall times captured on a busy machine drift. Future iterations will measure perf against noise.
  - **Mitigation:** capture on a quiet machine. Re-capture if the host changes.
