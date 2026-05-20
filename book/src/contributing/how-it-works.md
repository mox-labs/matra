# How this repo is run

vaani is a public OSS package and an intended exemplar for both Claude-managed open-source repositories and human–AI collaborative intelligence. The standards exist to make the project sustainable; they apply to every contributor, human or AI.

## Working values

- **Transparency** — decisions are visible. ADRs in `docs/decisions/`, iteration plans in `.claude/implans/`, the audit trail in CHANGELOG.md.
- **Auditability** — every change has a trail. Conventional commits, Co-Authored-By trailers, signed-off review.
- **Reversibility** — every change can be backed out cleanly. Atomic commits, ADR-supersede rather than ADR-edit, branches deleted after merge.

## The working model

vaani is developed in iterations. Each iteration addresses one structural concern (resilience, observability, streaming, etc.). The full plan is sequenced in `.claude/implans/` and the architecture it builds toward is documented in `.claude/arch/`. Anyone can read both before opening an issue or PR.

An iteration is implemented as a sequence of atomic commits on a short-lived branch, opened as a single PR against `main`, reviewed, then merged. Every commit on the branch is its own logical unit (a sub-task) so the history reads as a series of small, auditable steps. After merge, the branch is deleted.

The project's primary engineer is Claude (Anthropic's AI), working with human direction and review. Every commit carries a `Co-Authored-By` trailer identifying the model used. Humans review and approve every PR before merge; nothing lands without a human OK.

## Where things live

| Location | What lives there |
|---|---|
| `.claude/arch/` | Architecture docs: ports, adapters, domain model, evolution. Internal substrate. |
| `.claude/agents/` | The DAO — practitioner agents (maintainer, reviewer, portsmith, ffi-keeper, resilience, archivist). |
| `.claude/skills/` | Skill library — ACES, rust-craft, testing, architecture, ffi-surface, resilience-floor, docs-lockstep. |
| `.claude/implans/` | Iteration plans: I0, I1, I2, etc. |
| `book/src/` | User-facing documentation. **You are here.** |
| `docs/decisions/` | Architecture Decision Records (ADRs). |
| `CHANGELOG.md` | What shipped per release, with prose Highlights for load-bearing changes. |
| `CLAUDE.md` | Working rules for AI collaborators: boundary rules, conventions. |
| `scripts/` | Versioned tooling: pre-commit hook, boundary check, changelog rollover. |
| `justfile` | Single source of truth for repeatable workflows (CI, hook, humans all run the same commands). |

If something is unclear or contradictory across these surfaces, the order of authority is: **code > tests > `.claude/arch/` > `.claude/implans/` > ADRs > CHANGELOG > Issues > Discussions**. Closer to the running system wins.

## How decisions are made

Three surfaces depending on stakes:

- **Open-ended exploration** → GitHub Discussions. RFCs, "should we consider X", retrospectives. No commitment.
- **Architectural decisions that bind future work** → a `decision` issue, then an ADR in `docs/decisions/NNNN-short-name.md`.
- **Concrete changes** → a regular issue plus a PR that closes it.

ADRs are appended, not edited. Superseding an ADR is itself a new ADR.

## How releases work

**Trigger:** release when something user-visible justifies it (architecture change, breaking change, security fix, new feature). Not on a calendar.

**Cadence:** pre-1.0, releases happen at iteration boundaries. Post-1.0, [semver](https://semver.org/) binds.

**Process:**

1. Maintainer runs `just release-prep VERSION`. This rolls `[Unreleased]` → `[VERSION]` in CHANGELOG.md.
2. Maintainer reviews the diff, ensures the `[VERSION]` section has 2-4 Highlight paragraphs plus structured Keep-a-Changelog bullets.
3. Bumps `Cargo.toml` and `pyproject.toml` versions, commits.
4. `cargo publish --dry-run --features udpipe` for sanity check.
5. **Manual approval gate.** Publishing is a deliberate, per-call action. `cargo publish` and `git push --follow-tags` only after explicit approval.

The manual gate is by policy, not because automation is hard. Publishing is irreversible (yanking leaves a tombstone) and visible to every downstream consumer; it deserves an explicit human moment.

## How to contribute

### File an issue

- Bug: `.github/ISSUE_TEMPLATE/bug_report.md` (auto-applied).
- Feature: `.github/ISSUE_TEMPLATE/feature_request.md`. Ask whether the feature belongs in vaani's substrate role or in a downstream consumer.
- Architectural decision: `.github/ISSUE_TEMPLATE/decision_record.md`.

### Open a PR

1. Fork; create a branch named after the work (`i3/error-tracing`, `fix/symlink-rejection`, `docs/clarify-tree-walk`).
2. Run `just install-hooks` once on a fresh clone. The pre-commit hook runs the same gates CI runs.
3. Make atomic commits. One logical change per commit. [Conventional Commit](https://www.conventionalcommits.org/) prefix: `feat / fix / docs / chore / refactor / perf / test / ci / build`.
4. Update `CHANGELOG.md` `[Unreleased]` with a terse bullet. If architectural / breaking / security-relevant, add a Highlight paragraph.
5. Open the PR. The PR template asks for Summary, Why, Test plan.
6. CI runs the same gates the hook ran.
7. A human reviewer approves before merge.

### What "good" looks like in a commit

The commit message is a teaching moment for the next reader, not just release-note material. State the *why*, the alternatives considered, and the trade-off you accepted. Long bodies are welcome when the change is load-bearing; short subjects are mandatory.

## ACES + antifragility — non-negotiable

Every structural change is checked against the ACES boundary test (see `.claude/skills/aces/SKILL.md`) and the antifragility checklist (see `.claude/skills/resilience-floor/SKILL.md`). Good engineering that violates ACES is not good for vaani.

## Working with Claude

When Claude opens a PR, every commit has a `Co-Authored-By: Claude ...` trailer, the PR body shows what Claude did and why, and a human (the maintainer) approves before merge.

When you (a human) open a PR with Claude's help, the trailer is welcome. It is a fact, not a stigma. Attribution is part of the auditability discipline.

If you want to work on vaani with Claude Code on your machine, the `.claude/` directory in this repo is preloaded with the architecture docs, agent roster, and skills. Claude Code will read them automatically.

## Reporting security vulnerabilities

Do **not** file a public issue for security problems. See `SECURITY.md` in the repo root for the disclosure process.
