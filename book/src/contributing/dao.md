# The DAO: practitioner agents and skills

vaani is built by a human director and Claude working together, with Claude as primary author. To keep the work coherent across a growing surface area (Rust core, Python bindings, a future WASM crust, a public docsite, ADRs, a CHANGELOG), the collaboration is structured through a set of practitioner agents and skills.

**Agents** are roles with bounded scope: a specialized context, a clear mandate, a list of what they do not do. Agents prevent scope sprawl. You reach for the right agent when the task matches the scope; the agent carries the specialized knowledge so the main session does not have to.

**Skills** are reusable discipline sets: a checklist, a set of invariants, a boundary test. Skills are injected into agents (or into the main session) when a decision calls for that discipline.

The combination is a Diverse Agent Organization (DAO): a stable registry of named roles that covers the full substrate surface, not a monolith, not ad hoc delegation.

## The six practitioner agents

Full agent definitions live in `.claude/agents/` in the repo root. The summaries below describe scope and when to reach for each.

### maintainer

The substrate owner. Holds the full picture: hex layout, three ports, composition root, cross-language story, rust-mastery corpus prescriptions. Makes architectural decisions, adds features, fixes bugs, drives iterations. Writes ADRs for any decision that changes the public surface or relaxes a boundary rule. Delegates to the other agents when the task fits their scope.

Reach for `maintainer` on any non-trivial change that needs the full context of the codebase and its constraints.

### reviewer

The gate. The reviewer's job is to find what is wrong before a change merges. Not to find what is right. Every review runs a fixed set of gates: ACES compliance, boundary compliance, public surface integrity, error tier discipline, resilience floor, cost discipline, documentation lockstep, and tests.

Reach for `reviewer` before merging anything substantive, for boundary compliance audits, and for pre-release readiness checks.

### portsmith

The port trait specialist. Owns the three boundary traits (`Source`, `Decomposer`, `NlpProvider`) and decides what shape they take. Designs new ports when a genuine adapter need emerges. Evaluates whether to extract a port into a separately published crate (the Pattern 6 criterion: an external implementor ecosystem must exist before extraction makes sense). Audits port contracts for clarity, object-safety, and forward compatibility.

Reach for `portsmith` when adding a new port, changing an existing port contract, or evaluating whether a port belongs in-crate or as a separate crate.

### ffi-keeper

The FFI surface owner. Holds the Rust/Python boundary: PyO3 bindings, maturin build, `pyproject.toml`, the `From<domain::Error> for PyErr` routing table, the four pythonize blind spots, the 3-axis version pin rule. When the WASM/TS crust lands, it lands here. The dual-publish discipline (methods do not cross FFI; only fields do) lives here.

Reach for `ffi-keeper` when touching the Python bindings, maturin config, version pins on `pyo3`/`pythonize`, or anything that crosses the Rust/Python boundary.

### resilience

The robustness owner. Audits every new I/O path for size caps, symlink rejection, and atomic write semantics. Audits every external-library boundary for panic catching via `catch_unwind`. Audits hash-verify paths for TOCTOU windows. The five Taleb principles are the spine: single points of failure are bugs; bounded inputs everywhere; fail loud, not silent; atomic over racy; trust anchors pinned in source.

Reach for `resilience` when adding I/O, wrapping an external library boundary, adding a hash-verified load, or auditing failure modes.

### archivist

The documentation steward. Holds the audit trail durable. Writes CHANGELOG entries for every user-visible change. Writes ADRs for every decision that changes the public surface or supersedes a prior ADR. Keeps `.claude/arch/` docs in lockstep with the code. Enforces the aspirational-claim discipline: every claim in a shipping doc is grounded in code that exists or carries an explicit "planned" marker.

Reach for `archivist` when a change lands and CHANGELOG, ADRs, or arch docs need to update in lockstep.

## The seven skills

Full skill definitions live in `.claude/skills/` in the repo root. Each skill is a reusable discipline set injected into agents or sessions when a decision calls for that specialization.

| Skill | Discipline |
|---|---|
| `aces` | ACES design philosophy: Adaptable, Composable, Extensible. The boundary test every structural change runs against. Non-negotiable. |
| `rust-craft` | Rust design decisions: error tier, dependency pin, trait shape, version pin discipline. |
| `testing` | Test strategy: regression discipline, property tests, complexity benchmarks, what "tests verify requirements" means in practice. |
| `architecture` | Hex boundary, port design, composition root, canonical pattern application. |
| `ffi-surface` | PyO3 dual-publish: `unsendable`/`Bound`/pythonize/maturin/pin discipline. The four pythonize blind spots. |
| `resilience-floor` | Taleb patterns: `catch_unwind`, atomic ops, TOCTOU closure, size caps. The six operational disciplines. |
| `docs-lockstep` | CHANGELOG, ADRs, arch docs in sync with shipping code. The lockstep contract. |

## Human and AI collaboration framing

The agent model is not automation. It is structured collaboration with named roles and bounded scopes. The maintainer agent does not replace human judgment on strategic calls; it holds the technical substrate in focus so human judgment operates on the right level. The reviewer agent does not replace human approval; every PR still requires a human OK before merge.

What the agent model provides: continuity across sessions (the scope and discipline travel with the agent file, not with session memory), auditability (the agent's mandate is visible to anyone reading the repo), and composability (a change that involves ports, FFI, and resilience reaches portsmith, ffi-keeper, and resilience in sequence rather than having one session hold all three contexts simultaneously).

The discourse-to-docs-to-code chain holds for agent work the same as for human work: commitments form in dialogue, docs record them, code honors them. The working model document at `docs/collaboration-model.md` describes the full pattern, including roles, the two-state model, and what makes the audit trail queryable.
