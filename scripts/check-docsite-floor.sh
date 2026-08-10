#!/usr/bin/env bash
# Floor gates for the docsite. Runs in CI; can be invoked locally via `just docs-floor`.
#
# Four gates protect against the cheap-to-introduce, expensive-to-find regressions:
#
#   1. Link integrity     — lychee verifies all in-book Markdown links resolve.
#   2. Orphan detect      — every page under book/src/ is referenced in SUMMARY.md
#                            (with a small allowlist for include-only fragments).
#   3. Type-name parity   — every backtick-inline PascalCase identifier in book/src/
#                          EXCEPT under book/src/plans/. A plan describes types
#                          that do not exist yet; that is what makes it a plan.
#                          Gate 3 keeps reference pages honest about what ships,
#                          and applying it to plans would invert their purpose.
#                            either exists as an identifier in src/, or is on the
#                            external-types allowlist below. Catches rename drift.
#   4. mdbook clean build — `mdbook build` runs without warnings or errors.
#   5. No em dashes       — project prose convention, exempting quoted material.
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
BaseException
Edit
Bound
Box
Clone
Copy
Debug
Default
Display
Eq
Exception
FileNotFoundError
From
Hash
Ignore
Into
IntoIterator
Iterator
Mutex
None
OSError
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
Stream
String
Sync
ThreadPoolExecutor
TryFrom
TryInto
TypeError
TypedDict
ValueError
Vec
Write
EOF
)

# Universal Dependencies (UD) POS tag set + Penn Treebank tag set.
# Cross-language NLP standards external to matra. Referenced in
# concepts/pos-lemmas.md (UD as the `pos` field; Penn as the
# language-specific `xpos` field that UDPipe also emits).
# UD spec: https://universaldependencies.org/u/pos/
# Penn spec: https://www.ling.upenn.edu/courses/Fall_2003/ling001/penn_treebank_pos.html
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
CC
CD
DT
EX
FW
IN
JJ
JJR
JJS
LS
MD
NN
NNS
NNP
NNPS
PDT
POS
PRP
RB
RBR
RBS
RP
TO
UH
VB
VBD
VBG
VBN
VBP
VBZ
WDT
WP
WRB
Mood
Voice
Tense
Number
Person
Case
Aspect
Gender
Animacy
Degree
VerbForm
Polarity
Definite
PronType
NumType
Reflex
Foreign
Abbr
Typo
EOF
)

# Identifiers referenced in docs as planned adapters that do NOT yet exist in
# src/. When one ships, remove it from this list and the gate catches any
# subsequent rename drift. Keep this list short and load-bearing.
planned_allowlist=$(cat <<'EOF'
DocxDecomposer
PdfDecomposer
Finding
SourceSpan
Relation
Schema
Modality
SpeechAct
Stylometry
Rule
Predicate
Pattern
ParagraphKind
ParagraphRole
ParagraphMetrics
DocumentMetrics
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
done < <(rg -oIN --pcre2 -e '`([A-Z][a-zA-Z0-9_]+)`' --replace '$1' book/src/ --glob '!**/plans/**' | sort -u)

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
# Gate 5: no em dashes in prose
# ---------------------------------------------------------------------------
# Project convention forbids em dashes in documentation prose. The rule has
# existed since early on and nothing enforced it, so it survived only as long
# as whoever was writing happened to remember. It did not survive contact with
# files moved in from elsewhere.
#
# Lines carrying a double quote are exempt. Plans quote reviewers verbatim, and
# silently editing an attributed quote to satisfy a house style rule would be a
# worse fault than the em dash.
echo "=== Gate 5: no em dashes in prose ==="
offenders=$(grep -rn '\u2014' book/src --include='*.md' | grep -v '"' || true)
if [ -n "$offenders" ]; then
    echo "FAIL (gate 5): em dashes found in documentation prose:"
    echo "$offenders" | sed 's/^/  /'
    echo ""
    echo "        Replace with a colon, a comma, or a full stop."
    echo "        If the line is a verbatim quotation, leave the quote intact;"
    echo "        lines containing a double quote are exempt."
    fail=$((fail + 1))
else
    echo "PASS (gate 5): no em dashes outside quoted material"
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
