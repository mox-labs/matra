#!/usr/bin/env bash
# URL parity between the mdBook build and the SvelteKit build (EP-0012, M1).
#
# Published URLs are cited elsewhere: Cargo.toml's homepage, CITATION.cff,
# README.md, llms.txt, and whatever readers have bookmarked or linked. The
# move off mdBook must not break any of them. This lists every .html path the
# mdBook build serves and fails on any the SvelteKit build does not.
#
# It also checks heading anchors, since `page.html#section` is a URL too: for
# every page both builds serve, each heading id mdBook emitted must exist in
# the new page. The pages each generator writes for itself (print.html, which
# concatenates every page, toc.html and 404.html) are checked for their path
# only: their headings are the generator's wording, not a contract.
#
# Neither build is run here. The docsite floor (scripts/check-docsite-floor.sh)
# builds both and then calls this.
#
# Usage:
#   scripts/check-url-parity.sh [mdbook-build-dir] [sveltekit-build-dir]
#   defaults: book/book site/build

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

old="${1:-book/book}"
new="${2:-site/build}"

for d in "$old" "$new"; do
    if [ ! -d "$d" ]; then
        echo "FAIL (url parity): no build at $d"
        echo "        build both first: (cd book && mdbook build) and just docs-build"
        exit 1
    fi
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

(cd "$old" && find . -name '*.html' | sed 's#^\./##' | LC_ALL=C sort) > "$tmp/old"
(cd "$new" && find . -name '*.html' | sed 's#^\./##' | LC_ALL=C sort) > "$tmp/new"

old_count=$(wc -l < "$tmp/old" | tr -d ' ')
new_count=$(wc -l < "$tmp/new" | tr -d ' ')
if [ "$old_count" -eq 0 ]; then
    echo "FAIL (url parity): the mdBook build at $old has no .html files"
    exit 1
fi

LC_ALL=C comm -23 "$tmp/old" "$tmp/new" > "$tmp/missing"
missing=$(wc -l < "$tmp/missing" | tr -d ' ')

# Heading anchors, page by page.
anchor_pages=0
anchors=0
: > "$tmp/lost"
while IFS= read -r page; do
    case "$page" in print.html | toc.html | 404.html) continue ;; esac
    [ -f "$new/$page" ] || continue
    anchor_pages=$((anchor_pages + 1))
    while IFS= read -r id; do
        [ -z "$id" ] && continue
        anchors=$((anchors + 1))
        if ! grep -F -q "id=\"$id\"" "$new/$page"; then
            echo "  $page#$id" >> "$tmp/lost"
        fi
    done < <(grep -o '<h[1-6] id="[^"]*"' "$old/$page" | sed 's/.*id="\([^"]*\)"/\1/' || true)
done < "$tmp/old"
lost=$(wc -l < "$tmp/lost" | tr -d ' ')

echo "url parity: $old_count pages in $old, $new_count in $new, $missing missing"
echo "anchor parity: $anchors heading anchors across $anchor_pages pages, $lost missing"

status=0
if [ "$missing" -gt 0 ]; then
    echo "FAIL (url parity): served by mdBook, not by the new site:"
    sed 's/^/  /' "$tmp/missing"
    status=1
fi
if [ "$lost" -gt 0 ]; then
    echo "FAIL (anchor parity): heading anchors mdBook served that the new site does not:"
    cat "$tmp/lost"
    status=1
fi
[ "$status" -eq 0 ] && echo "PASS (url parity): every mdBook page and heading anchor resolves in the new site"
exit "$status"
