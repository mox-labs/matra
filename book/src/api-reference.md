# API reference

The full Rust API documentation is on [docs.rs](https://docs.rs/vaani) or — if you are reading this book from the project's GitHub Pages site — the [generated rustdoc](./api/vaani/index.html) at this site's `/api/` subpath.

## docs.rs

[https://docs.rs/vaani](https://docs.rs/vaani)

Built automatically on every release. The `[package.metadata.docs.rs]` section in `Cargo.toml` ensures the build uses `--all-features`, so the PyO3 surface is documented alongside the Rust-only surface.

## Local

Build and open the rustdoc locally:

```bash
cargo doc --no-deps --all-features --open
```

The `--all-features` flag includes the PyO3 bindings (`vaani::python::Vaani` and friends) in the generated docs.

## Python

The Python type stubs are at `python/vaani/_core.pyi`. Type checkers (`mypy`, `pyright`) pick them up automatically via the `py.typed` marker. The stub file documents:

- The `Vaani` class methods.
- The TypedDict shapes for `Token`, `Sentence`, `Paragraph`, `Section`, `Analysis`, `ScoredSentence`, `Keyphrase`.
- Which Python exception classes each method can raise.

## CLI

`vaani --help` lists the subcommands. Each subcommand has its own `--help`:

```bash
vaani --help
vaani analyze --help
vaani summarize --help
vaani keyphrases --help
```
