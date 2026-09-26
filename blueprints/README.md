# Blueprints

matra's design record. Two kinds of document live here, and the process
around them follows the Rust RFC process. [RFC-0019](rfcs/0019-rfc-and-ep-process.md)
introduced it.

| Kind | Where | Cited as | What it records |
|---|---|---|---|
| RFC | `rfcs/NNNN-name.md` | `RFC-NNNN` | A change to matra itself: its architecture, its affordances (what a caller can do), or its tuning. What and why. |
| EP | `eps/NNNN-name.md` | `EP-NNNN` | An enhancement plan: how work gets from decision to shipping. Iterations, milestones, test plan, ship criteria, status. Implements an accepted RFC, or stands alone. |

Records cited as `ADR-NNNN` before 2026-09-24 are the RFC of the same
number: `ADR-0008` is [RFC-0008](rfcs/0008-structural-primitives-are-fields.md).
Iteration plans cited as `iN` or `IN` are the EP of the same number, where
one exists: `i9` is
[EP-0009](eps/0009-embeddings-adapter.md).

## The process

**An RFC is proposed as a pull request.** Copy
[`rfcs/0000-template.md`](rfcs/0000-template.md) to the next free number,
fill every section, and open a pull request with the RFC alone or beside the
code it binds. An unaccepted RFC lives only as that open pull request.
Discussion happens there.

**Merging the pull request accepts the RFC.** The file reaches `main` with
its status set to `accepted`. Closing the pull request without merging
declines it, and nothing lands here.

**An accepted RFC gets an EP when its implementation spans more than one
pull request.** Copy [`eps/0000-template.md`](eps/0000-template.md) to the
next free EP number and fill `Implements`. Work that one
pull request delivers needs no EP; the RFC and the pull request are the
record.

**Work that does not change matra itself gets an EP and no RFC.** The
docsite, the harness, CI and release tooling change how matra is built,
checked, documented or delivered, not what it is or what a caller can do
with it. Their plan is a standalone EP with `Implements: none` and a Design
section holding the choices a later contributor needs. When in doubt, ask
whether a caller of matra would notice the change: if so, it is an RFC.

**An RFC is not rewritten after acceptance.** Two edits are allowed: the
status line, and a dated note directly under the header saying what changed
and why. A change of mind is a new RFC that supersedes the old one, and the
old one's status then reads `superseded by RFC-NNNN`. The lineage stays
readable because nothing in it is overwritten.

**An EP is a living plan until it ships.** If a milestone turns out to be
ambiguous, the plan is the bug: edit the plan first, then the code. Every
change of status adds a dated line to its status log. Once it ships or is
dropped, it is kept as the record of how the work went.

### Status

| Kind | Status | Meaning |
|---|---|---|
| RFC | `accepted` | Merged; the decision is in effect. |
| RFC | `implemented` | Accepted, and the CHANGELOG records it shipping. |
| RFC | `superseded by RFC-NNNN` | A later RFC replaced it. Kept unchanged apart from the status line and a dated note. |
| EP | `planned` | Written, not started. |
| EP | `in progress` | At least one milestone has landed. |
| EP | `shipped in X.Y.Z` | Every milestone landed, and release X.Y.Z carries the work. |
| EP | `dropped` | Will not be carried out as written. The status log says why. |

### Numbering

Numbers are four digits, zero-padded, and never reused. RFC numbers carried
over from the decision records keep their number. A number reserved for an
open pull request is listed below as `open, reserved` so that no other record
takes it.

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
| RFC-0017 | Harness layers | open, reserved | none |
| [RFC-0018](rfcs/0018-source-spans-and-provenance.md) | Source spans and provenance | accepted | none |
| [RFC-0019](rfcs/0019-rfc-and-ep-process.md) | The RFC and EP process | accepted | none |
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
| [EP-0000](eps/0000-template.md) | The template | not a record | none |
