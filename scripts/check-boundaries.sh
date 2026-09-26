#!/usr/bin/env bash
# Mechanical checks for matra's boundary rules and library conventions: the
# semgrep rules in .semgrep/, one file per rule group. EP-0014 is the design
# record; site/content/reference/boundary-rules.md says what each rule is
# for. Runs from `just boundary` (and so `just check`), the pre-commit hook,
# and the `Boundary check` job in .github/workflows/ci.yml.
#
# Four steps, each built to fail loudly rather than pass over nothing:
#
#   0. Path scopes. Every `paths: include:` entry must name a file git has,
#      or a glob that matches one; a rule scoped to a moved file scans
#      nothing. Runs in --scan-only mode too.
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

# `--scan-only` skips the rule tests: the pre-commit hook passes it when no
# rule or fixture is staged, because the tests cost a semgrep start per rule
# file (about a minute) and the rules did not change. `just boundary` and CI
# never pass it.
scan_only=0
if [ "${1:-}" = "--scan-only" ]; then
    scan_only=1
fi

# --- The rule files ----------------------------------------------------------

shopt -s nullglob
configs=(.semgrep/*.yml)
shopt -u nullglob
if [ "${#configs[@]}" -eq 0 ]; then
    echo "FAIL: no rule files in .semgrep/" >&2
    exit 2
fi

# --- 0. Path scopes ----------------------------------------------------------
#
# A rule whose `paths: include:` names a file that no longer exists, or a glob
# that matches nothing, scans nothing and passes; `semgrep --test` ignores
# paths, so the fixtures cannot notice. Moving a file is how that happens, so
# this runs in --scan-only mode too, where the hook commits the move.

git ls-files --cached --others --exclude-standard >"$tmp/tracked"
python3 - "$tmp/tracked" "${configs[@]}" <<'PY' || fail=1
import re, sys

tracked = [line.strip() for line in open(sys.argv[1]) if line.strip()]

def glob_to_regex(pattern):
    # Semgrep's gitignore-style paths: a leading "/" anchors at the repo
    # root, "**/" is any number of directories, "*" and "?" stay in one.
    anchored = pattern.startswith("/")
    body = pattern.lstrip("/")
    out, i = [], 0
    while i < len(body):
        if body.startswith("**/", i):
            out.append("(?:.*/)?"); i += 3
        elif body.startswith("**", i):
            out.append(".*"); i += 2
        elif body[i] == "*":
            out.append("[^/]*"); i += 1
        elif body[i] == "?":
            out.append("[^/]"); i += 1
        else:
            out.append(re.escape(body[i])); i += 1
    return re.compile(("^" if anchored else "^(?:.*/)?") + "".join(out) + "(?:/.*)?$")

QUOTED = re.compile(r'^"([^"]+)"$')

def includes(path):
    # Rule id -> include entries, and the lines this reader could not read.
    # It reads the two YAML shapes .semgrep/ uses, and only those:
    # `include: ["/a", "/b"]` on one line, and a block list of `- "/a"`
    # lines. No YAML parser is available to python3 both locally and in CI
    # without a new dependency, so anything else (single quotes, no quotes,
    # a flow list over several lines, a bare scalar) is reported rather than
    # skipped: a skipped entry would be a scope nobody checks.
    rules, unread, rule, in_include = {}, [], None, False
    for number, raw in enumerate(open(path), 1):
        line = raw.rstrip("\n")
        m = re.match(r"^  - id:\s*(\S+)", line)
        if m:
            rule, in_include = m.group(1), False
            rules[rule] = []
            continue
        m = re.match(r"^\s+include:\s*(.*)$", line)
        if m and rule:
            rest = m.group(1).strip()
            in_include = False
            if rest == "":
                in_include = True
            elif rest.startswith("[") and rest.endswith("]"):
                for item in (i.strip() for i in rest[1:-1].split(",")):
                    q = QUOTED.match(item)
                    if q:
                        rules[rule].append(q.group(1))
                    elif item:
                        unread.append((rule, number, item))
            else:
                unread.append((rule, number, rest))
            if not in_include and not rules[rule]:
                unread.append((rule, number, "an include: key with no entry read from it"))
            continue
        if in_include:
            m = re.match(r"^\s+-\s*(.*?)\s*$", line)
            if m:
                q = QUOTED.match(m.group(1))
                if q:
                    rules[rule].append(q.group(1))
                else:
                    unread.append((rule, number, m.group(1)))
                continue
            in_include = False
            if not rules[rule]:
                unread.append((rule, number, "an include: key with no entry read from it"))
    if in_include and not rules[rule]:
        unread.append((rule, number, "an include: key with no entry read from it"))
    return rules, unread

dead = 0
checked = 0
for cfg in sys.argv[2:]:
    scoped, unread = includes(cfg)
    for rule, number, text in unread:
        if text.startswith("an include: key"):
            print(f"FAIL: rule {rule} ({cfg}:{number}): {text}.")
        else:
            print(f"FAIL: rule {rule} ({cfg}:{number}): cannot read the include entry {text!r}.")
        print('      Write each entry double-quoted, as `include: ["/src/x.rs"]` or a `- "/src/x.rs"` line;')
        print("      an entry this check cannot read is a scope nobody checks.")
        dead += 1
    for rule, entries in scoped.items():
        for entry in entries:
            checked += 1
            regex = glob_to_regex(entry)
            if not any(regex.match(path) for path in tracked):
                kind = "glob matches no file" if any(c in entry for c in "*?[") else "path does not exist"
                print(f"FAIL: rule {rule} ({cfg}) scopes itself to {entry}, and that {kind} in git.")
                print("      The rule now scans nothing and passes silently. Update the rule's paths when you move a file.")
                dead += 1
print(f"check-boundaries: {checked} path scopes checked, {dead} dead or unread")
sys.exit(1 if dead else 0)
PY
if [ "$fail" -ne 0 ]; then
    echo "check-boundaries: a rule's path scope is dead or unread; the scan did not run"
    exit 1
fi

# --- 1. Rule tests -----------------------------------------------------------

rule_count=0
for cfg in "${configs[@]}"; do
    [ "$scan_only" -eq 1 ] && continue
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

if [ "$scan_only" -eq 1 ]; then
    echo "check-boundaries: ${#configs[@]} rule files scanned; rule tests skipped (--scan-only)"
else
    echo "check-boundaries: ${#configs[@]} rule files, $rule_count rules, each tested against its fixture"
fi
exit "$fail"
