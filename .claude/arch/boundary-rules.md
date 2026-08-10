# Boundary rules — the canonical list, with motivation

The eight rules that hold vaani's hex architecture together. This file is canonical for **why each rule exists and how it is enforced**. `CLAUDE.md`, `.claude/skills/architecture/SKILL.md`, and `.claude/arch/README.md` keep a summary list of the rules themselves and point here for the reasoning; `.claude/agents/reviewer.md` and `.claude/arch/architecture.md` point here without restating. When a rule's wording changes, this file changes first.

Each rule carries four things: the rule, why it exists, what breaks when it is violated, and what to read for when reviewing a diff. **The motivation is the load-bearing part.** A reviewer who knows only the pattern catches the spelling; a reviewer who knows the motivation catches the violation that is spelled differently.

## How these are actually enforced

Be honest about this, because the gap is where violations land.

| Rule | Enforcement today | Strength |
|---|---|---|
| 1. domain deps | nothing | judgment only |
| 2. ports import only domain | nothing | judgment only |
| 3. no cross-port import | `check-boundaries.sh`, pre-commit hook only | partial: literal `use crate::<port>` only |
| 4. single udpipe_rs importer | `check-boundaries.sh`, pre-commit hook only | partial: import lines only, not re-exports |
| 5. metrics/extraction purity | nothing | judgment only |
| 6. no-default-features builds | `ci.yml` rust matrix (check + clippy + test, 2 OSes) and MSRV job | real, mechanical |
| 7. composition root knows the whole | nothing | judgment only |
| 8. no tracing in domain or ports | `check-boundaries.sh`, pre-commit hook only | partial |

`scripts/check-boundaries.sh` runs from `just check` and from `scripts/pre-commit-hook.sh`. **No CI workflow invokes it.** The hook is opt-in via `scripts/install-hooks.sh`. Rule 6 is the only rule with a real gate. Note that CI fires only on pushes to `main`/`alpha` and on PRs targeting them, so feature-branch work is ungated until the PR opens.

Rust offers no directional-import control between modules inside one crate, so there is no compiler mechanism available here. ADR-0004 chose a single crate deliberately. That makes reasoned review the primary enforcement, and the script a cheap backstop for three of the eight mechanical cases.

---

## Rule 1 — `domain.rs` depends only on `serde`, `thiserror`, and `std`

**Why.** Domain types are the one layer every crust serializes: Rust callers, the Python wheel via pythonize, the future TypeScript surface via serde-wasm-bindgen. A dependency added here enters the closure of every consumer on every target. Keeping the set at three is what lets a consumer depend on vaani's types without inheriting a C++ toolchain.

**What breaks.** A dependency that does not build on `wasm32` silently blocks the WASM crust before anyone tries it. A dependency that touches the filesystem or the network puts I/O into a layer whose whole value is that it has none. And `cargo check --no-default-features` stops being a meaningful signal once the domain carries weight of its own.

**Read for.** New `use` lines at the top of `src/domain.rs`. New entries in `[dependencies]` that are not `optional = true`. A domain type whose field type comes from outside the three allowed crates.

**Changing it.** ADR required. thiserror was admitted this way: it emits no public API, versions safely under the 3-axis rule, and replaced ~35 lines of hand-rolled `Display`/`Error`/`From`.

## Rule 2 — port modules import only from `domain`

Ports are `src/source/mod.rs`, `src/decompose/mod.rs`, `src/nlp/mod.rs`.

**Why.** A port is a contract, not an implementation. Anything the contract imports becomes a requirement on everyone who implements it. Keeping ports at domain-only is what makes the trait implementable by someone who has never seen vaani's adapters.

**What breaks.** External implementors inherit dependencies unrelated to the contract they wanted. Pattern 6 extraction (lifting `NlpProvider` into a published `vaani-nlp-api` when an implementor ecosystem appears) turns into an untangling job instead of a move.

**Read for.** Any `use` in the three port files naming something other than `crate::domain` or `std`. Trait method signatures whose parameter or return types come from an adapter module.

**Note.** Nothing checks this today, despite four documents having claimed the script does.

## Rule 3 — no port module imports another port module

**Why.** The three ports are peers and are deliberately unaware of each other. Stage order (ingest, decompose, parse) belongs to the composition root, not to the contracts. If `Decomposer` knows `NlpProvider`, the pipeline's shape is encoded in the traits and you can no longer replace one stage without touching the other.

**What breaks.** The composition root stops being the single place that knows the whole. Reordering or skipping a stage becomes a multi-file change across contracts that had no reason to know about each other.

**Read for.** The script catches only the literal form `use crate::source`, `use crate::decompose`, `use crate::nlp`. It misses:
- grouped imports, `use crate::{nlp, domain};`
- fully qualified inline paths with no `use` line, `crate::nlp::NlpProvider::parse(..)`
- a trait bound in one port naming another port's trait
- a type alias that launders the path

Read the import block and the signatures, not the grep output.

## Rule 4 — `nlp/udpipe.rs` is the only file that imports `udpipe_rs`

**Why.** This is the resilience rule, not a tidiness rule. UDPipe is C++ across FFI holding non-Send state, and a panic on the C side aborts the host process rather than unwinding: interpreter death in Python, a trap in WASM. `catch_parse_panic` in `nlp/udpipe.rs` converts that into a `domain::Error`. The single-importer rule is what makes that seam **complete** rather than one entrance among several.

**What breaks.** A second path into the C++ side is a process-abort surface with no panic boundary in front of it. In Python that surfaces as an interpreter crash with no traceback, from a library that promised typed errors.

**Read for.** Imports, and also **re-exports**, which the grep cannot see. A `pub use udpipe_rs::Model;` inside `nlp/udpipe.rs` lets any other file name the C-backed type through `crate::nlp::udpipe::Model` while the check stays green and the invariant is gone. Look for any udpipe_rs type appearing in a signature outside that file.

**Changing it.** Do not. If a second NLP backend arrives it gets its own adapter file with its own panic boundary, which is the pattern working, not an exception to it.

## Rule 5 — `metrics/` and `extraction/` import only from `domain` and `stopwords`

**Why.** These are pure functions over already-parsed structure. Purity is what lets them run with no model loaded, be unit-tested without fixtures, and be called by a consumer who parsed elsewhere. It is what makes the public `parse` function's parse-once-use-many contract real.

**What breaks.** A metric that reaches for an `NlpProvider` re-parses internally. The caller who already parsed now pays twice, and the documented parse-once pattern quietly becomes a lie. Tests in these modules start needing a 16 MB model download.

**Read for.** Any `use crate::` in `src/metrics/` or `src/extraction/` naming something beyond `domain` and `stopwords`. A function in these trees taking `&dyn NlpProvider` or `&str` raw text rather than `&[Sentence]`.

## Rule 6 — `cargo check --no-default-features` must compile

**Why.** This is the mechanical proxy for "features are additive and the core stands alone." It proves domain plus ports compile with no UDPipe, which is precisely the configuration the WASM crust and any type-only consumer need.

**What breaks.** Default-feature entanglement. A consumer who wants the types without the C++ dependency cannot have them, and the WASM crust loses the foothold the spike established.

**Read for.** Nothing by eye. This one has a real gate in `ci.yml`. Trust it, and do not add code paths that only compile with `udpipe` enabled outside `#[cfg(feature = "udpipe")]`.

## Rule 7 — the composition root (`lib.rs`) is the only place that knows all adapters and ports

**Why.** Knowledge of the whole assembly is a cost you pay once. Concentrating it in one file means a reader learns how vaani is wired by reading one file, and a new adapter is a one-file wiring change.

**What breaks.** Two files that both know the full wiring drift. Adding an adapter silently becomes an N-place edit, and the places that were missed become the bugs.

**Read for.** Any file other than `lib.rs` importing from two or more adapter modules. A helper that matches on `Format` to select a decomposer, outside the composition root. Test modules are exempt.

## Rule 8 — `tracing` is forbidden in `domain.rs` and port modules

Burner amendment, 2026-04-28, from `book/src/plans/i3-error-tracing.md` (task step: "Update CLAUDE.md to record the rule 8 amendment"). It went unrecorded until 2026-08-02, when the summary line was added to `CLAUDE.md`. The motivation lives here.

**Why.** Observability is an adapter-tier and composition-root concern. A domain type that emits spans holds an opinion about the host's runtime and subscriber configuration. A port that traces forces that opinion onto every implementor. It is also rule 1 by another route: tracing in `domain.rs` is a fourth dependency.

**What breaks.** Consumers inherit a logging framework they did not choose. WASM builds pull a subscriber stack they cannot use.

**Read for.** `use tracing` or `tracing::` in `domain.rs` or the three port files. Note this rule is currently preemptive: `tracing` is not a dependency at all. It exists because i3 planned to instrument the adapters and the line was drawn before the first import landed.

---

## How to review against this list

For each changed file, the rules in scope follow from where the file sits:

| File touched | Rules in scope |
|---|---|
| `src/domain.rs` | 1, 8 |
| `src/{source,decompose,nlp}/mod.rs` | 2, 3, 8 |
| `src/nlp/udpipe.rs` | 4, 6 |
| other adapters | 6, 7 |
| `src/metrics/**`, `src/extraction/**` | 5, 6 |
| `src/lib.rs` | 6, 7 |
| `Cargo.toml` | 1, 6 |

Then ask the motivation question rather than the pattern question. Not "does this line match a forbidden string" but:

1. **Who inherits this?** Every dependency and every import in domain or a port propagates to implementors and to all three language crusts.
2. **What panics, and where does it land?** Any new path toward C or C++ code needs its own `catch_unwind` seam or it aborts the host.
3. **Who else now knows the whole?** If a second file learned the full wiring, rule 7 has already been broken even if nothing imports anything forbidden.
4. **Could this be spelled differently and still be wrong?** Re-exports, grouped imports, type aliases, and inline qualified paths all evade the mechanical check.

A violation is a merge blocker. The remedy is a fix to the structure, or an ADR that changes the rule deliberately. It is never a change to the check.
