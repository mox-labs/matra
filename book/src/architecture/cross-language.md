# Cross-language story

vaani publishes two artifacts from one codebase: a Rust crate (`vaani` on crates.io) and a Python wheel (`vaani` on PyPI). A WASM crust for TypeScript/browser is planned. The mechanism that makes this possible without duplicating the library is PyO3 and maturin.

## The governing constraint: names cross, methods don't

Every domain type in `src/domain.rs` is serializable via `serde`. When Python calls `vaani.analyze(text)`, the Rust side runs the full analysis pipeline, then `pythonize::pythonize(py, &analysis)` converts the `Document` struct into a Python dict following the same field names and nesting structure. Python gets exactly what the struct holds, not computed methods.

This is a deliberate architectural constraint, not an oversight.

`Document::passive_ratio()` is a Rust method that computes a fraction from the sentence slice. It is not a field on `Document`. Python does not see it. If a Python consumer wants a passive ratio, they compute it from the sentence-level data they do have, or vaani materializes a pre-computed field on a summary type.

The reasoning: methods can call other methods, navigate internal structure, check invariants, iterate. Fields are data. FFI boundaries are data boundaries. The moment a value crosses from Rust to Python (or to TypeScript via WASM), it stops being a live object with behavior and becomes a plain data structure. If a consumer needs a computed value, it must be computed on the Rust side and materialized into a field before the boundary is crossed.

This constraint scales. Every future cross-language surface (WASM, Node.js, a hypothetical Java binding) inherits the same discipline. The `Document` fields are the stable cross-language contract. The Rust methods are conveniences for Rust consumers.

## The Python surface

The Python surface is defined in two places that must stay in lockstep.

**`src/lib.rs`**: the PyO3 `#[pyclass]` and `#[pymethods]` blocks. The `Vaani` class has six methods: `from_path`, `english` (both static constructors), `analyze`, `analyze_markdown`, `tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`. Each method calls the corresponding Rust function and pipes the result through `pythonize`. The `Vaani` pyclass is marked `#[pyclass(unsendable)]` because the underlying UDPipe model holds C-side state that is not thread-safe. `unsendable` prevents accidental cross-thread use at runtime while leaving multi-process use (e.g., `ProcessPoolExecutor`) safe.

**`python/vaani/_core.pyi`**: the type stub file. This file describes the same surface that the Rust code exposes, using Python type annotations. mypy reads this stub to type-check Python code that calls vaani. Adding a `#[pymethod]` in Rust without updating the stub leaves the Python type checker in the dark about what arguments the method accepts and what it returns. Keeping these in lockstep is a manual discipline enforced by the pre-release checklist.

The public Python package (`python/vaani/__init__.py`) re-exports `Vaani` from the extension module and the `TypedDict` types from `python/vaani/types.py`. The extension module itself is named `vaani._core` (set in `pyproject.toml` under `module-name`); consumers import from `vaani` and never see `_core` directly.

## The build toolchain

**maturin** compiles the Rust crate with the `python` and `udpipe` feature flags active, links the resulting shared library into a Python wheel, and handles the packaging metadata from `pyproject.toml`. The build backend declaration is:

```toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"
```

The version range `>=1.0,<2.0` is intentionally wider than the pyo3 and pythonize pins because maturin's job is build tooling, not ABI. A maturin minor bump does not change the compiled extension's interface.

**pythonize** (`version = "0.28"`) converts `serde`-serializable Rust types to Python objects. It traverses the `serde` representation: structs become dicts, `Vec<T>` becomes lists, `Option<T>` becomes `None` or the value, enums serialize to their `serde` form. The field names in the Python dict match the Rust field names exactly, which is why field naming in `src/domain.rs` is a public contract, not an implementation detail.

**PyO3** (`version = "0.28"`) provides the macros that generate Python-callable Rust code. The pyo3 and pythonize version pins are locked together at "0.28" because the two crates share GIL management conventions. A version mismatch produces compilation errors; update both in the same commit.

## Error routing across the boundary

Rust errors must become Python exceptions with the right type. The `VaaniError` wrapper in `src/lib.rs` bridges `domain::Error` variants to Python exception classes:

| `domain::Error` variant | Python exception |
|---|---|
| `ModelNotFound(_)` | `FileNotFoundError` |
| `InputTooLarge` / `UnsupportedFormat` | `ValueError` |
| `Io(_)` | `OSError` |
| `ModelInvalid` / `ParseFailed` | `RuntimeError` |

The match in `From<VaaniError> for PyErr` is exhaustive without a wildcard. Adding a new variant to `domain::Error` is a compile error until it is wired into this impl with a chosen Python exception class. This is intentional: new error conditions should not silently route to `RuntimeError` because the author forgot to classify them.

The mapping also follows Python convention. A Python caller who catches `FileNotFoundError` when calling `Vaani.from_path` gets the expected behavior: the same exception class they would get from `open()` on a missing file.

## Version pin discipline

When pyo3 and pythonize release a new compatible version:

1. Update both `pyo3` and `pythonize` together in `Cargo.toml`. They must match.
2. Update `maturin` in `pyproject.toml` only if the new pyo3 version requires a newer maturin.
3. Run `maturin build --release` to verify the wheel builds cleanly.
4. Run `maturin develop` and import vaani in Python to verify the extension loads.
5. Run `mypy python/vaani/` to verify the stub is still consistent with the Rust surface.

## The WASM crust (planned 🛠️)

The WASM crust will expose the same domain types to TypeScript running in a browser or Node.js environment. The mechanism is different: `wasm-bindgen` rather than PyO3, and `serde-wasm-bindgen` rather than `pythonize`. The same constraint applies: methods do not cross; fields do.

The blocker is the NLP provider. The current `Udpipe` adapter uses C FFI and cannot run in a WASM sandbox. The WASM crust requires either a WASM-compiled variant of UDPipe or an alternative `NlpProvider` implementation that performs parsing server-side and returns annotated output. Until that provider exists, the WASM surface would expose all of vaani except the most important part: the structured parse.

See [Future direction](./future-direction.md) for the trigger conditions and what needs to be true before the WASM crust lands.
