# Vaani

Prose metrics engine. Text in, structured analysis out. Readability, POS, dependency, lexical density, compression ratio.

## Architecture

Hex architecture. Rust core with PyO3 Python bindings. Single crate, dual publish: `vaani` on crates.io, `vaani` on PyPI via maturin.

Pipeline: Source -> Decompose -> Annotate (NLP) -> Encode -> Extract

Domain depends on port traits (NlpProvider, Decomposer, Source), not on adapters directly. UDPipe is the default NLP adapter, behind the `udpipe` feature flag.

```
src/
  lib.rs                    # composition root + PyO3 module (feature-gated)
  domain.rs                 # ALL domain types (zero internal deps, only serde)
  source/
    mod.rs                  # Source trait ONLY
    file.rs                 # FileSource adapter
    directory.rs            # DirectorySource adapter
  decompose/
    mod.rs                  # Decomposer trait ONLY
    markdown.rs             # MarkdownDecomposer adapter
    plain.rs                # PlainTextDecomposer adapter
  nlp/
    mod.rs                  # NlpProvider trait ONLY
    udpipe.rs               # UDPipe adapter (only file importing udpipe_rs)
  encoders.rs               # encoder pipeline (domain + stopwords only)
  extraction/
    mod.rs                  # re-exports
    tfidf.rs                # tfidf_summarize
    textrank.rs             # textrank_summarize
    rake.rs                 # rake_keyphrases
    yake.rs                 # yake_keyphrases
  stopwords.rs              # shared utility
  markdown.rs               # legacy re-export (use decompose::markdown)
python/vaani/
  __init__.py               # re-exports Vaani from _core
  cli.py                    # click + rich CLI, auto-downloads model
tests/
  integration.rs            # full pipeline tests (require UDPipe model)
examples/
  basic.rs                  # getting-started example
```

## Boundary Rules

1. `domain.rs` has zero internal dependencies (only serde, std). Everything depends on it.
2. Port modules (source/mod.rs, decompose/mod.rs, nlp/mod.rs) import only from domain.
3. No port module imports another port module.
4. `nlp/udpipe.rs` is the ONLY file that imports `udpipe_rs`.
5. `encoders.rs` and `extraction/` import only from domain and stopwords.
6. `cargo check --no-default-features` must compile.
7. Composition root (lib.rs) is the only place that knows all adapters and ports.

## Conventions

- `domain::Result<T>` everywhere. No `Result<T, String>`. No panics in library code.
- `impl AsRef<Path>` for file paths, not `&str`.
- Feature flags are additive. Enabling `udpipe` adds UDPipe; disabling it removes only UDPipe.
- Conventional commits.
- Tests: `#[cfg(test)]` for unit, `tests/` for integration, `examples/` for usage demos.
- No em dashes in documentation prose.

## Build

```bash
cargo build                                    # default (with udpipe)
cargo build --no-default-features              # without udpipe
cargo test                                     # unit tests
cargo test --test integration -- --ignored     # integration (needs model)
maturin develop                                # Python local install
maturin build                                  # Python wheel
```

## Agents

| Agent | When to use |
|-------|-------------|
| `maintainer` | Architectural decisions, adding features, fixing bugs, long-term maintenance |
| `reviewer` | PR reviews, boundary compliance audits, pre-release checks |
| `benchmarker` | Performance measurement, bottleneck analysis, optimization decisions |

## Skills

| Skill | When to use |
|-------|-------------|
| `rust-craft` | Any Rust design decision (trait design, error types, dependencies, feature flags) |
| `testing` | Writing tests, reviewing coverage, debugging test failures |
| `architecture` | Adding modules, creating adapters, extending encoders, boundary compliance |

## Mastery References

569 insights from 28 elite codebases at `~/oss/research/`:

| File | When |
|------|------|
| `synthesis.md` | Any architectural decision (97 judgment patterns) |
| `implementation.md` | How to build specific patterns (26 patterns) |
| `verification.md` | Testing strategy: loom, miri, fuzzing, property (20 patterns) |
| `performance.md` | Optimization (37 patterns, measure first) |
| `extension-systems.md` | Plugin/encoder architecture (6 codebases) |
| `pyo3-mastery.md` | Python bindings (Bound<'py,T>, pyclass, GIL) |
| `serde-mastery.md` | Serialization, error design, API stability |
| `ripgrep-mastery.md` | CLI patterns, measurement-driven design |
| `tauri-mastery.md` | Cross-platform desktop |
| `uniffi-rs-mastery.md` | Mobile FFI |
