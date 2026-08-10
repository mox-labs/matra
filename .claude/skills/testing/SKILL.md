---
name: testing
description: Test strategy for matra — regression discipline (every fixed bug gets a test so it cannot recur), unit + integration + doctest layout, property tests where useful, complexity benches for algorithmic code. Use when writing tests, reviewing coverage, or debugging test failures.
---

# testing

Test strategy for matra. This skill codifies what kinds of tests matra has, what each kind verifies, and how to add new ones.

## When to invoke

- Writing tests for a new feature.
- Adding a regression test for a fixed bug.
- Designing complexity benches.
- Debugging a flaky or broken test.
- Reviewing coverage on a PR.

## The three test layers

### 1. Unit tests — `#[cfg(test)] mod tests`

In-file, per-module. Test the immediate API of the module they live in. Examples:

- `src/domain.rs` tests: `content_tokens_excludes_punct`, `is_passive_detects_passive`, `tree_depth_25_chain_returns_24`, `tree_depth_1000_chain_returns_999_in_bounded_time`, `tree_depth_returns_max_on_cycle`.
- `src/source/file.rs` tests: `reads_file_and_detects_markdown`, `rejects_symlink`, `rejects_oversized_file`, `accepts_file_at_size_cap`.
- `src/nlp/udpipe.rs` tests: `catch_parse_panic_converts_str_panic_to_parse_failed`, `read_and_verify_returns_bytes_on_match`, `read_and_verify_returned_bytes_are_what_was_hashed`.

Run via `cargo test`.

### 2. Integration tests — `tests/integration.rs`

End-to-end pipeline tests that exercise the public API. Many require the UDPipe model, so they're behind `#[ignore]` and run via:

```bash
cargo test --test integration -- --ignored
```

CI does not run the ignored set; they need the UDPipe model and are run by hand.

### 3. Doctests — `///` blocks with `# Examples`

Compile and run the examples in rustdoc. The `lib.rs` `parse` and `analyze_from` doctests confirm the public API examples don't bit-rot. Doctests with `no_run` confirm compilation only.

Run via `cargo test`.

## The regression discipline

**Every fixed bug gets a regression test before the fix lands.** The test exists so the specific failure cannot recur without somebody noticing.

Matra's i2 iteration codified this for the resilience floor:

- `parse_per_paragraph_scopes_sentences_to_originating_paragraph` (FM1 — the prefix-match defect)
- `parse_per_paragraph_no_inner_substring_theft` (FM1 — the inner-substring theft variant)
- `tree_depth_25_chain_returns_24` (the magic `< 20` ceiling)
- `tree_depth_returns_max_on_cycle` (the cycle silently truncating)
- `read_and_verify_returned_bytes_are_what_was_hashed` (the TOCTOU window)
- `catch_parse_panic_converts_str_panic_to_parse_failed` (the SPOF without panic boundary)

The pattern: name the test after the property being verified; the test body reproduces the historical failure conditions; the assertion checks the new correct behavior. The test is a falsifier — if the bug returns, it fires.

## Complexity benches

For algorithmic code with quadratic-class risk, add a bench that asserts the expected complexity holds. Example from `src/domain.rs`:

```rust
#[test]
fn tree_depth_1000_chain_returns_999_in_bounded_time() {
    let sent = straight_chain(1000);
    let start = std::time::Instant::now();
    let depth = sent.tree_depth();
    let elapsed = start.elapsed();
    assert_eq!(depth, 999);
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "tree_depth on 1000-chain took {elapsed:?} (expected < 50ms; suggests non-linear complexity)"
    );
}
```

The 50ms bound is generous on commodity hardware; a quadratic regression would take >1 second. The test passes loudly on linear-time and fails loudly on quadratic-time.

When adding a new algorithmic function, ask: what's the expected complexity? What input size would surface a quadratic regression? Add the bench.

## What tests verify (and don't)

**Verify requirements, not implementation.** A test that fails only because of an internal refactor is a bad test. The signature is: when the requirement still holds but the impl changes, the test still passes. Strategies:

- Test through the public API, not internals.
- Test invariants (idempotence, monotonicity, ordering), not specific call patterns.
- For algorithmic code, test the input-output relation, not the path through the algorithm.

**Don't make a test pass by weakening it.** When a test breaks, ask: did the requirement change, or is the code wrong? Never edit the assertion to match the new (broken) behavior.

## Property tests — where they help

Matra doesn't use `proptest` or `quickcheck` heavily today. Where they would help:

- Round-trip serialization tests (any type that derives Serialize+Deserialize: random instance → bytes → instance → compare).
- Tree-walk invariants (`subtree(id)` always contains the token with `id`; `head_of(id)` and `children_of(parent_id)` agree).
- Decomposer invariants (paragraph count never exceeds the line count of the input).

If you add property tests, gate them behind a `dev-dependency` and run them in CI on push to main; they can be slow.

## What tests don't verify

- **Cross-language behavior.** Python tests live in `python/` and exercise the wheel via the same pipeline. Integration with the Rust core happens through `cargo test --test integration` and the maturin build in CI.
- **Performance against absolute targets.** "Must be 10x faster than spaCy" is a benchmark, not a test. Tests verify *complexity class* (linear, not quadratic), not constant factors.
- **External library bugs.** When UDPipe ships a bug, the fix is to vendor or pin; the test that surfaces it is the integration test, not a unit test against UDPipe internals.

## CI gates

`ci.yml` declares seven jobs: **rust** (matrix over default and `--no-default-features`, on ubuntu and macos: fmt, check, clippy, test), **msrv**, **deny** (cargo-deny), **semver** (cargo-semver-checks), **docs** (rustdoc with `-Dwarnings`), **python** (maturin wheel build, install, import), and **pytype** (`mypy --strict`).

The load-bearing ones:

- **Rust matrix** — `default` features + `--no-default-features`. Runs `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **rustdoc** — `cargo doc --no-deps --all-features` with `RUSTDOCFLAGS=-Dwarnings`. Broken intra-doc links fail the build.
- **Python wheel** — `maturin build --release` then `uv pip install` then `python -c "from matra import Matra; print('ok')"`.

`just check` runs the Rust gates plus the boundary script and the docsite floor. It does not run cargo-deny, cargo-semver-checks, the wheel build or mypy, so green locally is a strong signal rather than a guarantee.

## What this skill won't tell you

- Property-test library selection (proptest vs. quickcheck) — case-by-case.
- Performance benchmarking — that's a separate tool (criterion if/when added).
- How to write good test names — that's craft, but the regression-test naming pattern above is a starting point.
