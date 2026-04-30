#!/usr/bin/env bash
# Verifies vaani's hex-architecture boundary rules from CLAUDE.md.
# Run in CI on every PR. Run locally before commit.
#
# Rules enforced here:
#   3. No port module imports another port module.
#   4. nlp/udpipe.rs is the ONLY file that imports udpipe_rs.
#   8. tracing is forbidden in domain.rs and port modules (Burner amendment, 2026-04-28).
#
# Other rules (1, 2, 5, 6, 7) are enforced by the type system + cargo check
# and do not need a script.

set -euo pipefail

fail=0

# Rule 4: only nlp/udpipe.rs imports udpipe_rs.
hits=$(rg -l 'use udpipe_rs|udpipe_rs::' src/ --glob '!src/nlp/udpipe.rs' 2>/dev/null || true)
if [ -n "$hits" ]; then
    echo "FAIL (rule 4): udpipe_rs imported outside src/nlp/udpipe.rs"
    echo "$hits" | sed 's/^/  /'
    fail=1
fi

# Rule 8: tracing forbidden in domain.rs and port modules.
hits=$(rg -l '(^|\s)use tracing|tracing::' \
    src/domain.rs \
    src/source/mod.rs \
    src/decompose/mod.rs \
    src/nlp/mod.rs \
    2>/dev/null || true)
if [ -n "$hits" ]; then
    echo "FAIL (rule 8): tracing imported in domain.rs or a port module"
    echo "$hits" | sed 's/^/  /'
    fail=1
fi

# Rule 3: port modules do not import each other.
hits=$(rg -l 'use crate::source|use crate::decompose|use crate::nlp' \
    src/source/mod.rs \
    src/decompose/mod.rs \
    src/nlp/mod.rs \
    2>/dev/null || true)
if [ -n "$hits" ]; then
    echo "FAIL (rule 3): cross-port import detected"
    echo "$hits" | sed 's/^/  /'
    fail=1
fi

if [ "$fail" -eq 0 ]; then
    echo "boundary checks pass (rules 3, 4, 8)"
fi
exit "$fail"
