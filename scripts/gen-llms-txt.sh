#!/usr/bin/env bash
# Generate site/content/llms.txt from site/content/SUMMARY.md.
#
# llms.txt (https://llmstxt.org/) is a single file at the root of a
# documentation site that gives an agent the map a human gets from the
# sidebar: what the project is in one line, then every page with a one-line
# summary and a link. The shape the proposal fixes is an H1, a blockquote
# summary, then H2 sections of links.
#
# Everything here is derived, nothing is written twice:
#
#   the H1            the crate name from Cargo.toml
#   the blockquote    the crate description from Cargo.toml
#   the H2 sections   the `# Section` headers in SUMMARY.md, in order
#   each link title   the link text in SUMMARY.md
#   each link target  the deployed page: <BASE>/<path with .md -> .html>
#   each summary      the first prose sentence of the page itself
#
# Only top-level SUMMARY.md entries are listed. A nested entry is reachable
# from its parent's page.
#
# The output goes under site/content/ beside the pages it maps. Both builds
# serve it at the site root with no step in the deploy workflow to keep in
# sync with this script: mdbook copies every non-chapter file there into its
# output, and the SvelteKit site prerenders site/src/routes/llms.txt from it.
#
# The file is committed. Gate 6 of scripts/check-docsite-floor.sh regenerates
# it and diffs, so a page added, retitled, or reworded without a regeneration
# fails the floor.
#
# Usage:
#   scripts/gen-llms-txt.sh              write site/content/llms.txt
#   scripts/gen-llms-txt.sh <path>       write somewhere else (the gate does this)

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

BASE_URL="https://mox-labs.github.io/matra"
SUMMARY="site/content/SUMMARY.md"
OUT="${1:-site/content/llms.txt}"

# --- the crate identity -----------------------------------------------------
# The first `name =` and `description =` under [package]; the range stops at
# the next table header, so a dependency's own fields cannot be picked up.
crate_name=$(sed -n '/^\[package\]/,/^\[/{s/^name = "\(.*\)"$/\1/p;}' Cargo.toml | head -1)
crate_desc=$(sed -n '/^\[package\]/,/^\[/{s/^description = "\(.*\)"$/\1/p;}' Cargo.toml | head -1)

if [ -z "$crate_name" ] || [ -z "$crate_desc" ]; then
    echo "gen-llms-txt: could not read name/description from Cargo.toml" >&2
    exit 1
fi

# --- the first prose sentence of a page -------------------------------------
# Walks the file, skipping everything that is not a prose paragraph: headings,
# blockquotes (a plan's shipped banner is one), fenced code, tables, lists,
# HTML, and images. The first line that survives is the opening paragraph, and
# its first sentence is the summary.
#
# A sentence ends at a period, question mark, or exclamation followed by a
# space and a capital letter, or by the end of the line. The lookbehinds
# exempt a capital-letter abbreviation ("U.S. Navy"), a version number
# ("0.1.0"), and "e.g." and "i.e.", so none of those ends a sentence early.
# Other lowercase abbreviations are not exempt. A sentence shorter than the floor takes the next
# one with it, because "Run `matra --skill`." names a page without describing
# it.
first_sentence() {
    perl -0777 -ne '
        my $line;
        my $in_fence = 0;
        for my $l (split /\n/, $_) {
            if ($l =~ /^\s*(```|~~~)/) { $in_fence = !$in_fence; next; }
            next if $in_fence;
            next if $l =~ /^\s*$/;          # blank
            next if $l =~ /^\s*#/;          # heading
            next if $l =~ /^\s*>/;          # blockquote / shipped banner
            next if $l =~ /^\s*[-*+]\s/;    # bullet
            next if $l =~ /^\s*\d+\.\s/;    # ordered item
            next if $l =~ /^\s*\|/;         # table row
            next if $l =~ /^\s*</;          # html / comment
            next if $l =~ /^\s*!\[/;        # image
            $line = $l;
            last;
        }
        exit 1 unless defined $line;

        # Markdown to plain text: links to their text, bold and italic away.
        $line =~ s/\[([^\]]*)\]\([^)]*\)/$1/g;
        $line =~ s/\*\*([^*]*)\*\*/$1/g;
        $line =~ s/(?<!\*)\*([^*]+)\*(?!\*)/$1/g;
        $line =~ s/^\s+|\s+$//g;

        my $out = "";
        my $rest = $line;
        while (length($out) < 40 && length($rest)) {
            if ($rest =~ /^(.*?(?<![A-Z])(?<!\d)(?<!\be\.g)(?<!\bi\.e)[.!?])(?:(\s+[A-Z(`"].*)|$)/) {
                $out .= ($out eq "" ? "" : " ") . $1;
                $rest = defined($2) ? $2 : "";
                $rest =~ s/^\s+//;
            } else {
                $out .= ($out eq "" ? "" : " ") . $rest;
                last;
            }
        }
        print "$out\n";
    ' "$1"
}

# --- emit -------------------------------------------------------------------
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

{
    printf '# %s\n\n' "$crate_name"
    printf '> %s\n' "$crate_desc"

    section=""
    pending_section="Introduction"   # SUMMARY.md prefix chapters land here

    # Held in variables: an unquoted regex with parentheses in it is read by
    # the shell before [[ =~ ]] ever sees it.
    heading_re='^#[[:space:]]+(.+)$'
    link_re='^(-[[:space:]]+)?\[([^]]+)\]\(\.?/?([^)]+\.md)\)'

    while IFS= read -r raw; do
        # A `# Heading` in SUMMARY.md opens a part. The first one is the book
        # title ("# Summary") and names no part.
        if [[ "$raw" =~ $heading_re ]]; then
            heading="${BASH_REMATCH[1]}"
            [ "$heading" = "Summary" ] && continue
            pending_section="$heading"
            continue
        fi

        # A link to a .md page at the start of the line: a prefix chapter or
        # a top-level list item. The anchor is what drops nested entries,
        # which SUMMARY.md indents.
        if [[ "$raw" =~ $link_re ]]; then
            title="${BASH_REMATCH[2]}"
            path="${BASH_REMATCH[3]}"
        else
            continue
        fi

        if [ ! -f "site/content/$path" ]; then
            echo "gen-llms-txt: SUMMARY.md points at a missing page: $path" >&2
            exit 1
        fi

        if [ "$pending_section" != "$section" ]; then
            section="$pending_section"
            printf '\n## %s\n\n' "$section"
        fi

        if ! summary=$(first_sentence "site/content/$path"); then
            echo "gen-llms-txt: no prose paragraph found in $path" >&2
            exit 1
        fi
        # mdbook renders a directory's README.md as its index.html.
        page="${path%.md}"
        page="${page%README}"
        [ "$page" != "${path%.md}" ] && page="${page}index"

        printf -- '- [%s](%s/%s.html): %s\n' \
            "$title" "$BASE_URL" "$page" "$summary"
    done < "$SUMMARY"
} > "$tmp"

mkdir -p "$(dirname "$OUT")"
# cat rather than cp: mktemp creates 0600 and cp would carry that mode onto a
# file the whole world is meant to read.
cat "$tmp" > "$OUT"
echo "gen-llms-txt: wrote $OUT"
