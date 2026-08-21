# Session resume

Run the OODA loop below at the start of every session in this directory. Then act.

---

## State (2026-08-21)

- **Branch:** `m2-docsite-ia-restructure`. No upstream configured, nothing pushed. The remote is still named for the project's previous name, so the GitHub rename is outstanding.
- **Working tree:** clean.
- **Code:** `cargo test` 106 pass, `cargo test --features cli` 106+ pass, `cargo check --no-default-features` clean, clippy clean. Conformance and all six integration tests verified against the real UDPipe model this session. `just docs-floor` all gates pass.

### Where the work is

**I8 is shipped, all eight milestones.** The six entry points are gone. The surface is `Ingest` (text/path constructors; a string is a stream of one) into `Engine` (`analyze`, `analyze_one`, `annotate`, `compose`). `annotate` is the only route from text to the parser, so the 8 MiB cap holds pipeline-wide; seven equivalence laws (L1 to L7) run as tests in `src/lib.rs`. ADR-0007 records the decision and supersedes ADR-0002; the vocabulary is `ingest -> decompose -> compose` with `abstract` reserved as the empty seam for rule evaluation. The I8 plan (`book/src/plans/i8-pipeline-surface.md`) carries a shipped banner and stays as the defect record.

Documentation is in lockstep: book pages, README, CHANGELOG, CLAUDE.md, the agent/skill files, the ADR index, evolution.md. I5 is retired (I8 subsumed Tasks A through D; Task E, `pub mod prelude`, waits for the 0.1.0 release pass).

**A domain survey landed 2026-08-21** (filed at `~/mox/research/drafts/matra-substrate/2026-08-21-domain-cartography-inquiry-hermeneutics-semiosis.md`). Its consequence for matra is a scoping principle now recorded on the roadmap: matra stays at the deterministic, verifiable tier; pragmatic enrichment and claim/fallacy certification stay out with the gap recorded, and coreference is specialist-adapter work above UDPipe, not a parse extension.

### Known traps, carried forward

- `cargo test --all-features` **fails to link.** The `python` feature builds against libpython with symbols deliberately left undefined. Not a regression; never make it a gate.
- After any Rust change, `maturin develop` must run before `python/tests/` tests the new internals; the installed wheel does not rebuild itself. Verified post-I8: all 9 Python tests pass against the rebuilt wheel, including the parametrized size-cap suite (extraction methods now gate at 8 MiB and decompose as plain text).
- UDPipe splits `Smith et al. reported` at the period in `et al.`; every sentence-scoped primitive inherits this.
- `vocabulary_ttr` is a raw type-token ratio, not comparable across document lengths.
- `Sentence::is_passive` is a method, so `python/matra/cli.py` re-implements passive detection. I7 M1 settles this.
- Floor gate 1 (`lychee`) runs without `--include-fragments`, so anchors are never checked.

---

## Next actions, in order

1. **I7, structural primitives.** Now unblocked: I8 M4 landed, so the field-versus-method question (I7 M1) can be decided against the real surface. Read `book/src/plans/i7-structural-primitives.md`.
2. **Confirm the voice-fingerprint consumer** still wants matra rather than the thin-wrapper alternative.
3. **Decide the publish identity.** `Cargo.toml` and `pyproject.toml` carry a personal address that becomes permanently public on first publish.

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
