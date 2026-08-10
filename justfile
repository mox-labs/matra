# matra task runner. Run `just` to see all recipes.
#
# Recipes are the single source of truth for repeatable workflows.
# CI, the pre-commit hook, and humans all run the same commands.

# Default: print the recipe list.
default:
    @just --list

# ---------------------------------------------------------------------------
# Quality gates — same commands CI runs.
# ---------------------------------------------------------------------------

# Run the local gate suite: the Rust gates CI runs, plus the boundary
# check and docsite floor, which CI does not run. CI additionally runs
# cargo-deny, cargo-semver-checks, the wheel build and mypy.
check: fmt-check check-rust check-rust-no-default clippy clippy-no-default doc test test-no-default boundary docs-floor
    @echo ""
    @echo "all gates pass"

# Run the Python type-check (mypy) over the Python sources + stubs.
# Requires `uv pip install -e '.[typecheck]'` once.
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

# Requires mdbook + mdbook-mermaid (install: `cargo install mdbook mdbook-mermaid`).
# lychee is optional locally (skip-with-warning); CI installs it and sets
# LYCHEE_REQUIRED=1 to escalate the skip into a hard failure.
# Floor gates for the docsite: link integrity, orphan detect, type-name parity, mdbook clean build.
docs-floor:
    bash scripts/check-docsite-floor.sh

# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

# Install the pre-commit hook into .git/hooks/.
install-hooks:
    bash scripts/install-hooks.sh

# ---------------------------------------------------------------------------
# Run the conformance suite across every crust. Requires the UDPipe model.
conformance:
    cargo test --test conformance -- --ignored
    uv run pytest python/tests/test_conformance.py -q


# ---------------------------------------------------------------------------
# Quality
# ---------------------------------------------------------------------------

# Lint every language surface.
lint: clippy clippy-no-default lint-py

# Ruff over the Python surface.
lint-py:
    uv run --extra lint ruff check python/
    uv run --extra lint ruff format --check python/

# Python tests. Conformance needs the model; use `just test-py-fast` without it.
test-py:
    uv run --extra test pytest

test-py-fast:
    uv run --extra test pytest -m "not model"

# Rust line coverage. Prints a summary and writes lcov.info.
#
# Needs llvm-tools in the active toolchain's sysroot:
#   rustup component add llvm-tools-preview
coverage:
    cargo llvm-cov --features cli --lcov --output-path lcov.info
    cargo llvm-cov --features cli --summary-only

# Python line coverage.
coverage-py:
    uv run --extra test pytest --cov --cov-report=term --cov-report=xml

# Coverage across both crusts.
coverage-all: coverage coverage-py

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

# Tag and push for the current VERSION. The actual cargo publish runs
# inside the `crates-io` GitHub environment via .github/workflows/publish.yml,
# which pauses for a required-reviewer approval before invoking
# `cargo publish` via Trusted Publishing (OIDC, no long-lived tokens).
# That environment gate is the canonical per-publish approval point;
# this recipe just creates and pushes the tag.
release VERSION:
    @echo "Pre-release checks:"
    @echo "  - git log/diff matches what you expect"
    @echo "  - cargo publish --dry-run --features udpipe is clean"
    @echo "  - the [{{VERSION}}] section of CHANGELOG.md is correct"
    @echo "  - Cargo.toml + pyproject.toml versions == {{VERSION}}"
    @echo ""
    @echo "When ready, push the tag:"
    @echo "  git tag -s v{{VERSION}} -m 'v{{VERSION}}'"
    @echo "  git push --follow-tags"
    @echo ""
    @echo "The publish workflow will then pause at the crates-io environment"
    @echo "gate. Approve in the GitHub Actions UI to fire cargo publish."
