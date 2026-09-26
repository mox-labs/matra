# Boundary rules

Eight rules hold matra's hexagonal architecture in place.

`CLAUDE.md` carries a summary list and points here for the reasoning.

## What enforcement means here

Rust offers no directional import control between modules inside a single crate, and matra is a single crate by [RFC-0004](https://github.com/mox-labs/matra/blob/main/blueprints/rfcs/0004-stay-single-crate.md), so the compiler enforces none of these rules except rule 6. Seven of the eight are checked by semgrep rules in `.semgrep/`, which read the import and path forms each rule forbids. Review remains the gate for what an import cannot show.

| Rule | Checked by | What the check catches | What review still reads for |
|---|---|---|---|
| 1. Domain dependency set | `boundary-rule1-*` | any `use` or inline path in `src/domain.rs` rooted outside `std`, `core`, `alloc`, `serde`, `thiserror` | a `#[macro_use]` macro used unqualified |
| 2. Ports import only domain | `boundary-rule2-*` | `crate::` or `super::` into anything but `domain`, external crates, the port's own adapters | a trait whose shape leaks an adapter |
| 3. No cross-port import | `boundary-rule3-*` | a port named by `use`, brace group, inline path or trait bound | macro-generated paths |
| 4. Single `udpipe_rs` importer | `boundary-rule4-*` | `udpipe_rs` named anywhere else, and anything `pub` in the adapter that names it | a private alias made public under another name |
| 5. Metrics and extraction purity | `boundary-rule5-*` | any crate module other than `domain` and `stopwords` | a function taking text instead of parsed structure |
| 6. No-default-features build | CI | the whole rule, by compiling | |
| 7. Composition root knows the whole | `boundary-rule7-*` | `src/cli/` reaching past `Engine`, `Ingest`, `extraction`, `config` and `domain` | wiring outside `src/lib.rs` in any other file |
| 8. No `tracing` in domain or ports | `boundary-rule8-*` | the identifier in code in the five files | |

Each finding names its rule, says why the boundary exists, and links the section of this page to read. The full table, with every form each check covers and misses and the reasons the checks are built the way they are, is [EP-0014](https://github.com/mox-labs/matra/blob/main/blueprints/eps/0014-architecture-guardrails.md). Code inside `#[cfg(test)]` modules is exempt from rules 1, 2, 3, 5 and 7, and read by rules 4 and 8.

`scripts/check-boundaries.sh` runs the checks: first each rule file against its fixture of violating and allowed code, then the scan of `src/`, then a check that the scan read every Rust file there. It runs from `just check`, from the pre-commit hook that `scripts/install-hooks.sh` installs, and from the `Boundary check` job in `ci.yml`. The hook is opt-in, runs on every commit regardless of which files are staged, and skips the check with a warning when semgrep is not installed; CI does not skip it.

Rule 6 is the only rule CI verifies by compiling. The `rust` job runs `cargo check`, `cargo clippy`, and `cargo test` with and without default features, on Linux and macOS. The `msrv` job runs `cargo check` on Rust 1.88, once with all features and once with no default features. CI fires on pushes to `main` and `alpha` and on pull requests targeting them, so work on a feature branch is ungated until the pull request opens.

## Rule 1: the domain dependency set

**The rule.** `src/domain.rs` depends on `serde`, `thiserror`, and `std`, and on nothing else.

**Scope.** One file, plus the non-optional entries in `[dependencies]`.

**Why it is drawn there.** Domain types are what every language surface serializes. A dependency added here enters the closure of every caller on every target. Changing the set takes an RFC. `thiserror` was admitted that way: it emits no public API and replaced roughly 35 lines of hand-written `Display` and `Error` implementations.

**Enforcement.** `boundary-rule1-domain-imports` fails on any `use` or `extern crate` in `src/domain.rs` rooted outside `std`, `core`, `alloc`, `serde` and `thiserror`, including `crate::`, `super::` and `self::`: the domain imports nothing from the rest of matra. `boundary-rule1-domain-inline-paths` fails on an inline path with any other lowercase root, because a crate listed in `Cargo.toml` can be named inline with no `use` line; for that reason the domain writes std paths in full (`std::fmt::Display`). A dependency added to `[dependencies]` and used in `domain.rs` is therefore caught wherever `domain.rs` names it. Review reads for the one route left, a macro or derive brought into scope crate-wide by `#[macro_use]` in `src/lib.rs` and invoked unqualified.

## Rule 2: ports import only from domain

**The rule.** Each port module imports from `crate::domain` and `std`, and from no other module or crate.

**Scope.** `src/source/mod.rs`, `src/decompose/mod.rs`, `src/nlp/mod.rs`, `src/embed/mod.rs`.

**Why it is drawn there.** A port is a contract. Whatever the contract imports becomes a requirement on everyone who implements it, so a domain-only port stays implementable by someone who has never read matra's adapters.

**Enforcement.** `boundary-rule2-port-imports` and `boundary-rule2-port-inline-paths` fail on any import or inline path in those four files that reaches past `crate::domain` and `std`: another crate module, a crate-root item such as `crate::Engine`, a glob, an external crate, or the port's own adapter through `self::`. In a port's `mod.rs`, `super::` is the crate root and is read as such. Review reads the trait method signatures for what an import cannot show: a contract shaped around one adapter's needs.

## Rule 3: no port module imports another port module

**The rule.** The four ports are peers and name each other nowhere.

**Scope.** The same four files.

**Why it is drawn there.** Stage order belongs to the composition root, not to the contracts. If `Decomposer` knew about `NlpProvider`, the pipeline's shape would be encoded in the traits and a stage could no longer be replaced on its own.

**Enforcement.** `boundary-rule3-port-names-port` fails when one of the four files names a port through `crate::` or `super::`: in a `use` line, a brace group (a comment inside a multi-line group does not hide it), a fully qualified inline path, or a trait bound. A type alias cannot launder the path, because the only module a port may import is the domain, and the domain may import nothing from the crate (rule 1). A crate-root re-export of another port's trait is caught by rule 2.

## Rule 4: one file imports `udpipe_rs`

**The rule.** `src/nlp/udpipe.rs` is the only file in the crate that imports `udpipe_rs`.

**Scope.** All of `src/`.

**Why it is drawn there.** This is a resilience rule. UDPipe is C++ across an FFI boundary holding state that is not `Send`, and a panic on the C side aborts the host process rather than unwinding, which means interpreter death in Python. The `catch_unwind` boundary in `nlp/udpipe.rs` converts that into a `domain::Error`. Confining the import is what makes that boundary the only entrance rather than one entrance among several. A second NLP backend gets its own adapter file with its own panic boundary, which is the pattern working rather than an exception to it.

**Enforcement.** `boundary-rule4-udpipe-rs-outside-adapter` fails on `udpipe_rs` anywhere in code under `src/` outside `src/nlp/udpipe.rs`, test modules included. `boundary-rule4-udpipe-rs-exposed` fails inside the adapter on anything `pub`, at any restriction, that names `udpipe_rs`: a re-export, a type alias, a signature or a field, any of which would let another file hold the C-backed type while the first check stays green. Review reads for the laundering route left: a private alias (`use udpipe_rs::Model as M;`) made public under another name.

**The analog.** `src/embed/model2vec.rs` is the only file that imports `safetensors` and `tokenizers`, enforced by the same pair of checks (`boundary-rule4-model2vec-crates-*`) with the same one route left to review. Those crates are pure Rust, so the confinement is not a C panic boundary; it is what keeps the adapter swappable and the model-format vocabulary out of every other file's reach. A second embedding backend gets its own adapter file with its own confinement line.

## Rule 5: metrics, extraction, and structure readers stay pure

**The rule.** No file under `src/metrics/` or `src/extraction/`, and no structure-reading module (`src/hearst.rs`), imports from any crate module other than `crate::domain` and `crate::stopwords`.

**Scope.** Intra-crate imports only. External crates that the computation itself needs are unaffected: the compression metric uses `brotli`, and several extraction files use `std::collections`.

**Why it is drawn there.** These are pure functions over already-parsed structure. Purity is what lets them run with no model loaded, be unit-tested without fixtures, and be called by someone who parsed elsewhere. It is also what makes the parse-once-use-many contract real: a metric that reached for an `NlpProvider` would re-parse internally, and a caller who had already parsed would pay twice.

**Enforcement.** `boundary-rule5-pure-imports` fails on any `crate::` path in those trees, in a `use` line, a brace group or inline, naming something beyond `domain` and `stopwords`, and on `super::super::`, which leaves the tree. `boundary-rule5-root-super` reads `super::` the same way in `src/metrics/mod.rs`, `src/extraction/mod.rs` and `src/hearst.rs`, where it is the crate root. Sibling imports within a tree (`super::textrank`) and external crates stay allowed. Review reads for what purity means rather than what it imports: any function there taking raw text instead of `&[Sentence]`.

## Rule 6: the no-default-features build compiles

**The rule.** `cargo check --no-default-features` succeeds.

**Scope.** The whole crate.

**Why it is drawn there.** This is the mechanical proxy for features being additive and the core standing alone. It proves that the domain and the ports compile with no UDPipe, which is the configuration a type-only caller needs.

**Enforcement.** CI, as described above. Code that only compiles with `udpipe` enabled belongs behind `#[cfg(feature = "udpipe")]`.

## Rule 7: the composition root knows the whole

**The rule.** `src/lib.rs` is the only place that knows all adapters and all ports.

**Scope.** Every file except `src/lib.rs`. Test modules are exempt.

**Why it is drawn there.** Knowledge of the full assembly is a cost paid once. Concentrating it in one file means a reader learns how matra is wired by reading one file, and adding an adapter is a one-file change. Two files that both know the wiring drift, and the place that was missed becomes the bug.

**Enforcement.** For `src/cli/`, the checks described below. Everywhere else, review: read for any file other than `lib.rs` importing from two or more adapter modules, and for any helper outside the composition root that matches on `Format` to pick a decomposer.

`src/config.rs` sits in this tier alongside `lib.rs`. It imports `domain`, `std`, `serde` and `toml`, and it imports no port and no adapter. The traffic runs the other way: an adapter may import `Config` to offer a `from_config` constructor (RFC-0011), which is why `Udpipe::from_config` lives in `src/nlp/udpipe.rs` and `Model2Vec::from_config` in `src/embed/model2vec.rs`, not in the composition root. That import gives the adapter a default, not a second opinion about the wiring, so rule 7 still holds: `lib.rs` remains the only file that knows every adapter and every port.

`src/cli/` sits above that tier: it is the application, compiled into the library so both launchers run one program. From the crate it uses the public surface `lib.rs` exports (`Engine`, `Ingest`), the `extraction` functions (`tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`), `config` and `domain`, and never a port module or an adapter. It reaches the pipeline through `Engine::from_config`, so rule 7 holds there too.

`boundary-rule7-cli-imports` reads both spellings of a violation, because the inline one is the one it would most likely take. The declared form is a `use crate::...` line in `src/cli/`; the inline form is a qualified path in a function body. The CLI already reaches `Engine`, `Ingest` and the four extractors by inline path rather than by `use`, so the inline form is the ordinary idiom here and the one a violation would blend into. The check is an allowlist: any `crate::` path in `src/cli/` must start with `Engine`, `Ingest`, `extraction`, `config`, `domain` or `cli`, so a port, an adapter, or a crate-root re-export of an adapter fails it. `boundary-rule7-cli-root-super` reads `super::` in `src/cli/mod.rs`, where it is the crate root, the same way. Either spelling would put adapter selection in the command line, which is the composition root's job.

## Rule 8: no `tracing` in the domain or the ports

**The rule.** `tracing` is forbidden in `src/domain.rs` and in the four port modules.

**Scope.** Those five files.

**Why it is drawn there.** Observability is an adapter and composition-root concern. A domain type that emits spans holds an opinion about the host's runtime and subscriber configuration, and a port that traces forces that opinion onto every implementor. In `domain.rs` it is also rule 1 by another route, since it would be a fourth dependency.

**Enforcement.** `boundary-rule8-no-tracing` fails on the identifier `tracing` anywhere in code in those five files, test modules included, whether in a `use` line, a brace group, a path or an attribute. The rule is preemptive: `tracing` is not a dependency of matra at all. The line was drawn before the first import could land.

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
pip install --require-hashes -r .github/requirements/semgrep.txt  # once
just boundary      # scripts/check-boundaries.sh on its own
just check         # every local gate, including the boundary check
just install-hooks # install the pre-commit hook that runs it
```

The check needs semgrep, at the version `.github/requirements/semgrep.txt` pins, and must run from the root of a git work tree: semgrep resolves each rule's paths against the work tree, and outside one a path-scoped rule matches nothing. Each finding prints the file, the line and the rule's message. On a clean tree the script ends with `check-boundaries: N of N Rust files in src/ read, 0 finding(s)`, and it exits non-zero on any finding, any failed rule test, or any Rust file under `src/` the scan did not read.

A violation is a merge blocker. The remedy is a change to the structure, or an RFC that changes the rule deliberately. It is never a change to the check. A check that flags correct code is a bug in the check: fix the rule and add the case to its fixture, as [EP-0014](https://github.com/mox-labs/matra/blob/main/blueprints/eps/0014-architecture-guardrails.md) describes.
