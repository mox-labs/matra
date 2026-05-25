# Boundary Rules Reference

vaani's source tree enforces a hexagonal architecture through seven boundary rules. The rules prevent the three structural failure modes: domain pollution (rule 1), port leakage (rules 2-3), adapter spread (rule 4), metric coupling (rule 5), and build fragmentation (rules 6-7).

---

## The seven rules

### Rule 1: Domain purity

`domain.rs` depends only on `serde`, `thiserror`, and `std`.

**Enforcement.** Type system and `cargo check`. Adding any other import to `domain.rs` breaks `cargo check --no-default-features` (rule 6). No script is needed; the compiler rejects the violation.

**Example violation.** Adding `use tokio::sync::Mutex` to `domain.rs`.

**Example fix.** Move the dependency to the adapter that needs it (`nlp/udpipe.rs`, `source/file.rs`, etc.). Domain types stay pure data.

**Why this matters.** Adapters depend on domain types. If domain depends on an adapter's dependency, the dependency graph has a cycle. More practically, once you add `tokio` to domain, no caller can use domain types without pulling in the async runtime.

---

### Rule 2: Port isolation from adapters

Port modules (`source/mod.rs`, `decompose/mod.rs`, `nlp/mod.rs`) import only from `domain`.

**Enforcement.** Type system and `cargo check`. A port module that imports an adapter's dependency cannot compile without that dependency present.

**Example violation.** `nlp/mod.rs` importing `use udpipe_rs::Model`.

**Example fix.** The port module defines the `NlpProvider` trait against `domain` types only. `udpipe.rs` implements the trait; `mod.rs` knows nothing about UDPipe.

---

### Rule 3: No cross-port imports

No port module imports another port module.

**Enforcement.** `scripts/check-boundaries.sh` in CI. The script checks `source/mod.rs`, `decompose/mod.rs`, and `nlp/mod.rs` for `use crate::source`, `use crate::decompose`, or `use crate::nlp`.

**Example violation.** `decompose/mod.rs` using `use crate::source::Source` to reference the `Source` trait.

**Example fix.** Pass the `RawDocument` value (a domain type) across the boundary; do not import the port module that produced it.

---

### Rule 4: Single UDPipe importer

`nlp/udpipe.rs` is the only file that imports `udpipe_rs`.

**Enforcement.** `scripts/check-boundaries.sh` in CI. The script searches all of `src/` (excluding `src/nlp/udpipe.rs`) for `use udpipe_rs` or `udpipe_rs::`.

**Example violation.** A new `src/metrics/parse_helper.rs` importing `use udpipe_rs::Model` for convenience.

**Example fix.** Expose what is needed through the `NlpProvider` trait. The `catch_unwind` seam that converts UDPipe panics to `ParseFailed` lives inside `nlp/udpipe.rs` by design; reintroducing direct imports elsewhere moves that boundary back into caller code.

---

### Rule 5: Metrics and extraction depend only on domain and stopwords

`metrics/` and `extraction/` import only from `domain` and `stopwords`.

**Enforcement.** Type system and `cargo check`. Adding a direct import of `udpipe_rs` or any adapter in these modules breaks the build under `--no-default-features`.

**Example violation.** `metrics/readability.rs` importing `use crate::nlp::udpipe::Udpipe` to access parse results directly.

**Example fix.** Receive `&[Sentence]` from the composition root. Metrics and extraction algorithms operate on domain types, not on the NLP adapter.

---

### Rule 6: No-default-features build must compile

`cargo check --no-default-features` must succeed.

**Enforcement.** CI gate. The `default` feature enables `udpipe`; disabling it removes UDPipe and `sha2`. The library core (domain types, port traits, metrics, extraction, stopwords) must compile without any optional dependency.

**Example violation.** Using `#[cfg(feature = "udpipe")]` but forgetting to gate an import, leaving a `use udpipe_rs::...` reachable on the no-default path.

**Example fix.** Gate the import correctly. The `nlp/udpipe.rs` file is only compiled when `feature = "udpipe"` is enabled.

---

### Rule 7: Composition root is the only place that knows all adapters

`lib.rs` is the only file that imports from both port modules and adapter modules.

**Enforcement.** Type system. No script is needed; rule 2 and rule 3 together mean that no module other than `lib.rs` can import across the port/adapter boundary and compile.

**Example violation.** A new `pipeline.rs` module that imports `Source`, `Decomposer`, `NlpProvider`, and `Udpipe` to wire up the pipeline itself.

**Example fix.** The wiring belongs in `lib.rs`. The composition root is allowed to know all the pieces precisely because it is the only place that does.

---

## check-boundaries.sh

The script runs in CI on every PR. Run it locally before committing:

```bash
bash scripts/check-boundaries.sh
```

**What it checks.**

| Check | Rule |
|---|---|
| No `use udpipe_rs` or `udpipe_rs::` in `src/` except `src/nlp/udpipe.rs` | Rule 4 |
| No `use crate::source`, `use crate::decompose`, or `use crate::nlp` in port modules | Rule 3 |
| No `use tracing` or `tracing::` in `domain.rs` or port modules | Burner amendment (tracing prohibition) |

Rules 1, 2, 5, 6, and 7 are enforced by the type system and `cargo check`; the script does not check them. The tracing prohibition (forbidding `tracing` in domain and port modules) was added as a Burner amendment; it does not appear in the numbered rules above but is enforced by the script.

**Exit codes.** `0` on pass. Non-zero on any failure. Each failure prints the offending file.

---

*For architecture context, see [architecture/hex.md](../architecture/hex.md) and [architecture/ports-adapters.md](../architecture/ports-adapters.md). For the ADR that governs domain purity, see [ADR-0001](https://github.com/mox-labs/vaani/blob/main/docs/decisions/0001-record-architectural-decisions.md).*
