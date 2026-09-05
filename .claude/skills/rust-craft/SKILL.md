---
name: rust-craft
description: Rust design decisions specific to matra — error tier choice, dep pin rules, trait shape, version pinning, feature-flag composition, `#[non_exhaustive]` discipline. Use when making any non-trivial Rust design decision in this codebase.
---

# rust-craft

Rust design decisions for matra. This skill codifies the choices matra has already made and the rules for making new ones.

## When to invoke

- Choosing an error type for a new module.
- Adding a dependency to `Cargo.toml`.
- Designing a new public trait or struct.
- Picking a version pin for a new dep.
- Adding a feature flag.
- Deciding between `#[non_exhaustive]` and an open enum.

## The error tier — what matra uses today

matra uses a single hand-derived `domain::Error` enum via thiserror at the library tier. There is no anyhow in the working code.

```rust
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("model not found: {}", .0.display())]
    ModelNotFound(PathBuf),
    #[error("invalid model: {0}")]
    ModelInvalid(String),
    #[error("parse failed: {0}")]
    ParseFailed(String),
    #[error("{what} input too large: {actual} > limit {limit}")]
    InputTooLarge { limit: usize, actual: usize, what: &'static str },
    #[error("unsupported format: {0:?}")]
    UnsupportedFormat(Format),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**Why thiserror, not anyhow:** The decision rule is: if a caller may *match* on the error variant, the crate is library-tier and concrete enums (thiserror) preserve variant identity. If only the application's top-level handler will display/log/exit, type erasure (anyhow) is ergonomic.

matra is a substrate library. Its callers will match on `InputTooLarge` (route to PyValueError, retry with smaller input, etc.), `ModelNotFound` (prompt for path, download, etc.), `Io(_)` (filesystem-specific recovery). Type preservation is required; thiserror is correct.

**Why not anyhow at any layer:** matra has no lib/app boundary inside its Rust surface — it is a single crate. The PyO3 boundary in `lib.rs::python` routes variants to PyErr subclasses via an exhaustive match (no wildcard). A new variant becomes a compile error there — exactly what we want.

If a future submodule (e.g., a CLI tool consuming matra) needs `.context()` chains, *that* code uses anyhow. matra itself stays on `domain::Result`.

## Dep pinning — the 3-axis rule

The rule for pinning macro-emitting crates:

Ask three questions in order:

1. Is this a proc-macro or a macro_rules! library?
2. Does the macro emit static path strings into consumer crates?
3. Are those paths INTERNAL HELPERS or STABLE PUBLIC API?

`__private<patch>` versioning is required IFF (proc-macro AND emits-static-paths AND internal-helpers). For everything else, the pin can be more relaxed because multi-version dep graphs surface as compile errors, not silent UB.

Applied to matra's deps:

| Dep | Pin | Axis 1 (path target) | Why |
|---|---|---|---|
| `pyo3 = "0.28"` | major.minor | PUBLIC API (`Bound`, `Python`, `PyResult`) | Compile-error on mismatch; major.minor is conservative |
| `pythonize = "0.28"` | major.minor | Version-locked to pyo3 by spec | Same as pyo3 |
| `thiserror = "2"` | major | INTERNAL HELPERS + `__private<patch>` | Multi-version safe by design |
| `serde = "1"` | major | Foundational; stable since 1.0 | Industry standard pin |
| `udpipe-rs = "0.2"` | major.minor | Stable-API consumer | Conservative |
| `sha2 = "0.10"` | major.minor | RustCrypto convention | Conservative |
| `brotli = "7"` | major | C-binding stability | Conservative |

When you add a new dep, ask the three questions and pick the pin. Document in the commit message if it's non-obvious.

## Trait design

Three principles:

1. **Minimal contract.** The longest-lived ecosystem traits (tower::Service, tracing::Subscriber, futures::Stream) have one or two methods, not five. matra's four ports (`Source`, `Decomposer`, `NlpProvider`, `Embedder`) follow.
2. **Object-safe.** Every port must be usable as `&dyn Trait`. The composition root needs runtime dispatch. No generic methods on Self.
3. **`Send` if cross-thread is plausible.** `Source`, `NlpProvider` are `Send`. `Decomposer` is not (it operates on `&str` only). Don't add `Sync` unless you have a concrete cross-thread sharing case.

## `#[non_exhaustive]` discipline

Every public enum and every public struct with public fields in `domain.rs` has `#[non_exhaustive]`. This is non-negotiable for matra because:

- matra is a substrate; downstream code reads every public field path as a contract.
- Adding a variant or field later without `#[non_exhaustive]` is a breaking change. With it, additive changes are minor-version bumps.
- Pattern-matches on `#[non_exhaustive]` enums must include `_` (the additive variant escape hatch) — forces consumers to write code that survives variant additions.

Exception: the PyErr routing match in `lib.rs::python` deliberately omits the wildcard so new variants become compile errors. This is *inside* matra, where exhaustiveness is the contract, not the breakage risk.

## Feature flag discipline

Matra has two features:

```toml
[features]
default = ["udpipe"]
udpipe = ["udpipe-rs", "sha2"]
python = ["pyo3", "pythonize"]
```

Rules:

- **Additive only.** Enabling a feature adds capability; never subtracts. Disabling `udpipe` removes UDPipe but never breaks something unrelated.
- **`cargo check --no-default-features` must compile.** This is a hard CI gate.
- **No cross-feature implications without an ADR.** If `python` had to imply `udpipe` (e.g., the Python class only works with UDPipe), document why.


## What this skill won't tell you

- Specific code patterns (which iterator combinator to use, etc.) — that's case-by-case.
- Performance optimization — use the `testing` skill's complexity benches first.
- FFI surface choices — that's `ffi-surface`.
- Failure modes — that's `resilience-floor`.
