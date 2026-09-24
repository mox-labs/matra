# EP-0000: Title in sentence case

- EP: EP-0000
- Implements: (the accepted RFCs this plan delivers, e.g. [RFC-0000](../rfcs/0000-template.md), or `none` for a standalone EP)
- Status: planned
- Shipped in: (the release that carries the finished work, or `not shipped`)

<!--
Copy this file to NNNN-short-name.md with the next free number. An EP exists
when an accepted RFC's implementation spans more than one pull request; it
says how the work gets done and how anyone knows it is finished. The RFC it
implements says what and why, and the EP does not restate it.

Status is one of `planned`, `in progress`, `shipped in X.Y.Z`, or `dropped`.
Every change of status adds a dated line to the Status log. A section with
nothing to say says so in one line rather than being deleted.
-->

## Summary

One paragraph: what this plan delivers, in terms of the RFC it implements.

## Design

Only in a standalone EP (`Implements: none`): the choices a later
contributor needs, with the reason for each. An EP that implements an RFC
omits this section; the RFC holds the design.

## Goals

What is true when this plan is finished: the surface that exists, the
behavior a caller can rely on, the gap that is closed.

## Non-goals

What this plan deliberately does not do, including the adjacent work it
will be tempting to fold in.

## Iterations and milestones

The work in order. Each milestone is one pull request or a small, named set
of them, and no milestone starts before the previous one meets its exit
criterion.

### M1: short name

- **Deliverable.** What lands: the files, the types, the tests.
- **Exit criterion.** The observable predicate that says the milestone is
  done. If it is false, the milestone is not done.

### M2: short name

(repeat)

## Test plan

How the work is verified: the unit and integration tests, the conformance
fixtures in `spec/tests/` for anything that crosses into Python or the CLI's
JSON, the gates in `just check`, and any measurement a milestone depends on.

## Ship criteria

The single predicate that says the plan is finished and can be released.

## Risks

What could go wrong, what would signal it early, and what the response is.

## Status log

- YYYY-MM-DD: planned.
