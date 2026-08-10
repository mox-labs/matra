#!/usr/bin/env bash
# Installs matra's git hooks into the local clone.
#
# Hooks live under scripts/ in the repo (so they are versioned and
# reviewable). This script copies them into .git/hooks/ where git
# actually invokes them.
#
# Re-run after pulling new hook content.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
hooks_dir="$repo_root/.git/hooks"

install_one() {
    local source="$1"
    local target="$2"
    install -m 0755 "$source" "$target"
    echo "  installed: $(basename "$target")"
}

mkdir -p "$hooks_dir"
install_one "$repo_root/scripts/pre-commit-hook.sh" "$hooks_dir/pre-commit"

echo "matra git hooks installed into $hooks_dir"
