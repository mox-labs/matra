---
name: Tracking issue
about: Track an RFC's implementation from acceptance to shipping
title: "tracking: RFC-NNNN "
labels: ["tracking"]
---

<!--
One tracking issue per RFC. It is opened when the RFC pull request is ready
for the owner's decision, so the RFC's `Tracking issue` header can link it
before the merge. If the owner declines the RFC, close this as not planned.
It closes when the work ships: the CHANGELOG records it and the RFC's status
moves to `implemented`. blueprints/README.md has the process.
-->

**RFC:** <!-- link to blueprints/rfcs/NNNN-name.md, or to the RFC pull request until it merges -->
**EP:** <!-- only when several agents execute the plan in parallel; otherwise `none` -->

## Milestones

<!-- One line per milestone, each with its deliverable and its exit criterion.
     Tick it and link the pull request that delivered it. -->

- [ ] M1: deliverable. Exit: criterion. PR:
- [ ] M2: deliverable. Exit: criterion. PR:

## Shipped

<!-- The release that carries the finished work, and the CHANGELOG entry. -->
