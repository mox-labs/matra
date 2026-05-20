# vaani vs. rust-mastery corpus — gap analysis

Audit produced 2026-05-20. Read-only pass; no code changed. Source: `~/radix-workspaces/rust-mastery/` (closed 2026-05-14, ~150 Frames across 50+ sources).

The corpus's M1 milestone was *literally mined for vaani+slick*. Its `vaani-readiness` cross-artifact Frame is a complete architectural prescription grounded in 6 cross-artifact + 11 file-Frames. This audit walks that prescription against vaani's current code (`Cargo.toml`, `src/`, `pyproject.toml`) and flags where we are doing the right thing, where we are doing the wrong thing, and where we are silent.

## Sample shape

- Read: vaani-readiness, slick-readiness, errors-tier-lib-vs-app, cli-ergonomics-and-app-discipline, rust-python-dual-publish, typed-extension-config-trio, proc-macro-parse-analyze-emit, dtolnay-derive-style-ecosystem, cross-iteration-pattern-consolidation, m8-i2-vector-db-deployment-shape-asymmetry, m8-i3-search-tier-pattern6-substrate-stability, m9-i3-rustc-query-system-pattern11-n3-substantiation.
- Code: `Cargo.toml`, `pyproject.toml`, `src/lib.rs`, `src/domain.rs`, `src/nlp/{mod.rs,udpipe.rs}`, `src/source/{mod.rs,file.rs,directory.rs}`, `src/decompose/mod.rs`, `.claude/arch/README.md`.
- Not read in depth (transferable craft confirmed at cross-artifact scale): M3 (geist-edge async/HTTP/tracing), M3.5 (reactive streams), M4 (LLM agent landscape), M5 (wasm/cedar enforcement), M5.5 (CRDTs), M6 (HUD/tauri), M7 (concurrency), M8 storage details, M9 incremental-computation details. Vaani's problem domain (one-shot text analysis) makes most of these orthogonal; the canonical patterns (5, 6, 10, 11) are absorbed.

## What vaani is doing right (with Frame citations)

### A. Hex-architecture commitment is intact

The M1 vaani-readiness prescription opens with: *"vaani's hex-architecture commitment is the meta-architectural choice that makes all 5 M1 substrate concerns compose without conflict."* Specifically:

- `domain.rs` has only `serde + std`. Frame ✓ — boundary rule #1 of `arch/README.md` is enforced.
- Port modules (`nlp/mod.rs`, `source/mod.rs`, `decompose/mod.rs`) import only from `domain`. Frame ✓.
- Each adapter implements one port; no cross-adapter imports. Frame ✓.
- `nlp/udpipe.rs` is the only file importing `udpipe_rs`. Frame ✓.
- `cargo check --no-default-features` compiles. Frame ✓ — `udpipe` and `python` features are additive.
- Composition root in `lib.rs` is the only file that knows all adapters and ports. Frame ✓.

### B. PyO3 admin tier matches M1.i4 prescription almost completely

Frame: `rust-python-dual-publish` (M1.i4) prescribes 4 layered disciplines + 3-axis pin rule. Vaani implements:

- `#[pyclass(unsendable)]` on the `Vaani` struct holding `Box<dyn NlpProvider>` — `lib.rs:206`. UDPipe is `!Send` (carries C-side state); the `unsendable` ThreadId-panic semantics are exactly the M1.i4 prescription for !Send wrappers.
- `Bound<'py, PyAny>` throughout the PyO3 surface — `lib.rs:197,230,237,...`. The 0.21+ `Bound<T>` type is the only sound type-state for Python objects per M1.i4; raw pointer access (gil-refs) was removed in 0.28.
- `pythonize::pythonize` for serde-derived → PyObject conversion — `lib.rs:198`. Matches M1.i4 prescription for cross-language config types crossing.
- `#[pymodule] pub fn _core` with `module-name = "vaani._core"` matching in pyproject.toml — `lib.rs:313`, `pyproject.toml:24`. Module-name precedence chain consistent.
- `pyo3 = "0.28"` (major.minor) — `Cargo.toml:19`. Per the dtolnay-derive-style ecosystem Frame's 3-axis rule, pyo3 emits against STABLE PUBLIC API (axis 1), so major.minor pin is the conservative pin. Multi-version dep graphs surface as compile errors not silent UB.
- `pythonize = "0.28"` co-versioned with pyo3 — `Cargo.toml:20`. Matches M1.i4 version-coupling discipline.
- `maturin` build backend, `[lib] crate-type = ["rlib", "cdylib"]` — `Cargo.toml:16`, `pyproject.toml:2`. Dual-publish contract: crates.io (rlib) + PyPI (cdylib via maturin). Frame ✓.
- Mixed Rust+Python layout — `pyproject.toml:23` (`python-source = "python"`). Matches M1.i4's layout discriminant.

### C. Pattern 10 (orthogonal-dispatch-axes) — single-axis is correct for vaani

Pattern 10 was substantiated N=19 corpus-wide. Vaani uses one dispatch axis: `&dyn NlpProvider` runtime trait object — `lib.rs:35,42,51,86,114`. `Decomposer` and `Source` are statically selected at the composition root (format enum → MarkdownDecomposer vs PlainTextDecomposer; path vs directory → FileSource vs DirectorySource). One axis is appropriate at vaani's scale; the corpus shows N=4 axes only at search engine scale (tantivy). No gap.

### D. ripgrep ignore-pattern adapted at the right scale

Frame: `cli-ergonomics-and-app-discipline` (M1.i3) — the 4 application-tier disciplines (per-file error tolerance, atomic output buffering, 3-way exit codes, broken-pipe handling) plus the WalkParallel patterns.

Vaani's `source/directory.rs` borrows the relevant ripgrep `ignore::Walk` craft:

- Per-file error tolerance via `read_collecting_errors` returning `(Vec<RawDocument>, Vec<(PathBuf, Error)>)` — `directory.rs:57`. Mirrors ripgrep's `err_message! + continue` pattern but in collecting form. Frame ✓.
- Symlink rejection via `symlink_metadata` (no traversal) — `directory.rs:42`, `file.rs:24-32`. Same craft as ripgrep's `same_file::Handle` approach (inode-based), though simpler — vaani isn't recursive so loop detection isn't needed.
- Lexicographic sort for reproducibility — `directory.rs:49`. Not in ripgrep's pattern but appropriate for vaani's reproducible-analysis goal.
- Dual API surface: `Source::read` (silent-failure) vs `read_collecting_errors` (errors-surfaced) — `directory.rs:72-79`. Both ripgrep-flavored patterns coexist. Frame ✓.

Not adopted (correctly): WalkParallel + crossbeam_deque LIFO work-stealing + AtomicUsize quorum termination. These are for recursive parallel walks; vaani's directory source is single-level non-parallel. No gap.

### E. Taleb-style SPOF mitigation at the C-side boundary

`nlp/udpipe.rs:197-213` defines `catch_parse_panic` wrapping `Model::parse` via `catch_unwind`. The doc comment names this as "Taleb #1: SPOF with no panic boundary." Without this, a UDPipe C-side panic aborts the host process (interpreter death in Python, trap in WASM).

This isn't an M1 Frame finding directly but it's the kind of resilience-floor work the corpus would endorse — making the C/C++ boundary a panic-converting seam rather than a process-abort surface. Good craft.

### F. TOCTOU-closing model loading

`nlp/udpipe.rs:163-181` (`read_and_verify`) returns the *verified bytes themselves* so the loader uses the same bytes that were hashed — no second disk read between verify and load. Doc comment explicitly names the TOCTOU window the previous `verify_file` had.

Combined with atomic-rename download via per-process temp subdir (`nlp/udpipe.rs:103-147`), this is solid security craft. The corpus doesn't have a direct Frame for this exact pattern but it's consistent with the M9 cargo-fingerprint-style stability discipline.

### G. `#[non_exhaustive]` on every public type — additive evolution preserved

`domain.rs:32,101,160,211,378,413,435,527,547,564,575,589,604`. Matches the Rust API guidelines and the M1 thiserror discipline (`#[non_exhaustive]` on Error enums). Frame ✓.

## What vaani is doing wrong (or where the prescription would buy something)

### 1. Error type is hand-rolled — thiserror would generate the same code with less boilerplate

**Current** (`domain.rs:30-90`):

```rust
#[derive(Debug)]
#[non_exhaustive]
pub enum Error { ModelNotFound(PathBuf), ModelInvalid(String), ... }

impl fmt::Display for Error { /* 18 lines */ }
impl std::error::Error for Error { /* source() impl */ }
impl From<std::io::Error> for Error { /* From */ }
```

**Frame prescription** (`errors-tier-lib-vs-app` claim 6, vaani-readiness mastery summary): *"DOMAIN layer (vaani-core or similar pure-Rust crate): `pub enum Error` via `#[derive(thiserror::Error)]` with `#[error('…')]` per variant; concrete types preserved for downstream matching."*

**Gap**: vaani's hand-rolled `impl Display`, `impl Error`, `impl From<io::Error>` is *exactly* what `#[derive(thiserror::Error)]` would emit. There is no functional difference. With thiserror:

```rust
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("model not found: {0}")] ModelNotFound(PathBuf),
    #[error("invalid model: {0}")] ModelInvalid(String),
    #[error("parse failed: {0}")] ParseFailed(String),
    #[error("{what} input too large: {actual} > limit {limit}")]
    InputTooLarge { limit: usize, actual: usize, what: &'static str },
    #[error("unsupported format: {0:?}")] UnsupportedFormat(Format),
    #[error("io error: {0}")] Io(#[from] std::io::Error),
}
```

This is ~25 lines vs vaani's ~60. Variant identity is preserved, so the boundary-rule "matchable, not opaque" stays.

**Blocker**: the boundary rule "`domain.rs` has only `serde + std`" forbids thiserror.

**Audit position**: the boundary rule is *aspirational, not load-bearing*. Per M1.i6 ecosystem Frame's 3-axis rule, thiserror is axis-1 internal-helpers + `__private<patch>` versioning safety; multi-version dep graphs are safe by design. thiserror does not appear in your public API (its README explicitly: *"switching from handwritten impls to thiserror or vice versa is not a breaking change"* — structurally enforced via `__private<patch>` + `#[doc(hidden)]` + exact-version pin per `errors-tier-lib-vs-app` claim 1).

**Recommendation**: relax the boundary rule to admit thiserror. Pin `thiserror = "2"` (major only, axis-1 safe). Net benefit: ~35 lines deleted; Display/Error/From impls become single-source-of-truth (the variant attribute). No public-API change because thiserror is structurally invisible to consumers.

**Counter-argument the audit takes seriously**: the hand-roll is also a teaching artifact — it makes the wire-up explicit. If the codebase is being read as a Rust craftsmanship example (per the "public substrate" memory feedback), the hand-roll has documentation value even if thiserror is cleaner. Decide on values, not just on lines.

### 2. No `anyhow` at any adapter tier — but the verdict is "don't add it"

The M1.i1 + M1.i6 prescription says domain → thiserror, adapter → anyhow with `.context()` chains, `?` at the boundary as the seam.

**Vaani uses `domain::Result<T>` everywhere**, including in:
- The composition root `lib.rs` (`analyze`, `analyze_file`, `analyze_directory`, `parse`, `analyze_from`)
- All adapter modules (`source/*`, `decompose/*`, `nlp/*`)
- The PyO3 layer (which then does `.map_err(|e| PyRuntimeError::new_err(e.to_string()))`)

**Frame says**: this is a structural choice. Anyhow's value is at the *lib/app boundary*. Vaani is a single crate; there is no lib/app boundary inside the Rust surface. The application that consumes vaani is *Python* (the click+rich CLI in `python/vaani/cli.py`), and that boundary is bridged by PyO3, not by anyhow.

**Where anyhow could plausibly add value**:

- At `lib.rs:217`: `.map_err(|e| PyFileNotFoundError::new_err(e.to_string()))` discards the concrete `Error::ModelNotFound(PathBuf)` variant. Anyhow with `.context("loading english model")` would attach a chain; but the simpler refinement is to *preserve more variant information at the PyO3 boundary* (map `Error::InputTooLarge` to `PyValueError`, `Error::ModelNotFound` to `PyFileNotFoundError` ✓ already done, `Error::ParseFailed` to a domain-specific `VaaniParseError` etc.). This doesn't need anyhow; it needs a small `From<Error> for PyErr` impl with variant routing.
- For `read_collecting_errors`'s `Vec<(PathBuf, Error)>` — anyhow context would *lose* the concrete variant, which is the opposite of what we want. domain::Error is correct here.

**Audit position**: no gap. Vaani's "single error type, matchable, hand-rolled" is fine for a substrate library. The M1 prescription is conditional on having a lib/app boundary, and vaani doesn't have one in Rust. Reject this part of the prescription as not-applicable.

**Sub-recommendation**: add a `From<domain::Error> for PyErr` impl that routes to the appropriate Python exception class per variant (per pyo3 conventions). This eliminates ~10 `.map_err(|e| PyRuntimeError::new_err(e.to_string()))` calls in `lib.rs:217,225,232,...` and preserves variant information across the FFI boundary. Cheap, semantic, and structurally aligned with M1.i4's PyO3 discipline.

### 3. `Cargo.toml` description hides the `.context()` story — minor

`Cargo.toml:8`: `description = "Prose metrics engine. Text in, structured analysis out."` — fine for crates.io. No gap.

### 4. CLAUDE.md's "Mastery References" section points at dead paths

`CLAUDE.md:67-79` lists `~/oss/research/synthesis.md`, `~/oss/research/implementation.md`, `~/oss/research/pyo3-mastery.md`, etc. **None of those exist.** The actual rust-mastery substrate is at `~/radix-workspaces/rust-mastery/` (closed 2026-05-14).

This is documentation rot — not a code gap, but a navigation gap. Future readers (including future-Claude in fresh sessions) will follow CLAUDE.md and dead-end.

**Recommendation**: update CLAUDE.md to point at the actual radix corpus. Specifically:
- vaani-direct mastery: `~/radix-workspaces/rust-mastery/frames/cross-artifact/frame__cross-artifact__vaani-readiness.json` (the integrating M1 Frame)
- Errors discipline: `frame__cross-artifact__errors-tier-lib-vs-app.json`
- PyO3 / dual-publish: `frame__cross-artifact__rust-python-dual-publish.json`
- Macro ecosystem: `frame__cross-artifact__dtolnay-derive-style-ecosystem.json` (the 3-axis pin rule)
- Resilience floor: `frame__cross-artifact__cli-ergonomics-and-app-discipline.json` (Walk patterns + per-file tolerance)
- Hex composition: `frame__cross-artifact__vaani-readiness.json` (already cited above)

### 5. CLAUDE.md's "Skills" table promises three skills that don't exist on disk

`CLAUDE.md:54-59`:

| Skill | When to use |
|-------|-------------|
| `rust-craft` | Any Rust design decision |
| `testing` | Writing tests, reviewing coverage |
| `architecture` | Adding modules, creating adapters |

No `.claude/skills/` directory exists in `mox/packages/vaani/`. The skill files were never created.

This is the *primary deliverable gap* — the skills CLAUDE.md promises are unbuilt. The corpus is the substrate to build them from.

**Recommendation**: see "Proposed skill scope" below.

### 6. `arch/README.md` describes an aspirational two-crate workspace (vaani-core + rumi-nlp); the actual code is a single crate

`arch/README.md:7`: *"workspace shape (`vaani-core` + `rumi-nlp`), hex layout inside `vaani-core`"*. The actual `Cargo.toml` is `name = "vaani"` (single crate). The workspace split is documented but not implemented.

Not a corpus gap — this is internal doc drift. Flagged so it doesn't compound. Either implement the split or update arch/README.md to describe the current single-crate shape.

## What vaani is silent on (where the corpus has more to say)

These are not gaps per se, because vaani may not need them yet. They're options the corpus preserves.

### a. No extensibility surface (typetag/inventory trio deferred)

Per M1.i2 + slick-readiness, the inventory + typetag + serde trio is the canonical Rust idiom for open-set late-bound polymorphic dispatch. Vaani currently has *one* extension point (`NlpProvider`) and no user-extensible config surface.

If vaani ever wants user-defined NLP pipeline stages, swappable model backends as JSON config, or plugin-style metric registration, the trio is the canonical answer. Caveats per M1.i2: linux/macOS/windows only (linker-section accumulation); object-safety required on the trait; discriminator-string uniqueness.

**Current decision**: defer until product trajectory clarifies. This is the corpus's own prescription (`vaani-readiness` notes: *"Whether vaani actually ships this depends on product trajectory; the architectural option is preserved by the corpus."*). No gap.

### b. No proc-macros — `syn`/`quote` mastery doesn't apply yet

Per M1.i5 + `proc-macro-parse-analyze-emit`, the parse-analyze-emit pattern is the canonical model for proc-macros. Vaani uses no proc-macros (it consumes `serde`'s derives but doesn't author any). If vaani ever ships a derive macro (e.g., `#[derive(VaaniMetric)]` for user-defined custom metrics), this is the substrate.

**Current decision**: not needed. The 3-axis pin rule from M1.i6 still applies to `pyo3` (axis-1 PUBLIC API, no `__private<patch>`).

### c. Pattern 6 candidate: `NlpProvider` as a separately published crate

Per `m8-i3-search-tier-pattern6-substrate-stability`, the criterion for separate publication of a minimal stability trait is *"whether external implementors exist who need to pin the contract independently of the main crate's version churn."*

Vaani's `NlpProvider` is structurally Pattern 6 material: minimal contract (1 method), `Send` bound, isolated module, no transitive deps beyond domain. The question is whether the ecosystem has external implementors (third-party NLP backends shipping their own crates) who would benefit from `vaani-nlp-api = "0.x"` while `vaani` churns at `0.x.y`.

**Current decision**: vaani has no external NlpProvider implementor ecosystem yet. Per the M8.i3 criterion, separate publication is premature. Keep `NlpProvider` in-crate. If external NLP adapters emerge (`vaani-stanza`, `vaani-spacy`, `vaani-trankit` etc.), extract `vaani-nlp-api` then.

### d. Pattern 5 deployment-shape — vaani is library-shaped, capability-composition flavor

Per `m8-i2-vector-db-deployment-shape-asymmetry`, lancedb is library-shaped with capability-composition feature flags (storage backend × execution mode × embedding provider); qdrant is server-shaped with operational-tuning feature flags.

Vaani is library-shaped, capability-composition flavor: `default = ["udpipe"]` + `python = ["pyo3", "pythonize"]`. Each flag adds a *capability* (NLP backend, Python surface) rather than tuning a fixed binary.

**Gap to watch**: the lancedb leakage issues (#2865, #2567) show how transitive deps' default features can break the compositionality. Vaani's deps are small enough today that this isn't an active risk, but if it grows (e.g., adding stanza or spacy as alternate NLP backends with their own transitive trees), audit the `default-features = false` discipline. No action now.

### e. Pattern 11 (incremental computation) — not applicable

Per M9, Pattern 11 is rustc/salsa/rust-analyzer's demand-driven memoized computation pattern. Vaani is a one-shot pipeline (parse → analyze → return). No incremental state to memoize. No gap.

## Proposed skill scope

The CLAUDE.md promises three skills (`rust-craft`, `testing`, `architecture`) that don't exist on disk. Building them is the natural next step. Based on the audit, the right scope is:

### `rust-craft` — Rust design decisions for this codebase

What it codifies:
- The dtolnay-derive-style 3-axis pin rule (when to pin major.minor vs major-only). Source: `dtolnay-derive-style-ecosystem` Frame. Already-applied to vaani's pyo3/pythonize/serde pins.
- The lib/app error tier discipline (when domain::Result is correct, when anyhow buys something, when thiserror buys something). Source: `errors-tier-lib-vs-app`. Concludes that vaani's hand-roll is defensible but thiserror is cleaner; anyhow doesn't apply at vaani's scale.
- The PyO3 `unsendable`/`frozen`/`Bound<'py>` discipline (when to use each, how to map errors to PyErr variants). Source: `rust-python-dual-publish`.
- The `#[non_exhaustive]` discipline for additive evolution. Already in use.
- The boundary rule "only serde, std" — load-bearing or relaxable? (See audit finding #1.)

When to invoke: any Rust design decision — error tier, dep pin, PyO3 attribute, trait shape, version pin.

### `testing` — testing strategy for this codebase

What it codifies:
- Per-paragraph parse regression tests (`lib.rs:386-425` — FM1 regression). Source: vaani's own i2 work.
- TOCTOU regression tests (`udpipe.rs:413-431`). Source: vaani's i2 work.
- catch_unwind boundary tests (`udpipe.rs:292-339`). Source: vaani's i2 work.
- `tree_depth` complexity tests (linear-time-on-1000-chain). Source: `domain.rs:851-865` — codifies the "no silent O(n^2)" invariant from arch/README.md.
- Reading from the corpus: `verification.md` from rust-mastery (loom, miri, fuzzing, property tests) — but vaani doesn't use loom (single-threaded NLP), so this is selective.

When to invoke: writing tests, reviewing coverage, debugging test failures.

### `architecture` — adding modules, adapters, boundary compliance

What it codifies:
- The hex boundary rules from `arch/README.md` (the 8 invariants).
- The composition root discipline (lib.rs is the only place that knows all adapters).
- The Source/Decomposer/NlpProvider port contracts.
- The feature-flag additive-discipline (`default = ["udpipe"]`, `python = [...]`).
- Pattern 6 criterion (when to extract a port into a separate crate — not yet).
- Pattern 10 axis-count discipline (vaani is single-axis; resist accidental N=2+).

When to invoke: adding a new format/backend/adapter, designing a new port, considering a crate split.

## Suggested order of operations

If we proceed from the audit:

1. **Update CLAUDE.md "Mastery References"** to point at the actual corpus paths (audit finding #4). Cheap, removes a dead-link trap.
2. **Decide on thiserror** (audit finding #1) — relax the boundary rule and adopt, or keep the hand-roll and document why. Either is defensible; pick one.
3. **Decide on workspace split** (audit finding #6) — implement `vaani-core` + `rumi-nlp` per arch/README.md, or update arch/README.md to describe the current single-crate shape. Don't let the docs drift indefinitely.
4. **Add `From<domain::Error> for PyErr`** (audit sub-finding under #2) — small, semantic, removes ~10 `.to_string()` calls at the PyO3 boundary.
5. **Build the three skills** (`rust-craft`, `testing`, `architecture`) at `.claude/skills/<name>/SKILL.md` — primary deliverable gap. Each skill cites the relevant Frames inline.

Steps 1–4 are independent and small. Step 5 is the substantial work and depends on whether you want one consolidated skill or three navigable skills. The three-skill option matches what CLAUDE.md already promises.

## Conclusion

Vaani is *substantially aligned* with the M1 corpus prescription. The hex commitment is intact; the PyO3 admin tier is correctly shaped; the resilience-floor work (i2) has already adopted the right craft (TOCTOU closure, atomic rename, catch_unwind, symlink rejection, size caps); the version pins follow the 3-axis ecosystem rule.

The genuine gaps are:
- **Documentation gap**: CLAUDE.md points at dead paths (audit #4) and promises skills that don't exist (audit #5).
- **Style gap**: hand-rolled error type does what thiserror would do but with more boilerplate (audit #1). Defensible either way.
- **Doc drift gap**: arch/README.md describes a workspace split that isn't implemented (audit #6).
- **Boundary refinement**: PyO3 layer could route variants to specific PyErr classes instead of one-size-fits-all PyRuntimeError (audit #2 sub-finding).

No structural gap — vaani's architecture is sound against the corpus's prescription. The work is *building the navigable skills* from the corpus, *updating the dead-link CLAUDE.md*, and *deciding on one or two style refinements* (thiserror, PyErr routing).

The corpus is the architectural decision substrate. Every choice in vaani (error tier, version pin, PyO3 attribute, hex shape) has a Frame as its rationale. The skills make those rationales reachable from the working directory.
