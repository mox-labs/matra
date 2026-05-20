---
description: Multi-agent review of the current PR or branch state
allowed-tools: Read, Glob, Grep, Bash(git diff:*), Bash(git status:*), Bash(git log:*), Bash(gh pr view:*), Bash(gh pr diff:*)
---

Convene the relevant practitioner agents to review the current change. Fan out
in parallel; collect only noteworthy feedback; then filter again before surfacing
the report.

## Who to invoke

Always:

- **reviewer** — the gate. Runs the boundary-compliance audit (rules 1-7),
  public-surface integrity check, error-tier discipline, resilience-floor
  checklist, cost discipline (no silent O(n²)), docs lockstep, test coverage.
  ACES is Gate 0 — does the change make the system more adaptable / composable
  / extensible, or less?

Conditionally:

- **portsmith** — if the diff touches `src/source/`, `src/decompose/`, `src/nlp/`,
  or any port trait. Audits the port contract, pre/post-conditions, object-safety,
  Pattern 6 criterion.
- **ffi-keeper** — if the diff touches `src/lib.rs` (PyO3 layer), `pyproject.toml`,
  `python/vaani/`, the `pyo3` / `pythonize` / `maturin` deps. Audits the dual-publish
  contract, error routing per variant, 4 pythonize blind spots, the 3-axis pin rule.
- **resilience** — if the diff adds I/O, external library calls, user-input handling,
  file writes, hash verification, or graph-walk algorithms. Audits the six
  antifragility disciplines (entry-point size cap, symlink rejection, atomic file
  write, TOCTOU closure, catch_unwind boundary, cycle-safety).
- **archivist** — if the diff touches the public surface or the boundary rules.
  Audits CHANGELOG.md `[Unreleased]`, the relevant ADR, the relevant arch doc.

## What to filter for

Each invoked agent provides **only noteworthy feedback** — concerns that would
block merge, suggest a structural change, or surface a non-obvious tradeoff.
Skip nits already caught by clippy / fmt / boundary script.

After the agents report, surface only the feedback **you also deem noteworthy**.
Drop:

- Duplicate findings across agents (the boundary violation that reviewer + portsmith
  both caught — say it once).
- Style preferences not encoded in `cargo fmt` or clippy.
- "I'd do it differently" without a structural argument behind it.

Keep:

- Anything that blocks merge under the boundary rules or ACES.
- Anything that would have been caught by the corresponding rust-mastery Frame.
- Anything where the diff drifts from a stated invariant (`#[non_exhaustive]`,
  domain purity, single UDPipe importer).

## Report format

Group findings by severity:

1. **Block merge** — boundary violation, missing regression test on a fix, new
   public surface without rustdoc, new error variant unrouted at the PyO3
   boundary, aspirational claim in shipping docs.
2. **Suggest** — non-blocking but worth addressing now (an ADR that should have
   been written, a CHANGELOG entry missing, a pythonize blind spot the new type
   needs to internalize).
3. **Nit** — small clarifications the author may take or leave.

For each finding: file:line, the specific concern, the corpus Frame or ADR it
grounds in. No paraphrase; quote the offending code if it helps.

End with a **ship / return** verdict per the reviewer agent's standard.
Ship if all blockers clear. Return with the specific changes required if not.

If no findings: say so explicitly. Do not invent issues.
