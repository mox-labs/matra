#!/usr/bin/env bash
# Every file that carries matra's version must carry the same one, and the
# citation file's release date must match the CHANGELOG entry for that
# version. ADR-0013 added CITATION.cff as a fifth version-carrying file and
# the only one that also carries a date, so this check exists rather than a
# printed reminder in the release recipe.
set -euo pipefail

cd "$(dirname "$0")/.."

fail=0
note() { printf '  %s\n' "$1"; }
bad() { printf 'FAIL: %s\n' "$1"; fail=1; }

# `|| true` on each plain assignment: under `set -e` with `pipefail`, an
# assignment takes its pipeline's status, so a grep that matches nothing would
# exit here silently and the diagnostics below would never print. A missing
# CHANGELOG heading is the routine mid-release state this check exists to
# catch, so it has to fail loudly rather than just fail.
canonical=$(grep -m1 '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/' || true)
[ -n "$canonical" ] || { echo "FAIL: no version line in Cargo.toml"; exit 1; }
echo "Cargo.toml version: $canonical"

check() {
    local file="$1" found="$2"
    if [ "$found" = "$canonical" ]; then
        note "ok   $file ($found)"
    else
        bad "$file says '$found', Cargo.toml says '$canonical'"
    fi
}

check pyproject.toml \
    "$(grep -m1 '^version = ' pyproject.toml | sed 's/.*"\(.*\)".*/\1/')"
check .claude-plugin/plugin.json \
    "$(grep -m1 '"version"' .claude-plugin/plugin.json | sed 's/.*"\([0-9][^"]*\)".*/\1/')"
check CITATION.cff \
    "$(grep -m1 '^version: ' CITATION.cff | sed 's/^version: *//')"
check skills/matra/SKILL.md \
    "$(grep -m1 '^version: ' skills/matra/SKILL.md | sed 's/^version: *//')"

# The citation file is the only place carrying a release date. It must agree
# with the CHANGELOG heading for the same version, or a citation will name a
# date the changelog contradicts.
cff_date=$(grep -m1 '^date-released: ' CITATION.cff | sed "s/^date-released: *'\{0,1\}//; s/'\{0,1\}$//" || true)
log_date=$(grep -m1 "^## \[$canonical\]" CHANGELOG.md | sed 's/.* - //' || true)

if [ -z "$cff_date" ]; then
    bad "CITATION.cff has no date-released"
elif [ -z "$log_date" ]; then
    bad "CHANGELOG.md has no '## [$canonical]' heading"
elif [ "$cff_date" != "$log_date" ]; then
    bad "CITATION.cff date-released is '$cff_date', CHANGELOG.md says '$log_date'"
else
    note "ok   release date ($cff_date) agrees with CHANGELOG.md"
fi

if [ "$fail" -eq 0 ]; then
    echo "Version sync: clean."
else
    echo
    echo "Every version-carrying file must move together. See ADR-0013."
    exit 1
fi
