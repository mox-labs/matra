# EP-0013: The docsite's identity, drawn from matra's own output

- EP: EP-0013
- Implements: none (standalone: the docsite changes how matra is documented, not matra)
- Status: in progress
- Shipped in: not shipped (the docsite deploys from `main`, not with a release)

## Summary

Give the docsite an identity that could not belong to any other project,
because it is made of matra's own output: a mark that is matra's parse of
its motto, pages that measure their own paragraphs in the margin, and
figures a reader can scrub and point into. The tokens and type are the
ratified mox design system's. One pull request delivers it.

## Design

### The brief, and where its authority comes from

The owner's brief (2026-09-25) found that the site "reads as the mean of its
category", which the mox design system's first law forbids ("refuse the mean
of the training distribution": every artifact should refuse to look like the
mean of its category), and asked for an identity where every choice is
derived, not decorative. Two amendments followed the same day:

1. Drop the mox branding items (a studio credit, a motto line in the footer,
   a separate trichrome easter egg). Everything else stands.
2. Add the maker's mark exactly once: one circle with three straight arms,
   nothing else, carved rather than lit, in the site's own ink, set on a form
   the site already has, labelled and unlinked. The owner's ruling on another
   of their works (a glowing green mark there was "too in the face") set the
   direction: not green, a carving, and carved marks do not glow.

A third amendment pointed the figures at the studio's motion research
(motion-for-data-graphics, findings F1 to F10, audited 2026-09-21): direct
manipulation within a frame, linked highlighting across figure and twin,
transitions only between states the reader caused, interruptible, opacity
only, and one predict-then-reveal prompt, on the semantic clusters guide.

The tokens are the mox design system's ratified token file (ratified
2026-06-18), copied verbatim to `site/src/lib/brand/mox.tokens.css` with its
SHA-256 (`750911b5…`) in the header, so a holder of the source can check the
copy. The canon's rules the site applies: trichrome by role (Spark,
Temperance, Emergence), never load-bearing in colour alone, the 9-grid, depth
with the 240° trace and never pure black, square corners, Alegreya for the
read at 19px / 1.6 / 66ch, IBM Plex Mono for the apparatus, IBM Plex Sans for
labels, and "mark, don't fill", "margins are content", "mono earns its
place".

### The decisions

- **The mark is matra's parse of "Amplify radical nonconformity.".** Every
  token hangs from a headline bar (the shirorekha); the root hangs longest,
  in Emergence, as a mātrā, the vowel sign that changes a letter without
  replacing it; each dependency hangs below as an arc. It is generated at
  build time from committed figure data that gate 8 regenerates, so it
  changes only when matra's parse does. Its variants, the favicon's
  reduction rule, and its clear-space and minimum-size rules are in
  `site/README.md` and on `/mark`.
- **Every page measures itself.** Each page's Markdown goes through the
  pipeline, and each prose paragraph's grade, lexical density and compression
  sit in the margin beside it. Numbers are matched to paragraphs by text, not
  trusted by order; a page that cannot be matched shows none. The cost is
  stated: any page edit now needs `just docs-figures`.
- **The void is the default.** The system is dark-first; paper is the
  reader's choice on the toggle. The site no longer follows the OS scheme.
- **Colour only by role.** Spark for links and what the reader touches,
  Emergence for code and converged results, Temperance for errors. Every
  figure's hues came down to role plus neutral, with line styles, glyphs,
  letters or names carrying what colour used to.
- **The maker's mark hangs from the footer's rule.** The header's rule
  already carries matra's own mark; the foot of the page is where a maker
  signs. A stroke in the muted ink, labelled "maker's mark", unlinked.

## Goals

- An identity derived from matra's output, with the derivation written down.
- WCAG AA contrast in both themes, checked mechanically.
- Every figure interactive within a frame, and readable with scripts off.

## Non-goals

- Changing any page's content beyond what the identity needs (the home page's
  opening line, the predict-then-reveal prompt).
- A theme per surface: one identity, two grounds.

## Iterations and milestones

### M1: the identity

- **Deliverable.** The mark and its variants; the tokens, type and colour
  roles; the shirorekha chrome and the maker's mark; the self-measuring
  margin; the home page's hero; figures recoloured, linked to their twins,
  and the clusters threshold as a scrub.
- **Exit criterion.** All eleven docsite gates pass, with the contrast check
  in gate 4 and the margin notes in gate 9; no page scrolls sideways at 360,
  390 or 1280px; everything renders with scripts off; the scrub holds 60fps
  on a throttled CPU.

## Test plan

- `scripts/check-contrast.ts` (gate 4) on every text and mark pair, both
  themes; seen to fail on a planted light Spark.
- Gate 8 on the page measures and the motto's parse; seen to fail on a hand
  edit of the parse.
- Gate 9 on every margin note's text against its data; seen to fail on a
  changed number.
- Interaction and frame timing in a browser, recorded in the pull request.

## Ship criteria

Merged, deployed, and the live check after deploy passes.

## Risks

- **Docs edits need the models.** Gate 8 now guards every page, so a typo fix
  needs `just docs-figures`, which needs the UDPipe model. Response: the gate
  says which command to run.
- **A page whose paragraphs cannot be matched loses its margin.** Response:
  the build names the page; none do today.

## Status log

- 2026-09-25: in progress. M1 delivered in one pull request.
