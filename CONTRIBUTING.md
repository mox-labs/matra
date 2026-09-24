# Contributing to matra

matra is a Claude-managed open-source project. This document explains how
the repository is run so that anyone (human contributor, AI collaborator,
or curious onlooker) can see how decisions are made, where plans live,
and how to participate.

If you are an agent contributing to matra, [AGENTS.md](./AGENTS.md) is the
short form of this document: the build and gate commands, the boundary
rules, and the shape of a pull request.

For the deeper exposition of *how* humans and AI work together on matra
(roles, discipline, the discourse-to-docs-to-code chain, the
two-state model), see [docs/collaboration-model.md](./docs/collaboration-model.md).

The working values are **transparency** (decisions are visible),
**auditability** (every change has a trail), and **reversibility** (every
change can be backed out cleanly).

---

## The working model

matra is developed in **iterations**. Each iteration addresses one
structural concern: resilience, observability, streaming, etc. Each is
planned as an EP in `blueprints/eps/`, implementing the RFCs in
`blueprints/rfcs/` that decided its design, and the architecture it builds
toward is documented in `.claude/arch/`. Anyone can read both before
opening an issue or PR.

An iteration is implemented as a sequence of atomic commits on a short-
lived branch, opened as a single PR against `main`, reviewed, then merged.
Every commit on the branch is its own logical unit (a sub-task) so the
history reads as a series of small, auditable steps. After merge, the
branch is deleted.

The project's primary engineer is Claude (Anthropic's AI), working with
human direction and review. Every commit carries a `Co-Authored-By` trailer
identifying the model used. Humans review and approve every PR before
merge; nothing lands without a human OK.

---

## Where things live

| Location | What lives there |
|---|---|
| `.claude/arch/` | Architecture docs: ports, adapters, domain model, evolution. Read this before changing structure. |
| `blueprints/rfcs/` | RFCs: one file per design-level decision, in the Rust RFC layout. The process and the index are `blueprints/README.md`. |
| `blueprints/eps/` | Enhancement plans (EPs): how an accepted RFC gets to shipping, with milestones, test plan, ship criteria, and a status log. |
| `CHANGELOG.md` | What shipped per release, with prose Highlights for load-bearing changes. |
| `CLAUDE.md` | Working rules for AI collaborators: pipeline shape, boundary rules, conventions. |
| `scripts/` | Versioned tooling: pre-commit hook, boundary check, changelog rollover, etc. |
| `justfile` | Single source of truth for repeatable workflows. |
| GitHub Issues | Tracking: bugs, features, decisions. Labels (`type:` / `status:` / `area:`) classify. |
| GitHub Discussions | Open-ended design space: early proposals, retrospectives, ideas, Q&A. |

If something is unclear or contradictory across these surfaces, the order
of authority is: code > tests > `.claude/arch/` > EPs > RFCs >
CHANGELOG > Issues > Discussions. Closer to the running system
wins.

---

## How decisions are made

Decisions go through three surfaces depending on stakes.

**Open-ended exploration** -> GitHub Discussions. Early proposals, "should
we consider X", retrospectives. No commitment, no labels.

**Architectural decisions that will bind future work** -> an RFC. Copy
`blueprints/rfcs/0000-template.md` to the next free number and open it as a
pull request; the pull request is the proposal and the place it is
discussed, and merging it accepts it. A `decision` issue
(`.github/ISSUE_TEMPLATE/decision_record.md`) can come first when the
options need airing before anyone writes the RFC; it closes pointing at the
RFC pull request. When the accepted RFC takes more than one PR to
implement, an EP in `blueprints/eps/` plans the work. The full process is
`blueprints/README.md`.

**Concrete changes** -> a regular issue (bug or feature) plus a PR that
closes it. The PR's body explains the why; the commits explain the what.

---

## How releases work

**Trigger:** release when something user-visible justifies it. Architecture
change, breaking surface change, security-relevant fix, new feature. Not
on a calendar.

**Cadence:** pre-1.0, releases happen at iteration boundaries (typically
every few iterations). Post-1.0, semver discipline binds.

**Process:**
1. Maintainer runs `just release-prep VERSION`. This rolls
   `[Unreleased]` -> `[VERSION]` in CHANGELOG.md.
2. Maintainer reviews the diff, ensures the [VERSION] section has 2-4
   Highlight paragraphs (for the load-bearing changes) plus the
   structured Keep-a-Changelog bullets.
3. Maintainer bumps `Cargo.toml` version and every other
   version-carrying file (`just version-sync` names them), commits, and
   lands it on `main`.
4. `cargo publish --dry-run --features udpipe` for sanity check.
5. **Dispatch the release from main**, with the version typed out:
   `gh workflow run release.yml --ref main -f version=VERSION`. There is
   no tag to push; `.github/workflows/release.yml` creates the annotated
   tag itself once its checks pass, so the tag can never name a commit
   the release did not verify. A release cannot be started from a branch
   other than `main`, and a version that disagrees with `Cargo.toml`
   stops the run before anything is built.
6. **Two manual approval gates.** The run pauses at the `crates-io`
   environment and again at `pypi`. Approving each deployment in the
   Actions UI is the per-publish approval point for that registry.
   Nothing publishes from a laptop.
7. After both publishes, a smoke job installs the released version from
   PyPI and from crates.io, on Linux and macOS, under a CPython one
   minor above the abi3 floor, and runs `matra --version` and
   `matra --skill` from each install.

To retry a publish that failed after the tag was created, dispatch again
with `-f skip_tag=true`. That reuses the existing tag and builds from it
rather than from whatever `main` has become, and it is the only
supported way to re-run a release without bumping the version.

The deliberate manual gate is by policy, not because automation is hard.
Publishing is irreversible (yanking leaves a tombstone) and visible to
every downstream consumer; it deserves an explicit human moment.

**Settings the workflow cannot enforce for itself.** These live in three
web UIs, and the 0.2.0 release is what proved each one matters:

- The `crates-io` and `pypi` GitHub environments each need **required
  reviewers**. Without them the workflow publishes unattended, which is
  what `pypi` did for 0.2.0 while `crates-io` correctly waited.
- Each environment's **deployment branch policy must allow `main`**,
  since the release is dispatched from there. A branch-only policy is
  also why the tag-triggered workflow failed outright for 0.2.0 with
  `Tag "v0.2.0" is not allowed to deploy to crates-io due to environment
  protection rules`. If a tag trigger is ever restored, both
  environments need a `v*` tag policy as well.
- The **Trusted Publishing configuration on both registries names a
  workflow filename**, and that filename is now `release.yml`. It was
  `publish.yml` on crates.io and `publish-pypi.yml` on PyPI. Until both
  are updated, the OIDC exchange fails and neither registry accepts an
  upload.

---

## How to contribute

### File an issue

- Bug: `.github/ISSUE_TEMPLATE/bug_report.md` (auto-applied).
- Feature: `.github/ISSUE_TEMPLATE/feature_request.md`. Ask whether the
  feature belongs in matra itself or in a downstream caller.
- Architectural decision: `.github/ISSUE_TEMPLATE/decision_record.md`.

### Open a discussion

For anything open-ended, use Discussions instead of Issues. We organize
discussions into categories (configured in the GitHub UI):

- **Announcements**: release notes, project status.
- **Ideas**: half-formed thoughts, "what if" questions.
- **RFCs**: design proposals you want feedback on before opening an issue.
- **Q&A**: usage questions.
- **Show and tell**: things you built with matra.

### Open a PR

1. Fork; create a branch named after the work (e.g. `i3/error-tracing`,
   `fix/symlink-rejection`, `docs/clarify-tree-walk`).
2. Run `just install-hooks` once on a fresh clone. The hook runs the Rust
   gates (fmt, check, clippy, doc, test on both feature configurations)
   plus the boundary check. CI runs those too, and additionally the
   docsite floor, cargo-deny, cargo-semver-checks, the maturin wheel build
   and `mypy --strict`. A green hook is a strong signal, not a guarantee.
   Run `just check` and `just typecheck` before pushing.
3. Make atomic commits. One logical change per commit. Conventional
   prefix: `feat / fix / docs / chore / refactor / perf / test / ci /
   build`. Optional scope in parens: `feat(extraction): ...`.
4. Update `CHANGELOG.md` `[Unreleased]` with a terse bullet. If the
   change is architectural / breaking / security-relevant, add a
   Highlight paragraph too.
5. Open the PR. The PR template asks for Summary, Why, Test plan.
   Fill it in.
6. CI runs the Rust gates the hook ran, plus cargo-deny,
   cargo-semver-checks, the wheel build and mypy. If anything fails, fix
   and push.
7. A human reviewer approves before merge.

### What "good" looks like in a commit

The commit message is a teaching moment for the next reader, not just
release-note material. State the WHY, the alternatives considered, and
the trade-off you accepted. Long bodies are welcome when the change is
load-bearing; short subjects are mandatory.

```
feat(metrics): cap brotli sliding window at 256 KiB

The previous lgwin=22 (4 MiB) per paragraph was a CPU pegging vector
on adversarial input. Vector's review flagged it HIGH. lgwin=18 is the
safe ceiling for prose-as-redundancy-proxy: cross-256-KiB long-range
redundancy is more than enough signal; beyond that we measure engine
plumbing, not linguistic structure.

Per-paragraph cap matches the new window so a single paragraph never
triggers more than one window of work. Oversize paragraphs slot into
the existing `Option<f64> = None` semantics (Chesterton fence 7).
```

---

## How verification works

Three layers, each answering a different question.

**Does the library behave?** `cargo test` runs the unit tests and doctests.
`just check` runs the whole Rust gate suite plus the boundary check and the
docsite floor gates.

**Does the binary behave?** `tests/cli.rs` invokes the `matra` binary and
asserts output shape and exit codes. The tests that need a parse are
`#[ignore]` because they require the UDPipe model:

```
cargo test --features cli --test cli -- --ignored
```

**Do the crusts agree?** matra ships one Rust core behind several bindings.
They all call the same parser, so a difference between them is never a
difference of behaviour: it is a binding defect, a renamed field or a lost
value or a rounded number. `spec/tests/*.json` holds language-agnostic
fixtures that every crust runs, with one runner per language. Read
[`spec/README.md`](./spec/README.md) for the fixture format and the rule
about the model being part of the contract.

```
just conformance      # every crust against the shared spec
just coverage-all     # line coverage, Rust and Python
just lint             # clippy and ruff
```

## Code style

**ACES + antifragility:** non-negotiable. ACES (Adaptable, Composable,
Extensible) is the structural design philosophy; antifragility is the
operational discipline (size caps, panic boundaries, atomic ops, TOCTOU
closure). See `.claude/skills/aces/SKILL.md` and
`.claude/skills/resilience-floor/SKILL.md`. Every structural change is
checked against the ACES boundary test; every new I/O or external-library
boundary is checked against the antifragile checklist.

**Boundary rules:** non-negotiable. See
[`book/src/reference/boundary-rules.md`](book/src/reference/boundary-rules.md) for the
canonical eight rules and how each is enforced; `CLAUDE.md` carries the
summary. `scripts/check-boundaries.sh` greps three of them and runs from
`just check`, the optional pre-commit hook and the `Boundary check` CI job;
it sees the literal import form only. Rule 6 is verified by compiling. The
rest rests on review, so run `just check` before opening a PR.

**Formatting:** `cargo fmt`. Enforced.

**Lints:** `cargo clippy --all-targets -- -D warnings` on both feature
configurations. Warnings are errors.

**Docs:** `cargo doc --no-deps --all-features` with `RUSTDOCFLAGS=-Dwarnings`.
No broken intra-doc links. Public items are documented.

**Tests:** unit tests in `#[cfg(test)] mod tests`, integration tests in
`tests/`. Tests verify requirements, not implementation. New bugs get
regression tests.

**Prose convention:** no em dashes in documentation. (Project rule.)

**Conventional commits:** required for the subject line. The body is
free-form prose; explain the *why*.

---

## Working with Claude

When Claude opens a PR:

- Every commit has a `Co-Authored-By: Claude ...` trailer.
- The PR body shows what Claude did and why.
- The commit messages are written by Claude with human review.
- A human (the maintainer) approves before merge.

When you (a human) open a PR with Claude's help:

- The trailer is welcome. It is a fact, not a stigma. Attribution is
  part of the auditability discipline.
- The same review and CI gates apply.

If you want to work on matra with Claude Code on your machine, the
`.claude/` directory in this repo is preloaded with the architecture
docs and iteration plans. Claude Code will read those automatically.

---

## Reporting security vulnerabilities

Do **not** file a public issue for security problems. See
`SECURITY.md` for the disclosure process.

---

## Code of conduct

This project follows the Contributor Covenant. See `CODE_OF_CONDUCT.md`.
