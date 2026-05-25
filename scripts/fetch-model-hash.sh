#!/usr/bin/env bash
# Fetches the UDPipe English model and prints its SHA-256.
#
# Used to populate the pinned hash constants in src/nlp/udpipe.rs.
# Run when updating to a new model version; paste the output into code.
#
# Usage: scripts/fetch-model-hash.sh [model-name]
#   model-name defaults to "english-ewt"

set -euo pipefail

MODEL="${1:-english-ewt}"
# LINDAT migrated the bitstream endpoint. The /xmlui/bitstream/... pattern
# now returns an HTML preview; the /server/api/core/bitstreams/... pattern
# returns the actual binary. Verified 2026-05-21.
URL="https://lindat.mff.cuni.cz/repository/server/api/core/bitstreams/handle/11234/1-3131/${MODEL}-ud-2.5-191206.udpipe?sequence=17&isAllowed=y"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Fetching ${MODEL} model to ${TMP}..." >&2
curl -fsSL -o "${TMP}/model.udpipe" "${URL}"

SIZE=$(stat -f%z "${TMP}/model.udpipe" 2>/dev/null || stat -c%s "${TMP}/model.udpipe")
HASH=$(shasum -a 256 "${TMP}/model.udpipe" | awk '{print $1}')

echo
echo "Model:  ${MODEL}-ud-2.5-191206.udpipe"
echo "Size:   ${SIZE} bytes"
echo "SHA256: ${HASH}"
echo
echo "Paste into src/nlp/udpipe.rs:"
echo "    const ENGLISH_MODEL_SHA256: &str = \"${HASH}\";"
echo "    const ENGLISH_MODEL_SIZE: u64 = ${SIZE};"
