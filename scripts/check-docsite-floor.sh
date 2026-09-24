#!/usr/bin/env bash
# Floor gates for the docsite. Runs in CI (the `Docsite floor` job in
# .github/workflows/ci.yml); can be invoked locally via `just docs-floor`.
#
# Seven gates protect against the cheap-to-introduce, expensive-to-find
# regressions. The pages live in site/content/ and the SvelteKit site in site/
# builds them (EP-0012). roadmap.md there is a symlink to the repository's
# ROADMAP.md, and gates 2 and 5 follow it.
#
#   1. Link integrity:      lychee verifies every link in site/content/ and in
#                            the built SvelteKit site, fragments included.
#   2. Orphan detect:       every page under site/content/ is referenced in SUMMARY.md.
#   3. Type-name parity:    every backtick-inline PascalCase identifier in site/content/
#                            and skills/ either exists as an identifier in src/,
#                            or is on the external-types allowlist below. Catches
#                            rename drift. Plans and design records live in
#                            blueprints/, outside the book, and are not scanned:
#                            an RFC or an EP names types that do not exist yet,
#                            which is what makes it a proposal or a plan.
#   4. Site build:          bun install --frozen-lockfile, svelte-check with
#                            warnings as failures, and a clean prerendered build.
#   5. No em dashes:        project prose convention, exempting quoted material.
#                            Covers site/content/, skills/ and blueprints/.
#   6. llms.txt currency:   site/content/llms.txt is what scripts/gen-llms-txt.sh
#                            writes today. The file is generated from SUMMARY.md
#                            and from the opening line of each page, so a page
#                            added, retitled, or reworded leaves it stale, and
#                            a stale map is worse for an agent than none.
#   7. URL manifest:        the build writes every published path in
#                            site/urls.txt, and every page it writes is listed
#                            there (scripts/check-url-manifest.sh --build).
#
# Execution order is 2, 3, 5, 6, 4, 1, 7: the build comes before the link check
# and the manifest check that read its output.
#
# Local invocation: lychee is optional locally (skip-with-warning); CI installs it.
# bun is required (gate 4 fails without it).
#
# Tunables:
#   LYCHEE_REQUIRED=1:    turn the "lychee missing" skip into a hard failure.
#                          The `Docsite floor` job in ci.yml sets it.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

fail=0
skipped=0

# ---------------------------------------------------------------------------
# Gate 2: orphan detect
# ---------------------------------------------------------------------------
echo "=== Gate 2: orphan detect ==="
orphans=()
pages=0
# -L follows symlinks, so roadmap.md (a link to ROADMAP.md) is a page here
# like any other. Without it, `-type f` skipped the link and the gate never
# checked that the roadmap was listed.
while IFS= read -r page; do
    pages=$((pages + 1))
    rel="${page#site/content/}"
    # SUMMARY.md references look like "(./path/to/page.md)" or "(path/to/page.md)".
    if ! grep -F -q -e "($rel)" -e "(./$rel)" site/content/SUMMARY.md; then
        orphans+=("$page")
    fi
done < <(find -L site/content -type f -name '*.md' \
    ! -name SUMMARY.md \
    | sort)
if [ "$pages" -eq 0 ]; then
    echo "FAIL (gate 2): found no pages under site/content/"
    fail=$((fail + 1))
elif [ ${#orphans[@]} -eq 0 ]; then
    echo "PASS (gate 2): all $pages site/content/ pages are in SUMMARY.md"
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

# rg is required here. The names come from an rg search, and with rg absent
# the loop below read zero names and printed a pass.
unknown=()
names=""
checked=0
gate3_ok=1
if ! command -v rg >/dev/null 2>&1; then
    echo "FAIL (gate 3): ripgrep (rg) not installed"
    echo "        install: brew install ripgrep, apt-get install ripgrep, or cargo install ripgrep"
    gate3_ok=0
else
    # rg exits 1 on no match and 2 on an error; only the error is a failure.
    rg_rc=0
    names=$(rg -oIN --pcre2 -e '`([A-Z][a-zA-Z0-9_]+)`' --replace '$1' site/content/ skills/) || rg_rc=$?
    if [ "$rg_rc" -gt 1 ]; then
        echo "FAIL (gate 3): rg exited $rg_rc extracting names from site/content/ and skills/"
        gate3_ok=0
    fi
fi
while IFS= read -r name; do
    [ -z "$name" ] && continue
    checked=$((checked + 1))
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
done < <(printf '%s\n' "$names" | sort -u)

if [ "$gate3_ok" -eq 0 ]; then
    fail=$((fail + 1))
elif [ "$checked" -eq 0 ]; then
    echo "FAIL (gate 3): found no backtick-inline names in site/content/ or skills/"
    fail=$((fail + 1))
elif [ ${#unknown[@]} -eq 0 ]; then
    echo "PASS (gate 3): all $checked backtick-inline names resolve in src/ or an allowlist"
else
    echo "FAIL (gate 3): backtick-inline identifiers in site/content/ or skills/ not found in src/:"
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
# Lines carrying a double quote are exempt. The RFCs and EPs in blueprints/
# quote reviewers verbatim, and silently editing an attributed quote to satisfy
# a house style rule would be a worse fault than the em dash.
echo "=== Gate 5: no em dashes in prose ==="
# The pattern is the literal U+2014 byte sequence. It used to be written
# '\u2014', which grep reads as the letter u followed by 2014: the gate
# had never matched an em dash in its life.
# grep exits 1 on no match and 2 on an error such as a missing directory; the
# old `|| true` read the second as a clean pass.
em_files=$(find -L site/content skills blueprints -type f \( -name '*.md' -o -name 'llms.txt' \) | wc -l | tr -d ' ') || em_files=0
em_rc=0
em_raw=$(grep -Rn '—' site/content skills blueprints --include='*.md' --include='llms.txt') || em_rc=$?
offenders=$(printf '%s\n' "$em_raw" | grep -v '"' || true)
if [ "$em_rc" -gt 1 ] || [ "$em_files" -eq 0 ]; then
    echo "FAIL (gate 5): could not scan site/content/, skills/ and blueprints/ (grep exit $em_rc, $em_files files)"
    fail=$((fail + 1))
elif [ -n "$offenders" ]; then
    echo "FAIL (gate 5): em dashes found in documentation prose:"
    echo "$offenders" | sed 's/^/  /'
    echo ""
    echo "        Replace with a colon, a comma, or a full stop."
    echo "        If the line is a verbatim quotation, leave the quote intact;"
    echo "        lines containing a double quote are exempt."
    fail=$((fail + 1))
else
    echo "PASS (gate 5): no em dashes outside quoted material in $em_files files"
fi
echo ""

# ---------------------------------------------------------------------------
# Gate 6: llms.txt currency
# ---------------------------------------------------------------------------
# site/content/llms.txt is the agent-facing map of the docsite: the H1, the
# blockquote summary and the H2 link sections the llms.txt proposal fixes.
# Every line of it is derived, so the file is generated rather than written,
# and it is committed so that a reader of the repository sees what the site
# serves. This gate regenerates it into a temporary file and diffs. A page
# added to SUMMARY.md, retitled, or whose opening sentence changed leaves the
# committed copy stale, and a map that points somewhere the site does not go
# is worse for an agent than no map at all.
echo "=== Gate 6: llms.txt currency ==="
llms_expected=$(mktemp)
if gen_out=$(bash scripts/gen-llms-txt.sh "$llms_expected" 2>&1); then
    if llms_diff=$(diff -u site/content/llms.txt "$llms_expected" 2>&1); then
        echo "PASS (gate 6): site/content/llms.txt is current"
    else
        echo "FAIL (gate 6): site/content/llms.txt is stale"
        echo "$llms_diff" | sed 's/^/  /'
        echo ""
        echo "        run scripts/gen-llms-txt.sh"
        fail=$((fail + 1))
    fi
else
    echo "FAIL (gate 6): scripts/gen-llms-txt.sh did not run"
    echo "$gen_out" | sed 's/^/  /'
    fail=$((fail + 1))
fi
rm -f "$llms_expected"
echo ""

# ---------------------------------------------------------------------------
# Gate 4: the site builds clean
# ---------------------------------------------------------------------------
# site/ is the docsite (EP-0012). Three steps, each of which fails the gate:
#
#   install        bun install --frozen-lockfile: bun.lock is the pin, and a
#                  package.json it does not match is a failure, not an update.
#   svelte-check   types and Svelte diagnostics, with warnings as failures.
#   build          prerenders every page. The renderer fails the build on a
#                  tag not in the registry, a link to no page, a fence language
#                  with no grammar, a page without a title, or a SUMMARY.md
#                  entry with no file; the crawler fails it on a link to a
#                  missing route or heading. Then the base-path check and the
#                  Pagefind index. Any warning in the log fails.
#
# BASE_PATH is empty here so the output can be link-checked from its own root
# (gate 1). docs.yml builds the deployed site with BASE_PATH=/matra.
echo "=== Gate 4: site build (svelte-check, build) ==="
site_log=$(mktemp)
trap 'rm -f "$site_log"' EXIT
# Vite and Rollup print warnings as "(!) ...", SvelteKit and plugins as
# "[warn]" or "warning:".
warn_re='^[[:space:]]*\(!\)|\[warn|warning:'
if ! command -v bun >/dev/null 2>&1; then
    echo "FAIL (gate 4): bun not installed"
    echo "        install: https://bun.sh (the version CI uses is BUN_VERSION in ci.yml)"
    fail=$((fail + 1))
elif ! (cd site && bun install --frozen-lockfile) >"$site_log" 2>&1; then
    echo "FAIL (gate 4): bun install --frozen-lockfile failed"
    sed 's/^/  /' "$site_log"
    fail=$((fail + 1))
elif ! (cd site && bun run check) >"$site_log" 2>&1; then
    echo "FAIL (gate 4): svelte-check reported errors or warnings"
    sed 's/^/  /' "$site_log"
    fail=$((fail + 1))
else
    checked_line=$(grep -E 'COMPLETED|svelte-check found' "$site_log" | tail -1)
    if ! (cd site && BASE_PATH='' bun run build) >"$site_log" 2>&1; then
        echo "FAIL (gate 4): the site did not build"
        sed 's/^/  /' "$site_log"
        fail=$((fail + 1))
    elif grep -E -i -q "$warn_re" "$site_log"; then
        echo "FAIL (gate 4): the site build produced warnings"
        grep -E -i "$warn_re" "$site_log" | sed 's/^/  /'
        fail=$((fail + 1))
    else
        html=$(find site/build -name '*.html' | wc -l | tr -d ' ')
        twins=$(find site/build -name '*.md' | wc -l | tr -d ' ')
        indexed=$(grep -E 'Indexed [0-9]+ pages' "$site_log" | tr -s ' ' | sed 's/^ //')
        echo "  svelte-check: ${checked_line:-no summary line}"
        echo "  $(grep 'verify-base-path:' "$site_log")"
        echo "  pagefind: ${indexed:-no index summary}"
        echo "PASS (gate 4): site/build holds $html pages and $twins Markdown twins"
    fi
fi
echo ""

# ---------------------------------------------------------------------------
# Gate 1: link integrity (lychee)
# ---------------------------------------------------------------------------
# Two inputs. The Markdown in site/content/, as authored: relative links
# between pages resolve on disk. And the built site/build/, as served: every
# link, asset and #fragment in the prerendered HTML resolves from the site
# root, extensionless routes resolving to their .html file as GitHub Pages
# does. /api/ is the one exclusion: rustdoc is assembled beside the site at
# deploy time and is not in this build.
echo "=== Gate 1: link integrity (lychee) ==="
if command -v lychee >/dev/null 2>&1; then
    gate1_ok=1
    if ! lychee --no-progress --offline 'site/content/**/*.md'; then
        echo "FAIL (gate 1): broken links in site/content/"
        gate1_ok=0
    fi
    if [ ! -d site/build ]; then
        echo "FAIL (gate 1): no site/build to check; gate 4 did not build it"
        gate1_ok=0
    elif ! lychee --no-progress --offline \
            --root-dir "$REPO_ROOT/site/build" \
            --fallback-extensions html \
            --include-fragments \
            --exclude '/api/?$' \
            'site/build/**/*.html'; then
        echo "FAIL (gate 1): broken links in the built site"
        gate1_ok=0
    fi
    if [ "$gate1_ok" -eq 1 ]; then
        echo "PASS (gate 1): all links in site/content/ and site/build/ resolve"
    else
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
        skipped=$((skipped + 1))
    fi
fi
echo ""

# ---------------------------------------------------------------------------
# Gate 7: the build writes every published URL
# ---------------------------------------------------------------------------
# site/urls.txt is the published URL contract: every path the site has served,
# the mdBook-era pages included. The build must write each one (api/ aside,
# which rustdoc supplies at deploy), and every page the build writes must be
# listed, so no page ships outside the contract. docs.yml checks the same list
# against the assembled artifact before upload and against the live site after
# deploy.
echo "=== Gate 7: URL manifest ==="
if [ ! -d site/build ]; then
    echo "FAIL (gate 7): no site/build to check; gate 4 did not build it"
    fail=$((fail + 1))
elif ! bash scripts/check-url-manifest.sh --build site/build; then
    fail=$((fail + 1))
fi
echo ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
gates=7
if [ "$fail" -eq 0 ]; then
    echo "docsite floor: $gates gates, $((gates - skipped)) passed, $skipped skipped"
    exit 0
else
    echo "docsite floor: $gates gates, $fail failed, $skipped skipped"
    exit 1
fi
