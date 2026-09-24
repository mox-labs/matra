# RFC-0000: Title in sentence case

- Feature Name: (a unique snake_case identifier, e.g. `semantic_clusters`)
- Start Date: (the day the pull request opens, YYYY-MM-DD)
- RFC PR: (the pull request that proposes this RFC, e.g. [#0](https://github.com/mox-labs/matra/pull/0))
- Tracking EP: (the EP that carries the implementation, or `none` when one PR delivers it)
- Status: accepted (the file reaches `main` only by merging, which is what accepts it)

<!--
Copy this file to NNNN-short-name.md with the next free number, fill every
section, and open a pull request. The pull request is the proposal; merging
it accepts the RFC. See blueprints/README.md for the process.

A section with nothing to say says so in one line rather than being deleted,
so that a reader can tell "considered, nothing to add" from "forgotten".
-->

## Summary

One paragraph explaining the change.

## Motivation

Why are we doing this? What does it make possible for a caller of matra,
in Rust, in Python, or on the command line, that is not possible or not
pleasant today? What is the expected outcome, and what in the code,
the tests, or a reported issue shows the need is real rather than
anticipated?

## Guide-level explanation

Explain the proposal as if it had already shipped and you were teaching it
to someone who uses matra. That generally means:

- Introducing the new named concepts, and the names they will carry across
  Rust, Python, and the JSON the CLI emits.
- Explaining the change largely through examples: the call, the value it
  returns, the field a caller reads.
- Explaining how a caller should think about the change and how it alters
  the way they use matra, as concretely as possible.
- Where it applies, the error a caller now sees, the deprecation they meet,
  or the migration they perform.
- Where it applies, how it reads differently to someone new to matra and to
  someone who already knows the current surface.

For an RFC about matra's own structure or toolchain rather than its public
surface, this section explains how a contributor should think about the
change and gives examples of its concrete effect.

## Reference-level explanation

The technical portion. Explain the design in enough detail that:

- Its interaction with the rest of matra is clear: which layer it lives in,
  which ports and adapters it touches, and which boundary rules it relies on.
- It is reasonably clear how it would be implemented.
- Corner cases are dissected by example.
- The invariants it introduces, and how each is checked (a test, a
  conformance fixture, a gate), are named.

Return to the examples in the guide-level explanation and explain how the
detailed design makes them work.

## Drawbacks

Why should we *not* do this? What does it cost in surface, dependencies,
maintenance, or the next contributor's attention?

## Rationale and alternatives

- Why is this design the best in the space of possible designs?
- What other designs were considered, and why were they not chosen?
- What is the impact of not doing this?
- Could this live in a caller's code, or behind an existing extension point,
  instead of in matra?

## Prior art

Discuss prior art, good and bad, in relation to this proposal: how
comparable libraries and tools handle the same problem, published papers or
posts that bear on it, and earlier RFCs in this repository. The aim is a
fuller picture for the reader and the lessons others already paid for. If
there is none, say so. Precedent elsewhere is context, not on its own a
reason to adopt something here.

## Unresolved questions

- What parts of the design are to be resolved in review before this merges?
- What parts are to be resolved during implementation, before it ships?
- What related issues are out of scope for this RFC and could be addressed
  later, independently of it?

## Future possibilities

What would the natural extension of this proposal be, and how would it
affect matra as a whole? If nothing comes to mind after trying, say so.

Writing something here is not a reason to accept this RFC or a later one;
an argument for a future change belongs in that change's own Motivation and
Rationale.
