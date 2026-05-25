# Iteration: Docsite IA Restructure

**Started:** 2026-05-25
**Status:** Discourse phase
**Author:** Claude (executor) + yzavyas (director)

This iteration restructures the docsite IA after the data-scientist read surfaced structural misalignment in `concepts/`, the architecture page, the intro, and the absence of a researcher-aligned methodology surface. Builds on the polish-rubric revision (researcher voice added) that landed before this iteration opened.

## Verification stack (per the cross-architecture pattern)

Each output artifact passes through:

1. Claude self-eval (low signal, present)
2. Intra-Claude guild specialists (per assignment — see Agent Assignments below)
3. Cross-architecture judge (`gemini -p` against rubric criteria — Gemini CLI 0.38.2 verified)
4. Human review (steward role: reads decision log + verification trace + approves)

## Decisions

### Decision #1: What is `concepts/` for?

**Question:** What is the `concepts/` section for in the restructured docsite?

**Options considered:**

- H1: NLP conceptual base only (UDPipe, dep parsing, CoNLL-U, POS, formulas, algorithms — pure foundations; implementation lives elsewhere)
- H2: Base + affordances merged (concepts/ holds both NLP ideas AND the "what vaani lets you do" inventory)
- H3: Two sibling sections — concepts/ for the NLP base; new affordances/ for "what vaani offers + will offer"

**Chosen:** H3 — Two sibling sections.

**Rationale:** "What is this NLP idea?" and "What can vaani do for me?" are orthogonal questions. Conflating them is what the current concepts/ does wrong. Reader picks: *understand* vs *do*.

**Verified:** Human (yzavyas) selected H3 via AskUserQuestion on 2026-05-25.
**Cross-arch verification:** Deferred (cross-arch judge runs against the full IA proposal once more decisions land, not against individual decisions).

**Implications cascading from this decision:**

- `concepts/` becomes the field theory under vaani (UDPipe, dep parsing, CoNLL-U, POS, lemmas, readability/TTR/nominalization formulas, TF-IDF/TextRank/RAKE/YAKE algorithms, passive detection; planned: speech-act theory, modality theory, stylometry theory).
- `affordances/` (new — placement TBD: own top-level section, or folded into `usage/`) holds "what vaani offers + will offer" (parse, measure, summarize, extract, HTML report; planned: rules, relations, schemas, modalities, speech-act classification, stylometry signatures).
- Some topics appear in BOTH sections at different angles: `concepts/speech-acts.md` (Austin / Searle / illocutionary force theory) AND `affordances/classify-speech-acts.md` (what vaani will do, API shape). Naming pattern TBD as a follow-up decision.

**Open cascade questions queued:**

- D2: Iteration scope (S / M / L) — next, asked now
- D3: Where do the existing `concepts/` pages go (`pipeline.md`, `domain-types.md`, `errors.md`)?
- D4: Where does `philosophy.md` live?
- D5: Does `affordances/` become its own top-level section, or fold into `usage/`?
- D6: Naming pattern when a topic has both a concept-page and an affordance-page
- D7: HTML report timing (this iteration / v0.1 / v0.2) and surface naming (`Document::to_html_report()` / `Vaani.report()` / `vaani report`)
- D8: Audience priority for the landing page (builder-primary / equal-weight / researcher-primary)

## Agent assignments (provisional, may revise as decisions land)

| Step | Owner |
|---|---|
| IA structure | `vyasa` (collection architecture) |
| Diagram + visual decisions | `tufte` |
| Source survey before new content | `magellan` (if scope reaches new pages) |
| Per-page content (new pages) | `feynman` (teaching) + `sagan` (memoria where conviction-carrying) |
| Voice cleanup | `orwell` |
| Per-page ship-or-return | `ebert` |
| Cross-arch verification | `gemini -p` invocation against iteration rubric |
| Final review | yzavyas (decision log + verification trace) |

## Escalation criteria (pause execution, surface to human)

To be set with D2 (scope). Provisional candidates:

- Any rename of an existing public page filename (breaks inbound links from main / from downstream consumers if any)
- Any change to SUMMARY.md top-level section structure
- Any new top-level directory under `book/src/`
- Any new dependency added to support the HTML report (e.g., a templating crate)
- Any structural change touching more than N pages without a sub-PR proposal

### Decision #2: Iteration scope

**Chosen:** M-scope — proposal + skeletal restructure. SUMMARY.md updated, pages relocated/renamed, new directories with stubs, floor gates updated. Content rewrites deferred to follow-up iterations.

**Verified:** yzavyas via AskUserQuestion 2026-05-25.

### Discourse phase (Step 2, socrates)

`craft-rhetoric:socrates` produced `.rhet/ground-truth.md` (232 lines) + `.rhet/voice.md` (155 lines). Voice features (12 protect) + voice habits (4 correct) + 3 hard invariants (no em dashes, no marketing register, no internal mox-labs names).

**Cross-arch verification:** `gemini -p` evaluated against 7 criteria. Verdict: **PASS** with no required fixes. Three downstream-action notes captured for vyasa (stub quality risk, philosophy.md migration risk, Diátaxis verbatim vs product voice).

### Cartography phase (Step 3, magellan)

`craft-rhetoric:magellan` surveyed three domains (existing vaani pages, 7 Rust OSS exemplars, 13 NLP/data-science/Diátaxis-implementer sources). Produced 6 files in `.rhet/map/` (MOC, SOURCES, four cluster files). Total: 1,207 lines.

**Cross-arch verification:** `gemini -p` evaluated against 7 criteria. Verdict: **PASS-WITH-CONCERNS**. Four corrections surfaced:

1. Methodology-page assumption challenged (gemini: researchers want citation-style, not pedagogical methodology pages, mirroring sentence-transformers)
2. Conviction-on-landing distinction (gemini: tagline stays on landing; full conviction.md page moves off — magellan conflated these)
3. Rust vs Python ecosystem tension (how much reference goes in mdbook vs delegates to docs.rs) — glossed over
4. Search/entry-point behavior missing — IA focused only on top-down nav

One of magellan's three escalations de-escalated by gemini: the broken `reference/roadmap.md` link in conviction.md is over-cautious for human review; vyasa fixes autonomously.

### Decisions #3 through #8 (resolved together via AskUserQuestion, 2026-05-25)

| # | Question | Chosen |
|---|---|---|
| D3 | Where do existing `concepts/` pages go? | Split: `pipeline.md` → `architecture/`; `domain-types.md` → `reference/` (corrected 2026-05-25 per ebert's catch + yzavyas confirmation; was a typo in my original D3 proposal — magellan's A-009 empirical classification was the right call); `errors.md` → `reference/` |
| D4 | Where does `philosophy.md` live? | **DELETED from book/src/.** Content lives in CLAUDE.md + `.claude/skills/aces/SKILL.md` + `.claude/skills/resilience-floor/SKILL.md`. |
| D5 | `affordances/` as own section or fold? | Fold into `concepts/affordances.md` (single explanatory page). |
| D6 | Section aliases vs verbatim Diátaxis | Confirm magellan's aliases: `tutorials/` + `guides/` (how-to) + `concepts/` (NLP base + affordances) + `architecture/` (system explanation) + `reference/`. Splits Diátaxis Explanation into concepts/ (NLP domain) + architecture/ (software domain). |
| D7 | HTML report timing & naming | Stub now, lock names. `Document::to_html_report()` (Rust) / `Vaani.report(text, format="html")` (Python) / `vaani report essay.md --format html` (CLI). Marked 🛠️ v0.1. |
| D8 | Methodology surface — page vs citation-style | **Full `reference/methodology.md` page.** User chose against gemini's citation-style pushback. Methodology page serves the "I want to understand the math vaani uses" case explicitly. (Citation requirement still satisfied via CITATION.cff at repo root.) |

### Decision #9 (autonomous to vyasa, per gemini pushback)

Broken `reference/roadmap.md` link in conviction.md — vyasa resolves autonomously (redirect to `architecture/future-direction.md` or stub the page) without human escalation.

### Decision #10 (instruction to vyasa, per gemini pushback)

Landing page (`introduction.md`) keeps a **conviction tagline** before the code block. The full `conviction.md` manifesto moves off the critical path (into `architecture/conviction.md` or `concepts/conviction.md`). Distinguish: conviction-tagline-on-landing (acceptable, exemplar pattern in serde / tokio / HF Transformers) from conviction-led-page-on-landing (the actual problem).

## Iteration rubric (per cross-arch + discourse, to apply at ebert)

Extends `.claude/rhetoric/polish-rubric/SYNTHESIS.md` with iteration-specific criteria:

- **IR1.** SUMMARY.md top-level structure matches D6 aliases verbatim.
- **IR2.** Every existing page is either (a) relocated to a new path documented in the proposal, (b) stubbed at the new path, or (c) explicitly marked for deletion. No silent drops.
- **IR3.** New directories include placeholder stubs with 🛠️ markers and one-paragraph algorithmic summaries (per cross-arch's flagged risk on "stub quality").
- **IR4.** Landing page (introduction.md) keeps conviction tagline (per D10); full conviction page moves to architecture/ or concepts/.
- **IR5.** HTML report stub at `concepts/affordances.md` or `reference/html-report.md` uses the locked surface naming (D7).
- **IR6.** philosophy.md deleted; no broken inbound links remain (floor gate 1 confirms).
- **IR7.** Floor gate updates: orphan-detect rule against new directory structure; type-name parity gate unchanged (covered by ADR-0006).
- **IR8.** No em dashes in any prose; no marketing register; no internal mox-labs names — voice.md invariants hold.

## Escalation criteria (finalized for execution phase)

Pause execution and surface to yzavyas if:

- Any new dependency must be added to support the restructure
- Any rename of a public page filename beyond the D3-D6 set decided here
- Any structural change to SUMMARY.md beyond the D6 aliases
- Any deletion of content the discourse phase didn't authorize
- Floor gate 3 (type-name parity) fails — would indicate a deeper drift than the IA restructure

Otherwise: vyasa proceeds autonomously, fills the decision log as work happens, gemini cross-arch verifies vyasa's output, ebert ship-or-returns.

## Pipeline trace (final, 2026-05-25)

| Step | Agent | Output | Cross-arch verdict | Status |
|---|---|---|---|---|
| Setup | bash script | `.rhet/` workspace + .gitignore | n/a | DONE |
| Discourse | `craft-rhetoric:socrates` | `.rhet/ground-truth.md` (232 lines), `.rhet/voice.md` (155 lines) | gemini PASS | DONE |
| Cartography | `craft-rhetoric:magellan` | `.rhet/map/` (6 files, 1207 lines): MOC + SOURCES + 4 cluster files (Domain A: existing vaani; Domain B: 7 Rust OSS exemplars; Domain C: 13 NLP/Diátaxis sources; IA synthesis) | gemini PASS-WITH-CONCERNS (4 corrections: methodology-page assumption challenged, conviction tagline vs page distinction, Rust vs Python ecosystem tension, search/entry-point omission) | DONE |
| Dispositio | `craft-rhetoric:vyasa` | `.rhet/arrangement/` (13 files): ia-proposal, new-SUMMARY, migration-manifest, 10 stubs | gemini PASS-WITH-CONCERNS (3 issues: web-rendered dead link from .claude/skills/, README/scripts/workflow path-rot risk, voice.md em-dash violation in vyasa's own artifacts) | DONE |
| Voice check | `craft-rhetoric:orwell` | 47 em-dash fixes in ia-proposal.md (46) and migration-manifest.md (1); stubs were clean | n/a (orwell is the voice gate) | DONE |
| Orchestrator overrides | (me) | Updated migration-manifest.md: contributing/ links repoint to architecture/conviction.md (not .claude/); docs/collaboration-model.md links updated; repo-level path-rot scan confirmed clean for README/scripts/workflows | n/a | DONE |
| Critique | `craft-rhetoric:ebert` | Verdict: **SHIP WITH CORRECTIONS** (3 corrections: D3 discrepancy, methodology.md missing "Planned" section, CITATION.cff forward marker) | n/a (ebert is the final gate) | DONE |
| Correction 1 (D3 discrepancy) | (me, with yzavyas) | Confirmed: domain-types → reference/ (typo in original D3 proposal; magellan's empirical classification + vyasa's call was correct); iteration log corrected | n/a | DONE |
| Correction 2 (methodology "Planned" section) | (me) | Added "Planned for this page" section with five paper citations + CITATION.cff forward marker to `.rhet/arrangement/stubs/reference/methodology.md` | n/a | DONE |
| Correction 3 (CITATION.cff forward marker) | (me) | Covered by Correction 2 (added to the same Planned section) | n/a | DONE |

## Execution phase (next, awaiting yzavyas authorization)

The orchestrator (me) executes the migration manifest in a single PR:

1. Apply all file moves (21 existing pages relocated/renamed/deleted per the disposition table)
2. Drop the 10 stubs at their new paths
3. Apply the link-update table (14 inbound link updates across book/src/ + docs/)
4. Delete the empty disbanded directories (`book/src/getting-started/`, `usage/`, `extending/`, `understand/`)
5. Replace `book/src/SUMMARY.md` with `.rhet/arrangement/new-SUMMARY.md`
6. Run floor gate suite — gate 1 (lychee) + gate 2 (orphan) + gate 3 (type-name parity) + gate 4 (mdbook clean build) must all PASS
7. Verify zero remaining inbound links to `book/src/philosophy.md`
8. Open PR with full rationale citing this iteration log + ebert's critique report
9. Cross-arch verify the PR diff with `gemini -p` before merge
10. Self-approve + rebase-merge per the established ritual

Pause point: yzavyas authorization required before step 1.
