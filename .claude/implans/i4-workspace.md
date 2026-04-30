# I4 — Workspace conversion + rumi-nlp skeleton

**Status:** not-started
**Boundary:** structural — converts the single crate into a workspace and adds the matcher-bridge crate skeleton.
**Depends on:** I3 (error restructure + tracing landed; MVP gate held)
**Branch:** `i4/workspace` off the I3 commit

## Why this iteration exists

vaani's domain knowledge — what `nsubj` means, what a triplet looks like, how dependency-tree walks work — belongs colocated with the substrate that produces it. The matcher-engine bridge that exposes this knowledge as `DataInput<Sentence>` implementations against a generic predicate engine is **part of vaani**, not a separate downstream project.

Rationale: the project that owns the *domain* owns the matcher-bridge crate for that domain. Other matcher-engine extensions (`rumi-http`, `rumi-claude`) live with their respective domain owners. NLP is vaani's domain. So `rumi-nlp` lives in vaani's workspace.

This is not a deferred-to-0.2 concern. The workspace structure is the **0.1.0 architecture**. What ships in `rumi-nlp` at 0.1.0 is conservative (primitives + skeleton, no domain-specific patterns); the structure locks first, the content fills later.

## What lands

### Task A: convert vaani to a Cargo workspace

**Files:** `Cargo.toml` (workspace root), `crates/vaani-core/Cargo.toml`, all of `src/` moves under `crates/vaani-core/src/`.

**Why:** a workspace separates the substrate (`vaani-core`) from peer crates that build on it (`rumi-nlp` initially; possibly more later). Without the workspace, `rumi-nlp` would either bloat the substrate's dep tree or live in a different repo where it cannot share CI / testing / publish discipline.

**Steps:**

1. Move existing `src/` and tests under `crates/vaani-core/`. Update relative paths in `Cargo.toml` accordingly.
2. Convert root `Cargo.toml` to a workspace manifest:
   ```toml
   [workspace]
   resolver = "2"
   members = ["crates/vaani-core", "crates/rumi-nlp"]

   [workspace.package]
   version = "0.1.0"
   edition = "2024"
   rust-version = "1.85"
   authors = ["yzavyas <yza.vyas@gmail.com>"]
   license = "MIT"
   repository = "https://github.com/mox-labs/vaani"

   [workspace.dependencies]
   serde = { version = "1", features = ["derive"] }
   tracing = { version = "0.1", default-features = false, features = ["std", "attributes"] }
   ```
3. `crates/vaani-core/Cargo.toml`:
   - `name = "vaani"` (the substrate keeps the headline name on crates.io)
   - Use workspace deps where possible
   - Crate-type stays `["rlib", "cdylib"]` only when `python` feature is on (Lotfi/Ixian carry-over from prior 13-agent review — fix folded in here)
4. Update `python/vaani/` paths if the maturin config moved.
5. Run `cargo build --workspace` and `cargo test --workspace`. All 56+ tests must still pass.
6. Update CI workflow (`.github/workflows/ci.yml`) to build/test the workspace.

**Acceptance:**
- `cargo build --workspace` and `cargo test --workspace --features udpipe` pass.
- `cargo test --workspace --no-default-features` passes.
- Test count `≥ N₀ + new tests added in I1–I3`.
- Crate publishes as `vaani` (not `vaani-core` on crates.io — the canonical name stays).

### Task B: feature-gate the cdylib crate-type

**Files:** `crates/vaani-core/Cargo.toml`.

**Why (Lotfi + Ixian, recovery 13-agent review):** today's `crate-type = ["rlib", "cdylib"]` always builds the dynamic library. Consumers using vaani as a pure Rust dependency pay the cost; PyO3-specific config can bleed through to non-Python consumers. Gate the cdylib behind the `python` feature so it only appears when actually needed.

**Steps:**

1. Cargo doesn't natively support feature-gated crate-types. Options:
   - **(a) Two `[lib]` blocks via build script.** Complex, fragile.
   - **(b) Conditional `[[bin]]` or workspace-level orchestration.** Possible but messy.
   - **(c) Document the practical workaround.** `crate-type = ["rlib", "cdylib"]` always. Consumers using only the rlib don't link the cdylib; cargo builds both but the rlib is what gets used. The wasted artifact is a build-time issue, not a runtime concern.
2. **Recommended: (c) for 0.1.0.** Document why both crate-types are present. If a consumer reports the cdylib build cost as a problem, revisit with a build script in 0.2.

**Acceptance:** Cargo.toml has a comment explaining the dual crate-type. Build artifacts include both rlib and cdylib (verifiable via `cargo build --features python` then `ls target/debug/`).

### Task C: skeleton `rumi-nlp` crate

**Files:** `crates/rumi-nlp/Cargo.toml`, `crates/rumi-nlp/src/lib.rs`, `crates/rumi-nlp/src/inputs/mod.rs`.

**Why:** vaani 0.1.0 ships a workspace where the matcher bridge exists but is intentionally minimal. The bridge structure locks the architecture; the content (DataInputs, compile path, NLP-specific patterns) fills in subsequent iterations and post-publish work.

**Scope for 0.1.0 (conservative — confirm with user before fleshing out):**

1. **Crate manifest:**
   ```toml
   [package]
   name = "rumi-nlp"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   repository.workspace = true
   description = "NLP DataInputs and matchers for the rumi matcher engine, over vaani's parsed Sentence."

   [dependencies]
   vaani = { path = "../vaani-core", version = "0.1.0" }
   rumi-core = "0.0.2"
   serde.workspace = true
   ```
2. **Public surface (skeleton):**
   - `pub use vaani::{Sentence, Token};` — re-export the context types.
   - `pub mod inputs;` — placeholder module for `DataInput<Sentence>` implementations.
   - **No actual DataInputs land in I4.** Just module structure and one trivial example so the dep wiring is verified.
   - One simple `PosInput` implementation as a smoke test:
     ```rust
     pub struct PosInput { pub token_id: usize }
     impl rumi_core::DataInput<Sentence> for PosInput {
         fn get(&self, sentence: &Sentence) -> rumi_core::MatchingData {
             sentence.tokens.iter()
                 .find(|t| t.id == self.token_id)
                 .map(|t| rumi_core::MatchingData::String(t.pos.clone()))
                 .unwrap_or(rumi_core::MatchingData::None)
         }
     }
     ```
3. **One smoke test** verifying `PosInput` returns the right `MatchingData` for a hand-built `Sentence`.
4. **README** for the crate describing intended scope and pointing at `.claude/arch/` for design context.

**What does NOT ship in 0.1.0 `rumi-nlp`:**
- The five extraction patterns (SVO, copular, prepositional, passive, nominal modifier) — deferred to a future rumi-nlp-specific iteration once a real consumer drives the requirements.
- A `compile_nlp_rules()` config compiler — deferred until pattern shapes are settled.
- Stance classification — out of vaani's scope entirely.

**Acceptance:**
- `cargo build -p rumi-nlp` succeeds.
- `cargo test -p rumi-nlp` passes (smoke test green).
- `rumi-nlp`'s public surface is a few items (re-exports + one DataInput).
- Crate documented as "0.1.0 ships the bridge structure; pattern content lands incrementally."

### Task D: update boundary checks for the workspace

**Files:** `scripts/check-boundaries.sh`.

**Why:** the boundary script written in I0 walks `src/` directly. After workspace conversion, paths shift to `crates/vaani-core/src/`. Update the patterns so the script still enforces rules 3, 4, 8.

**Steps:**

1. Update path patterns in the script to walk `crates/vaani-core/src/` for vaani's rules.
2. Add a check for `rumi-nlp`: it must depend on `vaani` and `rumi-core`, but `vaani-core` must **not** depend on `rumi-nlp` or `rumi-core`. (Verify via `cargo metadata` or by grepping each crate's Cargo.toml.)
3. Run the script; confirm it passes.

**Acceptance:** `bash scripts/check-boundaries.sh` exits 0 post-conversion, including the new dependency-direction check.

### Task E: documentation updates

**Files:** `README.md`, `crates/vaani-core/README.md` (new), `crates/rumi-nlp/README.md` (new), `CHANGELOG.md`.

**Why:** publishing as a workspace requires per-crate READMEs (each crate's README appears on crates.io). The top-level README explains the workspace shape; the per-crate READMEs explain each crate's role.

**Steps:**

1. **Top-level `README.md`:** update to mention the workspace shape — `vaani` is the substrate, `rumi-nlp` is the matcher bridge. ACE framing stays.
2. **`crates/vaani-core/README.md`:** copy of the substrate-relevant parts of the top-level README. This is what crates.io users see for `vaani`.
3. **`crates/rumi-nlp/README.md`:** brief description of the bridge crate, what it wraps, what's in 0.1.0 vs deferred.
4. **`CHANGELOG.md`** under `## [Unreleased]`:
   ```
   ### Changed
   - Restructured into a Cargo workspace. `vaani` (the substrate) is now `crates/vaani-core/`; the matcher-bridge crate `rumi-nlp` is added at `crates/rumi-nlp/`. Public crates.io name for the substrate stays `vaani`.
   ```

**Acceptance:** Each crate has a README. `cargo publish --dry-run -p vaani` succeeds. `cargo publish --dry-run -p rumi-nlp` succeeds.

## Validation

- Cross-iteration regression matrix items 1–9 pass for the workspace.
- `cargo test --workspace --features udpipe`: count `≥ N₀ + I1+I2+I3 additions + 1 (rumi-nlp smoke test)`.
- `cargo build -p vaani` builds without `rumi-core` in the dependency graph (verifiable with `cargo tree -p vaani | grep rumi`).
- `cargo build -p rumi-nlp` builds with both `vaani` and `rumi-core` in the graph.
- `bash scripts/check-boundaries.sh` exits 0.
- Smoke test: `PosInput` returns expected `MatchingData::String("VERB")` for a hand-built `Sentence`.

## Acceptance gate

After I4 lands:
- vaani is a workspace with `vaani-core` and `rumi-nlp` (skeleton).
- vaani-core's public surface is unchanged from I3.
- rumi-nlp builds, has one DataInput, has one passing test.
- All cross-iteration regression matrix items pass.
- Both crates `cargo publish --dry-run` clean.

## Risks

- **Risk:** workspace conversion touches every relative path in tests, examples, doc comments. Misses break the build.
  - **Mitigation:** `cargo test --workspace` after each significant move. Don't batch the move into a single commit.

- **Risk:** `rumi-nlp` shipping with only a skeleton invites speculation about what *will* go there. Consumers may write code against the structure that breaks when patterns land.
  - **Mitigation:** `rumi-nlp/README.md` is explicit about 0.1.0 scope and what's deferred. Patterns land in their own iterations with their own design discussion.

- **Risk:** `rumi-core 0.0.2` (the dependency) is itself early-stage and may have breaking changes before vaani 1.0.
  - **Mitigation:** pin `rumi-core` to a specific version in `rumi-nlp/Cargo.toml`. Track upstream changes and bump explicitly.

- **Resolved 2026-04-30:** crate name is **`rumi-nlp`**. Matches the rumi-* extension convention, accurate to the dependency. The brand-cleanliness argument for `vaani-matchers` was considered and dismissed; honesty about the dependency wins.

- **Resolved 2026-04-30:** initial scope is **primitives + one smoke test** (Task C as drafted). The five extraction patterns and `compile_nlp_rules()` config compiler are deferred to I6e, triggered by real consumer needs. Don't ship speculative.

- **Open decision (confirm with user):** does the cdylib feature-gating actually need a build script (Task B (a)/(b)) or is the documented workaround (Task B (c)) acceptable? My default is (c) for 0.1.0.

- **Consult:** Burner if the workspace dependency graph feels off. Ace if the per-crate publish UX is unclear. K if the iteration boundary feels wrong.
