# Docsite Polish — Iteration Plan

**Branch:** `alpha` (NOT main; do not push to main until 0.1.0 is ready to release)
**Cargo version:** 0.0.1 (pre-0.1.0 alpha cycle)
**Started:** 2026-05-24

## Framing

The governance rubric (`.claude/rhetoric/rubric/SYNTHESIS.md`) defines WHAT good documentation looks like structurally. The polish rubric (`.claude/rhetoric/polish-rubric/`) defines HOW each page achieves quality per-page through craft. This plan sequences the polish work into shippable milestones.

Three audiences served:
1. Prospective users / consumers (evaluating fit)
2. Contributors (joining or extending)
3. The human-AI collaboration itself (the docsite as situational-awareness mirror)

Three states the docsite oscillates between:
- **Current** — what mirrors the released code (whatever's on `main`)
- **Next / alpha** — what we're building (this branch reflects the alpha cycle pre-0.1.0)
- **Speculative** — explicitly marked 🛠️ planned for future releases

## Milestone M0 — FLOOR (in progress)

**Goal:** docsite + playground stub UP, internally consistent, builds clean.

| Item | Status |
|---|---|
| Fix 3 broken forward-links in introduction.md | ✅ Done |
| Add `understand/` (conviction + four-faces) to SUMMARY.md | ✅ Done |
| Create `playground/index.md` honest stub | ✅ Done |
| book.toml mdbook 0.5.3 compat | ✅ Done |
| mdbook-mermaid wired (`additional-js` in book.toml) | ✅ Done |
| Cargo version reflects alpha state (0.0.1) | ✅ Done |
| Restructure introduction.md (H1=vaani, tagline as blockquote, text-based before/after) | ✅ Done |
| `scripts/check-docsite-floor.sh` — 4 floor gates (lychee link check, orphan detect, type-name vs src, mdbook --warning-policy fail) | ⏸ Pending |
| Wire floor gates into `just check` | ⏸ Pending |
| Commit M0 to alpha + push remote | ⏸ Pending (awaiting authorization) |

**Exit criteria:** all 4 floor gates pass; user authorizes commit + push.

## Milestone M1 — Page-by-page polish (existing 22 pages)

**Goal:** every existing book/src/ page passes the polish rubric.

**Approach:** rubric-driven iteration. For each page, apply the relevant rubric subset:

| Page | Primary rubric | Voice (agent) |
|---|---|---|
| `introduction.md` | Sagan (landing memoria) + Jobs (pacing) + Orwell (voice) + Ebert (gate) | sagan → orwell → ebert |
| `understand/conviction.md` | Feynman (teaching) + Sagan (memoria) + Orwell (voice) + Ebert (gate) | feynman → sagan → orwell → ebert |
| `understand/four-faces.md` | Feynman (teaching) + Orwell (voice) + Ebert (gate) | feynman → orwell → ebert |
| `concepts/pipeline.md` | Feynman (teaching) + Tufte (pipeline diagram embed) + Orwell + Ebert | feynman → tufte → orwell → ebert |
| `concepts/domain-types.md` | Feynman + Tufte (hierarchy diagram) + Orwell + Ebert | similar |
| `concepts/errors.md` | Reference register; Karman (vocabulary coherence) + Ebert | karman → ebert |
| `architecture/hex.md` | Tufte (hex diagram) + Feynman + Burner (boundary correctness) + Ebert | tufte → feynman → burner → ebert |
| `architecture/ports-adapters.md` | Feynman + Burner + Ebert | similar |
| `architecture/boundary-rules.md` | Burner (canonical location per SSoT) + Ebert | burner → ebert |
| `architecture/cross-language.md` | Burner (methods-don't-cross-FFI canonical) + ffi-keeper (if needed) + Ebert | burner → ebert |
| `usage/rust.md` | Ace (DX) + Karman (vocab parity) + Ebert | ace → karman → ebert |
| `usage/python.md` | Ace + Karman + Ebert | similar |
| `usage/cli.md` | Ace + Ebert | ace → ebert |
| `extending/new-adapter.md` | Feynman (tutorial-shaped) + Burner + Ebert | feynman → burner → ebert |
| `extending/future-direction.md` | Becomes `reference/roadmap.md` per vyasa; Karman + Ebert | karman → ebert |
| `contributing/how-it-works.md` | Karman (coherence with CONTRIBUTING.md) + Ebert | karman → ebert |
| `contributing/dao.md` | Chesterton (DAO table sync with .claude/agents/) + Ebert | chesterton → ebert |
| `philosophy.md` | Sagan + Orwell + Ebert | sagan → orwell → ebert |
| `api-reference.md` | K (auto-derived; zero pub/fn/struct tokens) + Ebert | k → ebert |
| `getting-started/installation.md` | Ace (first-success path) + Ebert | ace → ebert |
| `getting-started/quickstart.md` | Ace (time-to-first-parse ≤5min) + Feynman (worked-example) + Ebert | ace → feynman → ebert |
| `playground/index.md` | Sagan (conviction-anchor) + Jobs (pre-WASM honesty) + Ebert | sagan → jobs → ebert |

**Sequencing within M1:**
- M1.a — landing + conviction-carrying pages (introduction.md, conviction.md, four-faces.md, philosophy.md). Highest leverage.
- M1.b — concepts + architecture. The reference-tier pages.
- M1.c — usage + getting-started. The first-success path.
- M1.d — extending + contributing. The contributor-tier pages.
- M1.e — reference/api-reference, roadmap consolidation.

**Exit criteria:** every page passes Ebert SHIP gate. Polish rubric scores recorded per page.

## Milestone M2 — New pages

**Goal:** fill the gaps the existing 22 pages don't cover.

| Page | Source agent | Notes |
|---|---|---|
| `tutorials/first-parse.md` (T1) | Feynman | 10-minute first-parse walkthrough; worked-example fading |
| `tutorials/document-to-summary.md` (T2) | Feynman | Markdown → sections → summary; TF-IDF vs TextRank by feel |
| `tutorials/keyphrases.md` (T3) | Feynman | RAKE vs YAKE, when each fits |
| `tutorials/bring-your-own-nlp.md` (T4) | Feynman + Burner | Advanced; implement NlpProvider, swap UDPipe |
| `how-to/use-from-python.md` | Ace | Beyond the lookup page |
| `how-to/analyze-corpus.md` | Ace + K (efficiency) | Large corpus handling |
| `how-to/handle-model-download.md` | Resilience | Custom paths, offline mirrors |
| `how-to/integrate-ci.md` | Ace | CI usage |
| `how-to/interpret-metrics.md` | Feynman | What each score means |
| `reference/roadmap.md` | Karman + K | The roadmap page (per vyasa) |
| `understand/collaboration-model.md` | Sagan + Feynman | The exemplar document (also `docs/collaboration-model.md` repo-root) |

**Exit criteria:** every new page passes Ebert SHIP gate. Coverage of Diátaxis quadrants verified by Ace's 3.1 predicate.

## Milestone M3 — Constitution refresh

**Goal:** CLAUDE.md split + CONTRIBUTING.md refresh + new collaboration-model.md.

Per `.claude/rhetoric/rubric/CONSTITUTION-PROPOSAL.md`:

1. CLAUDE.md → strictly AI-agent operational manual (preserve gotchas verbatim per Chesterton)
2. CONTRIBUTING.md → human-or-AI contributor manual (preserve trinity per Chesterton)
3. `docs/collaboration-model.md` (NEW) → the exemplar document
4. README.md → ≤200 lines per Burner C4
5. Update `book/src/contributing/` pages to reflect audience split

**Exit criteria:** Chesterton preservation rubric passes (every removed/reworded section cites authorizing change); Burner boundary rubric passes (each claim has one owner).

## Milestone M4 — CI hardening

**Goal:** mechanical predicates for the entire rubric.

1. `scripts/check-docsite.sh` — Ace's 23 DX criteria
2. `scripts/check-doc-boundaries.sh` — Burner's 14 boundary criteria
3. `scripts/check-docsite-ontology.sh` — Karman's 15 vocabulary criteria
4. `scripts/check-rename-complete.sh` — Ixian's cascade-completeness
5. `scripts/docs-stale-report.sh` — Taleb C14 monthly stale-page report
6. CODEOWNERS protection on `SUMMARY.md`, `book.toml`, `.github/workflows/docs.yml`
7. Wire all into `just check` + GH Pages deploy

**Exit criteria:** all 4 sister scripts pass on the alpha tree; CI workflow merged.

## Milestone M5 — Phase 1 rename (Analysis → Document)

**Goal:** execute the ontology guild's Phase 1.

Per `.claude/rhetoric/ontology-synthesis.md`:
1. `Analysis` → `Document` across ~50 files
2. ADR-0006 vocabulary lock
3. Rustdoc deprecation note on `in_blockquote`
4. CHANGELOG `[Unreleased]` Highlight
5. `pub type Analysis = Document;` for one cycle

**Stress-test for M0/M4 floor gates** — if they hold through the rename, the docsite is antifragile.

**Exit criteria:** Ixian validation criteria (`.claude/rhetoric/ixian-validation-criteria.md`) all pass; 72-hour rollback window with no triggers fired.

## Milestone M6 — WASM crust steps 3+4 + playground live

**Goal:** the playground stub goes live.

1. WASM-B step 3 — wasm-bindgen surface mirroring PyO3
2. WASM-B step 4 — IndexedDB caching + SHA-256 verify
3. Playground panels wired to live WASM artifact
4. Cross-language consistency tests (Ace 7.1, 7.2) activate
5. npm package skeleton

**Exit criteria:** playground first-paint ≤1500ms, first-result ≤3000ms (Ace 2.4); live demo works in 3 browsers.

## Milestone M7 — First alpha publish (0.0.1)

**Goal:** ship the first public alpha.

- `cargo publish --dry-run` then `cargo publish` (with explicit per-publish approval per memory)
- `maturin publish --dry-run` then `maturin publish` (same)
- npm publish (if WASM crust ready) — same discipline
- GH Pages deploy from alpha branch
- Tag `v0.0.1`

**Exit criteria:** all three crusts available; docsite live at public URL; first external user can install and run.

## Iteration unit within a milestone

Per project CONTRIBUTING.md discipline:
- One short-lived branch off alpha per logical change
- Atomic commits
- Single PR with rationale + rubric self-check
- Approve-and-merge ritual; delete branch
- Each iteration ships code AND updates the docsite to mirror it

For docsite polish iterations specifically:
- `alpha/polish/introduction-v2` (M1.a iteration)
- `alpha/polish/conviction-page` (M1.a iteration)
- `alpha/m2/tutorial-first-parse` (M2 iteration)
- etc.

## Sequencing summary

```
M0 — FLOOR              (in progress; mostly done)
   ↓
M1 — Page polish        (highest leverage; 22 pages)
   ↓
M4 — CI hardening       (mechanically locks the rubric)
   ↓
M2 — New pages          (tutorials + how-tos)
   ↓
M3 — Constitution       (CLAUDE.md split + collaboration-model.md)
   ↓
M5 — Phase 1 rename     (stress test the floor gates)
   ↓
M6 — WASM + playground  (live playground)
   ↓
M7 — First alpha publish (0.0.1 → public)
```

**M0 + M1.a are this session's natural exit.** M1.b onward begins next session.

## Per-session discipline (for the rolling work)

Each session:
1. Read `.claude/scratch/session-handoff.md` first
2. Identify the active milestone + iteration
3. Run the relevant rubric agents (or apply rubric directly)
4. Polish the page(s) in scope
5. Run the floor gates (when M0 ships them) — fail fast
6. Update session-handoff.md before context-managed exit
