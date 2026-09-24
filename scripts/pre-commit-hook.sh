#!/usr/bin/env bash
# matra pre-commit hook.
#
# Runs the Rust gates plus the boundary check. CI runs both, and
# additionally the docsite floor, cargo-deny, cargo-semver-checks, the wheel
# build and mypy. Green here is a strong signal, not a guarantee.
#
# Bypass with `git commit --no-verify` when intent justifies it.
# Default behavior is tight; that is the point.
#
# Install via: bash scripts/install-hooks.sh

set -euo pipefail

# The installed hook is a copy, so it goes on running whatever this file said
# when it was installed. One installed before the rename kept announcing
# itself under the old project name and claiming parity with CI that did not
# hold. Say so when the copy has drifted rather than run stale gates quietly.
if ! cmp -s "$0" scripts/pre-commit-hook.sh; then
    echo "WARNING: this pre-commit hook differs from scripts/pre-commit-hook.sh;" >&2
    echo "         run \`just install-hooks\` to update it" >&2
fi

# Heuristic: skip the heavy gates on commits that touch no Rust source.
# Pure docs / template commits do not need clippy and doc to run.
staged=$(git diff --cached --name-only --diff-filter=ACMR)
rust_touched=false
if echo "$staged" | grep -qE '\.rs$|^Cargo\.(toml|lock)$|^crates/'; then
    rust_touched=true
fi

echo "matra pre-commit gate"
echo "  staged files: $(echo "$staged" | wc -l | tr -d ' ')"
echo "  rust gates:   $rust_touched"

# Boundary script always runs — catches accidental imports of forbidden
# crates anywhere in src/, regardless of what was staged.
# Unconditional: the old `[ -x ... ]` guard skipped it without a word
# whenever the file lost its mode bit.
bash scripts/check-boundaries.sh

if [ "$rust_touched" = true ]; then
    cargo fmt --all -- --check
    cargo check --all-targets
    cargo check --all-targets --no-default-features
    RUSTFLAGS="-Dwarnings" cargo clippy --all-targets -- -D warnings
    RUSTFLAGS="-Dwarnings" cargo clippy --all-targets --no-default-features -- -D warnings
    RUSTDOCFLAGS="-Dwarnings" cargo doc --no-deps --all-features
    cargo test --features udpipe --quiet
    cargo test --no-default-features --quiet
fi

echo "pre-commit gate: pass"
