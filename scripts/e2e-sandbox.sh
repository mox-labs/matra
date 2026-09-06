#!/usr/bin/env bash
# A scrubbed environment for an end-to-end pass, so a cold start is really cold
# and the real home is never written to. See .claude/skills/e2e-validation.
#
#   eval "$(bash scripts/e2e-sandbox.sh new)"   # enter a fresh sandbox
#   bash scripts/e2e-sandbox.sh snapshot        # fingerprint the real locations
#
# Take a snapshot before and after a pass and diff them. An identical pair is
# the evidence that the pass stayed inside its sandbox, and the report should
# say so.
#
# `snapshot` refuses to run inside a sandbox, because in there $HOME points at
# the sandbox and it would happily fingerprint that instead, print three clean
# ABSENT lines, and hand an operator a false all-clear. That is the exact
# "polluted environment produces a clean report" failure the skill warns about,
# so the guard is the point of this script rather than a nicety.
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
else
    STAT=(stat -c '%n %s %Y')
fi

case "${1:-}" in
new)
    dir="${2:-$(mktemp -d "${TMPDIR:-/tmp}/matra-e2e.XXXXXX")}"
    mkdir -p "$dir"
    # Absolute, so a later inspection or cleanup looks where the model
    # actually landed rather than wherever the tester happened to be.
    dir="$(cd "$dir" && pwd)"
    mkdir -p "$dir/home/.config" "$dir/home/.local/share"
    # HOME moves too. The legacy model cache resolves from $HOME, so setting
    # the XDG variables alone leaves the pass warm and proves nothing.
    printf 'export HOME=%q\n' "$dir/home"
    printf 'export XDG_CONFIG_HOME=%q\n' "$dir/home/.config"
    printf 'export XDG_DATA_HOME=%q\n' "$dir/home/.local/share"
    printf 'export XDG_CACHE_HOME=%q\n' "$dir/home/.cache"
    # Every MATRA_* the environment carries, not a hardcoded three, so a
    # variable added later cannot leak into a pass unnoticed.
    while IFS='=' read -r name _; do
        case "$name" in MATRA_*) printf 'unset %q\n' "$name" ;; esac
    done < <(env)
    # A marker inside the sandbox home as well as the variable. The variable
    # can fail to be inherited; $HOME is the thing that moved, so a marker
    # living in it travels with the sandbox no matter how it was entered.
    : > "$dir/home/.e2e-sandbox"
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
    # matra resolves XDG_CONFIG_HOME and XDG_DATA_HOME ahead of $HOME, so
    # deriving these from $HOME alone missed a real config file whenever the
    # operator's XDG variables point outside home. That is the same false
    # all-clear the guard above exists to stop, reachable without a sandbox.
    for p in "${XDG_CONFIG_HOME:-$HOME/.config}/matra" \
             "${XDG_DATA_HOME:-$HOME/.local/share}/matra" \
             "$HOME/.matra"; do
        if [ -e "$p" ]; then
            # Errors are not suppressed. An unreadable subdirectory used to
            # abort the loop with the reason sent to /dev/null, leaving a
            # truncated fingerprint that diffs clean against another one.
            if ! find "$p" -exec "${STAT[@]}" {} + | sort; then
                echo "FAIL: could not fingerprint $p" >&2
                exit 4
            fi
        else
            printf '%s ABSENT\n' "$p"
        fi
    done
    ;;
*)
    usage
    ;;
esac
