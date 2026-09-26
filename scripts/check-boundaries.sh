#!/usr/bin/env bash
# Mechanical checks for matra's boundary rules and library conventions: the
# semgrep rules in .semgrep/, one file per rule group. EP-0014 is the design
# record; site/content/reference/boundary-rules.md says what each rule is
# for. Runs from `just boundary` (and so `just check`), the pre-commit hook,
# and the `Boundary check` job in .github/workflows/ci.yml.
#
# Three steps, each built to fail loudly rather than pass over nothing:
#
#   1. Rule tests. Every .semgrep/NAME.yml has a fixture .semgrep/NAME.rs in
#      which every rule id carries at least one `ruleid:` and one `ok:`
#      annotation, and `semgrep --test` passes on the pair. The pairs are
#      listed here rather than discovered: semgrep's test discovery skips a
#      dot-directory such as .semgrep/ and reports "No unit tests found"
#      with exit 0.
#   2. The scan. Every rule runs over src/; any finding fails the check.
#   3. Coverage. The scan must have read every Rust file under src/, with no
#      semgrep error. Semgrep drops files quietly (its size cap, its default
#      ignore list, which .semgrepignore replaces) and reports a timed-out
#      rule as an error rather than a finding, so a clean scan is evidence
#      only when this holds.
#
# Rule 6 (the no-default-features build) is compiled in ci.yml, not here.

set -euo pipefail

cd "$(dirname "$0")/.."

# Offline and deterministic: no usage metrics (also `--metrics=off` below)
# and no call home to ask whether a newer semgrep exists.
export SEMGREP_SEND_METRICS=off
export SEMGREP_ENABLE_VERSION_CHECK=0

if ! command -v semgrep >/dev/null 2>&1; then
    echo "FAIL: check-boundaries needs semgrep, which is not on PATH" >&2
    echo "      install the version CI pins:" >&2
    echo "      pip install --require-hashes -r .github/requirements/semgrep.txt" >&2
    exit 2
fi
if ! command -v python3 >/dev/null 2>&1; then
    echo "FAIL: check-boundaries needs python3 to read semgrep's JSON report" >&2
    exit 2
fi

# The rules scope themselves with `paths:`, and semgrep resolves those
# against the git work tree: outside one, a scoped rule matches nothing and
# the scan passes. Refuse to run anywhere but the root of a work tree.
top=$(git rev-parse --show-toplevel 2>/dev/null) || {
    echo "FAIL: check-boundaries must run inside a git work tree;" >&2
    echo "      semgrep's path-scoped rules match nothing outside one" >&2
    exit 2
}
if [ "$(cd "$top" && pwd -P)" != "$(pwd -P)" ]; then
    echo "FAIL: check-boundaries expected the work tree root at $(pwd -P), git says $top" >&2
    exit 2
fi

pinned=$(sed -n 's/^semgrep==\([0-9.]*\).*/\1/p' .github/requirements/semgrep.txt)
have=$(semgrep --version 2>/dev/null | tail -1)
if [ "$have" != "$pinned" ]; then
    echo "note: semgrep $have here, CI pins $pinned; the rule tests below say whether the rules behave the same" >&2
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail=0

# --- 1. Rule tests -----------------------------------------------------------

shopt -s nullglob
configs=(.semgrep/*.yml)
shopt -u nullglob
if [ "${#configs[@]}" -eq 0 ]; then
    echo "FAIL: no rule files in .semgrep/" >&2
    exit 2
fi

rule_count=0
for cfg in "${configs[@]}"; do
    fixture="${cfg%.yml}.rs"
    if [ ! -f "$fixture" ]; then
        echo "FAIL: $cfg has no fixture $fixture"
        fail=1
        continue
    fi
    ids=$(sed -n 's/^  - id: *//p' "$cfg")
    if [ -z "$ids" ]; then
        echo "FAIL: $cfg declares no rule ids"
        fail=1
        continue
    fi
    for id in $ids; do
        rule_count=$((rule_count + 1))
        for kind in ruleid ok; do
            if ! grep -Eq "//[[:space:]]*${kind}:([^,]*,)*[[:space:]]*${id}[[:space:]]*(,|$)" "$fixture"; then
                echo "FAIL: rule $id has no \`// ${kind}: $id\` case in $fixture"
                fail=1
            fi
        done
    done
    if ! semgrep --test --metrics=off --config "$cfg" "$fixture" >"$tmp/test.log" 2>&1; then
        echo "FAIL: semgrep --test for $cfg"
        sed 's/^/  /' "$tmp/test.log"
        fail=1
    fi
done

if [ "$fail" -ne 0 ]; then
    echo "check-boundaries: rule tests failed; the scan did not run"
    exit 1
fi

# --- 2. The scan -------------------------------------------------------------

scan_rc=0
semgrep scan \
    --config .semgrep \
    --metrics=off \
    --error \
    --max-target-bytes=0 \
    --timeout=0 \
    --json-output="$tmp/scan.json" \
    src || scan_rc=$?

# --- 3. Coverage -------------------------------------------------------------

git ls-files --cached --others --exclude-standard -- 'src/*.rs' >"$tmp/expected"

python3 - "$tmp/scan.json" "$tmp/expected" <<'PY' || fail=1
import json, sys

report = json.load(open(sys.argv[1]))
expected = {line.strip() for line in open(sys.argv[2]) if line.strip()}
scanned = set(report["paths"]["scanned"])
ok = True

if not expected:
    print("FAIL: git lists no Rust files under src/")
    ok = False
missing = sorted(expected - scanned)
if missing:
    print(f"FAIL: semgrep did not read {len(missing)} Rust file(s) under src/:")
    for path in missing:
        print(f"  {path}")
    ok = False
for error in report.get("errors", []):
    print(f"FAIL: semgrep error: {error.get('type')}: {error.get('message', '').strip()[:300]}")
    ok = False

findings = len(report["results"])
print(f"check-boundaries: {len(scanned & expected)} of {len(expected)} Rust files in src/ read, "
      f"{findings} finding(s)")
sys.exit(0 if ok else 1)
PY

if [ "$scan_rc" -ne 0 ]; then
    fail=1
fi

echo "check-boundaries: ${#configs[@]} rule files, $rule_count rules, each tested against its fixture"
exit "$fail"
