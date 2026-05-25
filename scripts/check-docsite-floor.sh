#!/usr/bin/env bash
# Floor gates for the docsite. Runs in CI; can be invoked locally via `just docs-floor`.
#
# Four gates protect against the cheap-to-introduce, expensive-to-find regressions:
#
#   1. Link integrity     — lychee verifies all in-book Markdown links resolve.
#   2. Orphan detect      — every page under book/src/ is referenced in SUMMARY.md
#                            (with a small allowlist for include-only fragments).
#   3. Type-name parity   — every backtick-inline PascalCase identifier in book/src/
#                            either exists as an identifier in src/, or is on the
#                            external-types allowlist below. Catches rename drift.
#   4. mdbook clean build — `mdbook build` runs without warnings or errors.
#
# Local invocation: lychee is optional locally (skip-with-warning); CI installs it.
# mdbook is required (this script fails gate 4 if missing).
#
# Tunables:
#   LYCHEE_REQUIRED=1   — turn the "lychee missing" skip into a hard failure.
#                          CI sets this after installing lychee.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

fail=0

# ---------------------------------------------------------------------------
# Gate 1: lychee link check
# ---------------------------------------------------------------------------
echo "=== Gate 1: link integrity (lychee) ==="
if command -v lychee >/dev/null 2>&1; then
    if lychee --no-progress --offline \
            --exclude-path 'book/book' \
            'book/src/**/*.md'; then
        echo "PASS (gate 1): all in-book links resolve"
    else
        echo "FAIL (gate 1): broken links detected"
        fail=$((fail + 1))
    fi
else
    if [ "${LYCHEE_REQUIRED:-0}" = "1" ]; then
        echo "FAIL (gate 1): lychee not installed and LYCHEE_REQUIRED=1"
        echo "        install: cargo install lychee"
        fail=$((fail + 1))
    else
        echo "SKIP (gate 1): lychee not installed; install with \`cargo install lychee\`"
        echo "        CI runs this gate after installing lychee."
    fi
fi
echo ""

# ---------------------------------------------------------------------------
# Gate 2: orphan detect
# ---------------------------------------------------------------------------
echo "=== Gate 2: orphan detect ==="
orphans=()
while IFS= read -r page; do
    rel="${page#book/src/}"
    # SUMMARY.md references look like "(./path/to/page.md)" or "(path/to/page.md)".
    if ! grep -F -q -e "($rel)" -e "(./$rel)" book/src/SUMMARY.md; then
        orphans+=("$page")
    fi
done < <(find book/src -type f -name '*.md' \
    ! -name SUMMARY.md \
    | sort)
if [ ${#orphans[@]} -eq 0 ]; then
    echo "PASS (gate 2): every book/src/ page is in SUMMARY.md"
else
    echo "FAIL (gate 2): pages not referenced in SUMMARY.md:"
    printf '  %s\n' "${orphans[@]}"
    fail=$((fail + 1))
fi
echo ""

# ---------------------------------------------------------------------------
# Gate 3: type-name parity (docs ↔ src/)
# ---------------------------------------------------------------------------
echo "=== Gate 3: type-name parity ==="

# External / stdlib / language types referenced in docs that do NOT need to
# exist in src/. Keep this list short; if it grows past ~30, the gate is
# probably miscalibrated.
external_allowlist=$(cat <<'EOF'
Arc
Bound
Box
Clone
Copy
Debug
Default
Display
Eq
Exception
From
Hash
Ignore
Into
IntoIterator
Iterator
Mutex
None
Option
Ord
PartialEq
PartialOrd
Path
PathBuf
ProcessPoolExecutor
Py
PyResult
PyRuntimeError
Read
Result
RuntimeError
Seek
Send
Some
String
Sync
ThreadPoolExecutor
TryFrom
TryInto
TypeError
ValueError
Vec
Write
EOF
)

# Universal Dependencies (UD) POS tag set. Cross-language NLP standard;
# external to vaani. Referenced in the concepts/pos-lemmas.md page.
# Spec: https://universaldependencies.org/u/pos/
ud_pos_allowlist=$(cat <<'EOF'
ADJ
ADP
ADV
AUX
CCONJ
DET
INTJ
NOUN
NUM
PART
PRON
PROPN
PUNCT
SCONJ
SYM
VERB
EOF
)

# Identifiers referenced in docs as planned adapters that do NOT yet exist in
# src/. When one ships, remove it from this list and the gate catches any
# subsequent rename drift. Keep this list short and load-bearing.
planned_allowlist=$(cat <<'EOF'
DocxDecomposer
PdfDecomposer
EOF
)

unknown=()
while IFS= read -r name; do
    [ -z "$name" ] && continue
    # On any allowlist? skip.
    if printf '%s\n' "$external_allowlist" | grep -Fxq -- "$name"; then
        continue
    fi
    if printf '%s\n' "$ud_pos_allowlist" | grep -Fxq -- "$name"; then
        continue
    fi
    if printf '%s\n' "$planned_allowlist" | grep -Fxq -- "$name"; then
        continue
    fi
    # Word-boundary grep across src/. If the name appears anywhere as a word,
    # we accept it (struct, enum, variant, fn, const, pyclass, doc-comment).
    if rg -q --word-regexp --fixed-strings -e "$name" src/; then
        continue
    fi
    unknown+=("$name")
done < <(rg -oIN --pcre2 -e '`([A-Z][a-zA-Z0-9_]+)`' --replace '$1' book/src/ | sort -u)

if [ ${#unknown[@]} -eq 0 ]; then
    echo "PASS (gate 3): every backtick-inline type name resolves in src/ or allowlist"
else
    echo "FAIL (gate 3): backtick-inline identifiers in book/src/ not found in src/:"
    printf '  %s\n' "${unknown[@]}"
    echo ""
    echo "        Fix one of: rename the doc reference, add the type to src/,"
    echo "        or extend the external_allowlist in scripts/check-docsite-floor.sh."
    fail=$((fail + 1))
fi
echo ""

# ---------------------------------------------------------------------------
# Gate 4: mdbook clean build
# ---------------------------------------------------------------------------
echo "=== Gate 4: mdbook clean build ==="
if ! command -v mdbook >/dev/null 2>&1; then
    echo "FAIL (gate 4): mdbook not installed"
    echo "        install: cargo install mdbook mdbook-mermaid"
    fail=$((fail + 1))
else
    build_log=$(mktemp)
    trap 'rm -f "$build_log"' EXIT
    if (cd book && mdbook build) >"$build_log" 2>&1; then
        # Warnings still cause a fail. mdbook 0.5.3 has no --warning-policy,
        # so we grep for the emitted patterns directly.
        if grep -E -q '\[WARN\]|^warning:|Could not find' "$build_log"; then
            echo "FAIL (gate 4): mdbook build produced warnings"
            grep -E '\[WARN\]|^warning:|Could not find' "$build_log" | sed 's/^/  /'
            fail=$((fail + 1))
        else
            echo "PASS (gate 4): mdbook build clean"
        fi
    else
        echo "FAIL (gate 4): mdbook build failed"
        sed 's/^/  /' "$build_log"
        fail=$((fail + 1))
    fi
fi
echo ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
if [ "$fail" -eq 0 ]; then
    echo "docsite floor: all gates pass"
    exit 0
else
    echo "docsite floor: $fail gate(s) failed"
    exit 1
fi
