# Blueprints

matra's design records, and the process that says which record a change
takes. Blueprints holds the proposals and plans the owner and Claude
converge on. The process follows the Rust RFC process. Two kinds of record live
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

## Awaiting you

Decisions waiting for your ruling, in dependency order, then assumptions to
confirm or strike.

<awaiting />

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
or beside the code it binds, labelled `epr`. Discussion happens there.

**A proposal can merge before it is accepted.** Its status line then reads
`proposed`, and the docsite renders it, so the owner can read it in place
and comment on it before deciding. Claude may merge a pull request that adds
or edits a `proposed` proposal, on the same standing authority as any other
pull request. A `proposed` proposal decides nothing: no code may rely on it.

**A ruling is recorded, never inferred.** The owner answers a decision
in a comment, on the page or on the pull request. A comment is the event:
by itself it changes nothing on the record. Claude then records the ruling
in the proposal as a named step, a `<ruling>` in the decision with the
owner's response and its date, in a pull request whose description names
the comment it transcribes. The ruling in the record is the standing state,
and it is all the docsite reads: the frame, the navigator and the queue
never read comments to decide whether a decision is open, so a quiet thread
never reads as assent. An assumption is confirmed or struck the same way:
Claude changes the claim's basis, or removes the claim, citing the comment.
The queue under Awaiting you is drawn from the proposals' components, never
from their prose, so a decision leaves it when its proposal records a
ruling, and an assumption when its claim is confirmed or struck.

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
in the navigation and in its frame. The pages in Docs describe what ships
and carry none.

Docs and Blueprints are two stances on one identity. Docs is the observe
stance: a reader learns matra there and changes nothing. Blueprints is the
collaborate stance: the owner and Claude work a record there until the
owner rules. Only the chrome changes; the colours and their roles, the
type, the grid and the reading face are the same. Each proposal and plan
has a frame above it, drawn from its header and its components and never
from its prose: its kind, id and title, its status, how many of its
decisions are settled, how many assumptions wait to be confirmed, the
commit it was read at, and its pull request and tracking issue. Beside it,
or on a phone in a sheet from the frame and inline under the title, a
navigator in three groups: Read (the sections), Decide (each decision, its
state, and the order the decisions depend on one another) and Confirm (each
assumption, and the decisions that rest on it). What awaits the owner's
touch, an open decision or an assumption, is marked in Spark; a decision the
owner accepted, in Emergence; each mark is a glyph and a word. A link from a record to a file outside the rendered records
leads to that file on GitHub, on `main`, except a citation of lines, which
opens at the commit the record was read at (below).

### Reading a proposal

A proposal is Claude's understanding of the code, its history and the
owner's intent, written so the owner can check it and answer it. Its parts
are components the docsite draws, each a tag in the Markdown, so they read
the same in the file and on the site. `site/README.md` ("The Blueprints
components") gives their syntax and the research behind each.

**The anatomy.** A proposal follows the template's order, and each section
has a role the navigator names: the orientation (the masthead, the Summary,
the pragmatics block and what changed), the case (Motivation through
Rationale and alternatives), the queue (the decisions under Unresolved
questions, and the assumptions), the references (Prior art, and every chip),
and the margin, where comments land. Future possibilities has no role; it
equips no decision.

**The header** at the top of the file gives the start date, the commit the
proposal was read at, who decides (the owner), and the pull request and
tracking issue. On screen the frame shows each of these; in print they are
a masthead under the title.

**The pragmatics block** follows the Summary: the ask, what Claude will do
if the proposal is accepted, what it needs from the owner, what it will not
do, and what happens if nobody answers. Silence is not assent. The owner's
answer to a proposal or to one of its decisions is one of four: accept,
accept with a reservation, object, or redirect.

**A claim** that the proposal rests on carries its basis, a word and a
glyph, and chips that open its grounds:

| Glyph | Basis | Meaning |
|---|---|---|
| ● | observed | Claude read it or ran it; a chip points at the evidence. |
| ◐ | inferred | Reasoned from observed claims; the reasoning is written out. |
| ○ | assumed | No grounds yet; the owner should confirm or strike it. The assumptions are also collected in one list. |

A chip to code opens the cited lines at the commit in the proposal's `Pinned
at` line, never at `main`, so the evidence cannot move under the claim.
The build reads those lines at that commit and fails when they no longer
hold the text the citation names. A check that a model made, a reading, is
said to be one; a test or a compiler is a different kind of evidence, and
several readings by Claude agents count as one source, not several.

#### Likelihood

A claim about what will happen, and only such a claim, may carry one word
from this closed list, whose meaning is the range beside it (the bands of
US Intelligence Community Directive 203). A proposal never puts a number on
its own confidence.

| Word | Means |
|---|---|
| almost no chance | 1 to 5% |
| very unlikely | 5 to 20% |
| unlikely | 20 to 45% |
| roughly even chance | 45 to 55% |
| likely | 55 to 80% |
| very likely | 80 to 95% |
| almost certain | 95 to 99% |

**A decision block** sets out the options, Claude's recommendation apart from
them as a judgment, the strongest case against it, and the owner's decision
once made. Each decision also says three things the build checks. How
readily the recommended choice can be undone once carried out
(`reversible`): □ reversible, ◧ costly to reverse, or ■ irreversible. The
decisions it depends on (`depends`), so the navigator and the queue show
them in order. And the claims and assumptions it rests on (`grounds`), each
linked from the decision and linking back to it, so a reader goes from a
decision to its grounds and from an assumption to every decision on it. The
build fails on an id that names nothing on the page, on a dependency cycle,
and on a decision without `reversible`. Where the decision's own text does
not settle its reversibility, the value rests on an assumed claim among its
grounds, for the owner to confirm. **What changed in my understanding** lists, on a revision, what
Claude understood before, what it understands now, and what changed it.

**A sketchy diagram means proposed.** Structure drawn with a hand-drawn line
is what the proposal would build, not what ships; structure drawn crisp
ships, at the commit the proposal was read at. Every such figure says so in
its legend and in words, and carries the structure as a table under "show
the data". Sketchiness is never used for numbers.

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

The enhancement proposals since 2026-10-04. On the docsite each is a card
with its status and its open decisions; the table is the same index.

**Baseline.** EPR-0001 to EPR-0005 describe matra's foundation as it
stands at the commit each one names on its `Pinned at` line, and carry
forward the reasons from the legacy records that still hold, with what no
longer does. A later proposal that changes one of them says so and cites
it. The legacy records about the process (RFC-0001, RFC-0019) and the
docsite (EP-0012, EP-0013) are not carried into a proposal: under this
process that work is tooling, and its rules live in this file and in
`site/README.md`.

<record-index kind="proposals" />

| Proposal | Title | Status | Tracking issue |
|---|---|---|---|
| [EPR-0000](proposals/0000-template.md) | The template | not a record | none |
| [EPR-0001](proposals/0001-pipeline-and-ports.md) | The pipeline and its ports | proposed | none |
| [EPR-0002](proposals/0002-data-model.md) | The data model | proposed | none |
| [EPR-0003](proposals/0003-distribution-and-provisioning.md) | Distribution and provisioning | proposed | none |
| [EPR-0004](proposals/0004-command-line-and-configuration.md) | The command line, configuration and the agent skill | proposed | none |
| [EPR-0005](proposals/0005-embeddings-and-semantic-clusters.md) | Embeddings and semantic clusters | proposed | none |
| [EPR-0006](proposals/0006-the-0-3-0-surface.md) | The 0.3.0 surface | proposed | none yet |

## Plans

The enhancement plans since 2026-10-04. None yet.

<record-index kind="plans" />

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
