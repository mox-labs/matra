#!/usr/bin/env bash
# Tests for scripts/check-claude-citations.sh. Runs from `just check` and the
# `Boundary check` job in .github/workflows/ci.yml.
#
# Each case builds a small git repository with a tracked .claude/ file, plants
# one citation, and asserts the check's exit status. The planted failures are
# the drift the check exists for: a file that was removed, a line past the end
# of a file, and a range that runs past it. The passing cases pin what must
# not be flagged: a citation that resolves, a bare Rust path that resolves
# under src/, a URL with a port, a `lib.rs::item` path, and a bad citation in
# an untracked file, which is local state rather than an instruction.
#
# CHECK_SCRIPT points the suite at another copy of the check, so it can be
# shown to fail against one that accepts everything.
set -uo pipefail

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)"
CHECK="${CHECK_SCRIPT:-$ROOT/scripts/check-claude-citations.sh}"

if ! command -v git >/dev/null 2>&1; then
    echo "FAIL: test-claude-citations needs git" >&2
    exit 2
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

passed=0
failed=0

# make_repo DIR CONTENT: a repository whose src/lib.rs has five lines and whose
# tracked .claude/agents/a.md holds CONTENT.
make_repo() {
    local dir=$1 content=$2
    mkdir -p "$dir/src" "$dir/.claude/agents"
    printf '1\n2\n3\n4\n5\n' >"$dir/src/lib.rs"
    printf '%s\n' "$content" >"$dir/.claude/agents/a.md"
    git -C "$dir" init -q
    git -C "$dir" add -A
}

# expect NAME WANT CONTENT: WANT is the exit status the check must return.
expect() {
    local name=$1 want=$2 content=$3 dir got
    dir="$work/$name"
    make_repo "$dir" "$content"
    bash "$CHECK" "$dir" >"$dir.out" 2>&1
    got=$?
    if [ "$got" -eq "$want" ]; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
        echo "FAIL: $name: wanted exit $want, got $got"
        sed 's/^/    /' "$dir.out"
    fi
}

expect resolves 0 'The class is at `src/lib.rs:3`.'
expect last-line 0 'See src/lib.rs:5.'
expect range-inside 0 'Lines src/lib.rs:2-5 hold it.'
expect bare-rust-path 0 'Compare `lib.rs:4`, as the crate names it.'
expect url-with-port 0 'Served at https://example.com:8080/x and http://localhost:3000.'
expect item-path 0 'Routed in `lib.rs::python`, with no line.'
expect no-citation 0 'Nothing here names a line.'

expect missing-file 1 'The notes are in `.claude/arch/ports.md:3`.'
expect past-end 1 'The class is at `src/lib.rs:6`.'
expect range-past-end 1 'Lines src/lib.rs:4-9 hold it.'
expect inverted-range 1 'Lines src/lib.rs:4-2 hold it.'
expect one-bad-among-good 1 'Good src/lib.rs:1, bad src/lib.rs:50.'

# An untracked file under .claude/ is not read, however wrong it is.
dir="$work/untracked"
make_repo "$dir" 'Good src/lib.rs:1.'
mkdir -p "$dir/.claude/logs"
printf 'Stale src/gone.rs:1\n' >"$dir/.claude/logs/notes.md"
if bash "$CHECK" "$dir" >"$dir.out" 2>&1; then
    passed=$((passed + 1))
else
    failed=$((failed + 1))
    echo "FAIL: untracked: an untracked file was read"
    sed 's/^/    /' "$dir.out"
fi

# A failure names the citing file and line, so the fix is one jump away.
dir="$work/names-the-site"
make_repo "$dir" $'one\nThe class is at `src/lib.rs:9`.'
bash "$CHECK" "$dir" >"$dir.out" 2>&1
if grep -q '\.claude/agents/a\.md:2 cites src/lib\.rs:9, and src/lib\.rs has 5 lines' "$dir.out"; then
    passed=$((passed + 1))
else
    failed=$((failed + 1))
    echo "FAIL: names-the-site: the report does not name the citing line"
    sed 's/^/    /' "$dir.out"
fi

echo "test-claude-citations: $passed passed, $failed failed"
[ "$failed" -eq 0 ]
