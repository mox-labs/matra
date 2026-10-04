# Contributing to matra

matra is a Claude-managed open-source project. This document explains how
the repository is run so that anyone (human contributor, AI collaborator,
or curious onlooker) can see how decisions are made, where plans live,
and how to participate.

If you are an agent contributing to matra, [AGENTS.md](./AGENTS.md) is the
short form of this document: the build and gate commands, the boundary
rules, and the shape of a pull request.

For how the project is maintained, explained for an outside reader (the
agents and their methods, routing, the verification gate, who decides what,
and how lessons become gates), see
[How matra is maintained](site/content/explanation/how-matra-is-maintained.md)
on the docsite.

The working values are **transparency** (decisions are visible),
**auditability** (every change has a trail), and **reversibility** (every
change can be backed out cleanly).

---

## The working model

A change takes the record its size calls for, after the Rust process
([`blueprints/README.md`](blueprints/README.md) has the full rule; read it
before opening an issue or PR):

- **An enhancement proposal** (`EPR-NNNN`) for a substantial change
  matra's users notice: the public surface in Rust, Python, the command
  line or the JSON schema; semantics or behaviour, the default model
  included; removing or deprecating a substantial feature; an architecture
  boundary. It is a pull request adding a file to `blueprints/proposals/`,
  discussed there. It may merge with its status `proposed`, so it renders
  on the docsite for review; it is accepted when the owner merges the
  change of its status line to `accepted`. Each accepted proposal has a
  tracking issue, labelled `tracking`, with its milestone checklist.
- **An API change proposal** for a minor addition to the public surface or
  a minor removal from it: a short issue labelled `acp`, accepted by the
  owner on the issue.
- **A plain pull request** for tooling, docs, CI, the harness, a refactor, a
  measured performance improvement or a bug fix. A rule it leaves behind
  lives in the file that governs it.
- **An enhancement plan** (`EPL-NNNN`) in `blueprints/plans/` only for a
  plan several agents execute in parallel.

Proposals and plans are numbered from 0001, from the 2026-10-04 baseline.
The RFCs and EPs written before it are in `blueprints/legacy/`, unchanged:
they are the reasons behind the code that exists, and `RFC-NNNN` and
`EP-NNNN` cite them.

Each milestone is one pull request: a sequence of atomic commits on a
short-lived branch, opened against `main`, reviewed, then merged. Every
commit on the branch is its own logical unit so the history reads as a
series of small, auditable steps. After merge, the branch is deleted and
the tracking issue ticks the milestone and links the pull request.

The project's primary engineer is Claude (Anthropic's AI), working with
human direction. Every commit carries a `Co-Authored-By` trailer
identifying the model used. Claude reviews and merges a PR once CI is
green and the review raises no blockers, and leaves a comment with its
rationale as the audit trail, with one exception: a pull request that
sets a proposal's status to `accepted` is merged only by the owner. The owner makes
the decisions and approves each release; approving the release's
deployment environments is the release decision (see the release process
below).

---

## Where things live

| Location | What lives there |
|---|---|
| `site/content/` | The docsite. The architecture of record is `architecture/design.md` and the `reference/` pages. Read them before changing structure. |
| `.claude/` | The agents, skills and review command Claude Code loads, and `arch/evolution.md`, the designs considered and rejected. |
| `blueprints/proposals/` | Enhancement proposals: one file per substantial change, in the Rust RFC layout. The process and the index are `blueprints/README.md`. |
| `blueprints/plans/` | Enhancement plans: plans several agents execute in parallel, with milestones, test plan, ship criteria, and a status log. |
| `blueprints/legacy/` | The RFCs and EPs written before 2026-10-04, unchanged: the reasons behind the code that exists. |
| `CHANGELOG.md` | What shipped per release, with prose Highlights for load-bearing changes. |
| `CLAUDE.md` | Working rules for AI collaborators: pipeline shape, boundary rules, conventions. |
| `scripts/` | Versioned tooling: pre-commit hook, boundary check, changelog rollover, etc. |
| `justfile` | Single source of truth for repeatable workflows. |
| GitHub Issues | Bugs and features; API change proposals (label `acp`); one tracking issue per accepted proposal (label `tracking`). |
| GitHub Discussions | Open-ended design space: early proposals, retrospectives, ideas, Q&A. |

If something is unclear or contradictory across these surfaces, the order
of authority is: code > tests > the docsite > plans > proposals > legacy records >
CHANGELOG > Issues > Discussions. Closer to the running system
wins.

---

## How decisions are made

Decisions go through four surfaces depending on stakes.

**Open-ended exploration** -> GitHub Discussions. Early proposals, "should
we consider X", retrospectives. No commitment, no labels.

**A substantial change matra's users notice** -> an enhancement proposal.
Copy `blueprints/proposals/0000-template.md` to the next free number and
open it as a pull request; the pull request is where it is discussed. It
may merge with its status `proposed`, which decides nothing and puts it on
the docsite for the owner to read; the owner accepts it by merging the
change of its status line to `accepted`. A `decision` issue
(`.github/ISSUE_TEMPLATE/decision_record.md`) can come first when the
options need airing before anyone writes the proposal; it closes pointing at
the proposal's pull request. Before the owner decides, a tracking issue
(`.github/ISSUE_TEMPLATE/tracking.md`) is opened for the proposal's
milestones.

**A minor addition to or removal from the public surface** -> an API change
proposal (`.github/ISSUE_TEMPLATE/api-change-proposal.md`), labelled `acp`.
The owner accepts or declines it on the issue.

**Everything else** -> a regular issue (bug or feature) if one is useful,
plus a PR. The PR's body explains the why; the commits explain the what. A
rule the change leaves behind goes in the file that governs it.

The full process is `blueprints/README.md`.

### Who decides

- **Only the owner accepts a decision.** Claude writes enhancement
  proposals and API change proposals and argues for them, and never accepts
  one. A proposal is accepted by a change of its status line to `accepted`,
  and only the owner merges a pull request that makes that change.
- **Claude's standing merge authority covers everything else**: code, docs,
  tooling and dependency updates, and a proposal whose status is
  `proposed`, once CI is green and the review raises no blockers, each with
  a merge comment giving the rationale.
- **An agent never approves a deployment environment.** Claude may prepare
  and dispatch a release; the run then waits at the `crates-io` and `pypi`
  environments for the owner, and the owner's approval is the release
  decision.

Today these rules hold by practice, not by a branch rule: every act is
recorded under one GitHub login, and nothing yet makes an acceptance wait
for the owner. The next section is how that changes.

### Identity

Claude is to get its own GitHub identity, a GitHub App, so that its merges
and comments can be told apart from the owner's on the record, and so that
a branch rule can require the owner's review where only the owner decides.
The owner creates it; until then this section is the plan, not the state.

1. **Create the App** (GitHub, Settings, Developer settings, GitHub Apps,
   New GitHub App), with no webhook and these repository permissions, and
   nothing else:
   - Contents: read and write (push branches, merge pull requests);
   - Pull requests: read and write (open, comment on, merge);
   - Issues: read and write (API change proposals, tracking issues).

   GitHub adds Metadata (read) to every App; it grants no write access.
   Leaving out the Workflows permission has one consequence: GitHub refuses
   an App's push that changes a file under `.github/workflows/`. Such a
   change is pushed by the owner, or the owner grants that permission later
   on purpose.
2. **Install it on this repository only** (the App's page, Install App,
   Only select repositories, `matra`), and generate a private key. The key
   stays with the owner, outside the repository.
3. **Run Claude's GitHub calls as the App.** A session mints an
   installation access token from the App's ID and private key (a signed
   JWT exchanged at `POST /app/installations/{installation_id}/access_tokens`;
   the token lasts an hour) and exports it as `GH_TOKEN`. Every `gh pr
   merge`, `gh pr comment` and `gh issue` call then acts as the App's bot
   account, `<app-slug>[bot]`, and so does every branch pushed over HTTPS
   with that token. The owner's login then means the owner.
4. **Then make acceptance need the owner.** A proposal may merge as
   `proposed` without the owner, so a path rule on `blueprints/proposals/`
   would hold too much: acceptance is a change to one line. The check that
   fits is a required status check that fails when a pull request changes
   a proposal's `- Status:` line to `accepted` and carries no approving
   review from the owner. Then, on `main`'s branch protection, turn on
   "Require a pull request before merging" with required approvals at 0,
   and make that check required. GitHub does not let an author approve
   their own pull request, so a pull request that accepts a proposal is
   opened by the App; one the owner opens under their own login has no one
   who can approve it. After turning the rule on, open one pull request
   that accepts a proposal and one that only adds a `proposed` one, and
   check that only the first waits. CODEOWNERS can then shrink to what
   should still reach the owner.

Branch protection is the owner's to change; no agent changes it.

---

## How releases work

**Trigger:** release when something user-visible justifies it. Architecture
change, breaking surface change, security-relevant fix, new feature. Not
on a calendar.

**Cadence:** pre-1.0, releases typically follow a proposal's work shipping. Post-1.0,
semver discipline binds.

**Process:** Claude prepares and dispatches a release, and the owner
decides when it publishes.

1. Run `just release-prep VERSION`. This rolls
   `[Unreleased]` -> `[VERSION]` in CHANGELOG.md.
2. Review the diff, and ensure the [VERSION] section has 2-4
   Highlight paragraphs (for the load-bearing changes) plus the
   structured Keep-a-Changelog bullets.
3. Bump the `Cargo.toml` version and every other
   version-carrying file (`just version-sync` names them), commits, and
   lands it on `main`.
4. `cargo publish --dry-run --features udpipe` for sanity check.
5. **Dispatch the release from main**, with the version typed out:
   `gh workflow run release.yml --ref main -f version=VERSION`. There is
   no tag to push; `.github/workflows/release.yml` creates the annotated
   tag itself once its checks pass, so the tag can never name a commit
   the release did not verify. A release cannot be started from a branch
   other than `main`, and a version that disagrees with `Cargo.toml`
   stops the run before anything is built.
6. **Two manual approval gates.** The run pauses at the `crates-io`
   environment and again at `pypi`. The owner approving each deployment in
   the Actions UI is the per-publish approval for that registry, and the
   release decision. An agent never approves one. Nothing publishes from a
   laptop.
7. After both publishes, a smoke job installs the released version from
   PyPI and from crates.io, on Linux and macOS, under a CPython one
   minor above the abi3 floor, and runs `matra --version` and
   `matra --skill` from each install.

To retry a publish that failed after the tag was created, dispatch again
with `-f skip_tag=true`. That reuses the existing tag and builds from it
rather than from whatever `main` has become, and it is the only
supported way to re-run a release without bumping the version.

The deliberate manual gate is by policy, not because automation is hard.
Publishing is irreversible (yanking leaves a tombstone) and visible to
every downstream consumer; it deserves an explicit human moment.

**Settings the workflow cannot enforce for itself.** These live in three
web UIs, and the 0.2.0 release is what proved each one matters:

- The `crates-io` and `pypi` GitHub environments each need **required
  reviewers**. Without them the workflow publishes unattended, which is
  what `pypi` did for 0.2.0 while `crates-io` correctly waited.
- Each environment's **deployment branch policy must allow `main`**,
  since the release is dispatched from there. A branch-only policy is
  also why the tag-triggered workflow failed outright for 0.2.0 with
  `Tag "v0.2.0" is not allowed to deploy to crates-io due to environment
  protection rules`. If a tag trigger is ever restored, both
  environments need a `v*` tag policy as well.
- The **Trusted Publishing configuration on both registries names a
  workflow filename**, and that filename is now `release.yml`. It was
  `publish.yml` on crates.io and `publish-pypi.yml` on PyPI. Until both
  are updated, the OIDC exchange fails and neither registry accepts an
  upload.

---

## How to contribute

### File an issue

- Bug: `.github/ISSUE_TEMPLATE/bug_report.md` (auto-applied).
- Feature: `.github/ISSUE_TEMPLATE/feature_request.md`. Ask whether the
  feature belongs in matra itself or in a downstream caller.
- API change proposal (a minor change to the public surface):
  `.github/ISSUE_TEMPLATE/api-change-proposal.md`.
- Tracking an accepted proposal: `.github/ISSUE_TEMPLATE/tracking.md`.
- A decision whose options need airing before a proposal:
  `.github/ISSUE_TEMPLATE/decision_record.md`.

### Open a discussion

For anything open-ended, use Discussions instead of Issues. We organize
discussions into categories (configured in the GitHub UI):

- **Announcements**: release notes, project status.
- **Ideas**: half-formed thoughts, "what if" questions.
- **Ideas for proposals**: design ideas you want feedback on before writing an enhancement proposal.
- **Q&A**: usage questions.
- **Show and tell**: things you built with matra.

### Open a PR

1. Fork; create a branch named after the work (e.g. `epr-0001/m2-skill-references`,
   `fix/symlink-rejection`, `docs/clarify-tree-walk`).
2. Run `just install-hooks` once on a fresh clone. The hook runs the Rust
   gates (fmt, check, clippy, doc, test on both feature configurations)
   plus the boundary check. CI runs those too, and additionally the
   docsite floor, cargo-deny, cargo-semver-checks, the maturin wheel build
   and `mypy --strict`. A green hook is a strong signal, not a guarantee.
   Run `just check` and `just typecheck` before pushing.
3. Make atomic commits. One logical change per commit. Conventional
   prefix: `feat / fix / docs / chore / refactor / perf / test / ci /
   build`. Optional scope in parens: `feat(extraction): ...`.
4. Update `CHANGELOG.md` `[Unreleased]` with a terse bullet. If the
   change is architectural / breaking / security-relevant, add a
   Highlight paragraph too.
5. Open the PR. The PR template asks for Summary, Why, Test plan.
   Fill it in.
6. CI runs the Rust gates the hook ran, plus cargo-deny,
   cargo-semver-checks, the wheel build and mypy. If anything fails, fix
   and push.
7. The PR is reviewed against the gates in
   [`.claude/skills/pr-review/SKILL.md`](.claude/skills/pr-review/SKILL.md).
   When CI is green and the review raises no blockers, Claude merges it and
   comments its rationale on the PR. A PR that sets a proposal's status to
   `accepted` is merged by the owner alone.

### What "good" looks like in a commit

The commit message is a teaching moment for the next reader, not just
release-note material. State the WHY, the alternatives considered, and
the trade-off you accepted. Long bodies are welcome when the change is
load-bearing; short subjects are mandatory.

```
feat(metrics): cap brotli sliding window at 256 KiB

The previous lgwin=22 (4 MiB) per paragraph was a CPU pegging vector
on adversarial input. Vector's review flagged it HIGH. lgwin=18 is the
safe ceiling for prose-as-redundancy-proxy: cross-256-KiB long-range
redundancy is more than enough signal; beyond that we measure engine
plumbing, not linguistic structure.

Per-paragraph cap matches the new window so a single paragraph never
triggers more than one window of work. Oversize paragraphs slot into
the existing `Option<f64> = None` semantics (Chesterton fence 7).
```

---

## How verification works

Three layers, each answering a different question.

**Does the library behave?** `cargo test` runs the unit tests and doctests.
`just check` runs the whole Rust gate suite plus the boundary check and the
docsite floor gates.

**Does the binary behave?** `tests/cli.rs` invokes the `matra` binary and
asserts output shape and exit codes. The tests that need a parse are
`#[ignore]` because they require the UDPipe model:

```
cargo test --features cli --test cli -- --ignored
```

**Do the crusts agree?** matra ships one Rust core behind several bindings.
They all call the same parser, so a difference between them is never a
difference of behaviour: it is a binding defect, a renamed field or a lost
value or a rounded number. `spec/tests/*.json` holds language-agnostic
fixtures that every crust runs, with one runner per language. Read
[`spec/README.md`](./spec/README.md) for the fixture format and the rule
about the model being part of the contract.

```
just conformance      # every crust against the shared spec
just coverage-all     # line coverage, Rust and Python
just lint             # clippy and ruff
```

## Code style

**ACES + antifragility:** non-negotiable. ACES (Adaptable, Composable,
Extensible) is the structural design philosophy; antifragility is the
operational discipline (size caps, panic boundaries, atomic ops, TOCTOU
closure). See `.claude/skills/aces/SKILL.md` and
`.claude/skills/resilience-floor/SKILL.md`. Every structural change is
checked against the ACES boundary test; every new I/O or external-library
boundary is checked against the antifragile checklist.

**Boundary rules:** non-negotiable. See
[`site/content/reference/boundary-rules.md`](site/content/reference/boundary-rules.md) for the
canonical eight rules and how each is enforced; `CLAUDE.md` carries the
summary. `scripts/check-boundaries.sh` checks seven of them with the semgrep
rules in `.semgrep/` and runs from `just check`, the optional pre-commit hook
and the `Boundary check` CI job; install the pinned semgrep once with
`pip install --require-hashes -r .github/requirements/semgrep.txt`. Rule 6 is
verified by compiling. The checks read import forms; what an import cannot
show rests on review, so run `just check` before opening a PR.

**Formatting:** `cargo fmt`. Enforced.

**Lints:** `cargo clippy --all-targets -- -D warnings` on both feature
configurations. Warnings are errors.

**Docs:** `cargo doc --no-deps --all-features` with `RUSTDOCFLAGS=-Dwarnings`.
No broken intra-doc links. Public items are documented.

**Tests:** unit tests in `#[cfg(test)] mod tests`, integration tests in
`tests/`. Tests verify requirements, not implementation. New bugs get
regression tests.

**Prose convention:** no em dashes in documentation. (Project rule.)

**Conventional commits:** required for the subject line. The body is
free-form prose; explain the *why*.

---

## Working with Claude

When Claude opens a PR:

- Every commit has a `Co-Authored-By: Claude ...` trailer.
- The PR body shows what Claude did and why.
- The commit messages are written by Claude.
- Claude merges once CI is green and the review raises no blockers, with a
  comment giving its rationale, except a pull request that accepts a
  proposal, which only the owner merges. The owner decides what is built and approves each release.

When you (a human) open a PR with Claude's help:

- The trailer is welcome. It is a fact, not a stigma. Attribution is
  part of the auditability discipline.
- The same review and CI gates apply.

If you want to work on matra with Claude Code on your machine, the
`.claude/` directory in this repo is preloaded with the agent
definitions, the skills, and the rejected designs, and `CLAUDE.md`
points it at the design record in `blueprints/`. Claude Code will read
those automatically.

---

## Reporting security vulnerabilities

Do **not** file a public issue for security problems. See
`SECURITY.md` for the disclosure process.

---

## Code of conduct

This project follows the Contributor Covenant. See `CODE_OF_CONDUCT.md`.
