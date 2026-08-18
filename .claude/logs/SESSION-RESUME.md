# Session resume

Run the OODA loop below at the start of every session in this directory. Then act.

---

## State (2026-08-18)

- **Branch:** `m2-docsite-ia-restructure`. No upstream configured, nothing pushed. The remote is still named for the project's previous name, so the GitHub rename is outstanding.
- **Working tree:** clean.
- **Code:** `cargo test` 85 pass, `cargo test --features cli` 90 pass, `cargo check --no-default-features` clean. `just docs-floor` all gates pass.
- **Docsite:** 17 entries. Live via `cd book && mdbook serve --port 3000`.

### Where the work is

**The next thing is I8, not I7.** A maintainer question about the six entry points opened a formal review that found two live defects and produced a redesign. I7 M1's field-versus-method question is entangled with that surface and should be decided after I8 M4, not before.

Plan: `book/src/plans/i8-pipeline-surface.md`. It carries the design, the milestone rubrics, the seven equivalence laws that are the acceptance test, and the costs.

**I8 M0 is done** (commit 368bac5). The input size cap was bypassable from Python: four PyO3 extraction methods called the parser with no gate, so `Matra.analyze(huge)` raised while `Matra.tfidf_summarize(huge, 3)` did not. Fixed, with a parametrized test over every text-taking method. And `analyze_from` returns a Document whose three paragraph metrics are unconditionally `None` while the docs claimed equivalence to `analyze_markdown`; that postcondition is now documented and pinned by a regression test. Its root cause, `run_suite` carrying the sentence set twice, is I8 M1.

**Milestones 1 through 3 are worth doing regardless.** None commits to the surface change. M4 adds the new surface alongside the old so nothing breaks mid-flight. M6, deleting the six, is the gate: free now, a breaking change after publish.

### What shipped earlier

The project was renamed to matra after a name collision on PyPI made dual publishing impossible. A CLI binary landed behind the `cli` feature, with a conformance suite running shared JSON fixtures through every crust. The docsite was cut from 19 pages to a working set, gained the roadmap, and then the plans moved out of `.claude/` into `book/src/plans/` so the whole planning surface is one thing a reader can follow.

`ROADMAP.md` now records that the rule-evaluation trigger has fired, and `book/src/plans/i7-structural-primitives.md` is the plan that follows from it.

### Known traps, all verified this session

- `cargo test --all-features` **fails to link.** The `python` feature builds against libpython with symbols deliberately left undefined. Not a regression, and it must never become a gate. Use `cargo test` and `cargo test --features cli`.
- UDPipe splits `Smith et al. reported a finding` into two sentences at the period in `et al.`, so attribution lands in a different sentence from its reporting verb. Every sentence-scoped primitive inherits this.
- `vocabulary_ttr` is a raw type-token ratio and falls as text grows, so it is not comparable across documents of different lengths. Documented on `capabilities.md`; a normalized measure is on the roadmap.
- `Sentence::is_passive` is a method, and methods do not cross FFI, so `python/matra/cli.py` re-implements passive detection. This is the decision I7 M1 exists to settle.
- Floor gate 1 (`lychee`) runs without `--include-fragments`, so **anchors are never checked**. A link to a heading that no longer exists still passes.

---

## Next actions, in order

1. **I8 M1.** `Metric` becomes `Fn(&mut Document)`; `run_suite` drops its sentence-slice parameter and derives the set from `Document::sentences()`. This removes the redundant representation that made Defect B possible. Four metric modules plus the suite. Breaks any external `Metric` impl, which is free now.
2. **I8 M2 and M3.** Domain additions, then the decomposer registry. Both additive.
3. **Confirm the voice-fingerprint consumer.** The roadmap entry says the trigger is met in substance but needs confirmation that the blocked consumer still wants matra rather than the thin-wrapper alternative it considered.
4. **Decide the publish identity.** `Cargo.toml` and `pyproject.toml` carry a personal address that becomes permanently public on first publish, and crates.io yanks do not remove metadata.

## Open, not blocking

- **`Error` is neither `Serialize` nor `Clone`** (it wraps `io::Error`), so `DocumentError` and `CorpusResult` inherit both gaps while `Corpus` has neither. Crossing to Python needs a projection with stable kind strings. Blocks I8 M5, not M1 through M4.
- **ADR-0002 should be superseded** by I8: its five verbs enumerate calling conventions rather than transformations. Recommended vocabulary is `ingest -> decompose -> compose` with `abstract` reserved as a named empty seam. Note `abstract` is a reserved keyword in Rust and can never name code.

- The docsite is gated but not voiced: it has not been through orwell, sagan, jobs, or ebert.
- `rumi-nlp` is named in the published plans. Already public via ADR-0003, but worth deciding before release.
- Floor gate 1 should gain `--include-fragments`.

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
