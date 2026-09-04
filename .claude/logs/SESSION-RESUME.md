# Session resume

Run the OODA loop below at the start of every session in this directory. Then act.

---

## State (2026-09-04)

- **0.1.0 is published everywhere.** crates.io (maintainer-run `cargo publish`, 2026-08-23), PyPI (2026-09-04, all four artifacts: linux x86_64 / macOS Intel / macOS arm64 wheels + sdist), GitHub release v0.1.0. The surface freeze is live; every change from here is post-publish and semver-governed (`cargo-semver-checks` is armed in CI).
- **Repo:** `mox-labs/matra` (renamed from the previous project name). Branch protection on `main`: 13 required checks, `enforce_admins`, linear history, no force pushes. Everything lands via PR now, including doc-only changes like this file.
- **Publish pipeline:** `publish-pypi.yml` uses PyPI Trusted Publishing via a direct OIDC token exchange plus pinned `twine` (zero third-party actions in the upload path); `publish.yml` gates crates.io behind the `crates-io` environment. Five real defects were found and fixed getting there; the ledger lives in PRs #36 to #42.
- **Branch:** `main`, clean.

### Where the work is

**I7 is shipped, all five milestones.** Structural primitives cross FFI as fields per ADR-0008 (derivations cross as serde-visible data computed once at a pipeline choke point; views over data already crossing stay methods, ADR-0009's `Token::feat` being the instance). `Sentence` now carries `negations`, `modals`, `bare_assertion`, `reportings`, `root_adverbials` and `hearst_pairs`; `Document.passive_ratio` is a materialized slot and `python/matra/cli.py` reads it instead of re-deriving passive detection. `spec/tests/` fixtures (negation, modal, modal-coordination, evidentiality, hearst) pin every crossing primitive across the Rust and Python crusts. The ROADMAP rule-evaluation entry records what the five revealed about the shape `Rule` and `Predicate` must take (arcs by relation and lemma, feats lookups at tree positions, multi-arc constructions with optional participants, caller-supplied lexicons for open classes, span pairs with token-id provenance). The plan carries a shipped banner and stays as the milestone record.

**I8 is shipped, all eight milestones.** The six entry points are gone. The surface is `Ingest` (text/path constructors; a string is a stream of one) into `Engine` (`analyze`, `analyze_one`, `annotate`, `compose`). `annotate` is the only route from text to the parser, so the 8 MiB cap holds pipeline-wide; seven equivalence laws (L1 to L7) run as tests in `src/lib.rs`. ADR-0007 records the decision and supersedes ADR-0002; the vocabulary is `ingest -> decompose -> compose` with `abstract` reserved as the empty seam for rule evaluation. The I8 plan (`book/src/plans/i8-pipeline-surface.md`) carries a shipped banner and stays as the defect record.

Documentation is in lockstep: book pages, README, CHANGELOG, CLAUDE.md, the agent/skill files, the ADR index, evolution.md. I5 is retired (I8 subsumed Tasks A through D; Task E, `pub mod prelude`, waits for the 0.1.0 release pass).

**A domain survey landed 2026-08-21** (filed at `~/mox/research/drafts/matra-substrate/2026-08-21-domain-cartography-inquiry-hermeneutics-semiosis.md`). Its consequence for matra is a scoping principle now recorded on the roadmap: matra stays at the deterministic, verifiable tier; pragmatic enrichment and claim/fallacy certification stay out with the gap recorded, and coreference is specialist-adapter work above UDPipe, not a parse extension.

### Known traps, carried forward

- `cargo test --all-features` **fails to link.** The `python` feature builds against libpython with symbols deliberately left undefined. Not a regression; never make it a gate.
- After any Rust change, `maturin develop` must run before `python/tests/` tests the new internals; the installed wheel does not rebuild itself. Verified post-I8: all 9 Python tests pass against the rebuilt wheel, including the parametrized size-cap suite (extraction methods now gate at 8 MiB and decompose as plain text).
- UDPipe splits `Smith et al. reported` at the period in `et al.`; every sentence-scoped primitive inherits this.
- `vocabulary_ttr` is a raw type-token ratio, not comparable across document lengths.
- Floor gate 1 (`lychee`) runs without `--include-fragments`, so anchors are never checked.

---

## Next actions, in order

Sequence agreed with the maintainer 2026-08-21, publish gate cleared 2026-09-04:

1. **Maintainer-only: crates.io Trusted Publishing + token revoke.** Configure Trusted Publishing on crates.io for `mox-labs/matra`, workflow `publish.yml`, environment `crates-io`, then revoke the bootstrap API token used for the 0.1.0 hand publish. Nothing else can do this; it closes the last credential in the release path.
2. **Claude review CI (PR #35).** Tabled during the publish sprint, un-tabled after. Plugin-as-harness plus the `pr-review` skill as criteria; the action refuses to run on PRs that modify its own workflow file, so the live test only happens on the first PR after merge.
3. **Embeddings adapter (i9).** New port (Embedder trait) plus a specialist adapter, feature-gated, tier stated in output per the roadmap scoping principle. Design constraint settled 2026-08-21: pure-Rust inference (candle), NOT ort/fastembed (C FFI), because the core already compiles to wasm32-unknown-unknown with --no-default-features (verified) and a candle adapter keeps the WASM/TS path open where an ONNX adapter would close it forever. Write the plan under `book/src/plans/` first.
4. **Redundancy metrics, both halves.** The deterministic family from the roadmap entry (clusters, redundancy ratio, rep-n, skeleton repetition, opener formulae, document-scope compression; TextRank's similarity matrix is the head start), then semantic-equivalence clustering over the embeddings adapter.
5. **TS package decision.** The hard blocker is UDPipe's C FFI only; everything else compiles to WASM today. Options: types-and-helpers package now, everything-but-parse WASM crust, full crust when a WASM-capable provider exists (candle embeddings move this materially closer).
6. **Rule vocabulary design** (Rule / Predicate / Finding) against the shape recorded on the roadmap; slots anywhere after 3. x.uma composes as a peer consumer, never a dependency.


## Open, not blocking

- **`Error` is neither `Serialize` nor `Clone`.** `DocumentError`/`CorpusResult` therefore stay Rust-side; crossing to Python needs a projection with stable kind strings. Documented in the book (errors page). Needed before any Python corpus surface.
- The docsite is gated but not voiced: it has not been through orwell, sagan, jobs, or ebert.
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
