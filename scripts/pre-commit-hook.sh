#!/usr/bin/env bash
# vaani pre-commit hook.
#
# Runs the same gates that CI runs, in the same order. If this passes
# locally, CI will pass too. If this fails, CI would have failed.
#
# Bypass with `git commit --no-verify` when intent justifies it.
# Default behavior is tight; that is the point.
#
# Install via: bash scripts/install-hooks.sh

set -euo pipefail

# Heuristic: skip the heavy gates on commits that touch no Rust source.
# Pure docs / template commits do not need clippy and doc to run.
staged=$(git diff --cached --name-only --diff-filter=ACMR)
rust_touched=false
if echo "$staged" | grep -qE '\.rs$|^Cargo\.(toml|lock)$|^crates/'; then
    rust_touched=true
fi

echo "vaani pre-commit gate"
echo "  staged files: $(echo "$staged" | wc -l | tr -d ' ')"
echo "  rust gates:   $rust_touched"

# Boundary script always runs — catches accidental imports of forbidden
# crates anywhere in src/, regardless of what was staged.
if [ -x scripts/check-boundaries.sh ]; then
    bash scripts/check-boundaries.sh
fi

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
