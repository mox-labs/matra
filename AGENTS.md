# AGENTS.md

matra is an NLP library: text in, structured analysis out. A Rust core, Python
bindings, and one command line reachable from either. This file is for an
agent contributing to matra; an agent about to *use* matra should run
`matra --skill`, which prints what the installed program does.

## Build and gates

```bash
just check                                   # the gate: every CI check, locally
cargo test                                   # unit tests and doctests
cargo test --features cli                    # the command line and the skill test
cargo test --test integration -- --ignored   # needs the UDPipe model
just conformance                             # every crust against spec/tests/
just docs-floor                              # the twelve docsite gates
```

Do not run `cargo test --all-features`. It turns on `python`, which links
against libpython with symbols left undefined until an interpreter loads them,
so it fails at link with a symbol error that reads like a regression and is not.

`site/content/llms.txt` is generated and committed, and gate 6 of the docsite floor
diffs it. Run `scripts/gen-llms-txt.sh` rather than editing it.

## Boundary rules

Eight rules hold the hexagonal architecture in place. Rule 6 is compiled on
every push; rules 1, 2, 3, 4, 5, 7 (its `src/cli/` part) and 8 are semgrep
checks in `.semgrep/`, run by `scripts/check-boundaries.sh`. The checks read
import forms, so review stays the gate for intent.

1. `domain.rs` depends only on `serde`, `thiserror` and `std`.
2. Port modules import only from `domain`.
3. No port module imports another port module.
4. `nlp/udpipe.rs` is the only file that imports `udpipe_rs`.
5. `metrics/` and `extraction/` import only from `domain` and `stopwords`.
6. `cargo check --no-default-features` must compile.
7. `lib.rs` is the only place that knows all adapters and ports.
8. `tracing` is forbidden in `domain.rs` and port modules.

What each one is for, what breaks when it is violated, and what to read for
when reviewing: [`site/content/reference/boundary-rules.md`](site/content/reference/boundary-rules.md).

## Proposing a change

Which record a change takes is in [`blueprints/README.md`](blueprints/README.md).
A substantial change matra's users notice (the public surface in Rust, Python,
the command line or the JSON schema; semantics or behaviour; removing a
substantial feature; an architecture boundary) is an enhancement proposal
(`EPR-NNNN`), proposed as a pull request. A minor addition or removal is an
API change proposal, an issue labelled `acp`. Tooling, docs, CI, refactors, measured performance work and
bug fixes are plain pull requests.

Proposals and enhancement plans (`EPL-NNNN`, only for work several agents
carry out in parallel) are numbered from 0001, from the 2026-10-04 baseline.
The RFCs and EPs written before it are in `blueprints/legacy/`, unchanged, as
the reasons behind the code that exists; `RFC-NNNN` and `EP-NNNN` cite them.

One milestone per pull request, in the order the proposal's tracking issue states.
Conventional commits (`feat`, `fix`, `docs`, `chore`, `refactor`, `perf`,
`test`, `ci`, `build`), and a commit body that says why, not what. Update
`CHANGELOG.md` under `[Unreleased]` in the same PR as the code. A review harness
runs on the pull request and its findings are applied before merge.

## Who decides

- **Only the owner accepts a decision.** An agent writes and argues
  enhancement proposals and API change proposals, and never accepts one. A
  proposal may merge with its status `proposed`, which an agent may merge
  so it renders on the docsite for review; acceptance is the change of its
  status line to `accepted`, and only the owner merges a pull request that
  makes it.
- **Everything else merges on standing authority.** Claude merges code, docs,
  tooling and dependency updates once CI is green and the review raises no
  blockers, and leaves a comment with its rationale as the audit trail.
- **An agent never approves a deployment environment.** A release stops at the
  `crates-io` and `pypi` environments until the owner approves each; that
  approval is the release decision. An agent may prepare and dispatch a release,
  and never clicks through its gates.

## Where to read next

- [`CLAUDE.md`](CLAUDE.md): the architecture, the conventions, and the
  non-obvious behaviors that will bite you.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): the working model, how decisions get
  made, how releases work, the full PR mechanics.
- [`blueprints/`](blueprints/README.md): the enhancement proposals that
  record each design decision, the legacy records behind the code that
  exists, and the process that says which record a change takes.
- [How matra is maintained](site/content/explanation/how-matra-is-maintained.md):
  the agents, the gates, and who decides what.
