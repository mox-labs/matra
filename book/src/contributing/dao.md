# The DAO

vaani's contributor surface is structured as a **diverse agent organization** (DAO): six practitioner agents that own specific responsibilities, plus seven skills that codify the disciplines. The DAO operationalizes the substrate's standards so any contributor -- human or AI -- can participate without re-deriving the substrate.

Find the agents at `.claude/agents/` and the skills at `.claude/skills/`.

## Practitioner agents

| Agent | Scope | When to reach |
|---|---|---|
| **maintainer** | Architectural decisions, features, bugs, evolution. Holds the whole codebase in view. | Non-trivial change that needs full-picture judgment. |
| **reviewer** | PR review gate. Boundary compliance audits. Pre-release readiness. The falsifier. | Before merging anything substantive. |
| **portsmith** | Port trait design. Pattern 6 evaluation (when to extract a port to its own crate). | Designing a new port or changing a port contract. |
| **ffi-keeper** | PyO3 + future WASM/TS surface integrity. Dual-publish discipline. | Touching Python bindings, maturin config, pyproject.toml, or pyo3/pythonize/maturin versions. |
| **resilience** | Failure modes. Bounds. Panic boundaries. TOCTOU closure. Security. | Adding new I/O, external-library boundaries, or user-input handling. |
| **archivist** | CHANGELOG. ADRs. Arch docs. README. Lockstep with code. | After a change lands, before a release. |

The agents are not exclusive. The maintainer can delegate to any other; any agent can be invoked directly. The reviewer always runs before merge.

## Skills

| Skill | Codifies |
|---|---|
| **aces** | The ACES design philosophy (Adaptable, Composable, Extensible) resisting the stasis / drag / opacity decay cycle. **Non-negotiable.** |
| **rust-craft** | Rust design decisions: error tier, dep pin, trait shape, version pin, feature-flag composition, `#[non_exhaustive]` discipline. |
| **testing** | Test strategy: regression discipline, unit + integration + doctest layout, property tests, complexity benches. |
| **architecture** | Hex boundary, port design, composition root, canonical pattern application (Pattern 5, 6, 10, 11 from the rust-mastery corpus). |
| **ffi-surface** | PyO3 dual-publish: `unsendable`/`Bound`/`pythonize`/`maturin` discipline; the 4 pythonize blind spots; the 3-axis pin rule. |
| **resilience-floor** | Antifragile operational discipline: size caps at entry, panic boundaries, atomic file writes, TOCTOU closure, cycle-safe graph walks. |
| **docs-lockstep** | CHANGELOG conventional-commit mapping, ADR supersede protocol, arch docs sync with code, aspirational-claim discipline. |

Each skill grounds in specific Frames from the rust-mastery corpus at `~/radix-workspaces/rust-mastery/` (closed 2026-05-14, ~150 Frames across 50+ Rust codebases). The vaani-readiness cross-artifact Frame is the integrating M1 prescription that the whole DAO grounds in.

## How the DAO works

When you make a change to vaani:

1. The relevant **practitioner agent** owns the change (e.g., adding a port adapter is `portsmith` territory).
2. The agent applies the relevant **skills** (e.g., `aces` boundary test, `architecture` for the structural decisions, `testing` for the regression discipline).
3. The **reviewer** gates the merge with the ACES check and the boundary audit.
4. The **archivist** updates CHANGELOG, ADRs, and arch docs in lockstep before the next release.

The DAO is the project's working memory. The skills are how disciplines persist across contributors. The agents are how responsibility is distributed. The whole point is that the disciplines survive the contributors -- any person or model that joins later inherits the same standards, not a degraded version of them.

## Why "diverse"

The agents are not duplicates. Each has a distinct lens: design (portsmith), gate (reviewer), execution (maintainer), resilience (resilience), boundary (ffi-keeper), stewardship (archivist). Diversity here is structural: each lens catches different failure modes.

A concrete example: suppose a PR adds a new `Source` adapter that reads from HTTP. The maintainer might approve it -- the new port implementation is structurally sound, cleanly bounded, and useful. The resilience agent would ask: what is the size cap? Is there a timeout? What happens if the server returns 10 GiB? Those questions are outside the maintainer's ACES lens and entirely inside the resilience lens. The reviewer sees the PR only after both have had their say. The lenses don't fully overlap, which is why all of them run.

## Human--AI collaboration

The DAO is designed to operate with both human and AI contributors. Humans direct; AI executes within the constraints. The skills exist so disciplines don't depend on which entity is reading them.

When Claude opens a PR, Claude has invoked the relevant agents and skills as part of producing the change. The audit trail shows which agent made which decision; the reviewer (human) gates the merge. Substitution of either side is degenerative. The human brings the divergent cognition; the AI brings the convergent throughput; the DAO is the structure that keeps both honest.

For more on the collaboration philosophy, see [ACES and antifragility](../philosophy.md).
