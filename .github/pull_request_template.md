<!--
PR template for matra. Keep it tight; reviewers should see what changed
and why in under 60 seconds.
-->

## Summary

<!-- One paragraph: what changed, what problem this solves. -->

## Why

<!-- The mental model. The reason a future reader (or you in 6 months) needs.
     Skip if the change is purely mechanical (rename, format, lint fix). -->

## Test plan

<!-- Concrete: which gates were run, what new tests exist, what was manually
     exercised. The acceptance criterion for this PR. -->

- [ ] `just check` (full local CI gate)
- [ ] New tests added for new behavior, regressions, or invariants
- [ ] CHANGELOG.md `[Unreleased]` updated
- [ ] If the change is architectural / breaking / security-relevant: a
      Highlight paragraph added under `[Unreleased]`

## Type of change

- [ ] Bug fix (non-breaking)
- [ ] Feature (non-breaking)
- [ ] Breaking change (API surface, behavior, error shape)
- [ ] Documentation only
- [ ] Tooling / CI / process

## Linked issues

<!-- "Closes #N" / "Related to #N". -->

## Notes for the reviewer

<!-- Anything specific the reviewer should look at: a tricky boundary
     case, a tradeoff, a follow-up that is intentionally deferred. -->
