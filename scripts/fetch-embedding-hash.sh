#!/usr/bin/env bash
# Fetches the three potion-base-8M artifacts at a pinned revision and
# prints the SHA-256 over all three, concatenated in the order the adapter
# hashes them: model.safetensors, tokenizer.json, config.json.
#
# Used to populate the pinned constants in src/embed/model2vec.rs and the
# artifact_hash in spec/tests/semantic/reference-model.json. Run when
# moving to a new revision; paste the output into both.
#
# Usage: scripts/fetch-embedding-hash.sh [revision]
#   revision defaults to the one the adapter's URL constants name, read
#   out of the source rather than repeated here: a pin written twice is a
#   pin that drifts the first time only one copy is updated.
#   Find a newer revision with:
#     curl -s https://huggingface.co/api/models/minishlab/potion-base-8M \
#       | python3 -c 'import json,sys; print(json.load(sys.stdin)["sha"])'

set -euo pipefail

REPO="minishlab/potion-base-8M"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ADAPTER="${SCRIPT_DIR}/../src/embed/model2vec.rs"
FILES=(model.safetensors tokenizer.json config.json)

PINNED=$(sed -n "s|.*huggingface.co/${REPO}/resolve/\([0-9a-f]\{40\}\)/.*|\1|p" \
  "${ADAPTER}" | head -1)
REVISION="${1:-${PINNED}}"
if [ -z "${REVISION}" ]; then
  echo "cannot read the pinned revision out of ${ADAPTER}; pass one as an argument" >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Fetching ${REPO} at ${REVISION} to ${TMP}..." >&2
for f in "${FILES[@]}"; do
  curl -fsSL -o "${TMP}/${f}" "https://huggingface.co/${REPO}/resolve/${REVISION}/${f}"
done

cd "$TMP"
# sha256sum on GNU userlands, shasum on macOS, which ships neither
# sha256sum nor a GNU coreutils by default.
if command -v sha256sum >/dev/null 2>&1; then
  HASH=$(cat "${FILES[@]}" | sha256sum | awk '{print $1}')
else
  HASH=$(cat "${FILES[@]}" | shasum -a 256 | awk '{print $1}')
fi

echo
echo "Revision: ${REVISION}"
for f in "${FILES[@]}"; do
  SIZE=$(stat -f%z "$f" 2>/dev/null || stat -c%s "$f")
  echo "  ${f}: ${SIZE} bytes"
done
echo "Three-file SHA256: ${HASH}"
echo
echo "Paste into src/embed/model2vec.rs:"
echo "    const POTION_BASE_8M_SHA256: &str = \"${HASH}\";"
echo "and update the revision in POTION_BASE_8M_URLS to ${REVISION}."
echo
echo "Paste into spec/tests/semantic/reference-model.json:"
echo "    \"artifact_hash\": \"${HASH}\""
