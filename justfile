# vaani task runner. Run `just` to see all recipes.
#
# Recipes are the single source of truth for repeatable workflows.
# CI, the pre-commit hook, and humans all run the same commands.

# Default: print the recipe list.
default:
    @just --list

# ---------------------------------------------------------------------------
# Quality gates — same commands CI runs.
# ---------------------------------------------------------------------------

# Run every CI gate locally. If this passes, CI will pass.
check: fmt-check check-rust check-rust-no-default clippy clippy-no-default doc test test-no-default boundary
    @echo ""
    @echo "all gates pass"

# Run the Python type-check (mypy) over the Python sources + stubs.
# Requires `pip install -e '.[typecheck]'` once.
typecheck:
    python -m mypy

# Format check (read-only).
fmt-check:
    cargo fmt --all -- --check

# Rewrite source to canonical formatting.
fmt:
    cargo fmt --all

# Type-check the workspace under default features.
check-rust:
    cargo check --all-targets

# Type-check the workspace with no default features.
check-rust-no-default:
    cargo check --all-targets --no-default-features

# Clippy under default features, warnings are errors.
clippy:
    RUSTFLAGS="-Dwarnings" cargo clippy --all-targets -- -D warnings

# Clippy with no default features, warnings are errors.
clippy-no-default:
    RUSTFLAGS="-Dwarnings" cargo clippy --all-targets --no-default-features -- -D warnings

# Build docs with all features, broken intra-doc links are errors.
doc:
    RUSTDOCFLAGS="-Dwarnings" cargo doc --no-deps --all-features

# Unit + doctest under default features.
test:
    cargo test --features udpipe

# Unit + doctest with no default features.
test-no-default:
    cargo test --no-default-features

# Boundary check: hex-architecture rules from CLAUDE.md (3, 4, 8).
boundary:
    bash scripts/check-boundaries.sh

# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

# Install the pre-commit hook into .git/hooks/.
install-hooks:
    bash scripts/install-hooks.sh

# ---------------------------------------------------------------------------
# Release
# ---------------------------------------------------------------------------

# Roll the changelog and bump Cargo.toml in preparation for a release.
# Does NOT publish. Inspect the result, then run `just release VERSION`.
release-prep VERSION:
    bash scripts/changelog-release.sh {{VERSION}}
    @echo ""
    @echo "release prep complete for {{VERSION}}"
    @echo ""
    @echo "next: review the diff, then:"
    @echo "  cargo publish --dry-run --features udpipe"
    @echo "  just release {{VERSION}}"

# Publish the current commit as VERSION. Requires explicit per-publish
# approval (see CONTRIBUTING.md). This recipe stops short of the actual
# publish; it prints the command so the human runs it deliberately.
release VERSION:
    @echo "STOP. Publishing is a deliberate, per-call action."
    @echo "Verify:"
    @echo "  - git log/diff matches what you expect"
    @echo "  - cargo publish --dry-run --features udpipe is clean"
    @echo "  - the [{{VERSION}}] section of CHANGELOG.md is correct"
    @echo ""
    @echo "Then run yourself:"
    @echo "  cargo publish --features udpipe"
    @echo "  git tag v{{VERSION}}"
    @echo "  git push --follow-tags"
