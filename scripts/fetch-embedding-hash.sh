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
#   revision defaults to the one currently pinned in the adapter.
#   Find a revision with:
#     curl -s https://huggingface.co/api/models/minishlab/potion-base-8M \
#       | python3 -c 'import json,sys; print(json.load(sys.stdin)["sha"])'

set -euo pipefail

REPO="minishlab/potion-base-8M"
REVISION="${1:-bf8b056651a2c21b8d2565580b8569da283cab23}"
FILES=(model.safetensors tokenizer.json config.json)

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Fetching ${REPO} at ${REVISION} to ${TMP}..." >&2
for f in "${FILES[@]}"; do
  curl -fsSL -o "${TMP}/${f}" "https://huggingface.co/${REPO}/resolve/${REVISION}/${f}"
done

cd "$TMP"
HASH=$(cat "${FILES[@]}" | shasum -a 256 | awk '{print $1}')

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
