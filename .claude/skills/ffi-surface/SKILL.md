---
name: ffi-surface
description: >-
  PyO3 dual-publish discipline for matra, and the one list of what the Python module exposes:
  unsendable and frozen, Bound<'py, T>, the four pythonize blind spots, the maturin dual-manifest
  contract, the pin rule, and From<domain::Error> for PyErr routing. Use when touching the Python
  bindings, python/matra/ or pyproject.toml, or planning the WASM/TS crust. Not for: Rust-only
  surface that never crosses into Python.
---

# ffi-surface

The Rust↔Python FFI surface for matra. This skill codifies the disciplines that hold the dual-publish together.

## When to invoke

- Adding a method to the PyO3 `Matra` class.
- Bumping pyo3, pythonize, or maturin.
- Changing module-name, python-source, or any maturin config.
- Adding a new error variant to `domain::Error` (requires PyErr routing update).
- Planning the WASM/TS crust.

## The PyO3 surface — what exists today

The `_core` module in `src/lib.rs::python` (matched by pyproject.toml's `module-name = "matra._core"`) registers two classes and two functions. This section is the one place in `.claude/` that lists them; the `ffi-keeper` agent points here.

- **`Matra`** (`src/lib.rs:461`), `#[pyclass(unsendable)]`: UDPipe is `!Send` due to internal C state, so cross-thread access panics at runtime.
  - Constructors: `from_path(model_path)`, `english(model_dir=None)` (both gated on the `udpipe` feature).
  - Methods: `analyze`, `analyze_markdown`, `analyze_path`, `tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`, `semantic_clusters`.
- **`Model2Vec`** (`src/lib.rs:676`), `#[pyclass(frozen)]`, registered only under the `model2vec` feature: an immutable embedder.
  - Constructors: `from_dir(dir)`, `potion_base_8m(dir=None)`.
  - Getters `model_hash` and `dimensions`; methods `identity` and `embed`.
- **Functions**: `semantic_clusters` (clusters vectors the caller already holds) and `cli_main` (the command line, which `python/matra/cli.py` launches).
- Error routing: `MatraError` wrapper + `From<MatraError> for PyErr` with exhaustive variant match (below).

## The four PyO3 disciplines


### 1. `'py` lifetime threading

`Python<'py>` is a ZST proof-token. `Bound<'py, T>` embeds the `'py` lifetime as the GIL-attachment proof. The lifetime IS the safety check; raw pointer access (gil-refs) was removed in pyo3 0.28.

**Rule**: every PyO3 method uses `Bound<'py, ...>` types in its return position and `Python<'py>` parameter when needed. No `&PyAny`. No `Py<T>` in the public surface (Py is for GIL-independent storage; Bound is for GIL-acquired access).

### 2. `#[pyclass]` option matrix

- `unsendable` — for `!Send` wrappers like UDPipe. Compile-time admission of thread-confinement; runtime ThreadId panic on cross-thread access. **Required** on the `Matra` class.
- `frozen`: for immutable config types. Eliminates runtime borrow check at three structural levels. **Not applicable** to `Matra` because the NLP provider is mutable state; `Model2Vec` is `frozen`.
- `hash` + `eq` together (or neither) — partial impl is rejected at codegen. None of matra's pyclasses need hash/eq today.
- `subclass` + `extends` — for inheritance. Not used in matra.

### 3. pythonize blind spots

The 4 documented divergences from JSON-equivalence:

1. **i128/u128 silently absent.** `serde` defaults fire and return `Err`. Workaround: string-encode if precision matters.
2. **PyByteArray functionally unsupported** on deserialize despite appearing in the dispatch table. Use `PyBytes` + `serde_bytes::Bytes`.
3. **bytes vs seq-of-u8 ambiguity.** Raw `&[u8]` serializes as a list of ints (because serde calls `serialize_seq`). Use `serde_bytes::Bytes` for byte-fidelity.
4. **Map keys can be non-string.** Python dict accepts non-string keys; pythonize accepts the widening; document on any boundary type.

When adding a new serde-derived type that crosses, run the 4-blind-spot checklist.

### 4. Synchronization (Python 3.13+ free-threading)

`PyOnceLock` is the canonical post-0.28 lock primitive. `GILOnceCell` is broken under free-threading and should not be used in new code.

Matra doesn't use either today (single-threaded composition), but if a future feature adds shared state across PyO3 callbacks, use `PyOnceLock` not `GILOnceCell`.

## Error routing — the load-bearing detail

`From<domain::Error> for PyErr` routes variants to the appropriate Python exception class. The match is exhaustive (no wildcard) so a new variant becomes a compile error — fix it at the routing site, don't paper over it.

```rust
match e.0 {
    ModelNotFound(_) => PyFileNotFoundError::new_err(msg),
    InputTooLarge { .. } | UnsupportedFormat(_) | InvalidInput(_) => {
        PyValueError::new_err(msg)
    }
    Io(_) => PyOSError::new_err(msg),
    ModelInvalid(_) | ParseFailed(_) => PyRuntimeError::new_err(msg),
}
```

When adding a `domain::Error` variant:

1. Add it to the enum with `#[error("…")]`.
2. The PyErr routing match will fail to compile until you add the new arm.
3. Pick the right Python exception class:
   - File-not-found → `PyFileNotFoundError`
   - Bad input (caller's fault) → `PyValueError`
   - I/O failure → `PyOSError`
   - Internal error → `PyRuntimeError`
   - Type error → `PyTypeError`
4. Update tests for any Python-side handling.

## maturin dual-manifest contract

`Cargo.toml` owns Rust build semantics:

```toml
[lib]
name = "matra"
crate-type = ["rlib", "cdylib"]

[features]
default = ["udpipe"]
udpipe = ["udpipe-rs", "sha2"]
python = ["pyo3", "pythonize"]
```

`pyproject.toml` owns Python packaging:

```toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[tool.maturin]
features = ["python", "udpipe"]
python-source = "python"
module-name = "matra._core"
```

Invariants:

- `crate-type = ["rlib", "cdylib"]`: rlib so Rust crates can `cargo add matra`; cdylib so Python loads the extension.
- `module-name = "matra._core"` matches `#[pymodule] pub fn _core(...)` in `lib.rs`. Mismatches produce extensions Python cannot load.
- `[tool.maturin].features = ["python", "udpipe"]` activates both features for the wheel build.
- `python-source = "python"` enables the mixed Rust+Python layout (Python sources alongside the Rust crate).

When you change `module-name` or rename `_core`, both files update together.

## Version pin discipline (the 3-axis rule)


| Dep | Pin | Why |
|---|---|---|
| `pyo3 = "0.28"` | major.minor | Axis 1: PUBLIC API. Multi-version mismatch produces compile errors, not silent UB. |
| `pythonize = "0.28"` | major.minor | Version-locked to pyo3 by spec. |
| `maturin >=1.0,<2.0` | major bound | Build-time tool, not runtime. |

When pyo3 releases 0.29, bump both pyo3 and pythonize together. The migration archaeology (0.20 → 0.28) showed each minor version may carry meaningful API changes (0.21: `Bound<T>` introduced; 0.28: gil-refs removed).

## What you don't put through FFI

- **Methods.** Only fields cross. If consumers need an aggregate, materialize it as a field on a summary type.
- **Lifetimes.** The `'py` lifetime stays in Rust; the Python side sees opaque dicts.
- **References.** Only owned values or `Bound<'py, T>` cross. No raw `&` to internal data.
- **Generics.** PyO3 cannot expose generic methods; monomorphize or use `Box<dyn Trait>`.

## Future direction — WASM/TS crust

When a TypeScript consumer commits, the WASM crust lands. The plan:

- `[features] wasm = ["wasm-bindgen", "serde-wasm-bindgen"]`.
- A new module `src/wasm.rs` with `#[wasm_bindgen]` types wrapping the domain types.
- A second `crate-type = "cdylib"` configuration (or a separate crate `matra-wasm` in a workspace if Pattern 6 fires for the WASM side too).
- Same methods-don't-cross rule; same `serde-wasm-bindgen` blind spots (analogous to pythonize but with their own quirks).

No work until the trigger fires.


## What this skill won't tell you

- General Rust design — that's `rust-craft`.
- Test strategy for FFI code — that's `testing` (CI builds the wheel and imports it).
- Future WASM-specific decisions — they come up when a TypeScript consumer commits.
