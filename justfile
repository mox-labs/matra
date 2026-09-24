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

# Run the local gate suite: the Rust gates, the boundary check and the
# docsite floor, which CI also runs, plus the Python lint, the end-to-end
# sandbox test and the version sync, which CI does not run on a pull request.
# CI additionally runs cargo-deny, cargo-semver-checks, the wheel build and
# mypy.
check: fmt-check check-rust check-rust-no-default clippy clippy-no-default doc test test-cli test-no-default lint-py boundary test-sandbox docs-floor version-sync
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

# Unit + the CLI's own tests. The `cli` feature is not in the default
# set, so `just test` alone never compiles src/cli/.
test-cli:
    cargo test --features cli

# Unit + doctest with no default features.
test-no-default:
    cargo test --no-default-features

# Boundary check: hex-architecture rules from CLAUDE.md (3, 4, 8).
boundary:
    bash scripts/check-boundaries.sh

# The end-to-end sandbox script cannot report a clean result for a tree it did
# not examine without saying so. Needs an unprivileged user for the unreadable and unwritable
# cases; under uid 0 it skips those and says so.
test-sandbox:
    bash scripts/test-e2e-sandbox.sh

# Every version-carrying file agrees, and CITATION.cff's release date matches
# the CHANGELOG heading for that version (RFC-0013).
version-sync:
    bash scripts/check-version-sync.sh

# Requires mdbook and ripgrep. lychee is optional locally (skip-with-warning);
# the `Docsite floor` job in ci.yml installs it and sets LYCHEE_REQUIRED=1 to
# escalate the skip into a hard failure.
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
# Run the conformance suite across every crust. Both models are fetched on
# the first run and cached after it: UDPipe into the resolved model
# directory, potion-base-8M into that directory or MATRA_MODEL2VEC_DIR.
# Each is pinned by a digest in the source, so a first run needs a network.
conformance:
    cargo test --test conformance -- --ignored
    cargo test --features model2vec --test semantic_conformance -- --include-ignored
    cargo test --test corpus_conformance
    uv run --extra test pytest python/tests/test_conformance.py python/tests/test_semantic_conformance.py python/tests/test_corpus_conformance.py -q


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

# Release VERSION. Nothing publishes from a laptop and nothing is tagged
# from one either: .github/workflows/release.yml is dispatched from main,
# verifies the version, creates the annotated tag itself, builds and
# attests every artifact, then pauses at the `crates-io` and `pypi`
# environment gates for a required-reviewer approval before either
# registry is written. Those two approvals are the canonical per-publish
# approval points. This recipe prints the dispatch command.
release VERSION:
    @echo "Pre-release checks (the workflow re-checks all of these):"
    @echo "  - the commit you want to release is merged to main"
    @echo "  - cargo publish --dry-run --features udpipe is clean"
    @echo "  - the [{{VERSION}}] section of CHANGELOG.md is correct"
    @echo "  - just version-sync is clean and reports {{VERSION}}"
    @echo "  - CITATION.cff date-released is the date you are actually releasing"
    @echo "  - the maturin image digest in release.yml is current enough"
    @echo ""
    @echo "When ready, dispatch the release from main:"
    @echo "  gh workflow run release.yml --ref main -f version={{VERSION}}"
    @echo ""
    @echo "To retry a publish against the tag it already created:"
    @echo "  gh workflow run release.yml --ref main -f version={{VERSION}} -f skip_tag=true"
    @echo ""
    @echo "The run pauses twice, once per registry. Approve each deployment"
    @echo "in the GitHub Actions UI. A post-publish smoke job then installs"
    @echo "{{VERSION}} from PyPI and crates.io on Linux and macOS."
