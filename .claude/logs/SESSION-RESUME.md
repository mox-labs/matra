# Session resume

Run the OODA loop below at the start of every session in this directory. Then act.

---

## State (2026-08-10)

- **Branch:** `m2-docsite-ia-restructure`. No upstream configured, nothing pushed. The remote is still named for the project's previous name, so the GitHub rename is outstanding.
- **Working tree:** clean.
- **Code:** `cargo test` 84 pass, `cargo test --features cli` 89 pass, `cargo check --no-default-features` clean. `just docs-floor` all gates pass.
- **Docsite:** 17 entries. Live via `cd book && mdbook serve --port 3000`.

### What shipped this session

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

1. **I7 M1.** Write the ADR deciding whether structural primitives are fields or methods, then implement negation on `Sentence`. The rest of I7 inherits that answer. Plan: `book/src/plans/i7-structural-primitives.md`.
2. **Confirm the voice-fingerprint consumer.** The roadmap entry says the trigger is met in substance but needs confirmation that the blocked consumer still wants matra rather than the thin-wrapper alternative it considered.
3. **Decide the publish identity.** `Cargo.toml` and `pyproject.toml` carry a personal address that becomes permanently public on first publish, and crates.io yanks do not remove metadata.

## Open, not blocking

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
