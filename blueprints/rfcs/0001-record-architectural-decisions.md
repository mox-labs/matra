# RFC-0001: Record architectural decisions

- Feature Name: `record_architectural_decisions`
- Start Date: 2026-05-02
- RFC PR: [#3](https://github.com/mox-labs/matra/pull/3)
- Tracking EP: none
- Status: superseded by [RFC-0019](0019-rfc-and-ep-process.md) (2026-09-24)
- Decider(s): project maintainer

> **Note (2026-09-24):** Converted from the decision-record layout to the RFC layout by [RFC-0019](0019-rfc-and-ep-process.md). Sections are reordered and re-headed; the decided text is unchanged apart from citations, which now read `RFC-NNNN`, links, which follow the move, and em dashes, which the house style rejects.

## Summary

We use **Option B (ADRs)**. They live under `docs/decisions/` in the repo, follow a simple template, and are linked from PRs that bind to them. They are append-only; obsolete ADRs are marked `deprecated` or `superseded by ADR-NNNN` rather than edited.

## Motivation

matra is a library: its public API surface and architectural
choices will be inherited by every downstream consumer. Once 0.1.0
ships, the surface binds. Future contributors (including future
versions of the maintainers) need to understand *why* a particular
shape was chosen, not just *what* exists.

Code, commit messages, and CHANGELOG entries each capture part of the
story. None capture the deliberation: what alternatives were considered,
what tradeoffs were accepted, what would falsify the choice. Without
that record, the reasoning is lost as soon as the original decider
forgets or moves on.

## Guide-level explanation

We use **Option B (ADRs)**. They live under `docs/decisions/` in the
repo, follow a simple template, and are linked from PRs that bind to
them. They are append-only; obsolete ADRs are marked `deprecated` or
`superseded by ADR-NNNN` rather than edited.

## Reference-level explanation

### Consequences

**Positive:**
- The "why" of every significant choice is preserved with the code.
- Future contributors (or future-us) can read the rationale before
  proposing alternatives.
- ADRs make decisions reversible by exposing the inputs.

**Neutral:**
- ADRs sit alongside `.claude/arch/` (architecture overview, evergreen)
  and `book/src/plans/` (sequenced execution plans). The three serve
  different purposes: arch is *what*, plans are *when*, ADRs are
  *why this and not that*.

### Validation

This ADR is right if, six months from now, a contributor evaluating
"should we change X?" can find the original reason for X under
`docs/decisions/` and either accept it or supersede it explicitly. It
is wrong if the directory becomes a graveyard nobody references.

A signal to revisit: if the maintainer keeps re-explaining a decision
in PR review comments, that decision needed an ADR and didn't get one;
the discipline needs tightening.

## Drawbacks

**Negative:**
- Adds ~30 minutes to non-trivial decisions.
- Requires discipline to decide when something deserves an ADR.

## Rationale and alternatives

### Option A: No formal decision record

Rely on commit messages, PR descriptions, and CHANGELOG Highlights to
capture rationale.

**Pros:** zero overhead; nothing new to maintain.
**Cons:** PR descriptions decay; commit messages get terse for big
decisions because nobody wants to write a 500-word commit; the
deliberation behind a choice is harder to find than the choice itself.

### Option B: ADRs (Architecture Decision Records)

Write one short markdown file per significant decision under
`docs/decisions/`. Sequence numbers, status lifecycle, append-only
history.

**Pros:** preserves the deliberation; discoverable; the format is
established (Michael Nygard's original 2011 post; MADR; Sun's variant);
git tracks edits but the convention is to never rewrite, only
supersede.
**Cons:** mild overhead per decision (~30 minutes to write a good ADR);
discipline cost (deciding when to write one).

### Option C: Wiki / external doc

Use GitHub Wiki, Notion, or similar.

**Pros:** rich formatting, easier collaborative editing.
**Cons:** lives outside the repo, lifecycle disconnected from code,
not part of the audit trail when the repo is cloned, no link from the
code to the decision (relative paths break).

## Prior art

- Michael Nygard, ["Documenting Architecture Decisions"](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) (2011) — the original.
- [MADR](https://adr.github.io/madr/): Markdown Any Decision Records template.
- `CONTRIBUTING.md`: explains where ADRs fit in the project's working model.

## Unresolved questions

None recorded when this was decided.

## Future possibilities

None recorded when this was decided.
