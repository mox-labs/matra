# How vaani is run

vaani is developed in **iterations**. Each iteration addresses one structural concern: a resilience floor, a supply-chain hardening pass, a new extraction algorithm. The full sequence lives in `.claude/implans/` in the repo root. The architecture those iterations build toward lives in `.claude/arch/`. Anyone can read both before opening an issue or a PR.

## Working values

Three values govern every decision surface.

**Transparency.** Decisions are visible before they are made. RFCs open in Discussions. Architectural options surface in decision issues before an ADR closes them. The CHANGELOG records what shipped, and the Highlight paragraphs say why it mattered.

**Auditability.** Every change has a trail. Commit messages explain the why, not just the what. ADRs record the alternatives that were rejected and the conditions that would reopen them. The `Co-Authored-By` trailer in every commit names the participating model. The chain is queryable.

**Reversibility.** Every change can be backed out. Atomic commits on short-lived branches mean a bad change is a revert, not a surgery. Feature flags are additive; enabling one cannot break a consumer who has not opted in. Public types carry `#[non_exhaustive]` so vaani can gain variants without breaking callers.

## The iteration model

An iteration is a short-lived branch, implemented as a sequence of atomic commits, merged to `main` via a single PR, then deleted. Each commit is one logical unit: a sub-task the reader can follow without loading the whole iteration in mind. After merge, the branch is gone; the history reads as a sequence of small, auditable steps.

The full plan for each iteration lives in `.claude/implans/iN.md`. Read it before picking up work in that area.

## Where things live

| Location | What lives there |
|---|---|
| `.claude/arch/` | Architecture docs: hex layout, ports, adapters, domain model, evolution. Read before changing structure. |
| `.claude/implans/` | Iteration plans: goals, tasks, validation, acceptance gate per iteration. |
| `docs/decisions/` | Architecture Decision Records (ADRs). One file per significant call. |
| `CHANGELOG.md` | What shipped, with Highlight paragraphs for load-bearing changes. |
| `CLAUDE.md` | Working rules for AI collaborators: pipeline shape, boundary rules, conventions. |
| `scripts/` | Versioned tooling: pre-commit hook, boundary check, changelog rollover. |
| `justfile` | Single source of truth for repeatable workflows. Run `just check` before a PR. |
| GitHub Issues | Tracking: bugs, features, decisions. Labels classify type and area. |
| GitHub Discussions | Open-ended design: RFCs, retrospectives, ideas, Q&A. |

When something looks contradictory across these surfaces, the order of authority is: code > tests > `.claude/arch/` > `.claude/implans/` > ADRs > CHANGELOG > Issues > Discussions. Closer to the running system wins.

## How decisions are made

Decisions move through three surfaces depending on stakes.

**Open-ended exploration** starts in GitHub Discussions. RFCs, "should we consider X", retrospectives. No commitment, no labels. This is where options breathe before they harden.

**Architectural decisions** that will bind future work start as a `decision` issue (template at `.github/ISSUE_TEMPLATE/decision_record.md`). The issue captures context, options, tradeoffs, and a recommendation. After deliberation, the chosen option lands in `docs/decisions/NNNN-short-name.md` and the issue closes pointing at the ADR. There are six ADRs as of this writing; each is a load-bearing commitment that any future change must reckon with.

**Concrete changes** land as a regular issue and a PR. The PR body explains why; the commits explain what.

## Release ritual

Release when something user-visible justifies it: a new capability, a breaking surface change, a security-relevant fix. Not on a calendar.

The process is four steps: `just release-prep VERSION` rolls the CHANGELOG, the maintainer reviews the diff and writes 2-4 Highlight paragraphs for load-bearing changes, `Cargo.toml` is bumped and committed, then `cargo publish --dry-run` runs for a sanity check. Publishing is a deliberate, per-call action, not a scripted step. Yanking leaves a tombstone; every downstream consumer sees it. The manual gate is by policy.

## The substrate posture

Two disciplines are non-negotiable.

**ACES** (Adaptable, Composable, Extensible) is the structural design philosophy. Every system decays through three endogenous forces: stasis (decisions harden, new requirements fight the architecture), drag (complexity accumulates, simple changes take weeks), and opacity (understanding fades, nobody knows why it works). ACES is the counter-force to each. Every structural change is checked against the ACES boundary test: does this make the system more adaptable, composable, and extensible, or less? The full discipline is in `.claude/skills/aces/SKILL.md`.

**Antifragility** is the operational discipline: size caps at entry points, `catch_unwind` at C/C++ FFI boundaries, atomic file writes, TOCTOU closure on hash-verified loads, cycle-safe graph walks. The library survives bad inputs loudly, not silently. The full discipline is in `.claude/skills/resilience-floor/SKILL.md`.

For why the substrate posture matters (why a library that illuminates structural makeup needs structural integrity of its own), see [What vaani illuminates](../architecture/conviction.md).

## Working with Claude

vaani's primary engineer is Claude (Anthropic's AI). Every commit carries a `Co-Authored-By` trailer naming the model. Humans review and approve every PR before merge; nothing lands without a human OK. The working model (roles, discipline, the discourse-to-docs-to-code chain) is documented in full at `docs/collaboration-model.md`.

If you want to contribute with Claude Code, the `.claude/` directory in the repo is preloaded with architecture docs and iteration plans. Claude Code reads those automatically.
