# What enforces matra's rules, and what only appears to

- **Date:** 2026-09-22
- **Method:** two read-only audits against `main` at `3ae125f`. The first
  collected every "always / never / must" statement from `CLAUDE.md`,
  `CONTRIBUTING.md`, `AGENTS.md`, the ADRs, the skills and the agent files, then
  traced each to whatever executes it. The second inventoried the static
  analysis and automation by layer and assessed candidate additions. Findings
  marked reproduced were re-run by hand before this file was written.

This is evidence, not a plan. The decision it supports is ADR-0017.

## The headline: a check that passes having examined nothing

`scripts/check-boundaries.sh` prints `boundary checks pass (rules 3, 4, 8)` and
exits 0 when `rg` is absent. Reproduced:

```console
$ PATH=/usr/bin:/bin bash scripts/check-boundaries.sh
boundary checks pass (rules 3, 4, 8)
exit=0
```

Every check in it is `hits=$(rg ... 2>/dev/null || true)`. The `|| true` absorbs
rg's exit 1 on no matches, which is what it was written for, and absorbs exit
127 on a missing binary identically. The same shape means a renamed port file
turns its rule into a no-op that still reports success.

Gate 3 of `scripts/check-docsite-floor.sh` has the same property for the same
reason.

## Three claims in the repository that are not true

1. `scripts/check-docsite-floor.sh:2` says "Runs in CI". It runs in no
   workflow. CI references three scripts and this is not among them, so the
   link check, the orphan check, the type-name gate, the em-dash gate and the
   `llms.txt` currency gate execute only on a contributor's machine.
2. `justfile:85` says CI installs lychee and sets `LYCHEE_REQUIRED=1`. That
   variable appears nowhere else in the repository. Gate 1 has never run
   anywhere except where lychee happens to be installed, where its absence
   prints `SKIP` and the suite still exits 0.
3. `.git/hooks/pre-commit:2` identifies itself as the hook of a project this
   repository was renamed from, and claims that passing locally means CI will
   pass. That is false in both directions (see the drift table below). The
   corrected text exists at `scripts/pre-commit-hook.sh:4-7`; the installed
   copy predates it, and `scripts/install-hooks.sh` copies without any drift
   check.

## Rules with no mechanical check, ranked by cost

1. **Boundary rules 1, 2, 5 and 7** (domain purity, port imports, metrics
   purity, composition root). `CLAUDE.md:68` states this accurately. A
   non-optional dependency used in `domain.rs` compiles clean under every CI
   configuration including `--no-default-features`.
2. **`#[non_exhaustive]` on public types.** Caught only downstream, by
   `cargo-semver-checks`, and only on a pull request.
3. **No panics in library code.** `Cargo.toml:96-100` lints `dbg_macro`,
   `todo` and `unimplemented`. `unwrap_used`, `expect_used` and `panic` are not
   configured, and no module carries `#![deny]`. One `.unwrap()` in an adapter
   aborts the host process for a Python caller.
4. **The resilience floor on new code.** The existing size caps, atomic writes
   and TOCTOU closures are tested. That new I/O inherits them is prose.
5. **Methods do not cross FFI.** A surface that looks complete in Rust and is
   empty in Python ships silently.
6. **Conventional commits, changelog-in-PR, and human approval before merge.**
   All three are stated as requirements. Branch protection carries no
   `required_pull_request_reviews` key at all.

## Checks that exist but run nowhere automatic

`check-boundaries.sh`, `check-docsite-floor.sh`, `test-e2e-sandbox.sh` and
`ruff` reach CI through no workflow. `check-version-sync.sh` runs at release
time only, not on pull requests. Each is reachable through `just check` or the
opt-in hook, so its enforcement depends on a contributor choosing to run it.

## Drift between the local suite and CI

Neither is a superset of the other.

| Runs in `just check` only | Runs in CI only |
|---|---|
| boundary check | `cargo-deny` |
| docsite floor (six gates) | `cargo-semver-checks` |
| e2e sandbox suite | wasm32 target check |
| version sync (on PRs) | the model2vec lane |
| `ruff` | wheel build and abi3 tag assertion |
| | `mypy`, CodeQL, macOS |

`justfile:14-17` and `scripts/pre-commit-hook.sh:4-7` both describe this
honestly. The installed hook does not.

## Checks that run but cannot fail on a finding

`Analyze (rust)` and `Analyze (python)` are required checks. CodeQL uploads
results to the Security tab; the job is green with alerts open. The required
check proves the scan ran, not that it was clean. `scorecard.yml` has the same
shape and is not required at all.

The em-dash gate exempts any line containing a double quote, so eight in-scope
lines currently sit behind that exemption.

## Lanes that can go red without blocking a merge

Four of eight Rust matrix cells and the wasm32 check are absent from the
required list, including `cli`, which is the only lane that compiles
`src/cli/` and runs `tests/skill.rs` and `tests/cli.rs`, and `model2vec`,
which carries the cross-architecture bit-identity assertion.

## Defect kinds owned by no layer

Boundary rules 1, 2, 5, 7; secret scanning; test quality and coverage
(`cargo-llvm-cov` exists as a recipe, runs nowhere, has no threshold); workflow
static analysis beyond Scorecard; shell linting; spelling. Python runtime
behaviour is owned by `just` alone: the six files under `python/tests/` never
run in CI.

## Tooling assessment

Run during the audit, not inferred:

- **zizmor**: 2 high, 1 medium. `docs.yml:10-11` declares `pages: write` and
  `id-token: write` at workflow level, so its build job, which downloads a
  tarball and runs `cargo doc`, carries deploy-capable tokens it does not need.
  This is the class ADR-0005 section 2 states it closed.
- **actionlint**: clean. Adding it would be a no-op today, and a guard going
  forward.
- **shellcheck**: seven findings, all style. Noise today, a guard over 1,895
  lines of shell going forward.

On candidates:

- **semgrep** supports Rust but has no cross-file analysis. That does not bite
  here, since these boundary rules are per-file import constraints, and it
  would beat the current regex on AST matching. It cannot see rule 1, which is
  a manifest fact rather than an import fact.
- **A `syn`-based check compiled as a test** fits better and has precedent in
  this repository: `tests/error_tables.rs` already parses source and pins doc
  tables to real match arms. It needs no new binary and runs inside a lane that
  is already required, which the shell script does not.
- **`cargo-public-api`** complements `cargo-semver-checks`, which by its own
  documentation does not catch every break and does not enumerate additions. A
  checked-in snapshot makes an accidental `pub` visible at review.
- **`cargo audit`** duplicates the advisory half of `cargo deny` and would
  split the ignore list. Not worth adding.
- **rust-analyzer** in a non-interactive lane duplicates clippy under
  `-D warnings`. No value here.

## What this evidence supports

The gap is not a shortage of checks. It is that several checks cannot
distinguish a clean result from an absent one, several run somewhere other than
where the repository says they run, and the rules with the highest cost of
violation are precisely the ones with no mechanical check. Adding tooling on
top of that foundation would produce more checks with the same defect.
