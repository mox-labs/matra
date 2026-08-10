# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
Per-release sections follow this shape:

  ## [X.Y.Z] - YYYY-MM-DD

  ### Highlights

  Two to four prose entries explaining the load-bearing changes for
  this release. Each entry teaches the mental model the change is
  built on, not just what shipped. Reserved for architectural decisions,
  breaking changes, security-relevant fixes, and deferred-vs-shipped
  tradeoffs. Bug fixes and minor refactors live in the structured
  sections below, not here.

  ### Added / Changed / Deprecated / Removed / Fixed / Security

  Terse Keep-a-Changelog bullets for everything else.

Style: no em dashes (project convention).
Rollover: scripts/changelog-release.sh moves [Unreleased] -> [X.Y.Z]
at release time. It does not touch Cargo.toml or pyproject.toml; bump
those by hand.
-->

## [Unreleased]

Nothing released yet. matra is pre-0.1.0 and unpublished on crates.io and PyPI.

### Highlights

**The project is now called matra.** The previous name collided with an existing package on PyPI, which makes dual publishing to crates.io and PyPI under one name impossible. The name is the public contract across Rust, Python and a future TypeScript crust, so the collision had to be resolved before the first release rather than after. There are no consumers and nothing has been published, so the change carries no aliases, shims or deprecations.

**A command-line interface ships behind the `cli` feature.** The library returns typed errors and structured data; the binary decides rendering, exit codes, and what to do when input is missing. Exit codes follow the ripgrep convention, so nothing-found is 1 and a genuine failure is 2, and an empty document is not an error. `summarize` and `keyphrases` route through the same format detection `analyze` uses, so markdown headings and fenced code are never ranked as prose.

**Conformance fixtures now bind the crusts together.** matra ships one Rust core behind several bindings that all call the same parser, so a difference between them is never a difference of behaviour: it is a binding defect, a renamed field or a value that lost precision crossing over. `spec/tests/*.json` holds language-agnostic fixtures with one runner per language. The UDPipe model is part of the contract, so a model version change is a spec change.

### Added

- `matra` binary behind the `cli` feature: `analyze`, `summarize`, `keyphrases`, each accepting `--json`.
- Conformance suite: `spec/tests/*.json` with Rust and Python runners.
- `tests/cli.rs` covering argument handling, format detection, output shape and exit codes.
- `rust-toolchain.toml` pinning stable. The MSRV claim is verified separately by CI.
- `ROADMAP.md`, the single register of unbuilt capability and its trigger conditions, rendered into the book.
- `book/src/plans/`, the iteration plans, with per-milestone rubrics.
- Docsite floor gate 5: no em dashes outside quoted material.

### Changed

- Documentation rebuilt around what a reader needs first: what matra returns, the type graph, and how a call runs. The architecture page is written from source and organised by the call path rather than by the pattern it happens to instantiate.
- Diagrams are hand-authored inline SVG. Mermaid is not installed; `book/book.toml` records the rule for choosing between them and the command to restore it.
- Architecture prose consolidated into the book, which is gated, and out of `.claude/`, which is not.

### Fixed

- `cargo metadata` reported seven public features where three were intended. Bare optional-dependency names in `[features]` mint public features implicitly; the `dep:` prefix closes that.
- `summarize` and `keyphrases` read files raw and parsed them as plain text, so markdown headings and fenced code were ranked as prose. Regression test added.
- Documentation described `analyze_from` as taking a `Source` and a `Decomposer`. It takes neither: it is the parse-once entry point.
- `metrics::default_suite` claimed to return metrics in dependency order. There is no inter-metric dependency; each reads only `Document` state and writes distinct slots.
