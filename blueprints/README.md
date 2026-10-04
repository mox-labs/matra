# Blueprints

matra's design records, and the process that says which record a change
takes. The process follows the Rust RFC process. Two kinds of record live
here.

| Kind | Where | Cited as | What it records |
|---|---|---|---|
| Enhancement proposal | `proposals/NNNN-name.md` | `EPR-NNNN` | A substantial change that matra's users notice: the decision, what and why, laid out with the Rust RFC template. |
| Enhancement plan | `plans/NNNN-name.md` | `EPL-NNNN` | A plan for work that several agents carry out in parallel. Rare: work done in sequence is a tracking issue instead. Laid out with the Rust RFC template, with its milestones, test plan, ship criteria and risks under Reference-level explanation and a status log after it. |

**A new series, from 2026-10-04.** That day is the baseline: proposals and
plans are numbered from 0001, and each kind has its own sequence. The
records written before it, the RFCs and the EPs, are in
[`legacy/`](legacy/) unchanged: they are the
reasons behind the code that exists, and a comment or a page that cites
`RFC-NNNN` or `EP-NNNN` means one of them. They are not rendered on the
docsite and not counted here. A new proposal that changes what a legacy
record decided says so and cites it; the legacy record is not edited.

## The process

### Which record a change takes

The grain follows the Rust project's: the bigger the change and the more
of matra's users who notice it, the heavier its record.

| Change | Record | Where it lives |
|---|---|---|
| A substantial change matra's users notice: the public surface in Rust, Python, the command line or the JSON schema; semantics or behaviour, the default model included; removing or deprecating a substantial feature; an architecture boundary | Enhancement proposal | A pull request adding `proposals/NNNN-name.md` |
| A minor addition to the public surface, or a minor removal from it | API change proposal | A short issue labelled `acp` |
| Tooling, docs, CI, the harness, a refactor, a measured performance improvement, a bug fix | None beyond the pull request | The pull request. A rule it leaves behind lives in the file that governs it. |

An API change proposal is enough when the change adds or removes one item a
caller may use and changes nothing about what the existing calls do or
return. A change to what an existing call returns or means, or one that
reshapes several items at once, is an enhancement proposal. When in doubt
whether a change needs a record at all, ask whether a caller of matra would
notice it: if not, the pull request is the record.

A change to this process is tooling too: a pull request that edits this
file, with the reason in its description.

### Enhancement proposals

**A proposal is a pull request.** Copy
[`proposals/0000-template.md`](proposals/0000-template.md) to the next free
number, fill every section, and open a pull request with the proposal alone
or beside the code it binds, labelled `rfc`. Discussion happens there.

**A proposal can merge before it is accepted.** Its status line then reads
`proposed`, and the docsite renders it, so the owner can read it in place
and comment on it before deciding. Claude may merge a pull request that adds
or edits a `proposed` proposal, on the same standing authority as any other
pull request. A `proposed` proposal decides nothing: no code may rely on it.

**Only the owner accepts a proposal.** Acceptance is a change of the status
line to `accepted`, and only the owner merges a pull request that makes that
change, whether it is the proposal's first pull request or a later one.
Claude writes proposals, argues for them and merges them while they are
`proposed`, and never merges a pull request that sets a status to
`accepted`. A proposal the owner declines leaves `main`: the pull request
that removes it says why, and its number is not reused.

**Each accepted proposal has a tracking issue.** It is labelled `tracking`,
opened from [`tracking.md`](../.github/ISSUE_TEMPLATE/tracking.md) when the
proposal is ready for the owner's decision, so the proposal's `Tracking
issue` header can link it before acceptance. It carries the milestone
checklist and links each pull request that delivers a milestone, and it
closes when the work ships.

**A proposal is not rewritten after acceptance.** Two edits are allowed: the
status line, and a dated note directly under the header saying what changed
and why. A change of mind is a new proposal that supersedes the old one, and
the old one's status then reads `superseded by EPR-NNNN`. While it is
`proposed`, a proposal may be edited freely; its pull requests are its
history.

### API change proposals

**A minor change to the public surface is an issue**, opened from
[`api-change-proposal.md`](../.github/ISSUE_TEMPLATE/api-change-proposal.md)
and labelled `acp`: the item, what a caller writes with it, and why it is
minor. Only the owner accepts one, by saying so on the issue. Claude writes
and argues them, and never accepts one. The pull request that carries an
accepted proposal out links the issue and closes it.

### Enhancement plans

**A plan exists only when several agents execute one plan in parallel.** It
says who carries which milestone, in what order, and what each hands the
next. Work carried out in sequence, however many pull requests it takes,
needs no plan: the proposal's tracking issue holds its checklist. Copy
[`plans/0000-template.md`](plans/0000-template.md) to the next free plan
number and fill `Implements`, which is `none` when the parallel work
implements no proposal.

**A plan is laid out with the Rust RFC template**, as a proposal is. The plan
itself is part of the reference-level explanation, as its subsections: the
milestones, the test plan, the ship criteria, and the risks. The status log
follows the sections. A plan that implements a proposal keeps its
Motivation and Guide-level explanation to a few lines and a link, because
the proposal holds the argument.

**A plan is living until it ships.** If a milestone turns out to be
ambiguous, the plan is the bug: edit the plan first, then the code. Every
change of status adds a dated line to its status log. Once it ships or is
dropped, it is kept as the record of how the work went.

### Status

| Kind | Status | Meaning |
|---|---|---|
| Proposal | `proposed` | Merged for review; the docsite renders it. Not a decision: nothing may rely on it. Claude may merge it. |
| Proposal | `accepted` | The owner merged the change of its status line; the decision is in effect. |
| Proposal | `implemented` | Accepted, the CHANGELOG records it shipping, and its tracking issue is closed. |
| Proposal | `superseded by EPR-NNNN` | A later proposal replaced it. Kept unchanged apart from the status line and a dated note. |
| Plan | `planned` | Written, not started. |
| Plan | `in progress` | At least one milestone has landed. |
| Plan | `shipped in X.Y.Z` | Every milestone landed, and release X.Y.Z carries the work. |
| Plan | `dropped` | Will not be carried out as written. The status log says why. |

### On the docsite

The docsite has three areas: Docs, the pages for matra's users; Blueprints,
this file at `/blueprints/` and each proposal and plan at
`/blueprints/proposals/<name>` and `/blueprints/plans/<name>`, read from these
files at build time; and Lab, for evals and experiments. Blueprints is the
one area that shows a status: each record's, read from its header, beside it
in the navigation and under its title. The pages in Docs describe what ships
and carry none. A link from a record to a file outside the rendered records
leads to that file on GitHub.

The legacy records are not rendered. Their old addresses,
`/blueprints/rfcs/<name>` and `/blueprints/eps/<name>`, still answer, each
with a short page that links the record on GitHub.

Locally, the docsite is also where the owner and Claude converge on a
record: in `just docs-serve`, text on any page can be selected and
commented on, and the threads are files in `discussion/` that any session
reads (`just comments`) and Claude answers in (`just comment-reply`). The
published site has no local comments; giscus is its public channel.
`site/README.md` describes both.

### Numbering

Numbers are four digits and zero-padded, and each kind has its own sequence
from 0001. A number whose record was merged to `main` is never reused. A
number taken by an open pull request is that pull request's until it merges
or closes.

### Checks

`scripts/check-blueprint-refs.sh` runs from `just check` and in the
`Docsite floor` CI job. Every citation in the tracked tree must resolve to a
record of its own kind: `EPR-NNNN` to a file in `proposals/`, `EPL-NNNN` to
one in `plans/`, `RFC-NNNN` to one in `legacy/rfcs/` and `EP-NNNN` to one in
`legacy/eps/`. A citation whose number exists only as another kind is named
as a wrong-kind citation. Every file in `proposals/` and `plans/` must have a
row below. The docsite floor's em-dash gate covers this directory too.

## Proposals

The enhancement proposals since 2026-10-04. None yet.

| Proposal | Title | Status | Tracking issue |
|---|---|---|---|
| [EPR-0000](proposals/0000-template.md) | The template | not a record | none |

## Plans

The enhancement plans since 2026-10-04. None yet.

| Plan | Title | Status | Implements |
|---|---|---|---|
| [EPL-0000](plans/0000-template.md) | The template | not a record | none |

## Legacy

The records written before 2026-10-04, in `legacy/rfcs/` and `legacy/eps/`,
cited as `RFC-NNNN` and `EP-NNNN`. Records cited as `ADR-NNNN` before
2026-09-24 are the RFC of the same number, and iteration plans cited as `iN`
are the EP of the same number where one exists. One RFC number appears in
them without a file:

| Legacy number | Why it has no file |
|---|---|
| RFC-0016 | Reserved for release automation by a pull request that never merged. |
