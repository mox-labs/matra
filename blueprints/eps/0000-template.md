# EP-0000: Title in sentence case

- EP: EP-0000
- Implements: (the accepted RFCs this plan delivers, e.g. [RFC-0000](../rfcs/0000-template.md), or `none` for a standalone EP)
- Start Date: (the day the pull request opens, YYYY-MM-DD)
- EP PR: (the pull request that proposes this EP, e.g. [#0](https://github.com/mox-labs/matra/pull/0))
- Status: planned
- Shipped in: (the release that carries the finished work, or `not shipped`)

<!--
Copy this file to NNNN-short-name.md with the next free number, fill every
section, and open a pull request. An EP is laid out with the Rust RFC
template, as an RFC is, so the two read the same way; what makes it a plan is
in Reference-level explanation (the milestones, the test plan, the ship
criteria and the risks) and in the Status log after it.

An EP that implements an RFC does not restate it: its Motivation and its
Guide-level explanation say in a few lines what the RFC decided and link it,
and the sections that follow are about getting the work done. A standalone
EP (`Implements: none`) is for work that changes how matra is built, checked,
documented or delivered rather than matra itself, so it carries its own
design, in full, in the same sections an RFC would.

Status is one of `planned`, `in progress`, `shipped in X.Y.Z`, or `dropped`.
Every change of status adds a dated line to the Status log. A section with
nothing to say says so in one line rather than being deleted, so that a
reader can tell "considered, nothing to add" from "forgotten".
-->

## Summary

One paragraph: what this plan delivers. For an EP that implements an RFC,
in terms of that RFC.

## Motivation

Why this work, and why now. For an EP that implements an RFC, a few lines
and a link: the RFC holds the argument. For a standalone EP, the argument
itself: what is missing or painful today, for whom, and what in the code,
the gates, or a reported issue shows the need is real rather than
anticipated.

## Guide-level explanation

The finished work, explained as if it had already shipped, to the person
who will use it: a caller of matra, a contributor, a reviewer, or whoever
runs the tooling. The names they meet, examples of what they do and see,
and how it changes the way they work. For an EP that implements an RFC,
point at the RFC's own guide-level explanation and add only what the plan
makes visible along the way (a flag that exists for one release, a
migration step between milestones).

## Reference-level explanation

The technical portion: in a standalone EP, the design in enough detail that
its interaction with the rest of the repository is clear, its corner cases
are dissected by example, and each invariant it introduces is named with
the test, fixture or gate that checks it. Then, in every EP, the plan.

### Milestones and iterations

The work in order. Each milestone is one pull request or a small, named set
of them, and no milestone starts before the previous one meets its exit
criterion.

#### M1: short name

- **Deliverable.** What lands: the files, the types, the tests.
- **Exit criterion.** The observable predicate that says the milestone is
  done. If it is false, the milestone is not done.

#### M2: short name

(repeat)

### Test plan

How the work is verified: the unit and integration tests, the conformance
fixtures in `spec/tests/` for anything that crosses into Python or the CLI's
JSON, the gates in `just check` and `just docs-floor`, and any measurement a
milestone depends on.

### Ship criteria

The single predicate that says the plan is finished and can be released.

### Risks

What could go wrong, what would signal it early, and what the response is.

## Drawbacks

Why should we *not* do this? What does it cost in surface, dependencies,
maintenance, or the next contributor's attention?

## Rationale and alternatives

- Why is this plan (and, in a standalone EP, this design) the best in the
  space of possible ones?
- What other plans or designs were considered, and why were they not
  chosen? This is where what the plan deliberately does not do is said,
  including the adjacent work it will be tempting to fold in.
- What is the impact of not doing this?

## Prior art

How comparable projects handled the same work, and earlier RFCs and EPs in
this repository that bear on it. If there is none, say so.

## Unresolved questions

- What is to be resolved in review before this merges?
- What is to be resolved during the work, and by which milestone?
- What related work is out of scope for this EP and could be done later,
  independently of it?

## Future possibilities

What the natural extension of this work would be. If nothing comes to mind
after trying, say so. Writing something here is not a commitment to it.

## Status log

The one appendix the process requires: every change of status, dated, with
the pull request or commit that made it.

- YYYY-MM-DD: planned.
