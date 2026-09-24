#!/usr/bin/env bash
# Verifies that matra's design record in blueprints/ is closed under citation.
# Runs from `just check` and the `Docsite floor` job in .github/workflows/ci.yml.
#
#   1. Every RFC or EP number cited anywhere in the tracked tree resolves: an
#      RFC to a file blueprints/rfcs/<number>-*.md or to a row of the index in
#      blueprints/README.md marked `open, reserved`; an EP to a file
#      blueprints/eps/<number>-*.md.
#   2. Every file in blueprints/rfcs/ and blueprints/eps/ has a row in that
#      index, linked by its path.
#
# A citation is the prefix RFC or EP, a hyphen, and four digits, as a whole
# word. Placeholders spelled with letters (the N-for-digit form the templates
# and the process docs use) never match.
#
# Scope is every file `git ls-files` lists, CHANGELOG.md included. Released
# CHANGELOG entries cite the older ADR prefix, which this does not match, so
# they need no exclusion; blueprints/README.md maps that prefix to the RFC of
# the same number. A new entry citing an RFC or an EP is checked like any
# other file, which is the point: a changelog line pointing at a record that
# does not exist is exactly the drift this catches.
#
# Exit status: 0 when both properties hold, 1 on a violation, 2 when the check
# could not run (rg or git missing, the index absent, nothing to examine).

set -euo pipefail

cd "$(dirname "$0")/.."

# rg is required. The citations come from an rg search, and a search that
# never ran returns the same empty output as a search that found nothing.
if ! command -v rg >/dev/null 2>&1; then
    echo "FAIL: check-blueprint-refs needs ripgrep (rg), which is not on PATH" >&2
    echo "      install: brew install ripgrep, apt-get install ripgrep, or cargo install ripgrep" >&2
    exit 2
fi
if ! command -v git >/dev/null 2>&1; then
    echo "FAIL: check-blueprint-refs needs git, to list the tracked tree" >&2
    exit 2
fi

index=blueprints/README.md
if [ ! -f "$index" ]; then
    echo "FAIL: $index does not exist" >&2
    exit 2
fi

# The tracked tree. A symlink (site/content/roadmap.md) is searched through, so
# its target's citations are counted twice; that changes a count, not a result.
tracked=()
while IFS= read -r -d '' f; do
    [ -f "$f" ] && tracked+=("$f")
done < <(git ls-files -z)
if [ "${#tracked[@]}" -eq 0 ]; then
    echo "FAIL: git ls-files listed no files" >&2
    exit 2
fi

# rg exits 0 on a match, 1 on no match, 2 on an error. Only 0 and 1 are results.
rg_rc=0
cites=$(rg -oIN --no-heading -e '\b(RFC|EP)-[0-9]{4}\b' -- "${tracked[@]}") || rg_rc=$?
if [ "$rg_rc" -gt 1 ]; then
    echo "FAIL: rg exited $rg_rc searching the tracked tree" >&2
    exit 2
fi

reserved=$(rg -oN --no-heading --replace '$1' -e '^\| RFC-([0-9]{4}) \|.*\| open, reserved \|' "$index") || true

fail=0
violations=0
flag() {
    echo "$1"
    printf '%s\n' "$2" | sed 's/^/  /'
    violations=$((violations + $(printf '%s\n' "$2" | wc -l)))
    fail=1
}

# Property 1: every citation resolves.
total=0
distinct=0
unresolved=""
while IFS= read -r id; do
    [ -z "$id" ] && continue
    distinct=$((distinct + 1))
    kind=${id%%-*}
    num=${id#*-}
    if [ "$kind" = RFC ]; then dir=blueprints/rfcs; else dir=blueprints/eps; fi
    if compgen -G "$dir/$num-*.md" >/dev/null; then
        continue
    fi
    if [ "$kind" = RFC ] && printf '%s\n' "$reserved" | grep -Fxq -- "$num"; then
        continue
    fi
    where=$(rg -lw --fixed-strings -e "$id" -- "${tracked[@]}" | head -5 | tr '\n' ' ') || true
    unresolved+="$id (cited in: ${where% })"$'\n'
done < <(printf '%s\n' "$cites" | sort -u)
if [ -n "$cites" ]; then
    total=$(printf '%s\n' "$cites" | wc -l | tr -d ' ')
fi
if [ -n "$unresolved" ]; then
    flag "FAIL: cited records with no file in blueprints/ and no reserved row in $index" "${unresolved%$'\n'}"
fi

# Property 2: every record file has an index row.
records=0
unindexed=""
for f in blueprints/rfcs/*.md blueprints/eps/*.md; do
    [ -f "$f" ] || continue
    records=$((records + 1))
    rel=${f#blueprints/}
    if ! grep -Fq -- "]($rel)" "$index"; then
        unindexed+="$f"$'\n'
    fi
done
if [ "$records" -eq 0 ]; then
    echo "FAIL: found no records under blueprints/rfcs/ or blueprints/eps/" >&2
    exit 2
fi
if [ -n "$unindexed" ]; then
    flag "FAIL: records with no row in $index" "${unindexed%$'\n'}"
fi

echo "check-blueprint-refs: $total citations of $distinct records across ${#tracked[@]} tracked files; $records record files checked against the index; $violations violation(s)"
exit "$fail"
