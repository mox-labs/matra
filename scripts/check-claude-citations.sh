#!/usr/bin/env bash
# Fails when a `path:line` citation in a tracked file under .claude/ points at
# a file that does not exist or at a line past that file's end. Runs from
# `just check` and the `Boundary check` job in .github/workflows/ci.yml;
# scripts/test-claude-citations.sh is its test.
#
# Why it exists: the agent and skill files under .claude/ are read as
# instructions, and nothing else checks them. They drifted twice: once into
# the architecture notes that .claude/arch/README.md records removing, and
# again after that lesson was written down, when an agent still cited a class
# at a line it had long since left. A note did not stop the second drift, so
# the lesson is a gate.
#
# A citation is a path with a file extension, a colon, and a line number or a
# range (`src/lib.rs:461`, `.claude/agents/reviewer.md:7-9`), not preceded by a
# character that could continue a path or a URL. The path resolves against the
# repository root, or, failing that, against src/, because .claude/ prose names
# Rust files as the crate does (`nlp/udpipe.rs`). A range is checked at its end.
#
# What it cannot see: a citation whose line still exists but no longer says
# what the prose claims. That stays a matter for review; this closes the cases
# a reader cannot recover from, a file that is gone and a line that is not
# there.
#
# Usage: scripts/check-claude-citations.sh [REPO_ROOT]
# Exit status: 0 when every citation resolves, 1 on a violation, 2 when the
# check could not run (git missing, not a work tree, nothing tracked under
# .claude/).

set -euo pipefail

root="${1:-$(dirname "$0")/..}"
cd "$root"

if ! command -v git >/dev/null 2>&1; then
    echo "FAIL: check-claude-citations needs git, to list the tracked files under .claude/" >&2
    exit 2
fi
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "FAIL: $(pwd) is not a git work tree" >&2
    exit 2
fi

# Tracked files only: .claude/ also holds local, untracked state (session
# notes, worktrees with a whole checkout in each) that is nobody's instruction.
files=()
while IFS= read -r -d '' f; do
    [ -f "$f" ] && files+=("$f")
done < <(git ls-files -z -- .claude)
if [ "${#files[@]}" -eq 0 ]; then
    echo "FAIL: git ls-files listed nothing under .claude/" >&2
    exit 2
fi

# Lines in a file, counting a last line that has no newline.
line_count() {
    local n
    n=$(wc -l <"$1" | tr -d ' ')
    if [ -s "$1" ] && [ "$(tail -c 1 "$1" | od -An -c | tr -d ' ')" != '\n' ]; then
        n=$((n + 1))
    fi
    echo "$n"
}

citations=0
violations=0
report=""
for f in "${files[@]}"; do
    # -n gives the citing line; -o one match per output line. The leading
    # class keeps a URL's host:port and a `lib.rs::item` path out.
    while IFS= read -r hit; do
        [ -z "$hit" ] && continue
        at=${hit%%:*}
        cite=${hit#*:}
        cite=$(printf '%s' "$cite" | sed -E 's/^[^A-Za-z0-9_.]//')
        path=${cite%:*}
        lines=${cite##*:}
        start=${lines%-*}
        end=${lines#*-}
        citations=$((citations + 1))
        target=""
        if [ -f "$path" ]; then
            target=$path
        elif [ -f "src/$path" ]; then
            target="src/$path"
        fi
        if [ -z "$target" ]; then
            report+="$f:$at cites $cite, and $path does not exist"$'\n'
            violations=$((violations + 1))
            continue
        fi
        have=$(line_count "$target")
        if [ "$start" -lt 1 ] || [ "$end" -lt "$start" ]; then
            report+="$f:$at cites $cite, which is not a line range"$'\n'
            violations=$((violations + 1))
        elif [ "$end" -gt "$have" ]; then
            report+="$f:$at cites $cite, and $target has $have lines"$'\n'
            violations=$((violations + 1))
        fi
    done < <(grep -noE '(^|[^A-Za-z0-9_./:@-])\.?[A-Za-z0-9_][A-Za-z0-9_./-]*\.[A-Za-z0-9]+:[0-9]+(-[0-9]+)?' -- "$f" || true)
done

if [ "$violations" -gt 0 ]; then
    echo "FAIL: citations under .claude/ that point at a missing file or past its end"
    printf '%s' "$report" | sed 's/^/  /'
fi
echo "check-claude-citations: $citations citation(s) in ${#files[@]} tracked files under .claude/; $violations violation(s)"
[ "$violations" -eq 0 ]
