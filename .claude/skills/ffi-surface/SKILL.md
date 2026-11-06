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

- Adding a method to the PyO3 `Engine` or `Document` class.
- Bumping pyo3, pythonize, or maturin.
- Changing module-name, python-source, or any maturin config.
- Adding a new error variant to `domain::Error` (requires a Python class in the routing).
- Planning the WASM/TS crust.

## The PyO3 surface — what exists today

The `_core` module in `src/python/mod.rs` (`_core`, `src/python/mod.rs:41`; matched by pyproject.toml's `module-name = "matra._core"`) registers three classes, four functions and, through `add_limits`, the eleven `domain::limits` constants that `python/matra/limits.py` re-exports as `matra.limits`. It sits under boundary rule 7 like `src/cli/`: the public surface and one port, `embed`, never another port, a reader or parser adapter, or a crate-private module. The other three ports are public, but Python implements no parser, reader or source in 0.3.0, so the binding has nothing to wrap as one. This section is the one place in `.claude/` that lists them; the `ffi-keeper` agent points here.

- **`Engine`** (`PyEngine`, `src/python/engine.rs:28`), `#[pyclass(unsendable)]`: the Rust `Engine` is `Send` but not `Sync`, and a pyclass without `unsendable` must be both, so cross-thread access panics at runtime.
  - Constructors: `Engine()` and `Engine.from_config(config_file=None)`.
  - Methods: `analyze(text=None, *, file, bytes, blocks, format)` and `analyze_all(*, dir, texts, format)`, which returns a lazy `DocumentStream` (`src/python/engine.rs:180`, also `unsendable`, not registered).
- **`Document`** (`PyDocument`, `src/python/document.rs:27`), `#[pyclass(frozen)]`: read-only mapping access (`doc["metrics"]`), and `to_dict()` for the schema v2 JSON.
- **`Model2Vec`** (`PyModel2Vec`, `src/python/embed.rs:20`), `#[pyclass(frozen)]`, registered only under the `model2vec` feature: an immutable embedder.
  - Constructors: `from_dir(dir)`, `potion_base_8m(dir=None)`.
  - Getters `model_hash` and `dimensions`; methods `identity` and `embed`.
- **Functions**: `summarize`, `keyphrases` and `clusters` (`src/python/outcomes.rs`), which take a `Document`, and `cli_main` (the command line, which `python/matra/cli.py` launches).
- **Exception classes**: `MatraError` and one class per variant, named for it with PEP 8's `Error` suffix (`ModelNotFoundError`, `ModelInvalidError`, `ParseFailedError`, `InputTooLargeError`, `UnsupportedFormatError`, `InvalidInputError`, `IoError`), plus `IoNotFoundError` and `IoPermissionError` refining `IoError`. Built once per interpreter with `type()` by `error_classes`, since a `#[pyclass]` cannot have two bases, and added to the module at init (below).

## The four PyO3 disciplines


### 1. `'py` lifetime threading

`Python<'py>` is a ZST proof-token. `Bound<'py, T>` embeds the `'py` lifetime as the GIL-attachment proof. The lifetime IS the safety check; raw pointer access (gil-refs) was removed in pyo3 0.28.

**Rule**: every PyO3 method uses `Bound<'py, ...>` types in its return position and `Python<'py>` parameter when needed. No `&PyAny`. No `Py<T>` in the public surface (Py is for GIL-independent storage; Bound is for GIL-acquired access).

### 2. `#[pyclass]` option matrix

- `unsendable` — for `!Send` wrappers like UDPipe. Compile-time admission of thread-confinement; runtime ThreadId panic on cross-thread access. **Required** on the `Engine` class.
- `frozen`: for immutable config types. Eliminates runtime borrow check at three structural levels. **Not applicable** to `Engine` because the parser is mutable state; `Document` and `Model2Vec` are `frozen`.
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

`From<MatraError> for PyErr` raises each `domain::Error` as a class of its own, named for the variant with PEP 8's `Error` suffix (the `Class` enum's Rust names drop it). Every class derives from `matra.MatraError` and from the builtin class matra raised before 0.3.0, so `except ValueError` still catches an `InputTooLarge`. The exception carries `kind` (a class attribute) and `details` (a dict, set when raised from `Error::details`), with the `Display` text as its message: a method's answer crosses as a field. `Class::of` picks the class with a match that is exhaustive over the variants (no wildcard), so a new variant becomes a compile error: fix it at the routing site, don't paper over it. The match:

```rust
match error {
    ModelNotFound(_) => Class::ModelNotFound,
    ...
    Io(e) => match e.kind() {
        std::io::ErrorKind::NotFound => Class::IoNotFound,
        std::io::ErrorKind::PermissionDenied => Class::IoPermissionDenied,
        _ => Class::Io,
    },
}
```

`Class::spec` names each class, its builtin, and its parent inside matra. The classes set `__module__` to `matra`, where `python/matra/__init__.py` re-exports them, so a traceback reads `matra.InputTooLargeError` and an exception pickles.

When adding a `domain::Error` variant:

1. Add it to the enum with `#[error("…")]`, its kind in `Error::kind`, and its keys in `Error::details`.
2. `Class::of` will fail to compile until you add the new arm; add a `Class` for it.
3. In `Class::spec`, pick the builtin it also derives from:
   - File-not-found → `FileNotFoundError`
   - Bad input (caller's fault) → `ValueError`
   - I/O failure → `OSError`
   - Internal error → `RuntimeError`
4. Declare the class in `python/matra/_core.pyi`, export it from `python/matra/__init__.py`, and list its details in `spec/schema/v2/error.schema.json`. `mypy.stubtest`, `python/tests/test_errors.py` and `tests/error_tables.rs` hold the stub, the module and the documented tables to it.

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
- `module-name = "matra._core"` matches `#[pymodule] pub fn _core(...)` in `src/python/mod.rs`. Mismatches produce extensions Python cannot load.
- `[tool.maturin].features = ["python", "udpipe"]` activates both features for the wheel build.
- `python-source = "python"` enables the mixed Rust+Python layout (Python sources alongside the Rust crate).

When you change `module-name` or rename `_core`, both files update together.

## Version pin discipline (the 3-axis rule)


| Dep | Pin | Why |
|---|---|---|
| `pyo3 = "0.29"` | major.minor | Axis 1: PUBLIC API. Multi-version mismatch produces compile errors, not silent UB. |
| `pythonize = "0.29"` | major.minor | Version-locked to pyo3 by spec. |
| `maturin >=1.0,<2.0` | major bound | Build-time tool, not runtime. |

When pyo3 releases 0.30, bump both pyo3 and pythonize together. The migration archaeology (0.20 → 0.28) showed each minor version may carry meaningful API changes (0.21: `Bound<T>` introduced; 0.28: gil-refs removed).

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
