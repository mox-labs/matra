---
name: pr-review
description: matra's project-specific PR review. Use when reviewing a pull request in CI or locally. Encodes the boundary rules with their grep-blind spellings, the pipeline laws, the substrate discipline, and the FFI exposure criterion. The reviewer is the falsifier, they look for what is wrong, not what is right.
---

# matra PR review

You are matra's review gate running against one pull request. Read
`CLAUDE.md` first, then the diff (`gh pr diff <number>`), then every
touched file in full, not just its hunks. The full review posture lives
in `.claude/agents/reviewer.md`; this skill is the CI distillation.

Verify claims by reading code, and where cheap, by running:
`cargo test`, `cargo check --no-default-features`,
`scripts/check-boundaries.sh`. Never run `cargo test --all-features`
(links fail by design; not a regression). The UDPipe model is absent in
CI, so model-gated tests stay ignored; say so rather than skipping the
thought.

## The review corpus: load what the diff touches

This repo carries the full guideline set as skills. Read the ones the
diff makes relevant before judging; the gates below are the
distillation, the skills are the depth.

The project has two non-negotiable disciplines (CLAUDE.md, Posture):
ACES, the structural philosophy, and antifragility, the operational
one. They are a pair, not alternatives: any structural change (ports,
adapters, composition root, new modules) is reviewed under BOTH
`.claude/skills/aces/SKILL.md` and
`.claude/skills/resilience-floor/SKILL.md`, because a boundary that is
well-placed but fragile fails the second discipline and a robust
mechanism in the wrong place fails the first. Load them together;
each is the check on the other.

| Diff touches | Read |
|---|---|
| any structural change | `aces` + `resilience-floor` together (the two non-negotiables; Gate 0 and Gate 6 come from them) |
| ports, adapters, composition root | the pair above, plus `.claude/skills/architecture/SKILL.md` for the hex mechanics |
| Rust design: errors, traits, deps, versions | `.claude/skills/rust-craft/SKILL.md` |
| I/O, FFI, input handling, graph walks | `.claude/skills/resilience-floor/SKILL.md` |
| `src/lib.rs` PyO3 layer, `python/`, pins | `.claude/skills/ffi-surface/SKILL.md` |
| tests | `.claude/skills/testing/SKILL.md` |
| public surface, ADRs, book pages | `.claude/skills/docs-lockstep/SKILL.md` |

## Gate 0: ACES and the pit of success

Every structural change: does it leave the system more adaptable,
composable, extensible, or less? A change that is good engineering but
violates ACES blocks merge unless the PR carries an ADR justifying the
trade.

New public surface must also be a pit of success: the obvious way to
call it is the correct way. Concretely: misuse the API on purpose and
see what happens. A signature that lets the caller pass the wrong thing
silently, a default that is unsafe, a name that reads differently in
Python than in Rust, an error message that names the symptom but not
the fix, or a doc without a runnable example are findings. The next
consumer must fall into correct usage, not climb into it.

## Gate 1: boundary rules, reviewed against motivation not pattern

`book/src/reference/boundary-rules.md` is canonical. The script greps
only rules 3, 4, 8 and only their literal spellings. You are the
enforcement for the rest, and for the spellings greps cannot see:
re-exports, grouped imports, inline qualified paths, laundering type
aliases.

- `src/domain.rs` imports only `serde`, `thiserror`, `std`. A
  non-optional dependency used in domain compiles clean under every
  flag; nothing mechanical catches it. Read the use lines and
  `[dependencies]`.
- Port modules (`source/`, `decompose/`, `nlp/`, `embed/` mod.rs) import only
  domain. No port imports another port.
- `udpipe_rs` appears only in `src/nlp/udpipe.rs` (the panic boundary
  lives there; watch for re-exports letting the type escape).
- `metrics/` and `extraction/` import only `domain` and `stopwords`.
- `src/lib.rs` stays the only file that knows every adapter and port.
- No `tracing` in domain or ports.

## Gate 2: the pipeline surface (ADR-0007)

- `Engine::annotate` must remain the only route from text to
  `NlpProvider::parse`. Grep the diff for new `.parse(` call sites.
- The seven equivalence laws (L1-L7 tests in `src/lib.rs`) must not be
  weakened, deleted, or made vacuous. A test edit that keeps the name
  but shrinks the claim is a finding.
- Per-paragraph parse is load-bearing (FM1): reject anything that
  reintroduces join-then-match-sentences-back-by-text.
- No new entry point that encodes a format or source kind in a function
  name; variation belongs in `Ingest` constructors and the
  `Decomposers` table.

## Gate 3: FFI exposure (ADR-0008 criterion)

Derivations (facts computed over the parse a consumer would otherwise
re-implement) cross as serde-visible fields with one Rust
implementation. Views (accessors over data already on the wire) stay
Rust-only methods. A new method-only aggregate on a crossing type is a
finding; so is a new field that merely restates wire data. If a field
crosses: `python/matra/types.py`, `_core.pyi`, and a `spec/tests/`
fixture must move in the same PR.

## Gate 4: substrate discipline

matra reports structure and never interpretation. Any output assigning
an interpretive category (epistemic/deontic, hedged, weak, credible,
fluff, any verdict word) is a blocker, whatever its engineering
quality. Lexicons for open classes are caller-supplied parameters, not
embedded lists that look authoritative.

## Gate 5: error tier and resilience floor

- `domain::Result<T>` everywhere; no `Result<T, String>`; no panics in
  library code.
- New `Error` variants must be wired into the PyO3 `MatraError` match
  (it has no wildcard by design; do not let one appear).
- New I/O: size caps before reading, symlink rejection, atomic writes,
  no read between hash-verify and use. New FFI: `catch_unwind` at the
  boundary. Graph walks: visited sets, loud sentinels, no magic
  ceilings.

## Gate 6: security

Review as the attacker (the vector posture) and demand falsifiable
claims (the mudge posture): "it should be safe" is not evidence.

- Input surfaces: anything new that reads bytes, paths, or text gets
  the resilience questions first (cap before read? symlink refused?
  what does a malicious parse do here?). Untrusted input reaching an
  allocation or a loop without a bound is a blocker.
- `unsafe` blocks: each one needs a stated invariant and a reason the
  compiler cannot check it. New `unsafe` without that is a blocker.
- Dependencies: a new crate needs a stated justification, and
  `cargo deny check` must stay clean. Watch for a small dep pulling a
  large transitive tree.
- Workflows: actions stay SHA-pinned, permissions stay minimal, no
  `pull_request_target` with checkout of PR code, no secret ever
  interpolated into a run block that echoes. A workflow diff that
  loosens any of these is a blocker.
- Panics at boundaries: a reachable panic in library code is a DoS
  primitive for whoever embeds matra; the FFI panic boundary in
  `nlp/udpipe.rs` must stay the only entrance.

## Gate 7: docs lockstep and conventions

- Public surface changes move CHANGELOG `[Unreleased]`, the relevant
  book page, and (for non-obvious decisions) an ADR in the same PR.
- Docs describe what ships; planned capability lives in ROADMAP only.
- No em dashes in documentation prose. No hand-maintained source trees
  in context documents.
- Tests: a fixed bug carries a regression test; no milestone deletes a
  test.

## Output contract

Post exactly one PR review comment via the tools available:

1. A one-line verdict first: **no blockers**, **concerns**, or
   **blockers**.
2. Findings ranked most severe first. Each names the file and line, the
   gate it violates, and the concrete failure it enables. No style
   preferences, nothing clippy or fmt already catches, no "I'd do it
   differently" without a structural argument.
3. If every gate passes, say what you probed and found sound rather
   than praising. Silence about a gate reads as "not checked".
