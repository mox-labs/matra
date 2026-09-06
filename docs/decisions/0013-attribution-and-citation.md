# 0013. Attribution and Citation

- **Status:** accepted
- **Date:** 2026-09-06
- **Decider(s):** owner decision, recorded by the maintainer role

## Context

matra is about to publish its first artifacts to crates.io and PyPI. Package
metadata on both registries is immutable per version: once 0.2.0 is uploaded,
its author string cannot be edited, only superseded by a later version. The
same string is what appears on the crate page, on the project page, in
`cargo metadata` output, and in the wheel's `METADATA` file for every
downstream tool that reads it.

Four files disagreed with each other before this decision.

| File | Value |
|---|---|
| `Cargo.toml` | `yzavyas` |
| `pyproject.toml` | `yzavyas` |
| `book/book.toml` | `yzavyas` |
| `LICENSE` | `mox.nexus` |
| `.claude-plugin/plugin.json` | `mox labs` |

Three different names for one project, and the one carrying legal weight, the
copyright line in the license, named an entity that appears nowhere else. A
reader trying to work out who holds copyright, who to attribute, and who to
contact would get a different answer from each file.

Separately, `CITATION.cff` was blocked on this. matra reports numbers that
people will put in papers, and its methodology reference already carries the
publication for every measure. Without a citation file, a researcher citing
matra itself has to invent a form, and the forms they invent will not agree.

Note that this is distinct from commit authorship. Commits in this repository
are authored by Claude with the owner as co-author, which records who did the
work. Package attribution records who publishes and holds the copyright. The
two answer different questions and are deliberately not the same.

## Options considered

### Option A: an individual, `yzavyas`

Keep the existing crate and wheel metadata and change the license to match.

**Pros:**
- No change to the registry metadata already prepared.
- Accurate for who currently writes the code.

**Cons:**
- The project is published under a `mox-labs` organization, at a `mox-labs`
  homepage, from a `mox-labs` repository. An individual author contradicts
  every URL in the same file.
- Ties a long-lived public artifact to one person's handle, so a change in
  who maintains it means a copyright change rather than a maintainer change.
- Gives no contact address, which several registry and citation consumers
  expect.

### Option B: the organization, `mox labs`

Name the organization as author and copyright holder everywhere, with a
contact address.

**Pros:**
- Agrees with the repository, homepage, and plugin manifest, which already
  say `mox labs`.
- Copyright rests with the entity that continues to exist as contributors
  change, so maintainer turnover is not a licensing event.
- Gives citation tooling an entity author, which is the form CFF expects for
  organizational software.

**Cons:**
- Less specific about who to reach for a given change; that job moves to the
  repository issue tracker, which is where it belongs anyway.

### Option C: both, individual and organization

List the individual as author and the organization as copyright holder.

**Pros:**
- Records both facts.

**Cons:**
- Reintroduces the disagreement this decision exists to remove, in a form
  that looks intentional and is therefore harder to spot when it drifts.
- Neither registry models the distinction, so the split survives only in
  files nobody reads together.

## Decision

We choose Option B. The canonical attribution is the organization `mox labs`,
with `yza.v@moxlabs.org` as the contact address, and it is identical in every
file that carries attribution. The copyright line reads
`Copyright (c) 2026 mox labs`.

The reason is that everything else about the project's public identity is
already organizational: the GitHub organization, the documentation homepage,
the repository URL, and the plugin manifest. Attribution that contradicts the
URLs printed beside it is not attribution, it is noise, and on an immutable
registry it is noise that cannot be corrected in place.

Concretely, the canonical forms are:

| File | Form |
|---|---|
| `Cargo.toml` | `authors = ["mox labs <yza.v@moxlabs.org>"]` |
| `pyproject.toml` | `authors = [{ name = "mox labs", email = "yza.v@moxlabs.org" }]` |
| `LICENSE` | `Copyright (c) 2026 mox labs` |
| `book/book.toml` | `authors = ["mox labs"]` |
| `.claude-plugin/plugin.json` | `"author": { "name": "mox labs", ... }` |
| `CITATION.cff` | entity author `mox labs`, email `yza.v@moxlabs.org` |

`CITATION.cff` ships at the repository root, which is where GitHub reads it
to render the "Cite this repository" control and where citation tooling looks
for it.

## Consequences

- Positive: one name, one contact, one copyright holder, across registries,
  documentation, plugin manifest, and citation file. A reader gets the same
  answer wherever they look.
- Positive: the citation file unblocks. Researchers citing matra get a form
  from the project rather than composing their own, and the file states
  explicitly that citing the software does not substitute for citing the
  publication behind each measure.
- Positive: copyright sits with an entity that outlives any one contributor.
- Negative: `CITATION.cff` adds a fifth place carrying the version number,
  alongside `Cargo.toml`, `pyproject.toml`, `.claude-plugin/plugin.json`, and
  the `CHANGELOG.md` heading. It also carries a release date, which the others
  do not. Both must move at each release.
- Negative: the contact address is a commitment. An address in immutable
  registry metadata that stops being read is worse than no address, so it must
  keep working for as long as the published versions exist.
- Neutral: commit authorship is untouched. Claude remains the commit author
  with the owner as co-author.

## Validation

This decision is right if, at the next release, no file disagrees with any
other about who wrote or holds copyright in matra, and the release checklist
catches the version and date in `CITATION.cff` without anyone remembering to
look.

It is falsified if the project acquires outside contributors who need
individual attribution in package metadata, or if the organization stops
being the publishing entity. Either would prompt a superseding ADR rather
than an edit to the files, so that the reason for the change is recorded.

## References

- `CITATION.cff`, `Cargo.toml`, `pyproject.toml`, `LICENSE`, `book/book.toml`,
  `.claude-plugin/plugin.json`.
- `book/src/reference/methodology.md` for the per-measure publications that a
  software citation does not replace.
- Citation File Format 1.2.0 schema guide, for the entity author form and the
  `date-released` field.
- ADR-0012, which introduced the plugin manifest that already carried the
  organizational form.
