#!/usr/bin/env bash
# Hold the docsite to its published URLs (site/urls.txt, EP-0012 M2).
#
# The manifest lists every path the site has published. A path that stops
# resolving breaks a citation somewhere: in Cargo.toml, CITATION.cff, the
# README, llms.txt, or a page outside this repository. Three modes:
#
#   --build DIR   The SvelteKit build (gate 7 of the docsite floor). Every
#                 listed path outside api/ must exist in DIR. And every .html,
#                 .md and .txt file DIR holds outside _app/ and pagefind/ must
#                 be listed, so a new page cannot ship without entering the
#                 contract.
#   --dir DIR     The assembled Pages artifact (docs.yml, before upload).
#                 Every listed path must exist, rustdoc's included.
#   --live URL    The deployed site (docs.yml, after deploy). Every listed
#                 path, and the root, must answer 200. Pages can take a moment
#                 to serve a new deploy everywhere, so a non-200 is retried
#                 before it counts.
#
# Usage: scripts/check-url-manifest.sh --build site/build
#        scripts/check-url-manifest.sh --dir _site
#        scripts/check-url-manifest.sh --live https://mox-labs.github.io/matra/

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
MANIFEST="$REPO_ROOT/site/urls.txt"

mode="${1:-}"
target="${2:-}"
if [ -z "$mode" ] || [ -z "$target" ]; then
    echo "usage: $0 --build DIR | --dir DIR | --live URL" >&2
    exit 2
fi
if [ ! -f "$MANIFEST" ]; then
    echo "FAIL (url manifest): $MANIFEST not found"
    exit 1
fi

paths=()
while IFS= read -r line; do
    case "$line" in '' | '#'*) continue ;; esac
    paths+=("$line")
done < "$MANIFEST"
if [ "${#paths[@]}" -eq 0 ]; then
    echo "FAIL (url manifest): $MANIFEST lists no paths"
    exit 1
fi

missing=()
case "$mode" in
--build | --dir)
    if [ ! -d "$target" ]; then
        echo "FAIL (url manifest): no directory at $target"
        exit 1
    fi
    checked=0
    for p in "${paths[@]}"; do
        if [ "$mode" = "--build" ] && [ "${p#api/}" != "$p" ]; then continue; fi
        checked=$((checked + 1))
        [ -f "$target/$p" ] || missing+=("$p")
    done
    unlisted=()
    if [ "$mode" = "--build" ]; then
        while IFS= read -r f; do
            printf '%s\n' "${paths[@]}" | grep -Fxq -- "$f" || unlisted+=("$f")
        done < <(cd "$target" && find . \( -name '*.html' -o -name '*.md' -o -name '*.txt' \) \
            -not -path './_app/*' -not -path './pagefind/*' | sed 's#^\./##' | LC_ALL=C sort)
    fi
    echo "url manifest: ${#paths[@]} paths listed, $checked checked in $target, ${#missing[@]} missing, ${#unlisted[@]} unlisted"
    status=0
    if [ "${#missing[@]}" -gt 0 ]; then
        echo "FAIL (url manifest): listed in site/urls.txt, absent from $target:"
        printf '  %s\n' "${missing[@]}"
        status=1
    fi
    if [ "${#unlisted[@]}" -gt 0 ]; then
        echo "FAIL (url manifest): written by the build, not listed in site/urls.txt:"
        printf '  %s\n' "${unlisted[@]}"
        echo "        A new page is a new published URL: add it to site/urls.txt."
        status=1
    fi
    [ "$status" -eq 0 ] && echo "PASS (url manifest): every published path is in $target"
    exit "$status"
    ;;
--live)
    base="${target%/}"
    attempts="${URL_CHECK_ATTEMPTS:-6}"
    delay="${URL_CHECK_DELAY:-10}"
    fetch() {
        curl -sS -o /dev/null -w '%{http_code}' --max-time 20 "$1" 2>/dev/null || echo 000
    }
    checked=0
    for p in "" "${paths[@]}"; do
        url="$base/$p"
        checked=$((checked + 1))
        code=$(fetch "$url")
        n=1
        while [ "$code" != "200" ] && [ "$n" -lt "$attempts" ]; do
            sleep "$delay"
            code=$(fetch "$url")
            n=$((n + 1))
        done
        if [ "$code" = "200" ]; then
            echo "  200  $url"
        else
            echo "  $code  $url  (after $n attempts)"
            missing+=("$url ($code)")
        fi
    done
    echo "url manifest: $checked URLs checked on $base, ${#missing[@]} not answering 200"
    if [ "${#missing[@]}" -gt 0 ]; then
        echo "FAIL (url manifest): published URLs that do not resolve:"
        printf '  %s\n' "${missing[@]}"
        exit 1
    fi
    echo "PASS (url manifest): every published URL answers 200"
    ;;
*)
    echo "usage: $0 --build DIR | --dir DIR | --live URL" >&2
    exit 2
    ;;
esac
