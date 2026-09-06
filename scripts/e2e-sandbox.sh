#!/usr/bin/env bash
# A scrubbed environment for an end-to-end pass, so a cold start is really cold
# and the real home is never written to. See .claude/skills/e2e-validation.
#
#   eval "$(bash scripts/e2e-sandbox.sh new)"   # enter a fresh sandbox
#   bash scripts/e2e-sandbox.sh snapshot        # fingerprint the real locations
#
# Take a snapshot before and after a pass and diff them. An identical pair is
# the evidence that the pass stayed inside its sandbox, and the report should
# say so. This script never creates the real locations, only reads them.
set -euo pipefail

REAL_CONFIG="${HOME}/.config/matra"
REAL_DATA="${HOME}/.local/share/matra"
REAL_LEGACY="${HOME}/.matra"

# BSD stat and GNU stat disagree on the format flag, and the containers this
# runs in are Linux while the maintainer's machine is not.
if stat -f '%N %z' . >/dev/null 2>&1; then
    STAT=(stat -f '%N %z')
else
    STAT=(stat -c '%n %s')
fi

usage() {
    cat >&2 <<'USAGE'
usage: e2e-sandbox.sh new [dir]   print the exports for a fresh sandbox
       e2e-sandbox.sh snapshot    print a fingerprint of the real locations
USAGE
    exit 2
}

case "${1:-}" in
new)
    dir="${2:-$(mktemp -d "${TMPDIR:-/tmp}/matra-e2e.XXXXXX")}"
    mkdir -p "$dir/home/.config" "$dir/home/.local/share"
    # HOME moves too, because the legacy model cache and anything else that
    # resolves from home must miss as well. XDG alone is not a cold start.
    printf 'export HOME=%q\n' "$dir/home"
    printf 'export XDG_CONFIG_HOME=%q\n' "$dir/home/.config"
    printf 'export XDG_DATA_HOME=%q\n' "$dir/home/.local/share"
    for v in MATRA_CONFIG_FILE MATRA_DATA_DIR MATRA_MODEL_DIR; do
        printf 'unset %s\n' "$v"
    done
    printf 'export MATRA_E2E_SANDBOX=%q\n' "$dir"
    printf '# sandbox at %s\n' "$dir" >&2
    ;;
snapshot)
    for p in "$REAL_CONFIG" "$REAL_DATA" "$REAL_LEGACY"; do
        if [ -e "$p" ]; then
            # Names and sizes, sorted. Not mtimes: reading a directory on some
            # filesystems updates atime, and a fingerprint that changes when
            # you look at it cannot prove anything.
            find "$p" -exec "${STAT[@]}" {} + 2>/dev/null | sort
        else
            printf '%s ABSENT\n' "$p"
        fi
    done
    ;;
*)
    usage
    ;;
esac
