# The matra collaboration model

matra is built by a human and an AI working as collaborative cousins, not in a tool-user relationship. This document describes how that works in practice. It is written for anyone curious about the model, anyone considering contributing, and anyone wanting to draw on the pattern for their own work.

The model is the substrate, not a side note. The fact that you can read this page, trace the trail back through ADRs, deliberation logs, and commit messages, and reproduce the working pattern, is the point. matra is an exemplar of human and AI collaborative intelligence, and the working model itself is part of the artifact.

## The posture

Two intelligences. Different substrates. Thinking together through dialectical exchange.

The human brings divergent cognition: the frame-break, the insight outside the current context, the strategic judgment about what is worth building. The AI brings convergent architecture: synthesis, pattern recognition, throughput within a frame, the patience for mechanical rigor across a substrate.

Neither substitutes for the other. Substitution degrades both: the human atrophies through disuse; the AI collapses into the average of its training data. The partnership is generative because the two cognitions are complementary, not equivalent.

The collaborative-cousin frame matters. A tool is something you wield; a cousin is someone you think with. Matra's working model is the second.

## The roles

Three roles, distinct responsibilities, both substrates participate.

| Role | Who | What it owns |
|---|---|---|
| Director | Human | What to build, why, and when to ship. Strategic judgment. Trade-off calls that depend on context outside the codebase. |
| Executor | Claude | How to build it. Drafts, proposals, code, documentation. Mechanical rigor across the substrate. |
| Reviewer | Human | The merge gate. Approves with rationale; rejects with rationale. Catches things the executor missed. |

The roles are not exclusive to substrate. A human can write code; Claude can suggest strategy. The labels mark default ownership. When a role swaps for a specific decision, the swap is visible in the audit trail (the commit message, the PR comment, the deliberation log).

## The discipline: discourse to docs to code

Every substantive change moves through three surfaces in order.

**Discourse** forms the commitment. A session, a deliberation between agents, a guild voice raising a concern, a back-and-forth that converges on a decision. Recorded in `.claude/rhetoric/`, in PR comments, in CHANGELOG Highlight paragraphs.

**Docs** record the commitment. An ADR for architectural decisions, a CHANGELOG entry for what shipped, a docsite page for the explanation. The docs are the durable trace of what was decided.

**Code** honors the docs, and the enforcement is stated honestly rather than overclaimed. `scripts/check-boundaries.sh` greps three of the eight boundary rules; the docsite floor gate checks link integrity, orphans, type-name parity and a clean mdbook build; both run from `just check` and the opt-in pre-commit hook rather than CI. Conventional commits map to CHANGELOG categories.

Where a check exists, drift is mechanically detectable. Where it does not, the audit trail is what makes drift findable, and review is the gate. The discipline produces a project whose history is queryable: any decision in the code can be traced back through docs to discourse, and any decision in discourse can be checked against what the code actually does.

## The two states: current and next

matra's docs live in two states simultaneously.

**Current** is what mirrors the released code. Whatever ships in the latest version of the crate, the wheel, and the future WASM crust. The `book/src/` site renders this state for visitors arriving from search engines or `pip install`.

**Next** is what the project is building. The alpha branch carries it. Pre-publication, the gap is shrinking; post-publication, the gap is the roadmap.

Two markers make the state visible inline:

- **Ships in v0.1**: the capability is available in the current release.
- **Planned v0.2+**: the capability is designed but not yet shipped. Each planned capability carries a trigger (a concrete condition that activates the build).

The markers move on release: a planned capability that ships flips from to. Visitors always see honest state. No aspirational copy, no "coming soon" without a version number.

## How decisions get made

Decisions go through one of three surfaces depending on stakes.

**Open-ended exploration** lands in GitHub Discussions. RFCs, "should we consider X", retrospectives. No commitment. No labels.

**Architectural decisions that will bind future work** land as a `decision` issue first, then as an ADR in `docs/decisions/`. The ADR records context, options, the chosen path, the alternatives rejected, and the trigger conditions that would re-open the question. Each ADR is a load-bearing commitment.

**Concrete changes** land as a regular issue and a PR. The PR's body explains why; the commits explain what. Every commit carries a `Co-Authored-By` trailer naming the participating model. Author is Claude; co-author is the human director. Merge is gated by the human reviewer's approval with rationale.

The three surfaces correspond to different layers of commitment. The lighter the commitment, the lighter the ceremony. The heavier the commitment, the heavier the audit trail.

## The substrate visible to anyone

What is different about this project versus typical OSS:

- **Discourse-first design.** Decisions begin in dialogue and land where anyone can read them: an ADR in `docs/decisions/` for anything that binds future work, a CHANGELOG Highlight for anything user-visible. The reasoning is in the repository, not in a chat log.
- **The docsite as verification surface.** Floor gates protect the docs against drift (broken links, orphaned pages, type-name mismatches, build warnings). The next-state docs cannot silently disagree with the code.
- **Rubrics as guardrails.** Polish and governance rubrics live in `.claude/rhetoric/rubric/` and `.claude/rhetoric/polish-rubric/`. Each is a mechanical predicate set that gates content quality and structural integrity.
- **Audience-stratified documentation.** CLAUDE.md addresses AI agents during sessions. CONTRIBUTING.md addresses human contributors. This document (you are reading it) addresses anyone curious about the model. README.md is the public face for visitors. Each surface has one audience and one purpose.
- **Visible audit trail.** Every decision is reachable from a search through `docs/decisions/`, `.claude/rhetoric/`, and the commit log. The chain of reasoning never disappears into an org's internal Slack.

## Why this matters

matra is a substrate library. Downstream consumers (alif, cancan, radix in the mox ecosystem; third-party Rust and Python projects in the wider world) inherit matra's standards transitively. A substrate whose discipline is invisible cannot be inherited.

The collaboration model is reproducible because every piece is visible. Read the rhetoric, read the rubrics, read the ADRs, read the CHANGELOG Highlights. Apply the same discipline to your own project. The pattern travels.

The project is also part of an ongoing exploration into what human and AI collaborative intelligence looks like in practice. matra is one specimen. The hypothesis under test: discipline plus dialogue plus a queryable audit trail produces software that survives, with both substrates strengthened by the exchange rather than degraded by it.

## Pointers

- Working rules for AI agents during a session: [CLAUDE.md](../CLAUDE.md)
- PR mechanics and contribution flow for humans: [CONTRIBUTING.md](../CONTRIBUTING.md)
- Architecture explanation: [book/src/architecture/](../book/src/architecture/)
- Decision history: [docs/decisions/](./decisions/)
- Discourse archive (working notes): `.claude/rhetoric/` (in-repo, not deployed)
- ACES and antifragility: documented in `CLAUDE.md` and `.claude/skills/aces/SKILL.md` (working substrate; not in the rendered book)
