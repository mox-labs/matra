# Contributing to vaani

vaani is a Claude-managed open-source project. This document explains how
the repository is run so that anyone (human contributor, AI collaborator,
or curious onlooker) can see how decisions are made, where plans live,
and how to participate.

For the deeper exposition of *how* humans and AI work together on vaani
(roles, discipline, the discourse-to-docs-to-code chain, the
two-state model), see [docs/collaboration-model.md](./docs/collaboration-model.md).

The working values are **transparency** (decisions are visible),
**auditability** (every change has a trail), and **reversibility** (every
change can be backed out cleanly).

---

## The working model

vaani is developed in **iterations**. Each iteration addresses one
structural concern: resilience, observability, streaming, etc. The full
plan is sequenced in `.claude/implans/` and the architecture it builds
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
| `.claude/implans/` | Iteration plans: I0, I1, I2, ... Each implan describes the goals, tasks, validation, and acceptance gate for one iteration. |
| `docs/decisions/` | Architecture Decision Records (ADRs). One file per significant call. |
| `CHANGELOG.md` | What shipped per release, with prose Highlights for load-bearing changes. |
| `CLAUDE.md` | Working rules for AI collaborators: pipeline shape, boundary rules, conventions. |
| `scripts/` | Versioned tooling: pre-commit hook, boundary check, changelog rollover, etc. |
| `justfile` | Single source of truth for repeatable workflows. |
| GitHub Issues | Tracking: bugs, features, decisions. Labels (`type:` / `status:` / `area:`) classify. |
| GitHub Discussions | Open-ended design space: RFCs, retrospectives, ideas, Q&A. |

If something is unclear or contradictory across these surfaces, the order
of authority is: code > tests > `.claude/arch/` > `.claude/implans/` >
ADRs > CHANGELOG > Issues > Discussions. Closer to the running system
wins.

---

## How decisions are made

Decisions go through three surfaces depending on stakes.

**Open-ended exploration** -> GitHub Discussions. RFCs, "should we
consider X", retrospectives. No commitment, no labels.

**Architectural decisions that will bind future work** -> a `decision`
issue (`.github/ISSUE_TEMPLATE/decision_record.md`). The issue captures
context, options, tradeoffs, and the recommendation. After deliberation,
the chosen option lands in `docs/decisions/NNNN-short-name.md` (an ADR)
and the issue closes pointing at the ADR.

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
3. Maintainer bumps `Cargo.toml` version, commits.
4. `cargo publish --dry-run --features udpipe` for sanity check.
5. **Manual approval gate.** Publishing is a deliberate, per-call action.
   When ready: `cargo publish` and `git push --follow-tags`.

The deliberate manual gate is by policy, not because automation is hard.
Publishing is irreversible (yanking leaves a tombstone) and visible to
every downstream consumer; it deserves an explicit human moment.

---

## How to contribute

### File an issue

- Bug: `.github/ISSUE_TEMPLATE/bug_report.md` (auto-applied).
- Feature: `.github/ISSUE_TEMPLATE/feature_request.md`. Ask whether the
  feature belongs in vaani's substrate role or in a downstream consumer.
- Architectural decision: `.github/ISSUE_TEMPLATE/decision_record.md`.

### Open a discussion

For anything open-ended, use Discussions instead of Issues. We organize
discussions into categories (configured in the GitHub UI):

- **Announcements**: release notes, project status.
- **Ideas**: half-formed thoughts, "what if" questions.
- **RFCs**: design proposals you want feedback on before opening an issue.
- **Q&A**: usage questions.
- **Show and tell**: things you built with vaani.

### Open a PR

1. Fork; create a branch named after the work (e.g. `i3/error-tracing`,
   `fix/symlink-rejection`, `docs/clarify-tree-walk`).
2. Run `just install-hooks` once on a fresh clone. The pre-commit hook
   runs the same gates CI runs; passing locally means passing CI.
3. Make atomic commits. One logical change per commit. Conventional
   prefix: `feat / fix / docs / chore / refactor / perf / test / ci /
   build`. Optional scope in parens: `feat(extraction): ...`.
4. Update `CHANGELOG.md` `[Unreleased]` with a terse bullet. If the
   change is architectural / breaking / security-relevant, add a
   Highlight paragraph too.
5. Open the PR. The PR template asks for Summary, Why, Test plan.
   Fill it in.
6. CI runs the same gates the hook ran. If anything fails, fix and push.
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

## Code style

**ACES + antifragility:** non-negotiable. ACES (Adaptable, Composable,
Extensible) is the structural design philosophy; antifragility is the
operational discipline (size caps, panic boundaries, atomic ops, TOCTOU
closure). See `.claude/skills/aces/SKILL.md` and
`.claude/skills/resilience-floor/SKILL.md`. Every structural change is
checked against the ACES boundary test; every new I/O or external-library
boundary is checked against the antifragile checklist.

**Boundary rules:** non-negotiable. See `CLAUDE.md` for the seven hex
rules. The boundary check script (`scripts/check-boundaries.sh`) runs in
the pre-commit hook and CI.

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

If you want to work on vaani with Claude Code on your machine, the
`.claude/` directory in this repo is preloaded with the architecture
docs and iteration plans. Claude Code will read those automatically.

---

## Reporting security vulnerabilities

Do **not** file a public issue for security problems. See
`SECURITY.md` for the disclosure process.

---

## Code of conduct

This project follows the Contributor Covenant. See `CODE_OF_CONDUCT.md`.
