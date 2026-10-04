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
# A last section, Blueprints, lists the design records the site renders from
# blueprints/ (site/src/lib/server/blueprints.ts): the process and index,
# every RFC, then every EP, each titled by its `# ` heading, with its status
# as the site's navigation shows it, and summarized by its first prose
# sentence. The templates come last in their kind.
#
# The output goes under site/content/ beside the pages it maps. The site
# serves it at its root with no step in the deploy workflow to keep in sync
# with this script: site/src/routes/llms.txt prerenders it as committed.
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
        my $in_comment = 0;
        for my $l (split /\n/, $_) {
            if ($in_comment) { $in_comment = 0 if $l =~ /-->/; next; }
            if ($l =~ /^\s*<!--/ && $l !~ /-->/) { $in_comment = 1; next; }
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
            if (defined $line) { $line .= " $l"; next; }
            $line = $l;
            next;
        } continue {
            # A paragraph hard-wrapped over several lines (the design records
            # are) is read whole: it ends at the first line that is not prose.
            last if defined $line && $l =~ /^\s*$|^\s*(#|>|[-*+]\s|\d+\.\s|\||<|!\[|```|~~~)/;
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
    # Leading space admits a nested entry (the Examples part nests its pages
    # under an index); an anchored pattern skipped them without a word.
    link_re='^[[:space:]]*(-[[:space:]]+)?\[([^]]+)\]\(\.?/?([^)]+\.md)\)'

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
        # The site serves a directory's README.md as its index.html
        # (routeOf in site/src/lib/server/summary.ts).
        page="${path%.md}"
        page="${page%README}"
        [ "$page" != "${path%.md}" ] && page="${page}index"

        printf -- '- [%s](%s/%s.html): %s\n' \
            "$title" "$BASE_URL" "$page" "$summary"
    done < "$SUMMARY"

    # The design records, in the order the site's navigation lists them. The
    # templates and the legacy records are not rendered, so not listed.
    printf '\n## Blueprints\n\n'
    records=(blueprints/README.md)
    for kind in proposals plans; do
        for f in "blueprints/$kind"/[0-9][0-9][0-9][0-9]-*.md; do
            case "$f" in */0000-template.md) ;; *) [ -f "$f" ] && records+=("$f") ;; esac
        done
    done
    for f in "${records[@]}"; do
        if [ ! -f "$f" ]; then
            echo "gen-llms-txt: missing design record $f" >&2
            exit 1
        fi
        case "$f" in
        blueprints/README.md) title="Blueprints: the process and the index"; page="blueprints/index" ;;
        *) title=$(sed -n 's/^# //p' "$f" | head -1); page="${f%.md}" ;;
        esac
        if ! summary=$(first_sentence "$f"); then
            echo "gen-llms-txt: no prose paragraph found in $f" >&2
            exit 1
        fi
        # The record's status, as the navigation shows it (shortStatus in
        # site/src/lib/server/blueprints.ts): the header's `- Status:` value,
        # links reduced to their text, backticks and any parenthetical dropped.
        case "$f" in
        blueprints/README.md) status="" ;;
        *)
            status=$(sed -n 's/^- Status:[[:space:]]*//p' "$f" | head -1 |
                sed -e 's/\[\([^]]*\)\]([^)]*)/\1/g' -e 's/`//g' -e 's/[[:space:]]*(.*$//' -e 's/[[:space:]]*$//')
            if [ -z "$status" ]; then
                echo "gen-llms-txt: $f has no - Status: line" >&2
                exit 1
            fi
            ;;
        esac
        if [ -n "$status" ]; then
            printf -- '- [%s](%s/%s.html): Status: %s. %s\n' "$title" "$BASE_URL" "$page" "$status" "$summary"
        else
            printf -- '- [%s](%s/%s.html): %s\n' "$title" "$BASE_URL" "$page" "$summary"
        fi
    done

    # The Lab: evals and experiments, read from lab/.
    printf '\n## Lab\n\n'
    if ! summary=$(first_sentence lab/README.md); then
        echo "gen-llms-txt: no prose paragraph found in lab/README.md" >&2
        exit 1
    fi
    printf -- '- [Lab](%s/lab/index.html): %s\n' "$BASE_URL" "$summary"
} > "$tmp"

mkdir -p "$(dirname "$OUT")"
# cat rather than cp: mktemp creates 0600 and cp would carry that mode onto a
# file the whole world is meant to read.
cat "$tmp" > "$OUT"
echo "gen-llms-txt: wrote $OUT"
