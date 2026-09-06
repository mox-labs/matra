# Session resume

Run the OODA loop below at the start of every session in this directory. Then act.

---

## State (2026-09-04, evening)

- **0.1.0 is published everywhere.** crates.io (maintainer-run `cargo publish`, 2026-08-23), PyPI (2026-09-04, all four artifacts), GitHub release v0.1.0. crates.io Trusted Publishing is configured (`publish.yml`, environment `crates-io`, required reviewers) and the maintainer was advised to enable require-trusted-publishing and revoke the bootstrap tokens.
- **Repo:** `mox-labs/matra`. Branch protection on `main`: 13 required checks, `enforce_admins`, linear history. Everything lands via PR, including this file.
- **The docsite is live** at mox-labs.github.io/matra (book + rustdoc under `/api/`) after fixing a never-deployed docs workflow (mdbook 0.5.3 for edition 2024; PR #46) and a consumer-docs comprehension pass (install paths for the published packages, I7 primitives on the capabilities page; PRs #47/#48).
- **Claude review CI works end to end** (PR #45 granted the posting channel: the two comment MCP tools plus track_progress). Its first day caught a design contradiction, a wrong Rust fact, an ADR-lockstep gap, a shipped-behavior error, and a merge-order dependency. Treat it as a real gate.
- **The i9 plan is merged** (`book/src/plans/i9-embeddings-adapter.md`): Embedder port, static-first adapter (model2vec-format loader over safetensors + tokenizers unstable_wasm, bit-parity across crusts as a tested property), candle BERT later behind the same port, `semantic_clusters` as proving consumer. Grounded by an internal survey, not in this repository (2026-09-04, three-lane landscape survey).
- **ROADMAP carries the LLM-audit program:** redundancy entry extended (CR-POS, syntactic template rate, span recurrence, slop-paper negative result, threshold-spread citation) and a new information-density entry (CPIDR-from-xpos English-only caveat, DEPID-equivalent with UD mapping, contested rules routed to an ADR). Both triggers fired.
- **Branch:** `main`, clean.

### Where the work is

**I7 is shipped, all five milestones.** Structural primitives cross FFI as fields per ADR-0008 (derivations cross as serde-visible data computed once at a pipeline choke point; views over data already crossing stay methods, ADR-0009's `Token::feat` being the instance). `Sentence` now carries `negations`, `modals`, `bare_assertion`, `reportings`, `root_adverbials` and `hearst_pairs`; `Document.passive_ratio` is a materialized slot and `python/matra/cli.py` reads it instead of re-deriving passive detection. `spec/tests/` fixtures (negation, modal, modal-coordination, evidentiality, hearst) pin every crossing primitive across the Rust and Python crusts. The ROADMAP rule-evaluation entry records what the five revealed about the shape `Rule` and `Predicate` must take (arcs by relation and lemma, feats lookups at tree positions, multi-arc constructions with optional participants, caller-supplied lexicons for open classes, span pairs with token-id provenance). The plan carries a shipped banner and stays as the milestone record.

**I8 is shipped, all eight milestones.** The six entry points are gone. The surface is `Ingest` (text/path constructors; a string is a stream of one) into `Engine` (`analyze`, `analyze_one`, `annotate`, `compose`). `annotate` is the only route from text to the parser, so the 8 MiB cap holds pipeline-wide; seven equivalence laws (L1 to L7) run as tests in `src/lib.rs`. ADR-0007 records the decision and supersedes ADR-0002; the vocabulary is `ingest -> decompose -> compose` with `abstract` reserved as the empty seam for rule evaluation. The I8 plan (`book/src/plans/i8-pipeline-surface.md`) carries a shipped banner and stays as the defect record.

Documentation is in lockstep: book pages, README, CHANGELOG, CLAUDE.md, the agent/skill files, the ADR index, evolution.md. I5 is retired (I8 subsumed Tasks A through D; Task E, `pub mod prelude`, waits for the 0.1.0 release pass).

**A domain survey landed 2026-08-21** (an internal survey, not in this repository). Its consequence for matra is a scoping principle now recorded on the roadmap: matra stays at the deterministic, verifiable tier; pragmatic enrichment and claim/fallacy certification stay out with the gap recorded, and coreference is specialist-adapter work above UDPipe, not a parse extension.

### Known traps, carried forward

- `cargo test --all-features` **fails to link.** The `python` feature builds against libpython with symbols deliberately left undefined. Not a regression; never make it a gate.
- After any Rust change, `maturin develop` must run before `python/tests/` tests the new internals; the installed wheel does not rebuild itself. Verified post-I8: all 9 Python tests pass against the rebuilt wheel, including the parametrized size-cap suite (extraction methods now gate at 8 MiB and decompose as plain text).
- UDPipe splits `Smith et al. reported` at the period in `et al.`; every sentence-scoped primitive inherits this.
- `vocabulary_ttr` is a raw type-token ratio, not comparable across document lengths.
- Floor gate 1 (`lychee`) runs without `--include-fragments`, so anchors are never checked.

---

## Next actions, in order

Sequence agreed with the maintainer 2026-08-21, publish and plan gates cleared 2026-09-04:

1. **i9 implementation, M1 through M6, per the merged plan.** M1 (names + ADR-0010) is underway: a Karman naming review of Embedder / Embedding / SemanticClusters / `embeddings` / `embed/static_model.rs` was dispatched 2026-09-04 evening; the ADR waits on its verdict. Then M2 (domain carrier + port), M3 (static adapter, pins verified live, parity fixture, wasm32 gate in CI), M4 (semantic_clusters, modelless tests), M5 (wiring + Python + shape fixture), M6 (conformance + docs lockstep).
2. **Redundancy + density families under one ADR.** The deterministic redundancy family (now nine outputs on the roadmap) and the information-density family (DEPID-equivalent, CPIDR-derived, tree-walk measures) design against I7's primitives; the ADR settles metric-family-vs-extractor-vs-rule-pack, the DEPID contested rules, and the non-English xpos story. The semantic half rides i9's adapter.
3. **TS package decision.** Everything but parse compiles to WASM today, and i9's static adapter is wasm-verified at the dependency level. Options unchanged (types-and-helpers, everything-but-parse crust, full crust on a WASM provider).
4. **Rule vocabulary design** (Rule / Predicate / Finding) against the shape recorded on the roadmap. x.uma composes as a peer consumer, never a dependency.
5. **Maintainer-side, still open:** enable require-trusted-publishing on crates.io and revoke the bootstrap tokens (advised 2026-09-04); the post-ship loop-closure file `scratch/post-ship-0.1.0.md` is due by ~2026-09-18 per the plans README.


## Open, not blocking

- **`Error` is neither `Serialize` nor `Clone`.** `DocumentError`/`CorpusResult` therefore stay Rust-side; crossing to Python needs a projection with stable kind strings. Documented in the book (errors page). Needed before any Python corpus surface.
- The docsite is gated but not voiced: it has not had a voice/editing pass.
- `rumi-nlp` is named in the published plans. Already public via ADR-0003, but worth deciding before release.
- Floor gate 1 should gain `--include-fragments`. I5 Task E (`prelude`) waits for the 0.1.0 pass.

---

## OODA

### Observe

Before any action, gather state:

```bash
git status --porcelain && git log --oneline -8
cargo test && cargo test --features cli && cargo check --no-default-features
just docs-floor
```

### Orient

Read this file's State section, then `CLAUDE.md` for the boundary rules and the gotchas.

If a plan is in flight, read it under `book/src/plans/`. If the work touches structure, read `book/src/architecture/design.md`, and read `.claude/arch/evolution.md` before proposing anything structural, because it records what has already been argued and rejected.

### Decide

Take the next action from the list above unless the user directs otherwise. When surfaces disagree, the order of authority is code, then tests, then the book, then everything else. Closer to the running system wins.

### Act

Small commits, conventional prefixes, one logical change each. Run the gates before committing.

Never publish without explicit per-publish approval. Stop at `--dry-run`.

---

## Pause points (when to ask)

- Before opening a PR, and before merging one
- Before adding a dependency
- Before any structural change not already authorized in a plan
- If a document and the code disagree in a way that changes what to build
- If the user has just sent a message, read it before acting

Do not pause for routine progress reporting. Surface progress at batch boundaries.

---

## Live serve

```bash
cd book && mdbook serve --hostname 0.0.0.0 --port 3000
```

Background it. If port 3000 is held by a stale process:

```bash
lsof -nP -iTCP:3000 -sTCP:LISTEN | awk 'NR>1 {print $2}' | xargs -r kill
```

---

## Update protocol

**Keep this file current.** `CLAUDE.md` makes it the first thing read on session start, so a stale entry here misinforms every future session before any other file is opened. It sat three months out of date and claimed `book/src/` was an empty placeholder long after the docsite existed.

After each batch ships, or any meaningful state change, update the State section, the next actions, and any new trap worth knowing.

---

**End of session resume. Begin OODA above.**
