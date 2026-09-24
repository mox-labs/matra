#!/usr/bin/env bash
# Verifies matra's hex-architecture boundary rules from CLAUDE.md.
# Runs from 'just check' and the opt-in pre-commit hook. NOT wired into CI.
#
# Rules enforced here:
#   3. No port module imports another port module.
#   4. nlp/udpipe.rs is the ONLY file that imports udpipe_rs.
#   8. tracing is forbidden in domain.rs and port modules (Burner amendment, 2026-04-28).
#
# Rule 6 is gated by cargo check --no-default-features in ci.yml. Rules 1, 2, 5, 7
# have no mechanical check (Rust offers no intra-crate directional-import control);
# review is the gate. See book/src/reference/boundary-rules.md for the full table.

set -euo pipefail

cd "$(dirname "$0")/.."

# rg is required. Every rule below is an rg search, and a search that never
# ran returns the same empty output as a search that found nothing, so a
# missing rg used to print a pass having examined nothing.
if ! command -v rg >/dev/null 2>&1; then
    echo "FAIL: check-boundaries needs ripgrep (rg), which is not on PATH" >&2
    echo "      install: brew install ripgrep, apt-get install ripgrep, or cargo install ripgrep" >&2
    exit 2
fi

# rg exits 0 on a match, 1 on no match, and 2 on an error such as a path that
# does not exist. Only 0 and 1 are results. A renamed port file used to land
# on 2, which the old `2>/dev/null || true` read as a clean pass.
scan() {
    local out rc=0
    out=$(rg "$@") || rc=$?
    if [ "$rc" -gt 1 ]; then
        echo "FAIL: rg exited $rc for: rg $*" >&2
        exit 2
    fi
    printf '%s' "$out"
}

fail=0

# Rule 4: only nlp/udpipe.rs imports udpipe_rs.
hits=$(scan -l 'use udpipe_rs|udpipe_rs::' src/ --glob '!src/nlp/udpipe.rs')
if [ -n "$hits" ]; then
    echo "FAIL (rule 4): udpipe_rs imported outside src/nlp/udpipe.rs"
    echo "$hits" | sed 's/^/  /'
    fail=1
fi

# Rule 4 analog: only embed/model2vec.rs imports safetensors and tokenizers.
hits=$(scan -l 'use safetensors|safetensors::|use tokenizers|tokenizers::' src/ --glob '!src/embed/model2vec.rs')
if [ -n "$hits" ]; then
    echo "FAIL (rule 4 analog): safetensors/tokenizers imported outside src/embed/model2vec.rs"
    echo "$hits" | sed 's/^/  /'
    fail=1
fi

# Rule 8: tracing forbidden in domain.rs and port modules.
hits=$(scan -l '(^|\s)use tracing|tracing::' \
    src/domain.rs \
    src/source/mod.rs \
    src/decompose/mod.rs \
    src/nlp/mod.rs \
    src/embed/mod.rs)
if [ -n "$hits" ]; then
    echo "FAIL (rule 8): tracing imported in domain.rs or a port module"
    echo "$hits" | sed 's/^/  /'
    fail=1
fi

# Rule 3: port modules do not import each other.
hits=$(scan -l 'use crate::source|use crate::decompose|use crate::nlp|use crate::embed' \
    src/source/mod.rs \
    src/decompose/mod.rs \
    src/nlp/mod.rs \
    src/embed/mod.rs)
if [ -n "$hits" ]; then
    echo "FAIL (rule 3): cross-port import detected"
    echo "$hits" | sed 's/^/  /'
    fail=1
fi

if [ "$fail" -eq 0 ]; then
    echo "boundary checks pass (rules 3, 4, 8)"
fi
exit "$fail"
