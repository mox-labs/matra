---
name: API change proposal
about: Propose a minor addition to matra's public surface, or a minor removal from it
title: "acp: "
labels: ["acp"]
---

<!--
An API change proposal is for a minor change: one item a caller may use,
added or removed, that changes nothing about what the existing calls do or
return. A change to what an existing call returns or means, or one that
reshapes several items at once, is an RFC instead. blueprints/README.md has
the grain.

Only the owner accepts a proposal, by saying so on this issue. The pull
request that carries an accepted proposal out links this issue and closes it.
-->

## The item

<!-- The name, its signature in Rust, and how it appears in Python, on the
     command line and in the JSON output, where it appears at all. -->

## What a caller writes with it

<!-- A short example of the call, before and after. -->

## Why it is minor

<!-- What existing calls it leaves untouched, and why no caller has to change
     code or read results differently. -->

## Why matra, and why now

<!-- What shows the need is real: the code, a test, a reported issue, a
     downstream caller. Whether it belongs in matra or in the caller. -->

## Outcome

<!-- Filled in by the owner: accepted or declined, and why. -->
