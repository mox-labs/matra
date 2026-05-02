---
name: Bug report
about: Something does not work the way the docs say it should
title: "bug: "
labels: ["type:bug", "status:triage"]
---

## What happened

<!-- The unexpected behavior, in one or two sentences. -->

## What you expected

<!-- The behavior the docs / API surface implied. -->

## Reproduction

<!-- Minimal code or input that demonstrates the bug. The shorter, the better. -->

```rust
// or shell, or python — whatever reproduces it
```

## Environment

- vaani version (or git SHA): 
- Rust version (`rustc --version`): 
- OS / arch: 
- Features enabled (`udpipe`, `python`, etc.): 

## Logs / errors

<!-- Stack traces, panic output, or error messages. Trim to the relevant frames. -->

## Have you checked

- [ ] The CHANGELOG.md for a known limitation or pending fix
- [ ] Existing issues with the `type:bug` label
- [ ] The relevant section of `.claude/arch/` for intended behavior
