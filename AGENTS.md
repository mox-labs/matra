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
just docs-floor                              # the six docsite gates
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

One milestone per pull request, in the order its plan states. Conventional
commits (`feat`, `fix`, `docs`, `chore`, `refactor`, `perf`, `test`, `ci`,
`build`), and a commit body that says why, not what. Update `CHANGELOG.md`
under `[Unreleased]` in the same PR as the code. A review harness runs on the
pull request and its findings are applied before merge; a human approves.

## Where to read next

- [`CLAUDE.md`](CLAUDE.md): the architecture, the conventions, and the
  non-obvious behaviors that will bite you.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): the working model, how decisions get
  made, how releases work, the full PR mechanics.
- [`blueprints/`](blueprints/README.md): the RFCs that record each design
  decision and the EPs that plan their implementation, with ship criteria.
