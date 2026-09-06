#!/usr/bin/env bash
# Tests for scripts/e2e-sandbox.sh. Runs from `just check`.
#
# The sandbox script has one property, and everything here asserts a corner of
# it: it must not be able to report a clean result for a tree it did not
# examine. Three review rounds each closed a hole in that property and left
# another, because the script had no test and no gate. This is the gate.
#
# Two shapes of failure are covered, because the script has two entry points
# and each one fails differently.
#
#   `new` must leave stdout EMPTY when anything goes wrong. The documented
#   entry is `eval "$(... new)"`, whose exit status is the status of the
#   exports it evaluates rather than of the script, so a partial line on
#   stdout moves HOME into a sandbox nothing can recognise.
#
#   `snapshot` must put its failures INSIDE the artifact that gets diffed. The
#   documented procedure takes one before and one after and diffs the pair, and
#   never says to check the status, so a failure visible only on stderr and in
#   an exit code is a failure a diff-clean pair hides.
#
# Cases that need an unwritable or unreadable directory are skipped under uid
# 0, where the permission bits do not hold, and the summary says how many were
# skipped and why.
set -uo pipefail

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)"
# Overridable so the suite can be pointed at an older copy of the script and
# shown to fail. A test suite nobody has watched fail is not evidence, and this
# one exists because three review rounds argued about prose instead. Run
# `SANDBOX_SCRIPT=<old copy> bash scripts/test-e2e-sandbox.sh` to check it.
SCRIPT="${SANDBOX_SCRIPT:-$ROOT/scripts/e2e-sandbox.sh}"

if [ ! -f "$SCRIPT" ]; then
    echo "FAIL: cannot find $SCRIPT" >&2
    exit 1
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/matra-e2e-test.XXXXXX")"
# Physical, because the script resolves the sandbox directory with `pwd -P`
# and on macOS $TMPDIR sits behind /var -> /private/var. Comparing an
# exported path against an unresolved fixture path fails for the wrong reason.
WORK="$(CDPATH='' cd -- "$WORK" && pwd -P)"
# The cleanup refuses anything that is not the scratch tree this run created.
# It is a recursive delete in a trap, so it checks rather than trusts: a name
# this script made with mktemp, still a directory, and not the root.
cleanup() {
    case "$WORK" in
        */matra-e2e-test.??????) ;;
        *) echo "refusing to clean an unexpected path: ${WORK:-empty}" >&2; return ;;
    esac
    [ -d "$WORK" ] || return
    # u+rwX first: several cases leave a directory unreadable or unwritable on
    # purpose, and a cleanup that cannot descend leaves the scratch tree behind.
    chmod -R u+rwX "$WORK" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

passed=0
failed=0
skipped=0

pass() { passed=$((passed + 1)); printf 'ok    %s\n' "$1"; }
skip() { skipped=$((skipped + 1)); printf 'skip  %s\n        %s\n' "$1" "$2"; }
fail() {
    failed=$((failed + 1))
    printf 'FAIL  %s\n' "$1"
    shift
    while [ $# -gt 0 ]; do printf '        %s\n' "$1"; shift; done
}

is_root() { [ "$(id -u)" -eq 0 ]; }

# Run the script under test with an environment this file controls rather than
# whatever the contributor's shell carries. Everything matra resolves through
# is unset first, so a MATRA_DATA_DIR in the caller's environment cannot widen
# or narrow a fixture.
#
#   run_sandbox <home> [VAR=VALUE ...] -- <script argument ...>
#
# Pass the empty string as <home> to leave HOME unset.
run_sandbox() {
    local home="$1"
    shift
    local kv
    (
        unset MATRA_CONFIG_FILE MATRA_DATA_DIR MATRA_MODEL_DIR
        unset XDG_CONFIG_HOME XDG_DATA_HOME XDG_CACHE_HOME
        unset E2E_SANDBOX_ROOT CDPATH
        if [ -n "$home" ]; then export HOME="$home"; else unset HOME; fi
        while [ "${1:-}" != "--" ]; do
            kv="$1"
            shift
            export "${kv?}"
        done
        shift
        bash "$SCRIPT" "$@"
    )
}

# ---------------------------------------------------------------------------
# `new` fails closed: stdout is empty on every induced failure
# ---------------------------------------------------------------------------

# The sandbox home exists and already has the subdirectories in it, so
# `mkdir -p` succeeds as a no-op and the first thing that can fail is the
# marker write. This is the shape that made the marker ordering necessary.
case_unwritable_home() {
    local name="new: unwritable sandbox home, mkdir -p a no-op"
    if is_root; then
        skip "$name" "uid 0 writes through a mode that denies write"
        return
    fi
    local d="$WORK/unwritable"
    mkdir -p "$d/home/.config" "$d/home/.local/share"
    chmod a-w "$d/home"
    local out status
    out="$(run_sandbox "$WORK/real" -- new "$d" 2>/dev/null)"
    status=$?
    chmod u+w "$d/home"
    if [ -n "$out" ]; then
        fail "$name" "stdout was not empty: $out"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    else
        pass "$name"
    fi
}

case_readonly_parent() {
    local name="new: read-only parent, mkdir cannot create the sandbox"
    if is_root; then
        skip "$name" "uid 0 writes through a mode that denies write"
        return
    fi
    local d="$WORK/ro-parent"
    mkdir -p "$d"
    chmod a-w "$d"
    local out status
    out="$(run_sandbox "$WORK/real" -- new "$d/sbx" 2>/dev/null)"
    status=$?
    chmod u+w "$d"
    if [ -n "$out" ]; then
        fail "$name" "stdout was not empty: $out"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    else
        pass "$name"
    fi
}

case_marker_is_a_directory() {
    local name="new: marker path occupied by a directory"
    local d="$WORK/marker-dir"
    mkdir -p "$d/home/.e2e-sandbox"
    local out status
    out="$(run_sandbox "$WORK/real" -- new "$d" 2>/dev/null)"
    status=$?
    if [ -n "$out" ]; then
        fail "$name" "stdout was not empty: $out"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    else
        pass "$name"
    fi
}

case_tmpdir_nowhere() {
    local name="new: TMPDIR points nowhere and no directory is named"
    local out status
    out="$(run_sandbox "$WORK/real" "TMPDIR=$WORK/no-such-tmpdir" -- new 2>/dev/null)"
    status=$?
    if [ -n "$out" ]; then
        fail "$name" "stdout was not empty: $out"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    else
        pass "$name"
    fi
}

# H1. With CDPATH set, a bare `cd relative` resolves against it and echoes the
# resolved path, so the command substitution captured two lines, the sandbox
# was built inside the CDPATH target, and HOME was exported as a two-line
# string naming a directory nothing had been written to.
case_cdpath() {
    local name="new: CDPATH does not move the sandbox out of the named directory"
    mkdir -p "$WORK/cdpath/target/sbx" "$WORK/cdpath/cwd/sbx"
    local out status
    out="$(
        cd "$WORK/cdpath/cwd" || exit 1
        unset MATRA_CONFIG_FILE MATRA_DATA_DIR MATRA_MODEL_DIR
        unset XDG_CONFIG_HOME XDG_DATA_HOME XDG_CACHE_HOME E2E_SANDBOX_ROOT
        HOME="$WORK/real" CDPATH="$WORK/cdpath/target" bash "$SCRIPT" new sbx 2>/dev/null
    )"
    status=$?
    local homes
    homes="$(printf '%s\n' "$out" | grep -c '^export HOME=')"
    if [ "$status" -ne 0 ]; then
        fail "$name" "exited $status"
    elif [ "$homes" -ne 1 ]; then
        fail "$name" "expected exactly one exported HOME, found $homes"
    elif ! printf '%s\n' "$out" | grep -q "^export HOME=.*$WORK/cdpath/cwd/sbx/home\$"; then
        fail "$name" "HOME does not name the directory that was asked for:" \
            "$(printf '%s\n' "$out" | grep '^export HOME=')"
    elif [ ! -f "$WORK/cdpath/cwd/sbx/home/.e2e-sandbox" ]; then
        fail "$name" "the marker did not land in the named directory"
    elif [ -e "$WORK/cdpath/target/sbx/home" ]; then
        fail "$name" "the sandbox was built inside the CDPATH target"
    else
        pass "$name"
    fi
}

# M1. mkdir -p is a no-op through a symlink, so the tree looked built and the
# marker was written into the link's target. When that target is the real home,
# the real home is left holding a marker that makes every later snapshot refuse.
case_symlinked_sandbox_home() {
    local name="new: refuses a symlinked sandbox home"
    mkdir -p "$WORK/decoy" "$WORK/slink"
    ln -s "$WORK/decoy" "$WORK/slink/home"
    local out status
    out="$(run_sandbox "$WORK/real" -- new "$WORK/slink" 2>/dev/null)"
    status=$?
    if [ -n "$out" ]; then
        fail "$name" "stdout was not empty: $out"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    elif [ -e "$WORK/decoy/.e2e-sandbox" ]; then
        fail "$name" "a marker was written into the link's target"
    else
        pass "$name"
    fi
}

case_sandbox_home_is_real_home() {
    local name="new: refuses when the sandbox home resolves to the real home"
    mkdir -p "$WORK/selfhome/home"
    local out status
    out="$(run_sandbox "$WORK/selfhome/home" -- new "$WORK/selfhome" 2>/dev/null)"
    status=$?
    if [ -n "$out" ]; then
        fail "$name" "stdout was not empty: $out"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    else
        pass "$name"
    fi
}

case_home_unset() {
    local name="HOME unset produces the script's own error"
    local err status
    err="$(run_sandbox "" -- snapshot 2>&1 >/dev/null)"
    status=$?
    if [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    elif ! printf '%s\n' "$err" | grep -q 'HOME is unset'; then
        fail "$name" "stderr did not name HOME: $err"
    else
        pass "$name"
    fi
}

case_new_happy_path() {
    local name="new: the happy path exports a usable sandbox"
    local d="$WORK/happy"
    local out status
    out="$(run_sandbox "$WORK/real" -- new "$d" 2>/dev/null)"
    status=$?
    if [ "$status" -ne 0 ]; then
        fail "$name" "exited $status"
    elif [ ! -f "$d/home/.e2e-sandbox" ]; then
        fail "$name" "no marker at $d/home/.e2e-sandbox"
    elif ! printf '%s\n' "$out" | grep -q '^export XDG_CACHE_HOME='; then
        fail "$name" "XDG_CACHE_HOME is not exported"
    elif ! printf '%s\n' "$out" | grep -q '^export E2E_SANDBOX_ROOT='; then
        fail "$name" "E2E_SANDBOX_ROOT is not exported"
    else
        pass "$name"
    fi
}

# ---------------------------------------------------------------------------
# The guard: snapshot refuses from inside a sandbox, by either route
# ---------------------------------------------------------------------------

case_guard_environment_route() {
    local name="snapshot: refuses via E2E_SANDBOX_ROOT"
    mkdir -p "$WORK/guard-env"
    local out status
    out="$(run_sandbox "$WORK/guard-env" "E2E_SANDBOX_ROOT=$WORK/guard-env" -- snapshot 2>/dev/null)"
    status=$?
    if [ -n "$out" ]; then
        fail "$name" "stdout was not empty: $out"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    else
        pass "$name"
    fi
}

# The environment route is the one the documented procedure destroys: the skill
# tells a tester to unset every MATRA_* variable and a tester who over-scrubs
# takes E2E_SANDBOX_ROOT with it. The marker is what is left.
case_guard_marker_route() {
    local name="snapshot: refuses via the marker file alone"
    mkdir -p "$WORK/guard-marker"
    : > "$WORK/guard-marker/.e2e-sandbox"
    local out status
    out="$(run_sandbox "$WORK/guard-marker" -- snapshot 2>/dev/null)"
    status=$?
    if [ -n "$out" ]; then
        fail "$name" "stdout was not empty: $out"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0"
    else
        pass "$name"
    fi
}

# ---------------------------------------------------------------------------
# The union: a leak at any of the eight targets shows up in a before/after diff
# ---------------------------------------------------------------------------

# Builds a home plus the external directories every tier can name, so all eight
# union targets exist and are distinct.
make_fixture() {
    local r="$1"
    mkdir -p "$r/home/.config/matra" "$r/home/.local/share/matra" "$r/home/.matra"
    mkdir -p "$r/xdgconfig/matra" "$r/xdgdata/matra" "$r/data" "$r/models"
    : > "$r/config.toml"
}

# Runs a snapshot over a fixture built by make_fixture.
fixture_snapshot() {
    local r="$1"
    run_sandbox "$r/home" \
        "MATRA_CONFIG_FILE=$r/config.toml" \
        "MATRA_DATA_DIR=$r/data" \
        "MATRA_MODEL_DIR=$r/models" \
        "XDG_CONFIG_HOME=$r/xdgconfig" \
        "XDG_DATA_HOME=$r/xdgdata" \
        -- snapshot
}

case_union_targets() {
    local i=0
    local r target
    # The eight the script names, in the order it names them. A tier added to
    # src/config.rs and not to the script is caught by the parity test in
    # tests/sandbox_env_parity.rs; this list is about the walk, not the names.
    local targets='config.toml data models xdgconfig/matra xdgdata/matra home/.config/matra home/.local/share/matra home/.matra'
    for target in $targets; do
        i=$((i + 1))
        local name="snapshot: a leak at union target $i ($target) shows in the diff"
        r="$WORK/union-$i"
        make_fixture "$r"
        local before after
        before="$(fixture_snapshot "$r" 2>/dev/null)"
        if [ -d "$r/$target" ]; then
            : > "$r/$target/leaked-model.bin"
        else
            echo 'leaked = true' >> "$r/$target"
        fi
        after="$(fixture_snapshot "$r" 2>/dev/null)"
        if [ "$before" = "$after" ]; then
            fail "$name" "the pair is identical, so the leak is invisible"
        else
            pass "$name"
        fi
    done
    # Eight is the count the union is supposed to have. A target dropped from
    # the script would still pass every case above, because each one only
    # checks its own path.
    local lines
    r="$WORK/union-count"
    make_fixture "$r"
    lines="$(fixture_snapshot "$r" 2>/dev/null | grep -c 'ABSENT$\|/config.toml \|/data \|/models \|/matra \|/\.matra ')"
    if [ "$lines" -lt 8 ]; then
        fail "snapshot: the union covers eight targets" \
            "the fingerprint of an empty fixture named $lines of them"
    else
        pass "snapshot: the union covers eight targets"
    fi
}

# B2. `find` without -H stats a symlinked target and stops. Writing behind the
# link moves the target's mtime and not the link's, so the fingerprint did not
# move. src/config.rs says in its own comment that a config file under a home
# directory is routinely a symlink into a dotfiles repository.
case_symlinked_target() {
    local name="snapshot: a leak behind a symlinked target shows in the diff"
    local r="$WORK/symlink-target"
    make_fixture "$r"
    mkdir -p "$r/elsewhere"
    rmdir "$r/data"
    ln -s "$r/elsewhere" "$r/data"
    local before after
    before="$(fixture_snapshot "$r" 2>/dev/null)"
    : > "$r/elsewhere/leaked-model.bin"
    after="$(fixture_snapshot "$r" 2>/dev/null)"
    if [ "$before" = "$after" ]; then
        fail "$name" "the pair is identical, so the leak is invisible"
    else
        pass "$name"
    fi
}

case_symlink_inside_a_target() {
    local name="snapshot: a leak through a symlink inside a target shows in the diff"
    local r="$WORK/symlink-nested"
    make_fixture "$r"
    # The shape src/config.rs names, at the depth it actually occurs: the
    # config file is one level inside the target, not the target itself.
    mkdir -p "$r/dotfiles"
    printf 'a = 1\n' > "$r/dotfiles/matra.toml"
    if ! ln -s "$r/dotfiles/matra.toml" "$r/xdgconfig/matra/config.toml"; then
        fail "$name" "could not create the nested symlink fixture"
        return
    fi
    local before after
    before="$(fixture_snapshot "$r" 2>/dev/null)"
    printf 'leaked = true\n' >> "$r/dotfiles/matra.toml"
    after="$(fixture_snapshot "$r" 2>/dev/null)"
    if [ "$before" = "$after" ]; then
        fail "$name" "the pair is identical, so a write through a nested link is invisible"
    else
        pass "$name"
    fi
}

case_exports_survive_eval() {
    local name="new: the exports survive eval, and MATRA_* is swept"
    # Two things nothing else covers. A path with a space proves the %q
    # quoting: without it, eval sets HOME to the first word and runs the
    # rest as a command. And a MATRA_* variable in the environment proves
    # the unset loop, which is the sandbox's whole answer to the tier the
    # resolvers consult first.
    local r="$WORK/eval home"
    mkdir -p "$r"
    local got_home got_matra
    if ! out="$(MATRA_MODEL_DIR=/somewhere/real bash "$SCRIPT" new "$r" 2>/dev/null)"; then
        fail "$name" "new failed on a directory whose name contains a space"
        return
    fi
    got_home="$(MATRA_MODEL_DIR=/somewhere/real bash -c "eval \"\$1\"; printf '%s' \"\$HOME\"" _ "$out")"
    got_matra="$(MATRA_MODEL_DIR=/somewhere/real bash -c "eval \"\$1\"; printf '%s' \"\${MATRA_MODEL_DIR-unset}\"" _ "$out")"
    if [ "$got_home" != "$r/home" ]; then
        fail "$name" "HOME came back as '$got_home', wanted '$r/home'"
    elif [ "$got_matra" != "unset" ]; then
        fail "$name" "MATRA_MODEL_DIR survived the sweep as '$got_matra'"
    else
        pass "$name"
    fi
}

case_dangling_symlink_target() {
    local name="snapshot: a dangling symlink at a target is not reported ABSENT"
    local r="$WORK/dangling"
    make_fixture "$r"
    rmdir "$r/home/.matra"
    ln -s "$r/nowhere" "$r/home/.matra"
    local out
    out="$(fixture_snapshot "$r" 2>/dev/null)"
    if printf '%s\n' "$out" | grep -q "^$r/home/.matra ABSENT\$"; then
        fail "$name" "a link that exists was called ABSENT"
    else
        pass "$name"
    fi
}

# B1. The loop used to exit at the first unreadable target, having already
# written the earlier targets to stdout. Both snapshots of a pair fail in the
# same place, so the two files are byte-identical while every target after the
# failing one went unexamined: one unreadable directory under the config path
# hid leaks in the data and legacy paths entirely.
case_unreadable_target() {
    local name="snapshot: an unreadable target cannot yield a clean-looking pair"
    if is_root; then
        skip "$name" "uid 0 reads through a mode that denies read"
        return
    fi
    local r="$WORK/unreadable"
    make_fixture "$r"
    mkdir -p "$r/xdgconfig/matra/locked"
    chmod a-rx "$r/xdgconfig/matra/locked"
    local before after status
    before="$(fixture_snapshot "$r" 2>/dev/null)"
    # The leak goes into a target the loop reaches AFTER the unreadable one.
    : > "$r/home/.matra/leaked-model.bin"
    after="$(fixture_snapshot "$r" 2>/dev/null)"
    status=$?
    chmod u+rx "$r/xdgconfig/matra/locked"
    if [ "$before" = "$after" ]; then
        fail "$name" "the pair is identical: the targets after the unreadable" \
            "one were never examined"
    elif ! printf '%s\n' "$after" | grep -q 'UNREADABLE$'; then
        fail "$name" "the failure is not in the artifact that gets diffed"
    elif [ "$status" -eq 0 ]; then
        fail "$name" "exited 0 with an unreadable target"
    else
        pass "$name"
    fi
}

# ---------------------------------------------------------------------------

printf 'testing %s\n\n' "$SCRIPT"

case_unwritable_home
case_readonly_parent
case_marker_is_a_directory
case_tmpdir_nowhere
case_cdpath
case_symlinked_sandbox_home
case_sandbox_home_is_real_home
case_home_unset
case_new_happy_path
case_guard_environment_route
case_guard_marker_route
case_union_targets
case_symlinked_target
case_symlink_inside_a_target
case_exports_survive_eval
case_dangling_symlink_target
case_unreadable_target

printf '\n%d passed, %d failed, %d skipped\n' "$passed" "$failed" "$skipped"
if [ "$skipped" -gt 0 ]; then
    echo "skipped cases need a directory mode that uid 0 ignores; run as a" \
         "non-root user to cover them."
fi
[ "$failed" -eq 0 ]
