#!/usr/bin/env bash
# Verifies that matra's design record in blueprints/ is closed under citation.
# Runs from `just check` and the `Docsite floor` job in .github/workflows/ci.yml.
#
#   1. Every record cited anywhere in the tracked tree resolves to a file of
#      its own kind:
#        EPR-NNNN  blueprints/proposals/NNNN-*.md   enhancement proposal
#        EPL-NNNN  blueprints/plans/NNNN-*.md       enhancement plan
#        RFC-NNNN  blueprints/legacy/rfcs/NNNN-*.md  legacy record
#        EP-NNNN   blueprints/legacy/eps/NNNN-*.md   legacy record
#      A legacy number listed in the Legacy table of blueprints/README.md
#      (a number reserved and never written) also resolves. A citation whose
#      number has a file only under another kind is reported as a wrong-kind
#      citation, naming the file it probably meant.
#   2. Every file in blueprints/proposals/ and blueprints/plans/ has a row in
#      that index, linked by its path.
#
# A citation is a prefix, a hyphen, and four digits, as a whole word.
# Placeholders spelled with letters (the N-for-digit form the templates and
# the process docs use) never match.
#
# Scope is every file `git ls-files` lists but the local comments in
# discussion/ and the components fixture in site/scripts/fixtures/,
# CHANGELOG.md included.
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
# discussion/ is left out: it is the docsite's local comments, a conversation
# about the records rather than a record, and a comment may name the proposal
# it suggests before that number has a file. site/scripts/fixtures/ is left
# out too: the Blueprints components' test fixture numbers a made-up record
# (numbered 9999) to draw its index card, and that number cites nothing.
tracked=()
while IFS= read -r -d '' f; do
    case "$f" in discussion/* | site/scripts/fixtures/*) continue ;; esac
    [ -f "$f" ] && tracked+=("$f")
done < <(git ls-files -z)
if [ "${#tracked[@]}" -eq 0 ]; then
    echo "FAIL: git ls-files listed no files" >&2
    exit 2
fi

# rg exits 0 on a match, 1 on no match, 2 on an error. Only 0 and 1 are results.
rg_rc=0
cites=$(rg -oIN --no-heading -e '\b(EPR|EPL|RFC|EP)-[0-9]{4}\b' -- "${tracked[@]}") || rg_rc=$?
if [ "$rg_rc" -gt 1 ]; then
    echo "FAIL: rg exited $rg_rc searching the tracked tree" >&2
    exit 2
fi

# Legacy numbers that were reserved and never written: rows of the Legacy table.
unwritten=$(rg -oN --no-heading --replace '$1' -e '^\| ((RFC|EP)-[0-9]{4}) \| Reserved' "$index") || true

dir_of() {
    case "$1" in
    EPR) echo blueprints/proposals ;;
    EPL) echo blueprints/plans ;;
    RFC) echo blueprints/legacy/rfcs ;;
    EP) echo blueprints/legacy/eps ;;
    esac
}

fail=0
violations=0
flag() {
    echo "$1"
    printf '%s\n' "$2" | sed 's/^/  /'
    violations=$((violations + $(printf '%s\n' "$2" | wc -l)))
    fail=1
}

# Property 1: every citation resolves, to a file of its own kind.
total=0
distinct=0
unresolved=""
wrongkind=""
while IFS= read -r id; do
    [ -z "$id" ] && continue
    distinct=$((distinct + 1))
    kind=${id%-*}
    num=${id##*-}
    if compgen -G "$(dir_of "$kind")/$num-*.md" >/dev/null; then
        continue
    fi
    if printf '%s\n' "$unwritten" | grep -Fxq -- "$id"; then
        continue
    fi
    where=$(rg -lw --fixed-strings -e "$id" -- "${tracked[@]}" | head -5 | tr '\n' ' ') || true
    other=""
    for k in EPR EPL RFC EP; do
        [ "$k" = "$kind" ] && continue
        hit=$(compgen -G "$(dir_of "$k")/$num-*.md" | head -1) || true
        [ -n "$hit" ] && other+="$k-$num is $hit; "
    done
    if [ -n "$other" ]; then
        wrongkind+="$id names no file in $(dir_of "$kind")/; ${other}cited in: ${where% }"$'\n'
    else
        unresolved+="$id (cited in: ${where% })"$'\n'
    fi
done < <(printf '%s\n' "$cites" | sort -u)
if [ -n "$cites" ]; then
    total=$(printf '%s\n' "$cites" | wc -l | tr -d ' ')
fi
if [ -n "$wrongkind" ]; then
    flag "FAIL: citations of the wrong kind (EPR proposals/, EPL plans/, RFC legacy/rfcs/, EP legacy/eps/)" "${wrongkind%$'\n'}"
fi
if [ -n "$unresolved" ]; then
    flag "FAIL: cited records with no file in blueprints/" "${unresolved%$'\n'}"
fi

# Property 2: every proposal and plan has an index row.
records=0
unindexed=""
for f in blueprints/proposals/*.md blueprints/plans/*.md; do
    [ -f "$f" ] || continue
    records=$((records + 1))
    rel=${f#blueprints/}
    if ! grep -Fq -- "]($rel)" "$index"; then
        unindexed+="$f"$'\n'
    fi
done
if [ "$records" -eq 0 ]; then
    echo "FAIL: found no files under blueprints/proposals/ or blueprints/plans/ (not even the templates)" >&2
    exit 2
fi
if [ -n "$unindexed" ]; then
    flag "FAIL: records with no row in $index" "${unindexed%$'\n'}"
fi

legacy=$(find blueprints/legacy -name '*.md' -type f 2>/dev/null | wc -l | tr -d ' ')
echo "check-blueprint-refs: $total citations of $distinct records across ${#tracked[@]} tracked files; $records proposal and plan files checked against the index; $legacy legacy records resolvable; $violations violation(s)"
exit "$fail"
