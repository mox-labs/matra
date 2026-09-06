# Boundary rules

Eight rules hold matra's hexagonal architecture in place.

`CLAUDE.md` carries a summary list and points here for the reasoning.

## What enforcement means here

Rust offers no directional import control between modules inside a single crate, and matra is a single crate by [ADR-0004](https://github.com/mox-labs/matra/blob/main/docs/decisions/0004-stay-single-crate.md). No compiler mechanism is available for most of these rules. Reasoned review is the primary enforcement; a script and one CI job cover the mechanical cases.

| Rule | Enforced by | What that catches |
|---|---|---|
| 1. Domain dependency set | review | judgment only |
| 2. Ports import only domain | review | judgment only |
| 3. No cross-port import | `scripts/check-boundaries.sh` | the literal form `use crate::<port>` |
| 4. Single `udpipe_rs` importer | `scripts/check-boundaries.sh` | import lines, not re-exports |
| 5. Metrics and extraction purity | review | judgment only |
| 6. No-default-features build | CI | the whole rule, mechanically |
| 7. Composition root knows the whole | review | judgment only |
| 8. No `tracing` in domain or ports | `scripts/check-boundaries.sh` | `use tracing` and `tracing::` in five named files |

`scripts/check-boundaries.sh` runs from `just check` and from the pre-commit hook that `scripts/install-hooks.sh` installs. No CI workflow invokes it, and the hook is opt-in. The hook runs the script on every commit regardless of which files are staged.

Rule 6 is the only rule with a CI gate. The `rust` job runs `cargo check`, `cargo clippy`, and `cargo test` with and without default features, on Linux and macOS. The `msrv` job runs `cargo check` on Rust 1.85, once with all features and once with no default features. CI fires on pushes to `main` and `alpha` and on pull requests targeting them, so work on a feature branch is ungated until the pull request opens.

Rule 6 also catches a subset of rules 1, 2, and 5: a violation that reaches for a feature-gated dependency fails the no-default-features build. A violation that adds an unconditional dependency compiles cleanly, and review is the only thing that catches it.

## Rule 1: the domain dependency set

**The rule.** `src/domain.rs` depends on `serde`, `thiserror`, and `std`, and on nothing else.

**Scope.** One file, plus the non-optional entries in `[dependencies]`.

**Why it is drawn there.** Domain types are what every language surface serializes. A dependency added here enters the closure of every caller on every target. Changing the set takes an ADR. `thiserror` was admitted that way: it emits no public API and replaced roughly 35 lines of hand-written `Display` and `Error` implementations.

**Enforcement.** Review. Read the `use` lines at the top of `src/domain.rs`, new non-optional entries in `[dependencies]`, and any domain field whose type comes from outside the three allowed crates.

## Rule 2: ports import only from domain

**The rule.** Each port module imports from `crate::domain` and `std`, and from no other module or crate.

**Scope.** `src/source/mod.rs`, `src/decompose/mod.rs`, `src/nlp/mod.rs`, `src/embed/mod.rs`.

**Why it is drawn there.** A port is a contract. Whatever the contract imports becomes a requirement on everyone who implements it, so a domain-only port stays implementable by someone who has never read matra's adapters.

**Enforcement.** Review. Read the import block and the trait method signatures in those four files.

## Rule 3: no port module imports another port module

**The rule.** The four ports are peers and name each other nowhere.

**Scope.** The same four files.

**Why it is drawn there.** Stage order belongs to the composition root, not to the contracts. If `Decomposer` knew about `NlpProvider`, the pipeline's shape would be encoded in the traits and a stage could no longer be replaced on its own.

**Enforcement.** `scripts/check-boundaries.sh` greps the four port files for `use crate::source`, `use crate::decompose`, `use crate::nlp`, and `use crate::embed`. The check sees only that literal form. Grouped imports, fully qualified inline paths with no `use` line, a trait bound naming another port's trait, and a type alias that launders the path all pass the script and still violate the rule.

## Rule 4: one file imports `udpipe_rs`

**The rule.** `src/nlp/udpipe.rs` is the only file in the crate that imports `udpipe_rs`.

**Scope.** All of `src/`.

**Why it is drawn there.** This is a resilience rule. UDPipe is C++ across an FFI boundary holding state that is not `Send`, and a panic on the C side aborts the host process rather than unwinding, which means interpreter death in Python. The `catch_unwind` boundary in `nlp/udpipe.rs` converts that into a `domain::Error`. Confining the import is what makes that boundary the only entrance rather than one entrance among several. A second NLP backend gets its own adapter file with its own panic boundary, which is the pattern working rather than an exception to it.

**Enforcement.** `scripts/check-boundaries.sh` searches `src/` for `use udpipe_rs` and `udpipe_rs::`, excluding `src/nlp/udpipe.rs`. It cannot see re-exports: a `pub use udpipe_rs::Model;` inside the adapter would let any other file name the C-backed type while the check stays green. Review reads for re-exports and for any `udpipe_rs` type appearing in a signature outside that file.

**The analog.** `src/embed/model2vec.rs` is the only file that imports `safetensors` and `tokenizers`, enforced by the same script with the same re-export blind spot. Those crates are pure Rust, so the confinement is not a C panic boundary; it is what keeps the adapter swappable and the model-format vocabulary out of every other file's reach. A second embedding backend gets its own adapter file with its own confinement line.

## Rule 5: metrics, extraction, and structure readers stay pure

**The rule.** No file under `src/metrics/` or `src/extraction/`, and no structure-reading module (`src/hearst.rs`), imports from any crate module other than `crate::domain` and `crate::stopwords`.

**Scope.** Intra-crate imports only. External crates that the computation itself needs are unaffected: the compression metric uses `brotli`, and several extraction files use `std::collections`.

**Why it is drawn there.** These are pure functions over already-parsed structure. Purity is what lets them run with no model loaded, be unit-tested without fixtures, and be called by someone who parsed elsewhere. It is also what makes the parse-once-use-many contract real: a metric that reached for an `NlpProvider` would re-parse internally, and a caller who had already parsed would pay twice.

**Enforcement.** Review. Read for any `use crate::` in those trees naming something beyond `domain` and `stopwords`, and for any function there taking `&dyn NlpProvider` or raw text instead of `&[Sentence]`.

## Rule 6: the no-default-features build compiles

**The rule.** `cargo check --no-default-features` succeeds.

**Scope.** The whole crate.

**Why it is drawn there.** This is the mechanical proxy for features being additive and the core standing alone. It proves that the domain and the ports compile with no UDPipe, which is the configuration a type-only caller needs.

**Enforcement.** CI, as described above. Code that only compiles with `udpipe` enabled belongs behind `#[cfg(feature = "udpipe")]`.

## Rule 7: the composition root knows the whole

**The rule.** `src/lib.rs` is the only place that knows all adapters and all ports.

**Scope.** Every file except `src/lib.rs`. Test modules are exempt.

**Why it is drawn there.** Knowledge of the full assembly is a cost paid once. Concentrating it in one file means a reader learns how matra is wired by reading one file, and adding an adapter is a one-file change. Two files that both know the wiring drift, and the place that was missed becomes the bug.

**Enforcement.** Review. Read for any file other than `lib.rs` importing from two or more adapter modules, and for any helper outside the composition root that matches on `Format` to pick a decomposer.

`src/config.rs` sits in this tier alongside `lib.rs`. It imports `domain`, `std`, `serde` and `toml`, and it imports no port and no adapter. The traffic runs the other way: an adapter may import `Config` to offer a `from_config` constructor (ADR-0011), which is why `Udpipe::from_config` lives in `src/nlp/udpipe.rs` and `Model2Vec::from_config` in `src/embed/model2vec.rs`, not in the composition root. That import gives the adapter a default, not a second opinion about the wiring, so rule 7 still holds: `lib.rs` remains the only file that knows every adapter and every port.

`src/cli/` sits above that tier: it is the application, compiled into the library so both launchers run one program. From the crate it uses the public surface `lib.rs` exports (`Engine`, `Ingest`), the `extraction` functions (`tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`), `config` and `domain`, and never a port module or an adapter. It reaches the pipeline through `Engine::from_config`, so rule 7 holds there too.

Read for both spellings of a violation, because a grep over `use` lines alone is blind to the shape it would most likely take. The declared form is `use crate::nlp`, `use crate::source`, `use crate::decompose` or `use crate::embed` at the top of a file in `src/cli/`. The inline form is a qualified path in the body: `crate::nlp::`, `crate::source::`, `crate::decompose::` or `crate::embed::`. The CLI already reaches `Engine`, `Ingest` and the four extractors by inline path rather than by `use`, so the inline form is the ordinary idiom here and the one a violation would blend into. Either form would put adapter selection in the command line, which is the composition root's job.

## Rule 8: no `tracing` in the domain or the ports

**The rule.** `tracing` is forbidden in `src/domain.rs` and in the four port modules.

**Scope.** Those five files.

**Why it is drawn there.** Observability is an adapter and composition-root concern. A domain type that emits spans holds an opinion about the host's runtime and subscriber configuration, and a port that traces forces that opinion onto every implementor. In `domain.rs` it is also rule 1 by another route, since it would be a fourth dependency.

**Enforcement.** `scripts/check-boundaries.sh` greps those five files for `use tracing` and for `tracing::`. The rule is preemptive: `tracing` is not a dependency of matra at all. The line was drawn before the first import could land.

## Which rules apply to which files

| File touched | Rules in scope |
|---|---|
| `src/domain.rs` | 1, 8 |
| `src/source/mod.rs`, `src/decompose/mod.rs`, `src/nlp/mod.rs`, `src/embed/mod.rs` | 2, 3, 8 |
| `src/nlp/udpipe.rs` | 4, 6 |
| Other adapters | 6, 7 |
| `src/config.rs` | 6, 7 |
| `src/cli/` | 7 |
| `src/metrics/`, `src/extraction/`, `src/hearst.rs` | 5, 6 |
| `src/lib.rs` | 6, 7 |
| `Cargo.toml` | 1, 6 |

## Running the checks

```bash
just boundary      # scripts/check-boundaries.sh on its own
just check         # every local gate, including the boundary check
just install-hooks # install the pre-commit hook that runs it
```

The script prints the offending files and exits non-zero on any failure, and prints `boundary checks pass (rules 3, 4, 8)` otherwise.

A violation is a merge blocker. The remedy is a change to the structure, or an ADR that changes the rule deliberately. It is never a change to the check.
