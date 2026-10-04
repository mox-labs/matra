---
name: docs-lockstep
description: >-
  Documentation hygiene for matra: the CHANGELOG mapping from conventional commits, which record a
  change takes (proposal, API change proposal, plain pull request, plan) and the supersede protocol,
  tracking issues, the README pitch, and the rule that shipping docs describe only what ships. Use
  when a change lands and the records must follow, or when deciding whether a change needs an enhancement proposal
  at all. Not for: docsite page design and gates (site/README.md).
---

# docs-lockstep

Documentation discipline for matra. The audit trail is the only durable artifact when a stranger inherits the project; this skill codifies what stays in sync with the code.

## When to invoke

- A change has landed in `src/` and the documentation needs to follow.
- Preparing for a release.
- Deciding which record a change takes, or writing an enhancement proposal, an API change proposal or a plan.
- Reviewing whether a doc claim still holds.

## The lockstep contract

When code changes, exactly the right docs change in the same PR. The mapping:

| Code change kind | What also updates |
|---|---|
| New public function/type | rustdoc, `CHANGELOG.md [Unreleased] Added` |
| Fix a bug | regression test, `CHANGELOG.md [Unreleased] Fixed` |
| Performance change | `CHANGELOG.md [Unreleased] Changed` (with perf note) |
| Internal refactor | usually nothing (unless invariants change) |
| New module under `src/` | `site/content/architecture/design.md` (the diagram) |
| New adapter | `site/content/architecture/design.md` |
| New port | `site/content/architecture/design.md` + proposal (an architecture boundary) |
| New domain type or field | `site/content/reference/domain-types.md` |
| Boundary rule change | `site/content/architecture/design.md` + proposal |
| New feature flag | `Cargo.toml`, `site/content/architecture/design.md`, `README.md` (if user-visible), `CLAUDE.md` (if structural) |
| Dep added/removed/bumped | `Cargo.toml`, `CHANGELOG.md`; an enhancement proposal only if it moves a boundary (a new dependency in `domain.rs`) |
| Public surface change | All of the above, plus an enhancement proposal if substantial or an accepted API change proposal if minor |

When the change is non-trivial and you cannot tell which docs are affected, search `site/content/` and `.claude/` for the names you changed and ask of each hit "does this claim still hold?" `scripts/check-claude-citations.sh` catches a `path:line` citation in `.claude/` that points past its file; it cannot tell whether the line still says what the citation claims.

## CHANGELOG conventions

matra follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

- `## [Unreleased]` at the top accumulates changes until the next release.
- Section headers: Added / Changed / Fixed / Deprecated / Removed / Security.
- Each entry is a bullet, present-tense, terse.
- Group sub-changes under a `### Highlights` subheading when multiple commits together produce a single user-visible delta.

Conventional-commit mapping:

| Commit prefix | CHANGELOG section |
|---|---|
| `feat:` | Added |
| `fix:` | Fixed |
| `perf:` | Changed (with perf note) |
| `refactor:` | Changed (refactor note) |
| `docs:` | Changed (docs note, only if user-visible) |
| `test:`, `ci:`, `chore:` | Usually skipped unless user-visible |

The `scripts/changelog-release.sh` script rolls `## [Unreleased]` into a versioned section when preparing a release. Run `just release-prep VERSION` to invoke it.

## Proposal, API change proposal and plan conventions

The process is `blueprints/README.md`. It follows the Rust RFC process, grain included. Proposals and plans are numbered from 0001, from the 2026-10-04 baseline; the RFCs and EPs written before it are in `blueprints/legacy/`, unchanged, as the reasons behind the code that exists, and `RFC-NNNN` and `EP-NNNN` cite them. A legacy record is never edited.

- **Which record.** A substantial change matra's users notice (the public surface in Rust, Python, the CLI or the JSON schema; semantics or behaviour, the default model included; removing or deprecating a substantial feature; an architecture boundary) is an enhancement proposal. A minor addition to or removal from the public surface is an API change proposal. Tooling, docs, CI, the harness, refactors, measured performance improvements and bug fixes are plain pull requests, and a rule they leave behind lives in the file that governs it. A change to the process itself is tooling: a pull request that edits `blueprints/README.md`.
- **Enhancement proposal** (`blueprints/proposals/NNNN-<slug>.md`, cited `EPR-NNNN`). Copy `blueprints/proposals/0000-template.md`. Header: Feature Name, Start Date, Proposal PR, Tracking issue, Status. Sections: Summary, Motivation, Guide-level explanation, Reference-level explanation, Drawbacks, Rationale and alternatives, Prior art, Unresolved questions, Future possibilities.
- **Acceptance.** A proposal is opened as a pull request labelled `epr`, and may merge with `Status: proposed`, so the docsite renders it for review; Claude may merge that. Acceptance is the change of the status line to `accepted`, and only the owner merges a pull request that makes it.
- **Tracking issue.** Each accepted proposal has one, labelled `tracking` and opened from `.github/ISSUE_TEMPLATE/tracking.md` before the owner accepts, so the header can link it. It holds the milestone checklist, links each delivering PR, and closes when the work ships.
- **API change proposal.** An issue labelled `acp`, from `.github/ISSUE_TEMPLATE/api-change-proposal.md`. Only the owner accepts one; the PR that carries it out links and closes it.
- **Enhancement plan** (`blueprints/plans/NNNN-<slug>.md`, cited `EPL-NNNN`): only for a plan several agents execute in parallel; otherwise the tracking issue holds the plan. Copy `blueprints/plans/0000-template.md`, which keeps the Rust RFC layout with the plan (milestones, test plan, ship criteria, risks) under Reference-level explanation and a status log after it. Header: Plan, Implements, Start Date, Plan PR, Status, Shipped in.
- **Status.** Proposal: `proposed`, `accepted`, `implemented` (once the CHANGELOG records it shipping and its tracking issue is closed), or `superseded by EPR-NNNN`. Plan: `planned`, `in progress`, `shipped in X.Y.Z`, or `dropped`, with a dated status-log line for each change.
- **Index.** Every proposal and plan has a row in `blueprints/README.md`. `scripts/check-blueprint-refs.sh` (in `just check` and the `Docsite floor` CI job) fails when a file has no row, or a cited `EPR-NNNN`, `EPL-NNNN`, `RFC-NNNN` or `EP-NNNN` resolves to no file of its own kind.

Records cited as `ADR-` plus a number before 2026-09-24 are the legacy RFC of the same number; released CHANGELOG entries keep that wording.

### Superseding an enhancement proposal

An accepted proposal is not rewritten. Superseding one changes exactly two things in it: the status line

```markdown
- Status: superseded by [EPR-NNNN](NNNN-slug.md) (YYYY-MM-DD)
```

and a dated note under the header:

```markdown
> **Note (YYYY-MM-DD):** Superseded by [EPR-NNNN](NNNN-slug.md), which [the new decision]. Read this proposal for historical context only.
```

Never delete the original content; the audit trail is the value.

The new proposal has `- Supersedes: [EPR-NNNN](NNNN-slug.md)` in its header and explains in its Motivation *why* the prior decision is being changed. A proposal that replaces what a legacy record decided cites it (`RFC-NNNN`) in its Motivation; the legacy record is not edited.

Example from the legacy records: `blueprints/legacy/rfcs/0003-workspace-with-rumi-nlp.md` was superseded by `blueprints/legacy/rfcs/0004-stay-single-crate.md` on 2026-05-20.

## Arch notes

`.claude/arch/` holds two files: `README.md`, which points at the docsite pages that replaced the old architecture notes, and `evolution.md`, the decisions considered and rejected. The architecture of record is on the docsite (`site/content/architecture/design.md`, `site/content/reference/`), where `just docs-floor` gates it; a fact about the architecture goes there, not here.

## Aspirational-claim discipline

Matra's docs went through a substantial cleanup on 2026-05-20 because they had drifted to describe an aspirational two-crate workspace, an `Engine` struct, `analyze_directory_iter`, `MatraError` (the old shape), `otel` feature, and tracing-always-on — none of which existed in code at the time (EP-0008 later shipped a real `Engine`, deliberately; the defect was docs asserting one before it existed).

**Rule**: every claim in a shipping doc must be grounded in either:

- Code that exists in `src/`, `python/`, or `Cargo.toml`.
- `ROADMAP.md`, for intended-but-not-shipped capabilities. It is the only page that describes what does not ship; no other page carries a planned marker.

When in doubt, check the claim against the code.

## The README elevator pitch

`README.md`'s first three sentences are the project's identity. They cannot drift. The current shape:

> NLP library. Text in, structured analysis out.
>
> UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

If matra's scope shifts substantially, update README first, then everywhere else cascades. If a doc claim contradicts README's elevator pitch, fix one or the other in the same PR.


## Pre-release checklist

Before running `just release-prep VERSION`:

- [ ] `## [Unreleased]` in `CHANGELOG.md` describes every user-facing change since the last release.
- [ ] Every proposal that lands this release has `Status: accepted`, and every proposal this release ships reads `implemented`.
- [ ] Every plan this release completes reads `shipped in X.Y.Z`, with a dated status-log line.
- [ ] Arch docs match the shipping code (run the audit if uncertain).
- [ ] README's elevator pitch is current.
- [ ] No aspirational claims in shipping docs.
- [ ] CI is green on `main`.

Then `just release-prep VERSION` rolls the CHANGELOG only. It does not touch `Cargo.toml` or `pyproject.toml` and does not commit: bump the versions by hand, review the diff, then commit.

## What this skill won't tell you

- How to write the substance of an enhancement proposal — that's a thinking activity per case.
- Whether a borderline change is substantial (an enhancement proposal) or minor (an API change proposal): the test in `blueprints/README.md` decides most cases, and the owner decides the rest.
- Specific commit message wording — follow conventional commits, keep the imperative mood.
