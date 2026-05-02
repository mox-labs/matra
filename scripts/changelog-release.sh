#!/usr/bin/env bash
# Rolls CHANGELOG.md for a release.
#
# Renames the existing `## [Unreleased]` section to `## [X.Y.Z] - YYYY-MM-DD`
# and prepends a fresh empty `## [Unreleased]` section above it.
#
# Usage: bash scripts/changelog-release.sh 0.2.0
#
# Does NOT publish, does NOT touch Cargo.toml, does NOT git commit.
# Run this, review the diff, then commit deliberately.

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "usage: $0 <version>" >&2
    echo "  e.g. $0 0.2.0" >&2
    exit 2
fi

version="$1"
date="$(date -u +%Y-%m-%d)"
changelog="CHANGELOG.md"

if [ ! -f "$changelog" ]; then
    echo "$changelog not found; run from repo root" >&2
    exit 1
fi

if ! grep -qE '^## \[Unreleased\]\s*$' "$changelog"; then
    echo "no '## [Unreleased]' section found in $changelog" >&2
    echo "expected the canonical Keep-a-Changelog header" >&2
    exit 1
fi

if grep -qE "^## \[$version\]" "$changelog"; then
    echo "version $version already present in $changelog" >&2
    exit 1
fi

# Replace the first `## [Unreleased]` line with a fresh empty Unreleased
# block followed by the dated release header.
tmp=$(mktemp)
awk -v ver="$version" -v dt="$date" '
    /^## \[Unreleased\][[:space:]]*$/ && !done {
        print "## [Unreleased]"
        print ""
        print "## [" ver "] - " dt
        done = 1
        next
    }
    { print }
' "$changelog" > "$tmp"
mv "$tmp" "$changelog"

echo "rolled $changelog: [Unreleased] -> [$version] - $date"
echo ""
echo "next steps:"
echo "  1. review the diff: git diff $changelog"
echo "  2. ensure the [$version] section has Highlights and structured bullets"
echo "  3. bump version in Cargo.toml"
echo "  4. cargo publish --dry-run --features udpipe"
echo "  5. when ready, just release $version (manual gate)"
