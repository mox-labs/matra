# Blueprints

matra's design record. Two kinds of document live here, and the process
around them follows the Rust RFC process. [RFC-0019](rfcs/0019-rfc-and-ep-process.md)
introduced it, and its dated notes record each change to it since.

| Kind | Where | Cited as | What it records |
|---|---|---|---|
| RFC | `rfcs/NNNN-name.md` | `RFC-NNNN` | A substantial change that matra's users notice. What and why. |
| EP | `eps/NNNN-name.md` | `EP-NNNN` | An enhancement plan for work that several agents carry out in parallel. Laid out with the Rust RFC template, like an RFC, with its milestones, test plan, ship criteria and risks under Reference-level explanation and a status log after it. |

Records cited as `ADR-NNNN` before 2026-09-24 are the RFC of the same
number: `ADR-0008` is [RFC-0008](rfcs/0008-structural-primitives-are-fields.md).
Iteration plans cited as `iN` or `IN` are the EP of the same number, where
one exists: `i9` is
[EP-0009](eps/0009-embeddings-adapter.md).

## The process

### Which record a change takes

The grain follows the Rust project's: the bigger the change and the more
of matra's users who notice it, the heavier its record.

| Change | Record | Where it lives |
|---|---|---|
| A substantial change matra's users notice: the public surface in Rust, Python, the command line or the JSON schema; semantics or behaviour, the default model included; removing or deprecating a substantial feature; an architecture boundary | RFC | A pull request adding `rfcs/NNNN-name.md` |
| A minor addition to the public surface, or a minor removal from it | API change proposal | A short issue labelled `acp` |
| Tooling, docs, CI, the harness, a refactor, a measured performance improvement, a bug fix | None beyond the pull request | The pull request. A rule it leaves behind lives in the file that governs it. |

An API change proposal is enough when the change adds or removes one item a
caller may use and changes nothing about what the existing calls do or
return. A change to what an existing call returns or means, or one that
reshapes several items at once, is an RFC. When in doubt whether a change
needs a record at all, ask whether a caller of matra would notice it: if
not, the pull request is the record.

A change to this process is tooling too. It takes a pull request and a dated
note on [RFC-0019](rfcs/0019-rfc-and-ep-process.md), not a new RFC.

**Records written before 2026-10-03 stay as written.** Some of them would
take a lighter record under this grain:
[RFC-0020](rfcs/0020-deprecate-unread-config-keys.md) would be an API change
proposal, and the standalone EPs for tooling (EP-0012, EP-0013, EP-0014) would
be pull requests. Their headers keep the fields they were written with,
`Tracking EP` included.

### RFCs

**An RFC is proposed as a pull request.** Copy
[`rfcs/0000-template.md`](rfcs/0000-template.md) to the next free number,
fill every section, and open a pull request with the RFC alone or beside the
code it binds, labelled `rfc`. An unaccepted RFC lives only as that open pull
request. Discussion happens there.

**Only the owner accepts an RFC.** Merging its pull request accepts it, and
only the project owner merges an RFC pull request; the file reaches `main`
with its status set to `accepted`. Closing the pull request without merging
declines it, and nothing lands here. Claude writes RFCs and argues for them,
and never merges or closes an RFC pull request.

**Each accepted RFC has a tracking issue.** It is labelled `tracking`, opened
from [`tracking.md`](../.github/ISSUE_TEMPLATE/tracking.md) when the RFC pull
request is ready for the owner's decision, so the RFC's `Tracking issue`
header can link it before the merge. It carries the milestone checklist and
links each pull request that delivers a milestone, and it closes when the
work ships. If the RFC is declined, the issue closes as not planned.

**An RFC is not rewritten after acceptance.** Two edits are allowed: the
status line, and a dated note directly under the header saying what changed
and why. A change of mind is a new RFC that supersedes the old one, and the
old one's status then reads `superseded by RFC-NNNN`. The lineage stays
readable because nothing in it is overwritten.

### API change proposals

**A minor change to the public surface is an issue**, opened from
[`api-change-proposal.md`](../.github/ISSUE_TEMPLATE/api-change-proposal.md)
and labelled `acp`: the item, what a caller writes with it, and why it is
minor. Only the owner accepts one, by saying so on the issue. Claude writes
and argues them, and never accepts one. The pull request that carries an
accepted proposal out links the issue and closes it.

### EPs

**An EP exists only for a plan that several agents execute in parallel.**
It says who carries which milestone, in what order, and what each hands the
next. Work carried out in sequence, however many pull requests it takes,
needs no EP: the RFC's tracking issue holds its checklist. Copy
[`eps/0000-template.md`](eps/0000-template.md) to the next free EP number and
fill `Implements`, which is `none` when the parallel work implements no RFC.

**An EP is laid out with the Rust RFC template**, as an RFC is: Summary,
Motivation, Guide-level explanation, Reference-level explanation, Drawbacks,
Rationale and alternatives, Prior art, Unresolved questions, Future
possibilities. The plan is part of the reference-level explanation, as its
subsections: the milestones and iterations, the test plan, the ship
criteria, and the risks. The status log follows the sections, as the one
appendix the process requires. An EP that implements an RFC keeps its
Motivation and Guide-level explanation to a few lines and a link, because
the RFC holds the argument.

**An EP is a living plan until it ships.** If a milestone turns out to be
ambiguous, the plan is the bug: edit the plan first, then the code. Every
change of status adds a dated line to its status log. Once it ships or is
dropped, it is kept as the record of how the work went.

**The EPs written before 2026-10-03 keep their layout.** EP-0007 to EP-0014
were written to the earlier EP template (Summary, Design, Goals, Non-goals,
Iterations and milestones, Test plan, Ship criteria, Risks, Status log).
They are records, and they stay as written; the Rust layout applies to
every EP written since.

### On the docsite

The docsite renders this directory as its Blueprints part, at `/blueprints/`
for this file and `/blueprints/rfcs/<name>` and `/blueprints/eps/<name>` for
the records, read from these files at build time; nothing is copied. It is
the one part of the site that shows a status: each record's, read from its
header, beside it in the navigation and under its title. The pages that
describe what ships carry none. A link from a record to a file outside
`blueprints/` leads to that file on GitHub; a link between records stays on
the site.

Locally, the docsite is also where the owner and Claude converge on a
record: in `just docs-serve`, text on any page can be selected and
commented on, and the threads are files in `discussion/` that any session
reads (`just comments`) and Claude answers in (`just comment-reply`). The
published site has no local comments; giscus is its public channel.
`site/README.md` describes both.

### Status

| Kind | Status | Meaning |
|---|---|---|
| RFC | `accepted` | Merged; the decision is in effect. |
| RFC | `implemented` | Accepted, the CHANGELOG records it shipping, and its tracking issue is closed. |
| RFC | `superseded by RFC-NNNN` | A later RFC replaced it. Kept unchanged apart from the status line and a dated note. |
| EP | `planned` | Written, not started. |
| EP | `in progress` | At least one milestone has landed. |
| EP | `shipped in X.Y.Z` | Every milestone landed, and release X.Y.Z carries the work. |
| EP | `dropped` | Will not be carried out as written. The status log says why. |

### Numbering

Numbers are four digits and zero-padded. RFC numbers carried over from the
decision records keep their number. A number reserved for an open pull
request is listed below as `open, reserved` so that no other record takes it.
When that pull request closes without merging, the number is released and its
row removed. A number whose record was merged to `main` is never reused.
RFC number 0018 was released this way when its proposal's design moved into
EP-0012, as 0017 was when its pull request closed; a released number cites no
record, so it is written without the `RFC-` prefix. EP number 0015 was
released the same way: its pull request planned the docsite's Blueprints
part as an EP, and the plan left the record when developer tooling stopped
taking one; the design is in `site/README.md`.

### Checks

`scripts/check-blueprint-refs.sh` runs from `just check` and in the
`Docsite floor` CI job. It fails when an `RFC-NNNN` or `EP-NNNN` cited
anywhere in the tracked tree resolves to no file here and to no reserved
row below, and when a file in `rfcs/` or `eps/` is missing from these
tables. The docsite floor's em-dash gate covers this directory too.

## RFCs

| RFC | Title | Status | EP |
|---|---|---|---|
| [RFC-0001](rfcs/0001-record-architectural-decisions.md) | Record architectural decisions | superseded by RFC-0019 | none |
| [RFC-0002](rfcs/0002-pipeline-vocabulary.md) | Pipeline vocabulary: ingest / decompose / parse / measure (+ peer extract) | superseded by RFC-0007 | none |
| [RFC-0003](rfcs/0003-workspace-with-rumi-nlp.md) | Cargo workspace with `matra-core` and `rumi-nlp` | superseded by RFC-0004 | none |
| [RFC-0004](rfcs/0004-stay-single-crate.md) | Stay single-crate; supersede the workspace split proposal | accepted | none |
| [RFC-0005](rfcs/0005-supply-chain-hardening.md) | Supply-chain hardening posture | accepted | none |
| [RFC-0006](rfcs/0006-abstract-tier-vocabulary-lock.md) | Abstract-tier vocabulary lock | accepted | none |
| [RFC-0007](rfcs/0007-one-pipeline.md) | One pipeline: ingest -> decompose -> compose, with abstract reserved | implemented | [EP-0008](eps/0008-pipeline-surface.md) |
| [RFC-0008](rfcs/0008-structural-primitives-are-fields.md) | Structural primitives are fields | implemented | [EP-0007](eps/0007-structural-primitives.md) |
| [RFC-0009](rfcs/0009-feats-lookup-accessor.md) | Feats lookup accessor, Rust-only | implemented | [EP-0007](eps/0007-structural-primitives.md) |
| [RFC-0010](rfcs/0010-embeddings-adapter.md) | Embeddings: a Tier-2 channel behind an Embedder port, static adapter first | implemented | [EP-0009](eps/0009-embeddings-adapter.md) |
| [RFC-0011](rfcs/0011-out-of-the-box.md) | Out of the box: configuration, paths, and one CLI | implemented | [EP-0010](eps/0010-foundations.md) |
| [RFC-0012](rfcs/0012-agent-surface.md) | The agent surface: a skill the binary prints | implemented | [EP-0011](eps/0011-agent-surface.md) |
| [RFC-0013](rfcs/0013-attribution-and-citation.md) | Attribution and Citation | implemented | none |
| [RFC-0014](rfcs/0014-distribution-matrix.md) | The Distribution Matrix | implemented | none |
| [RFC-0015](rfcs/0015-provisioning-failures.md) | Provisioning is matra's own, and a failure to fetch is not an invalid model | implemented | none |
| RFC-0016 | Release automation | open, reserved | none |
| [RFC-0019](rfcs/0019-rfc-and-ep-process.md) | The RFC and EP process | accepted | none |
| [RFC-0020](rfcs/0020-deprecate-unread-config-keys.md) | Deprecate the config keys nothing reads | accepted | none |
| [RFC-0000](rfcs/0000-template.md) | The template | not a record | none |

## EPs

EP numbers follow the plans they replace, so the sequence has gaps. Plans
that were retired once their work landed, retracted when RFC-0004
superseded RFC-0003, or
written against surfaces that no longer exist have no EP; their history is
in git.

| EP | Title | Status | Implements |
|---|---|---|---|
| [EP-0007](eps/0007-structural-primitives.md) | Five structural primitives | shipped in 0.1.0 | RFC-0008, RFC-0009 |
| [EP-0008](eps/0008-pipeline-surface.md) | One pipeline, not six entry points | shipped in 0.1.0 | RFC-0007 |
| [EP-0009](eps/0009-embeddings-adapter.md) | Embeddings as a specialist adapter | shipped in 0.2.0 | RFC-0010 |
| [EP-0010](eps/0010-foundations.md) | Out of the box | shipped in 0.2.0 | RFC-0011 |
| [EP-0011](eps/0011-agent-surface.md) | The agent surface | shipped in 0.2.0 | RFC-0012 |
| [EP-0012](eps/0012-docsite.md) | The docsite on SvelteKit, with figures and examples | shipped (docsite) | none |
| [EP-0013](eps/0013-docsite-identity.md) | The docsite's identity, drawn from matra's own output | in progress | none |
| [EP-0014](eps/0014-architecture-guardrails.md) | Architecture guardrails: the boundary rules as semgrep checks | shipped (CI) | none |
| [EP-0000](eps/0000-template.md) | The template | not a record | none |
