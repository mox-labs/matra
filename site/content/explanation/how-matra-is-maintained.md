# How matra is maintained

matra is maintained by its owner and by Claude, Anthropic's AI, working through a set of agents, skills and checks that live in the repository beside the code. This page explains that arrangement for a reader outside it: someone deciding whether to depend on matra, someone thinking of contributing, or someone who wants to borrow the pattern. The mechanics of opening a pull request are in [CONTRIBUTING.md](https://github.com/mox-labs/matra/blob/main/CONTRIBUTING.md); this page says why the project is run the way it is.

## Two roles

The owner decides, and Claude does most of the engineering. The owner brings what is outside the codebase: what is worth building, when to ship it, and the judgment calls that depend on context no file holds. Claude brings the convergent work inside a frame: drafting, implementing, reviewing and keeping a large codebase consistent with its own rules. Neither substitutes for the other, and the working rule is to think with, not for: a claim that does not hold is challenged, from either side, with the evidence for the challenge.

| Who | What they do | What only they do |
|---|---|---|
| The owner | Sets direction, reviews what matters to them, comments on records in progress | Accepts an enhancement proposal by merging the change of its status to `accepted`, accepts an API change proposal, approves each release |
| Claude | Writes code, docs and proposals; reviews; merges; prepares and dispatches releases | Nothing the owner cannot also do. Claude merges every pull request except one that accepts a proposal, each with a comment giving its reason |

## The agents

The work is split across seven agents, each a Claude Code subagent defined by one file in [`.claude/agents/`](https://github.com/mox-labs/matra/tree/main/.claude/agents). They are not separate people or separate models: each is Claude with a different set of instructions, and what makes one worth having is that it reasons in a way the others do not. Every agent file states the method it owns and what it refuses to do, and no two agents own the same method.

| Agent | Its method | What it leaves to others |
|---|---|---|
| maintainer | Orchestration: holds the whole codebase in view, makes the call on a change, authors and argues proposals, and hands each part of the work to the agent whose method fits | Accepting its own proposals, which only the owner does |
| reviewer | Falsification: steel-mans a change, then demands evidence for it, against the review gates | Resilience findings, and writing the fix |
| portsmith | Minimal-contract design for the four port traits, and the test for moving one into its own crate | Adapter internals that leave a trait unchanged |
| ffi-keeper | Keeping the Rust and Python surfaces in agreement: what crosses, how errors map, how the packages are built and pinned | Rust-only surface that never crosses |
| resilience | Failure-mode analysis: size caps at the entry, panic boundaries at the C++ parser, atomic writes, verified bytes used once, cycle-safe tree walks | General review. It is the one agent that blocks a merge on these grounds |
| archivist | Lockstep: the CHANGELOG, the index of records, statuses and tracking issues move in the same pull request as the code they describe | Authoring proposals |
| newcomer | Naive literal execution: arrives knowing nothing, follows the pages exactly, records what was expected against what happened, and fixes nothing | Reading the source to explain a failure |

The newcomer is the agent most unlike the rest. Its first two passes, before the 0.2.0 release, found six blocking defects that every automated check had passed, the worst of them on the boundary between what a page promised and what a platform did.

## The skills

The judgment the agents apply lives in [`.claude/skills/`](https://github.com/mox-labs/matra/tree/main/.claude/skills), one directory per practice: what good looks like in this codebase, and why. Two are non-negotiable and are read together on every structural change. ACES (adaptable, composable, extensible) is the design test: does the change leave the system easier or harder to change, to recombine and to extend. The resilience floor is the operational one: does it survive hostile input, a crashing dependency and two processes racing. The others cover the hexagonal architecture, Rust design choices, the Python binding, test strategy, keeping records in step, the review gates, and checking that a built artifact installs for a real user.

They are practices rather than tool instructions, so they survive a change of tooling, and they are versioned with the code, so a change to one is reviewed like a change to the code.

## Routing

Each agent's and each skill's description, at the top of its file, says what it is for and what it is not for. Claude Code reads those descriptions and picks by them, and they are the only routing table: there is no second list to drift out of step with the first. A pull request review is where several are convened, and only by what the diff touches: the reviewer always, the portsmith when a port module changes, the ffi-keeper when the Python binding or its packaging changes, resilience when a change reads files, writes them or takes outside input, and the archivist when the public surface or the records change. That rule is written once, in the `/review` command.

## The gate before a merge

Two layers stand between a pull request and `main`, and they are not equal.

**The deterministic checks are the verification.** `just check` runs them locally. On GitHub, branch protection on `main` requires twenty status checks to pass, and holds administrators to them: the Rust build and tests on Linux and macOS under each feature set, the minimum supported Rust version, a WebAssembly build check, the API docs, the dependency audit, the semver check, the Python wheels and type check, CodeQL, the boundary rules as tested semgrep rules, and the docsite floor, which builds the site and checks its links, its published URLs, its figures and its worked examples. The practice for a new check is to plant the violation it exists for and watch it fail before trusting it.

**The model review is a cold re-read.** Every pull request is also read by Claude against the review gates in the `pr-review` skill. That reader is a model of the same family as the author, so it is a cold re-read, not independent verification: it shares the author's blind spots, and a separate reader of the same family has not been shown to catch more than the author's own second pass would. What it adds is a reading of intent, the part of a change no check can see. Its verdict is not a required status check. Claude does not merge over a "blockers" verdict, and fixes it and asks for a re-read instead; that holds by discipline, and the comment thread on each pull request shows whether it held.

## Who is accountable

Two kinds of act are the owner's alone, whatever trust has built up.

- **Decisions.** An enhancement proposal is accepted only when the owner merges the change of its status line to `accepted`, and an API change proposal only when the owner says so on its issue. Claude writes and argues both, and may merge a proposal while its status is `proposed`, so it can be read on this site; that decides nothing, and Claude never merges a pull request that accepts one.
- **Releases.** Claude prepares a release and dispatches the workflow, which verifies, tags, builds and attests it, then stops at two deployment environments, one per registry, each requiring the owner's approval. That approval is the release decision, because a published version cannot be withdrawn. An agent never approves a deployment.

Everything else merges on Claude's standing authority: once CI is green and the review raises no blockers, Claude merges and leaves a comment giving its reasoning, which is the audit trail. Every commit Claude makes carries a `Co-Authored-By` trailer naming the model, and every release the release workflow builds carries an attestation that [SECURITY.md](https://github.com/mox-labs/matra/blob/main/SECURITY.md) shows how to verify.

One limit is worth stating plainly. Today every merge, comment and approval is recorded under the owner's GitHub login, so the record alone cannot show which of them the owner made by hand, and no branch rule yet makes an acceptance wait for the owner. The rules above hold by practice. The [Identity section of CONTRIBUTING.md](https://github.com/mox-labs/matra/blob/main/CONTRIBUTING.md#identity) sets out how Claude is to get a GitHub identity of its own, and how a required check then holds an acceptance for the owner's review.

## Which record a change takes

The process follows the Rust project's RFC process, and the size of a change sets the weight of its record.

| Change | Record |
|---|---|
| A substantial change users notice: the public surface in Rust, Python, the command line or the JSON output; behaviour, including the default model; removing a substantial feature; an architecture boundary | An enhancement proposal (`EPR-NNNN`), proposed as a pull request. It may merge as `proposed`, so it renders for review, and is accepted when the owner merges the change of its status to `accepted` |
| A minor addition to the public surface, or a minor removal | An API change proposal: a short issue labelled `acp` |
| Tooling, docs, CI, refactors, measured performance work, bug fixes | The pull request itself. A rule it leaves behind lives in the file that governs it |

Each accepted proposal gets a tracking issue, labelled `tracking`, that holds its milestone checklist and links the pull request for each milestone; it closes when the work ships. A plan that several agents carry out in parallel gets an enhancement plan (`EPL-NNNN`) beside the proposal. An accepted proposal is never rewritten: a change of mind is a new proposal that supersedes it, so the reasoning behind every decision stays readable. The full process and the index of records are in [`blueprints/README.md`](https://github.com/mox-labs/matra/blob/main/blueprints/README.md).

Proposals and plans are numbered from 0001, from a baseline on 2026-10-04. The records written before it, cited as `RFC-NNNN` and `EP-NNNN`, are kept unchanged in [`blueprints/legacy/`](https://github.com/mox-labs/matra/tree/main/blueprints/legacy): they are the reasons behind the code that exists, and the comments and pages that cite them still mean them.

## Where a record converges

The proposals and plans are rendered on this site, in its Blueprints area, each with its status; it is the one area of the site that shows a status, and the documentation keeps to what ships. Before a record is settled, the owner reads it there in the local development server, where any passage on any page can be selected and commented on. The threads are saved as files in the repository, so the next Claude session reads them, answers in them and changes the record, and the owner reads the answer in the same place. The discussion that shaped a record stays beside it rather than in a chat log that is gone the next day.

## How a lesson becomes a check

When something goes wrong twice, the fix is a check rather than a note, because a note is read only by whoever already knows to look for it. The pattern is the same each time: write the check, plant the failure it exists for and watch it fail, then run it from `just check` and in CI.

- The boundary rules were partly a text search and partly review alone, and the search missed whole forms of import. They are now semgrep rules, each tested against a fixture that holds a violation.
- A check twice passed silently on a machine where the tool it needed was missing. Every check now fails when its tool is absent.
- When the design records moved to their present process, every citation of a record in the tree changed at once. A check now fails when a cited record has no file of its own kind: a proposal, a plan, or a legacy RFC or EP.
- The agent and skill files drifted from the code twice: an agent cited a class at a line it had long left, and another counted files that had been removed. A check now fails when a file and line cited under `.claude/` points at a missing file or past its end.

What a check cannot see, such as whether a design is sound or whether a paragraph still describes the code it names, stays with review. The repository's own check scripts say which is which: each ends by naming what it examined, so a pass is not mistaken for a reading.

## Where the parts live

| Part | Where |
|---|---|
| The agents | [`.claude/agents/`](https://github.com/mox-labs/matra/tree/main/.claude/agents) |
| The skills | [`.claude/skills/`](https://github.com/mox-labs/matra/tree/main/.claude/skills) |
| The review command | [`.claude/commands/review.md`](https://github.com/mox-labs/matra/blob/main/.claude/commands/review.md) |
| The rules Claude works under | [`CLAUDE.md`](https://github.com/mox-labs/matra/blob/main/CLAUDE.md) and [`AGENTS.md`](https://github.com/mox-labs/matra/blob/main/AGENTS.md) |
| Who decides, and how a release runs | [`CONTRIBUTING.md`](https://github.com/mox-labs/matra/blob/main/CONTRIBUTING.md) |
| The records and the process | [`blueprints/`](https://github.com/mox-labs/matra/blob/main/blueprints/README.md) |
| The boundary rules | [Boundary rules](../reference/boundary-rules.md) |
| The local checks | [`justfile`](https://github.com/mox-labs/matra/blob/main/justfile) and [`scripts/`](https://github.com/mox-labs/matra/tree/main/scripts) |
