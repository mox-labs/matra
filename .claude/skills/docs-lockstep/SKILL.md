---
name: docs-lockstep
description: Documentation hygiene for matra — CHANGELOG conventional-commit mapping, the RFC and EP process + supersede protocol, arch docs sync with code, README elevator pitch, aspirational-claim discipline. Use when a change lands and CHANGELOG / RFCs / arch docs / README need to update in lockstep.
---

# docs-lockstep

Documentation discipline for matra. The audit trail is the only durable artifact when a stranger inherits the project; this skill codifies what stays in sync with the code.

## When to invoke

- A change has landed in `src/` and the documentation needs to follow.
- Preparing for a release.
- Writing a new RFC or EP.
- Reviewing whether a doc claim still holds.

## The lockstep contract

When code changes, exactly the right docs change in the same PR. The mapping:

| Code change kind | What also updates |
|---|---|
| New public function/type | rustdoc, `CHANGELOG.md [Unreleased] Added` |
| Fix a bug | regression test, `CHANGELOG.md [Unreleased] Fixed` |
| Performance change | `CHANGELOG.md [Unreleased] Changed` (with perf note) |
| Internal refactor | usually nothing (unless invariants change) |
| New module under `src/` | `book/src/architecture/design.md` (the diagram) |
| New adapter | `book/src/architecture/design.md` |
| New port | `book/src/architecture/design.md` + RFC |
| New domain type or field | `book/src/reference/domain-types.md` |
| Boundary rule change | `book/src/architecture/design.md` + RFC |
| New feature flag | `Cargo.toml`, `book/src/architecture/design.md`, `README.md` (if user-visible), `CLAUDE.md` (if structural) |
| Dep added/removed/bumped | `Cargo.toml`, `CHANGELOG.md`, RFC (if non-trivial) |
| Public surface change | All of the above + RFC |

When the change is non-trivial and you cannot tell which docs are affected, run the audit: read each `.claude/arch/*.md` and ask "does any claim here mention what I just changed?"

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
| `i2(j):`, `i2(d):` etc. | Custom iteration prefix; goes under that iteration's Highlights block |

The `scripts/changelog-release.sh` script rolls `## [Unreleased]` into a versioned section when preparing a release. Run `just release-prep VERSION` to invoke it.

## RFC and EP conventions

The process is `blueprints/README.md`, and RFC-0019 records why it has this shape. It follows the Rust RFC process.

- **RFC** (`blueprints/rfcs/NNNN-<slug>.md`, cited `RFC-NNNN`): a design-level change to the architecture, the systems around it, the framework, or the toolchain. Copy `blueprints/rfcs/0000-template.md`. Header: Feature Name, Start Date, RFC PR, Tracking EP, Status. Sections: Summary, Motivation, Guide-level explanation, Reference-level explanation, Drawbacks, Rationale and alternatives, Prior art, Unresolved questions, Future possibilities.
- **EP** (`blueprints/eps/NNNN-<slug>.md`, cited `EP-NNNN`): the enhancement plan that takes an accepted RFC to shipping, written when the implementation spans more than one PR. Copy `blueprints/eps/0000-template.md`. Header: EP, Implements, Status, Shipped in. Sections: Summary, Goals, Non-goals, Iterations and milestones (each with a deliverable and an exit criterion), Test plan, Ship criteria, Risks, Status log.
- **Acceptance.** An unaccepted RFC is an open pull request. Merging it accepts it, and it lands with `Status: accepted`.
- **Status.** RFC: `accepted`, `implemented` (once the CHANGELOG records it shipping), or `superseded by RFC-NNNN`. EP: `planned`, `in progress`, `shipped in X.Y.Z`, or `dropped`, with a dated status-log line for each change.
- **Index.** Every RFC and EP has a row in `blueprints/README.md`. `scripts/check-blueprint-refs.sh` (in `just check` and the `Docsite floor` CI job) fails when a file has no row or a cited `RFC-NNNN` / `EP-NNNN` resolves to nothing.

Records cited as `ADR-` plus a number before 2026-09-24 are the RFC of the same number; released CHANGELOG entries keep that wording.

### Superseding an RFC

An accepted RFC is not rewritten. Superseding one changes exactly two things in it: the status line

```markdown
- Status: superseded by [RFC-NNNN](NNNN-slug.md) (YYYY-MM-DD)
```

and a dated note under the header:

```markdown
> **Note (YYYY-MM-DD):** Superseded by [RFC-NNNN](NNNN-slug.md), which [the new decision]. Read this RFC for historical context only.
```

Never delete the original content; the audit trail is the value.

The new RFC has `- Supersedes: [RFC-NNNN](NNNN-slug.md)` in its header and explains in its Motivation *why* the prior decision is being changed.

Example: `blueprints/rfcs/0003-workspace-with-rumi-nlp.md` was superseded by `blueprints/rfcs/0004-stay-single-crate.md` on 2026-05-20, and `blueprints/rfcs/0001-record-architectural-decisions.md` by `blueprints/rfcs/0019-rfc-and-ep-process.md` on 2026-09-24.

## Arch doc structure

`.claude/arch/` has six files; each has a specific scope:

| File | Scope |
|---|---|
| `README.md` | Index. Brief. Points at the others. |
| `architecture.md` | Big picture: hex layout, composition root, boundary rules. |
| `domain-model.md` | The types. Every field, every variant, every method. |
| `ports.md` | The boundary traits. |
| `adapters.md` | Concrete adapter implementations. |
| `evolution.md` | What's locked, what's allowed to change, what's deferred. |
| `boundary-rules.md` | The eight boundary rules, with motivation, failure modes, and review guidance. |

If a single code change requires updating more than two of these, you're probably changing the architecture and need an RFC.

## Aspirational-claim discipline

Matra's docs went through a substantial cleanup on 2026-05-20 because they had drifted to describe an aspirational two-crate workspace, an `Engine` struct, `analyze_directory_iter`, `MatraError` (the old shape), `otel` feature, and tracing-always-on — none of which existed in code at the time (I8 later shipped a real `Engine`, deliberately; the defect was docs asserting one before it existed).

**Rule**: every claim in a shipping doc must be grounded in either:

- Code that exists in `src/`, `python/`, or `Cargo.toml`.
- A clear "planned" marker for intended-but-not-shipped capabilities.

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
- [ ] Every RFC that lands this release has `Status: accepted`, and every RFC this release ships reads `implemented`.
- [ ] Every EP this release completes reads `shipped in X.Y.Z`, with a dated status-log line.
- [ ] Arch docs match the shipping code (run the audit if uncertain).
- [ ] README's elevator pitch is current.
- [ ] No aspirational claims in shipping docs.
- [ ] CI is green on `main`.

Then `just release-prep VERSION` rolls the CHANGELOG only. It does not touch `Cargo.toml` or `pyproject.toml` and does not commit: bump the versions by hand, review the diff, then commit.

## What this skill won't tell you

- How to write the substance of an RFC — that's a thinking activity per case.
- Whether a specific change deserves an RFC — judgment call; default to "yes" if you'd want a stranger to know in six months.
- Specific commit message wording — follow conventional commits, keep the imperative mood.
