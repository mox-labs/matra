# RFC-0019: The RFC and EP process

- Feature Name: `rfc_and_ep_process`
- Start Date: 2026-09-24
- RFC PR: [#95](https://github.com/mox-labs/matra/pull/95)
- Tracking EP: none
- Status: accepted
- Decider(s): project owner (the two kinds, the numbering, and the process), maintainer (the conversion and the checks)
- Supersedes: [RFC-0001](0001-record-architectural-decisions.md)

## Summary

matra's design record moves to a new root folder, `blueprints/`, and takes
the shape of the Rust RFC process. Two kinds of record replace the
architecture decision records and the iteration plans. An RFC
(`blueprints/rfcs/NNNN-name.md`, cited `RFC-NNNN`) records a design-level
change and is laid out with the Rust RFC template. An EP
(`blueprints/eps/NNNN-name.md`, cited `EP-NNNN`) is an enhancement plan: how
an accepted RFC gets from decision to shipping. An unaccepted RFC is an open
pull request, and merging it accepts it. Existing records keep their numbers.

## Motivation

Two kinds of record already existed, in two places, with overlapping and
partly unstated jobs.

- **Decision records** lived in `docs/decisions/` under RFC-0001. Their
  template ran Context, Options considered, Decision, Consequences,
  Validation, References. Their status vocabulary included `proposed`,
  linked to a `decision` issue, and no record here was ever in that state:
  each one arrived in a pull request already accepted, so the pull request
  was where the proposal actually lived, without the process saying so.
- **Iteration plans** lived under `plans/` in the docsite. Every
  other page of the docsite describes what ships today, and a plan by
  definition describes what does not. The docsite floor had to exempt
  `plans/` from its type-name gate (gate 3) because plans name types that do
  not exist yet, and `SUMMARY.md` carried eight plan pages under "What is
  planned", beside the roadmap.
- **Plan status had no common form.** Some plans opened with a dated
  `Shipped` banner, some carried a `**Status:** not-started` line that
  nothing kept current (one plan still read `not-started` while two
  releases shipped without it), and another carried no status at all, so
  its shipping is recorded only in `ROADMAP.md` and the CHANGELOG.
- **What a decision and a plan are for was never separated.** The i9 plan
  and RFC-0010, for example, both recorded the static-first adapter decision and
  its reasons, and the plan's amendment trail and the decision's amendment
  section had to be kept in step by hand.

The Rust project solved the same shape of problem with the RFC process: a
proposal is a pull request, the merge is the acceptance, the template splits
the teaching explanation from the technical one, and implementation is
tracked separately from the design. matra adopts that shape, with the
implementation half made explicit as the EP.

## Guide-level explanation

There are two kinds of record, and a contributor reaches for them in order.

**When a change alters matra's design, write an RFC.** "Design" means the
architecture, the systems around it (CI, release, the docsite), the
framework a contribution follows, or the toolchain. A public-surface change,
a relaxed boundary rule, a new dependency in `domain.rs`, and a change to how
matra is released each take an RFC. Copy `blueprints/rfcs/0000-template.md`
to the next free number, fill every section, and open a pull request. The
pull request is the proposal; review happens on it. Merging it accepts the
RFC, and the file lands with `Status: accepted`.

**When an accepted RFC takes more than one pull request to implement, write
an EP.** The EP names the RFCs it implements and lays out the iterations and
milestones, each with a deliverable and an exit criterion, the test plan,
the ship criteria, and the risks. Its status moves from `planned` through
`in progress` to `shipped in X.Y.Z`, or to `dropped`, and each move adds a
dated line to its status log.

**An RFC is not edited after it is accepted,** apart from its status line
(`accepted`, `implemented`, or `superseded by RFC-NNNN`) and a dated note.
Changing a decision means writing a new RFC that supersedes it.

**Citations carry over.** A record cited with the `ADR-` prefix before
2026-09-24 is the RFC of the same number; a plan cited as `i9` is
`EP-0009`. `blueprints/README.md` says so once, and holds the index of both
kinds.

## Reference-level explanation

**Layout.**

```text
blueprints/
  README.md              the process, and the index of both kinds
  rfcs/0000-template.md  the Rust RFC template, adapted
  rfcs/NNNN-name.md      one RFC per file
  eps/0000-template.md   the EP template
  eps/NNNN-name.md       one EP per file
```

`blueprints/` sits at the repository root, outside `book/`, so no record is
part of the docsite. `ROADMAP.md` links a planned item to its EP by its
GitHub URL, because the link checker runs offline over `book/src/` and a
relative path out of the book would not resolve on the deployed site.
`Cargo.toml` excludes `blueprints/*` from the published crate.

**The RFC header** carries `Feature Name`, `Start Date`, `RFC PR`,
`Tracking EP` and `Status`, and its sections are the Rust template's:
Summary, Motivation, Guide-level explanation, Reference-level explanation,
Drawbacks, Rationale and alternatives, Prior art, Unresolved questions,
Future possibilities.

**The EP header** carries `EP`, `Implements`, `Status` and `Shipped in`, and
its sections are Summary, Goals, Non-goals, Iterations and milestones,
Test plan, Ship criteria, Risks, and Status log.

**Numbering.** Four digits, never reused. RFC-0001 to RFC-0015 are the
decision records under their existing numbers. EP numbers follow the
iteration plans they replace (i7 is EP-0007, i10 is EP-0010). Only the
plans that describe shipped work became EPs: 0007 to 0011. The plans
numbered 0001, 0002 and 0004 were retired or retracted before this change,
and the plans numbered 0003, 0005 and 0006 were dropped by it rather than
converted: 0003 targeted entry points EP-0008 deleted, 0005 was already
retired with its work delivered by EP-0008, and 0006 was planned against
the workspace crate RFC-0004 retracted. Converting them would have meant
rewriting plans for surfaces that do not exist, so they were removed and
their history is in git. What each described that is still wanted remains
on the roadmap as a plain entry. RFC-0016, 0017 and 0018 are reserved for proposals open
on other branches, which convert separately; the index lists them as
`open, reserved`. This RFC is RFC-0019.

**Conversion rules.** Each existing record was restructured, not rewritten.
For a decision record: Context became Motivation; the decision became the
guide-level explanation; specifics, invariants and validation became the
reference-level explanation; negative consequences became Drawbacks;
options considered became Rationale and alternatives; references became
Prior art; open items became Unresolved questions. For a plan: its reasoning
became the Summary, its surface the Goals, its tasks or milestones the
iterations, its validation the test plan, its acceptance gate the ship
criteria, and its banner or status line the status log. A section with no
source material says `None recorded`. Citations changed from `ADR-NNNN` to
`RFC-NNNN`, links followed the move, and em dashes outside quoted lines were
replaced. Each converted record carries a dated note saying so. A record's
status became `implemented` only where its own text or the CHANGELOG says it
shipped.

The plans index (the `README.md` of that `plans/` directory) was not a
plan, so it has no EP. Its cross-iteration regression matrix and 0.1.0 ship
predicate were cited only by the dropped plans, so they go with them; the
history is in git. The decision-record template is replaced by the two new
templates.

**Checks.** `scripts/check-blueprint-refs.sh` fails when an `RFC-NNNN` or
`EP-NNNN` cited anywhere in the tracked tree resolves neither to a file in
`blueprints/` nor to a reserved row of the index, and when a file in
`blueprints/rfcs/` or `blueprints/eps/` has no row in the index. It fails
when `rg` is absent, and ends with a line naming what it examined. It runs
from `just check` and as a step of the `Docsite floor` job in `ci.yml`.
Gate 5 of the docsite floor (no em dashes outside quoted lines) extends to
`blueprints/`, and gate 3 loses its `plans/` exemption because no plan is
left under `book/src/`.

**What does not change.** Released CHANGELOG entries keep the `ADR-NNNN`
wording they shipped with; the index note maps them. The session resume
under `.claude/logs/` stops being tracked: it is a local convenience, not a
record, and `blueprints/README.md` is where a session starts.

## Drawbacks

- Every citation in the tree changes at once: code comments, workflow
  comments, the docsite, the agent and skill files. Released CHANGELOG
  entries keep `ADR-NNNN`, so a reader meets both spellings and needs the
  mapping note.
- The plans leave the docsite, so a reader of the deployed site reaches an
  EP only through a GitHub link from the roadmap.
- The RFC template is heavier than the decision-record template was. A small
  decision now fills nine sections, several of them with one line.
- Two kinds of record are one more thing for a new contributor to learn
  than one kind would be.

## Rationale and alternatives

- **Keep decision records and plans as they were.** Costs nothing now, and
  keeps each problem in the Motivation: plans inside a docsite that
  otherwise describes only what ships, a status vocabulary with a state
  nothing uses, and status that drifts because nothing has a place to put
  it.
- **Keep decision records, move the plans out of the docsite.** Fixes the
  docsite half, and leaves the proposal-as-pull-request gap and the
  duplicated reasoning between plan and decision.
- **One kind of record carrying design and implementation together**, as a
  Python PEP largely does. Fewer files, but the design text and the milestone
  text change at different rates, and an accepted design would be reopened
  every time a milestone moves. Separating them is what lets an RFC be
  frozen at acceptance while its EP stays a living plan.
- **Renumber everything from 0001 under the new kinds.** Cleaner sequences,
  and every existing citation, in comments, commit messages and released
  CHANGELOG entries, would
  point at the wrong record. Keeping the numbers makes the mapping a rename
  of the prefix.

## Prior art

- The Rust RFC process ([rust-lang/rfcs](https://github.com/rust-lang/rfcs)):
  proposals as pull requests, acceptance by merge, and the `0000-template.md`
  whose sections the RFC template here adapts.
- Kubernetes Enhancement Proposals, which track implementation stages and
  graduation criteria beside the design, the concern the EP takes on here.
- Python Enhancement Proposals, the single-document alternative considered
  above.
- [RFC-0001](0001-record-architectural-decisions.md), which introduced the
  decision records this replaces, after Michael Nygard's architecture
  decision records and MADR.

## Unresolved questions

- The three open proposals reserved as RFC-0016, 0017 and 0018 are converted
  on their own branches, and their pull requests become the RFC PR of each.
- The `decision` issue template remains for discussion before a proposal is
  written. Whether it is still worth keeping, now that the pull request is
  where an RFC is discussed, is left to use.

## Future possibilities

The index tables in `blueprints/README.md` are maintained by hand, and the
check only proves that every file has a row. Generating the tables from each
record's header, as `llms.txt` is generated from `SUMMARY.md`, would make the
status column impossible to leave stale. It is not worth doing until the
tables drift once.
