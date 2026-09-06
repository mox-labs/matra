#!/usr/bin/env bash
# A scrubbed environment for an end-to-end pass, so a cold start is really cold
# and the real home is never written to. See .claude/skills/e2e-validation.
#
#   eval "$(bash scripts/e2e-sandbox.sh new)"   # enter a fresh sandbox
#   bash scripts/e2e-sandbox.sh snapshot        # fingerprint the real locations
#
# `new` prints exports and moves nothing by itself. Only the `eval` around it
# moves the environment, which is why the exports and the marker are ordered
# the way they are below.
#
# Take a snapshot before and after a pass and diff them. An identical pair is
# the evidence that the pass stayed inside its sandbox, and the report should
# say so.
#
# `snapshot` refuses to run inside a sandbox, because in there $HOME points at
# the sandbox and it would happily fingerprint that instead, print clean
# ABSENT lines, and hand an operator a false all-clear. That is the exact
# "polluted environment produces a clean report" failure the skill warns about,
# so the guard is the point of this script rather than a nicety.
#
# The one property this script has to hold is that it cannot report a clean
# result for a tree it did not examine. `scripts/test-e2e-sandbox.sh` is where
# that property is asserted, and it runs from `just check`. Three review rounds
# each closed a hole here and left another, which is what an untested script
# buys.
#
# What "cold" covers: matra's own resolution, which is the XDG variables plus
# the legacy cache under $HOME. Moving HOME also relocates the default cargo,
# rustup, uv and pip caches. It does NOT override those when the operator's
# environment already points them somewhere absolute, so a wheel can still be
# served from a warm cache. Say which you had if the timing matters.
set -euo pipefail

# Not MATRA_-prefixed on purpose: the skill tells a tester to unset every
# MATRA_* variable, and a marker that the documented procedure destroys is a
# guard that fails open.
: "${E2E_SANDBOX_ROOT:=}"

usage() {
    cat >&2 <<'USAGE'
usage: e2e-sandbox.sh new [dir]   print the exports for a fresh sandbox
       e2e-sandbox.sh snapshot    print a fingerprint of the real locations
USAGE
    exit 2
}

# BSD stat and GNU stat disagree on the format flag, and this runs both on
# macOS and inside Linux containers. Name, size and mtime: mtime is included
# because a directory's mtime changes when an entry is added or removed, which
# is precisely the signal being looked for, and reads do not touch it.
if stat -f '%N %z %m' . >/dev/null 2>&1; then
    STAT=(stat -f '%N %z %m')
    # -L for the referent pass: size and mtime of what a link points at, with
    # the name dropped because the link's own name is printed beside it.
    STAT_L=(stat -Lf '%z %m')
else
    STAT=(stat -c '%n %s %Y')
    STAT_L=(stat -Lc '%s %Y')
fi

if [ -z "${HOME:-}" ]; then
    echo "FAIL: HOME is unset, and every location this script reasons about" \
         "is resolved relative to it. Set HOME and try again." >&2
    exit 6
fi

case "${1:-}" in
new)
    dir="${2:-$(mktemp -d "${TMPDIR:-/tmp}/matra-e2e.XXXXXX")}"
    mkdir -p -- "$dir"
    # Absolute and physical, so a later inspection or cleanup looks where the
    # sandbox actually landed rather than wherever the tester happened to be.
    #
    # CDPATH is cleared for this cd and nothing else. With CDPATH set, `cd
    # relative` resolves against it AND echoes the resolved path, so the
    # command substitution captured two lines, the sandbox was built inside
    # the CDPATH target rather than the directory the operator named, and the
    # exported HOME was a two-line string. That is `new` writing into the real
    # home, silently, and exiting 0. The `--` covers a directory argument that
    # begins with a dash.
    dir="$(CDPATH='' cd -- "$dir" && pwd -P)"

    sandbox_home="$dir/home"
    # A symlinked sandbox home puts the marker in whatever it points at.
    # `mkdir -p` is a no-op through a link, so the tree looks built and the
    # marker write succeeds into the link's target. When that target is the
    # real home, the real home is left holding a marker that makes every later
    # snapshot refuse until someone removes it by hand. Refuse both shapes.
    #
    # Only these two shapes. A sandbox that merely sits somewhere under the
    # real home is legitimate: $HOME then names a distinct tree, and none of
    # the snapshot targets collide with the real ones.
    if [ -L "$sandbox_home" ]; then
        echo "FAIL: $sandbox_home is a symlink. mkdir is a no-op through a" \
             "link, so the marker would be written into its target rather" \
             "than into the sandbox. Refusing to print exports." >&2
        exit 7
    fi
    if ! real_home="$(CDPATH='' cd -- "$HOME" 2>/dev/null && pwd -P)"; then
        real_home="$HOME"
    fi
    if [ "$sandbox_home" = "$real_home" ]; then
        echo "FAIL: the sandbox home resolves to the real home ($real_home)." \
             "A sandbox that is the thing it is meant to protect is not a" \
             "sandbox. Refusing to print exports." >&2
        exit 7
    fi

    mkdir -p "$sandbox_home/.config" "$sandbox_home/.local/share"
    # The marker is written BEFORE anything reaches stdout, and this ordering is
    # load-bearing. The documented entry is `eval "$(... new)"`, whose exit
    # status is the status of the exports it evaluates, not of this script. So
    # if the marker write failed after the exports had already been printed,
    # HOME moved into a sandbox that snapshot could not recognise, and the next
    # snapshot returned a clean all-clear for the wrong tree. Failing here means
    # stdout is empty, eval is a no-op, and HOME never moves. Every check above
    # is ordered ahead of stdout for the same reason.
    if ! : > "$sandbox_home/.e2e-sandbox"; then
        echo "FAIL: cannot write the sandbox marker at" \
             "$sandbox_home/.e2e-sandbox. Refusing to print exports, because" \
             "a sandbox snapshot cannot recognise is worse than no sandbox." >&2
        exit 5
    fi
    # HOME moves too. The legacy model cache resolves from $HOME, so setting
    # the XDG variables alone leaves the pass warm and proves nothing.
    printf 'export HOME=%q\n' "$sandbox_home"
    printf 'export XDG_CONFIG_HOME=%q\n' "$sandbox_home/.config"
    printf 'export XDG_DATA_HOME=%q\n' "$sandbox_home/.local/share"
    printf 'export XDG_CACHE_HOME=%q\n' "$sandbox_home/.cache"
    # CARGO_HOME as well. Moving HOME relocates cargo's default, but rustup
    # and asdf setups routinely export an absolute one, and then a pass that
    # builds or installs runs against the operator's real cargo home while
    # the snapshot diffs clean, because ~/.cargo is not a matra location and
    # never will be. The launcher the macOS pilot used moved it; this script
    # replaced that launcher, so it moves it too.
    printf 'export CARGO_HOME=%q\n' "$sandbox_home/.cargo"
    # Every MATRA_* the environment carries, not a hardcoded three, so a
    # variable added later cannot leak into a pass unnoticed.
    while IFS='=' read -r name _; do
        case "$name" in MATRA_*) printf 'unset %q\n' "$name" ;; esac
    done < <(env)
    printf 'export E2E_SANDBOX_ROOT=%q\n' "$dir"
    printf '# sandbox at %s\n' "$dir" >&2
    ;;
snapshot)
    if [ -n "$E2E_SANDBOX_ROOT" ] || [ -e "${HOME}/.e2e-sandbox" ]; then
        echo "refusing: snapshot must run outside the sandbox, but" \
             "this is a sandbox (E2E_SANDBOX_ROOT=${E2E_SANDBOX_ROOT:-unset}," \
             "marker $( [ -e "${HOME}/.e2e-sandbox" ] && echo present || echo absent ))." \
             "In here \$HOME is the sandbox, so this would fingerprint" \
             "the wrong tree and report a false all-clear." >&2
        exit 3
    fi
    # matra resolves each location through three tiers, MATRA_* over XDG over
    # HOME (see the resolvers in src/config.rs). This has now been got wrong
    # twice by following that precedence and stopping one tier short, each time
    # producing exactly the false all-clear the guard above exists to prevent.
    #
    # So do not follow the precedence. Fingerprint the UNION of every location
    # any tier could name. Precedence answers "which one would matra use", and
    # the question here is the different and strictly wider one: "is there
    # anywhere matra might have written". A union cannot be wrong by missing a
    # tier, only by naming a directory that was never going to be touched.
    #
    # The union is not free in both directions. A spurious ABSENT line costs
    # nothing, but a MATRA_DATA_DIR pointing at a large tree makes the walk
    # traverse all of it, twice per pass. That is the price of not being able
    # to miss a tier and it is worth paying; when a snapshot is slow, a
    # variable pointing somewhere enormous is the first thing to look at.
    #
    # The set of variables named between the two sentinels below is asserted
    # against the resolvers in src/config.rs by tests/sandbox_env_parity.rs, so
    # a tier added there fails a gate rather than quietly narrowing the union
    # here. The sentinels are what that test reads; do not remove them.
    # parity: union targets begin
    targets=()
    [ -n "${MATRA_CONFIG_FILE:-}" ] && targets+=("$MATRA_CONFIG_FILE")
    [ -n "${MATRA_DATA_DIR:-}" ] && targets+=("$MATRA_DATA_DIR")
    [ -n "${MATRA_MODEL_DIR:-}" ] && targets+=("$MATRA_MODEL_DIR")
    [ -n "${XDG_CONFIG_HOME:-}" ] && targets+=("$XDG_CONFIG_HOME/matra")
    [ -n "${XDG_DATA_HOME:-}" ] && targets+=("$XDG_DATA_HOME/matra")
    targets+=("$HOME/.config/matra" "$HOME/.local/share/matra" "$HOME/.matra")
    # parity: union targets end

    # A failure has to land in the artifact that gets diffed, not only on
    # stderr. The documented procedure is "take one before, take one after,
    # diff them", and it never says to check the exit status, exactly as the
    # marker ordering above assumes for `new`. A loop that aborted at the first
    # unreadable target had already written the earlier targets to stdout, and
    # since both snapshots of a pair fail in the same place the two files were
    # byte-identical while every target after the failing one went unexamined.
    # One root-owned unreadable directory under the config path hid leaks in
    # the data and legacy paths entirely.
    #
    # So: record the failure, put an UNREADABLE line in the artifact, and carry
    # on to the remaining targets. The status is still non-zero at the end, for
    # a caller that does check it.
    failed=0
    for p in "${targets[@]}"; do
        # -L as well as -e: a dangling symlink at a target is something that
        # was created, and -e alone calls it ABSENT in both snapshots of a
        # pair, which is a false all-clear of the same family.
        if [ -e "$p" ] || [ -L "$p" ]; then
            # Two passes, and the second is the one that took four rounds to
            # get right. The walk uses -H, which follows the command-line
            # argument only, so a symlinked target is descended without any
            # cycle risk from links met further down. But stat then records
            # every link inside the tree as the link: its own name, its own
            # size (the length of the target path) and its own mtime. Writing
            # through such a link moves nothing this listing carries.
            #
            # That is not a special case, it is every symlink in the tree, and
            # matra's own layout puts the two shapes src/config.rs names one
            # level inside a target rather than at it: the config file is
            # `<config target>/config.toml` and the models live in
            # `<data target>/models`. Fixing the target alone fixed a depth,
            # not the defect. So the second pass records what each link points
            # at, at any depth, and a dangling one is recorded as dangling
            # rather than dropped.
            #
            # Both listings are buffered before printing: `find | sort` emits
            # partial output before failing, and a partial listing followed by
            # an UNREADABLE line reads as though the part before the failure
            # had been examined.
            listing=""
            referents=""
            if listing="$(find -H "$p" -exec "${STAT[@]}" {} + | sort)" &&
               referents="$(find -H "$p" -type l -print | sort | while IFS= read -r link; do
                   if target="$("${STAT_L[@]}" "$link" 2>/dev/null)"; then
                       printf '%s -> %s\n' "$link" "$target"
                   else
                       printf '%s -> DANGLING\n' "$link"
                   fi
               done)"; then
                [ -z "$listing" ] || printf '%s\n' "$listing"
                [ -z "$referents" ] || printf '%s\n' "$referents"
            else
                printf '%s UNREADABLE\n' "$p"
                failed=1
            fi
        else
            printf '%s ABSENT\n' "$p"
        fi
    done
    if [ "$failed" -ne 0 ]; then
        echo "FAIL: at least one target could not be fingerprinted. The" \
             "UNREADABLE lines in the snapshot say which." >&2
        exit 4
    fi
    ;;
*)
    usage
    ;;
esac
