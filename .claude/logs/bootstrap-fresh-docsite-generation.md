# Bootstrap: docsite generation for matra

Pipeline-driven production of matra's docsite under `book/src/`, batch by batch, applying the per-bucket specialist subsets from the polish rubric and the cross-architecture verification gate at each ship point.

Copy this prompt into a fresh `/clear`-ed session and begin from "First task" at the bottom.

---

## Project

matra, an NLP library. This repository. Rust core + Python bindings via PyO3 + planned WASM crust. Branch: `m2-docsite-ia-restructure`. Do not push to remote `main`; alpha is the working branch.

## Repo state for this work

```
book/src/                   placeholder SUMMARY.md only; you fill this in
book/book.toml              create-missing = false (SUMMARY entries must exist on disk)
scripts/check-docsite-floor.sh   the four floor gates; runs in CI and pre-commit
justfile                    `just docs-floor` runs the gate suite

.rhet/                      craft-rhetoric workspace (gitignored)
  ground-truth.md           load-bearing: audience anchors, conviction tagline, misconceptions
  voice.md                  load-bearing: voice features to protect, habits to correct, hard invariants
  map/
    MOC.md                  cartography index
    SOURCES.md              source inventory
    cluster-ia-synthesis.md
    cluster-domain-b-rust-oss.md   exemplar Rust OSS landing patterns
    cluster-domain-c-nlp-diataxis.md   NLP + Diátaxis-implementer evidence
  arrangement/ia-proposal.md   the IA target (which pages exist, where)

.claude/
  arch/                     architecture working docs (read for system context)
  plans/                  iteration plans
  logs/                     iteration logs (this file lives here)
  rhetoric/polish-rubric/SYNTHESIS.md   per-bucket specialist subsets + criteria
```

## Authority order (when surfaces conflict)

code > tests > `.claude/arch/` > `book/src/plans/` > ADRs > CHANGELOG > polish rubric > rhetoric artifacts.

Closer to the running system wins.

## The IA target

Per `.rhet/arrangement/ia-proposal.md` + `.rhet/map/cluster-ia-synthesis.md`:

```
book/src/
  introduction.md       landing
  tutorials/            learning-oriented
    installation.md
    quickstart.md
  guides/               task-oriented (how-to)
    rust.md
    python.md
    cli.md
    new-adapter.md
  concepts/             explanation (NLP foundations)
    affordances.md          capability inventory; first concepts/ entry
    udpipe.md
    dependency-parsing.md
    pos-lemmas.md
    readability.md
    tfidf-textrank.md
    rake-yake.md
    passive-nominalization.md
  architecture/         explanation (system / software)
    conviction.md       the manifesto. Linked from introduction.md
    four-faces.md
    pipeline.md
    hex.md
    ports-adapters.md
    cross-language.md
    future-direction.md
  reference/            information (lookup)
    domain-types.md
    errors.md
    methodology.md      researcher anchor (R1, R3, R5)
    html-report.md      🛠️ Planned v0.1
    boundary-rules.md
    api-reference.md
  playground/
    index.md            honest 🛠️ stub
  contributing/         meta
    how-it-works.md
    dao.md
```

31 pages total. Section aliases (tutorials / guides / concepts / architecture / reference) are chosen over verbatim Diátaxis names per the cartography evidence (zero of 13 surveyed exemplars use verbatim `explanation/`).

## Pipeline workflow per batch

```
1. invoke feynman (inventio)            draft from foundations
                                        ⬇
2. invoke orwell (voice check 1)        em-dashes, marketing register, LLM tells
                                        ⬇
3. invoke sagan (memoria) WHERE the bucket subset includes it
                                        ⬇
4. invoke orwell (voice check 2, after sagan)
                                        ⬇
5. invoke the bucket-specific specialists per the subset below
                                        ⬇
6. invoke ebert (critique)              SHIP / SHIP-WITH-CORRECTIONS / RETURN
                                        ⬇
7. gemini -p cross-architecture verification
                                        ⬇
8. integrate to book/src/<bucket>/
                                        ⬇
9. update book/src/SUMMARY.md
                                        ⬇
10. run floor gates (just docs-floor)
                                        ⬇
11. commit (conventional prefix, Co-Authored-By: yzavyas)
                                        ⬇
12. next batch
```

## Per-bucket specialist subsets (from the polish rubric)

| Bucket | Specialist subset |
|---|---|
| introduction.md (landing) | sagan + jobs + orwell + ebert |
| tutorials/ | feynman + researcher (R1, R2, R4) + jobs + orwell + ebert |
| guides/ (how-to) | jobs + ace + researcher (R4) + orwell + ebert |
| concepts/ | feynman + tufte + researcher (R1, R2, R5) + orwell + ebert |
| architecture/ | tufte + feynman + burner + ebert |
| reference/ | karman + researcher (R3, R5) + ebert |
| contributing/ | karman + chesterton + ebert |

Invoke via Agent calls with `subagent_type: craft-rhetoric:<name>` or `guild-arch:<name>`.

What each catches:

- **tufte** — diagram type selection (mermaid vs ASCII vs SVG), information-structure encoding, data-ink ratio
- **jobs** — pacing, progressive disclosure, beat structure, hand-off discipline
- **ace** — developer experience, API discoverability, first-success path
- **karman** — vocabulary coherence, ontology-vs-code alignment, reserved-name discipline
- **burner** — boundary correctness in architecture (dependency direction, port rules)
- **chesterton** — preservation of load-bearing prior artifacts (the DAO table, the trinity of working values)
- **researcher** — methodology transparency (R1), reproducibility (R2), citation (R3), researcher-shaped output (R4), explicit non-claims (R5)

## Voice invariants (hard, non-negotiable)

Per `.rhet/voice.md`:

1. **No em dashes in prose.** Both `—` (glyph) and `--` (double-hyphen). Mermaid node labels count as prose. Code-fence content depicting actual computed output is exempt.
2. **No marketing register.** "just", "simply", "easy", "powerful", "blazingly", "robust", "production-ready".
3. **No internal product names** in any shipping surface (book/src/, README, CLAUDE.md, CONTRIBUTING.md, docs/collaboration-model.md). Translate to product-agnostic language. The list of forbidden names lives in `.rhet/voice.md`.
4. **Substrate framing preserved.** "matra structures; the interpreter analyzes." "matra measures; your application decides."
5. **Honest status markers.** ✅ ships in v0.0.x; 🛠️ planned vX.Y. Verify against `src/` before claiming ✅.
6. **No LLM tells.** "notably", "however" (sentence-initial), "importantly", "in essence", "ultimately", "it is worth noting", "delve", "tapestry", "leverage" (as verb).

The conviction tagline ("matra illuminates the internal structural makeup of text, enabling effective higher-order reasoning on text") is verbatim from `.rhet/ground-truth.md`. Do not paraphrase.

## Verification + gates

### Floor gates (`just docs-floor`)

All four must pass before each commit:

- **Gate 1 (lychee link integrity)** — skip-with-warning locally; CI runs with `LYCHEE_REQUIRED=1`.
- **Gate 2 (orphan detect)** — every `book/src/*.md` must be referenced in SUMMARY.md.
- **Gate 3 (type-name parity)** — every backtick-inline PascalCase identifier in `book/src/` must resolve in `src/` OR be on one of the three allowlists in `scripts/check-docsite-floor.sh` (external, ud_pos, planned). Extend allowlists when adding legitimate external identifiers or ADR-reserved planned names.
- **Gate 4 (mdbook clean build)** — `book.toml` has `create-missing = false`; SUMMARY entries must reference files that exist on disk.

### Cross-architecture verification (`gemini -p`)

Gemini CLI 0.38.2 is installed and authenticated. Use after each batch passes ebert:

```bash
gemini -p "$(cat <<'EOF'
[final drafts of the batch]
[evaluation criteria: propagation, three doors, evidence, voice invariants, bucket-specific criteria]
[ask for cross-architecture independent verdict]
EOF
)" 2>&1 | tail -100
```

Pattern: feed gemini the batch's final drafts + the relevant rubric criteria, ask for PASS / PASS-WITH-CONCERNS / RETURN-FOR-FIX with per-criterion findings. Gemini's different model family catches what the Claude-side specialists' shared training cannot.

### Ebert ship gate

Per the rhetoric protocol's `Gate: critique → ship`:

- Propagation test across the audiences relevant to the bucket
- Three Doors traversal (Universal / Constituency / Self) + at least one dimensional shift
- Evidence verification (every fact traces to src/)
- No "uncertain" presented as "solid"
- Comprehension test on final output
- Verdict: SHIP / SHIP-WITH-CORRECTIONS / RETURN

Max 2 returns per step per the protocol. After that, escalate to yzavyas.

## Commit + PR ritual

1. Atomic commits on `m2-docsite-ia-restructure` branch
2. Conventional commit prefix (`docs(book):`)
3. `Co-Authored-By:` trailer matching the address in `Cargo.toml`
4. Local pre-commit hook runs floor gates automatically; the hook is the gate
5. After all batches ship: `gh pr create --base alpha`, comment with rationale (the audit trail since GitHub blocks self-approval on solo projects), then `gh pr merge --rebase --delete-branch` after explicit user authorization
6. Never push to `main`. Alpha branch is the working surface pre-0.1.0.

## Pause points (when to ask yzavyas)

- Before opening the PR
- Before merging the PR
- If ebert returns a second time on the same batch
- If a structural change is needed that wasn't authorized in the IA proposal
- If a new dependency is added
- If you find an inconsistency between the IA target and the cartography evidence

Do not pause for routine progress reporting. Surface progress at batch boundaries only.

## Artifacts to USE (foundational)

- `.rhet/ground-truth.md` — tagline, audience anchors, misconceptions
- `.rhet/voice.md` — voice invariants
- `.rhet/map/MOC.md` + cluster files — cartography, exemplar OSS evidence
- `.rhet/arrangement/ia-proposal.md` — structural target (pages and their homes)
- `.claude/rhetoric/polish-rubric/SYNTHESIS.md` — per-bucket voice subsets + criteria
- `CLAUDE.md` — project posture, conventions, things-that-will-bite-you, boundary rules
- `CONTRIBUTING.md` — working values, iteration model, decision flow
- `docs/collaboration-model.md` — the exemplar working-model document
- `docs/decisions/*.md` — ADRs; ADR-0006 (abstract-tier vocabulary lock) is load-bearing for reserved names
- Source code: `src/*.rs`, `python/matra/*.py`. Verify every code claim against these.

## Per-bucket batch order (suggested)

Dependencies between buckets are minimal (each bucket cross-links forward-defined paths to others). Suggested order, by audience priority and structural dependency:

1. introduction.md (landing — entry surface)
2. tutorials/ (builder cold-arrival path; first-success)
3. concepts/ (NLP foundations; data-scientist + researcher anchor)
4. reference/ (lookup; researcher methodology surface)
5. architecture/ (system explanation; receives links from intro + concepts)
6. guides/ (how-to; receives links from tutorials)
7. contributing/ (meta; least-trafficked)

Each batch is an isolated pipeline cycle. Commit per batch.

## First task

Invoke `craft-rhetoric:feynman` for **introduction.md** with a clean-room brief.

Inputs the agent reads:
- `.rhet/ground-truth.md`
- `.rhet/voice.md`
- `.rhet/map/cluster-ia-synthesis.md`
- `.rhet/map/cluster-domain-b-rust-oss.md` (exemplar landing patterns)
- `.rhet/arrangement/ia-proposal.md` (for structural cross-references)
- `src/lib.rs`, `src/domain.rs`, `src/nlp/udpipe.rs` (for verified capability claims)
- `python/matra/__init__.py`, `python/matra/_core.pyi` (Python surface)
- `Cargo.toml`, `pyproject.toml` (versions + MSRV)

Output:
- `.rhet/inventio/v2/introduction.md` (use `v2/` to keep the workspace clean)
- `.rhet/inventio/v2/introduction-trace.md` (comprehension trace)

Constraints in the brief:
- Code-first (install + working example within the first 30 rendered lines)
- Conviction tagline verbatim from ground-truth (line 3)
- ✅/🛠️ status markers, each capability verifiable against src/
- All voice invariants
- Honest depiction of matra's actual output: matra returns `Document` (Rust) / dict (Python); the CLI prints a metrics table; an HTML report is planned v0.1 but does not ship in v0.0.x — do not depict output that does not exist
- Diagram for the structural example uses mermaid (a diagram of structure), not ASCII output (which would falsely imply CLI output that does not exist)
- Cross-architecture verification (gemini -p) is the final gate before integration

After feynman's draft lands, continue per the pipeline: orwell → sagan → orwell → jobs → orwell → ebert → gemini cross-arch → integrate → commit.

---

**End of bootstrap. Begin with the feynman invocation for introduction.md.**
