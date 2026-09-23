# 0017. The Harness: One Suite, Two Runners, Durable Facts

- **Status:** proposed
- **Date:** 2026-09-22
- **Decider(s):** owner direction (a harness that carries the deterministic
  work, loads wherever the project loads, and leaves the reasoning to the
  steward); evidence and options by the maintainer role

## Context

matra is stewarded by an agent with a human directing. That arrangement only
works if the mechanical half is genuinely mechanical: a rule that lives in a
document is enforced by whoever happens to have read it, and a session that
must remember a rule will eventually not.

Today the project has a lot of harness and not much enforcement. Nine skills,
seven agents, eleven scripts, twenty-three `just` recipes, seven workflows,
thirteen required checks. The audit of 2026-09-22
([survey](../surveys/2026-09-22-harness-enforcement.md)) found the problem is
not coverage but three structural defects:

1. **Checks that cannot fail.** `scripts/check-boundaries.sh` reports success
   and exits 0 when `rg` is absent, reproduced by hand. Its `|| true` absorbs
   "no matches" and "no such binary" identically.
2. **Claims about the harness that are not true.** A script header says it runs
   in CI and it runs in no workflow; a `justfile` comment names an environment
   variable nothing sets; the installed hook is from the repository's previous
   name and claims a local pass implies a CI pass.
3. **The expensive rules are the unenforced ones.** Boundary rules 1, 2, 5 and
   7 have no mechanical check at all, and neither does "no panics in library
   code", the resilience floor on new code, or "methods do not cross FFI".

Three failures during the 0.2.x releases came from the same family. A tag guard
read a 404 body as an existing tag. A sandbox snapshot reported clean for
writes it could not see. A dependency scanner was green while an advisory sat
unfixed. In each case the check answered without having looked.

The forces in tension: enforcement should be fast enough to run constantly and
strict enough to be worth trusting; the local loop should not require a network
or a vendor; and the agent doing the stewarding should spend its judgment on
what needs judgment, not on re-deriving what a script already knows.

## Options considered

### Option A: add the missing tools

Bring in semgrep, `cargo-public-api`, `typos`, secret scanning and coverage
thresholds, and wire them into CI.

**Pros:**
- Directly addresses the defect kinds no layer owns today.
- Each tool is independently useful and well understood.

**Cons:**
- Builds on a foundation where a check cannot distinguish clean from absent.
  New tools inherit the defect: three of the candidates have a silent-pass mode
  of their own (a scanner whose SARIF output always exits 0, a linter skipped
  when its binary is missing).
- More checks in a place the repository is already wrong about, since several
  existing checks do not run where their own headers say they do.
- Does nothing for the rules that cost the most, which need an import-graph
  assertion rather than another scanner.

### Option B: repair what exists, then layer

Make every check fail when its tool is absent and report what it examined; wire
the checks that exist into CI; make the skippable lanes required; then add
tooling against a foundation that holds.

**Pros:**
- The cheapest work with the highest value: today's checks become worth their
  green.
- Makes later additions safe, because the property they must have is
  established first.

**Cons:**
- Adds no new coverage on its own. The defect kinds with no owner stay unowned
  until the second step.
- Requires touching many small files at once, which is a review burden with
  little visible product.

### Option C: one environment, one suite, and facts in the repository

Option B, plus three structural commitments: a pinned container that carries
every tool the deterministic suite needs, so local and CI stop being two
different suites; a single suite definition both runners consume; and every
check writing its result into the repository as data rather than only an exit
code, so the reasoning layer reads facts instead of re-running tools.

**Pros:**
- Removes the tool-absence failure mode by construction rather than by guard,
  since the container always has the tools.
- Ends the drift: the local suite is the definition and CI runs it, plus a
  named set that only a remote runner can do.
- The durable facts make the steward's orientation cheap and offline: what is
  unreleased, which advisories are open, what the public surface was last
  release, what the last scan actually examined.
- Unlocks coverage that exists but never runs: the model-gated conformance and
  integration tests do not execute in CI today, because the models are not
  there.

**Cons:**
- An image is a new artifact to pin, rebuild and watch. ADR-0014 already
  records that nothing watches the maturin image digest, and this adds a
  second.
- A container tempts us to run everything in it, which would hide exactly the
  platform differences the e2e sandbox suite exists to catch (BSD, GNU and
  busybox behave differently, and macOS wheels cannot be built on Linux).
- Baking models into an image is redistribution, which is a licence question
  before it is an engineering one, and it would stop exercising the
  provisioning path that two releases were spent hardening.

## Decision

We choose Option C, sequenced so that Option B lands first and stands alone.

The harness is three layers, not four. An earlier sketch separated "CI checks"
from "committed facts"; they are the same layer seen twice, and the distinction
that matters is not where a check runs but whether its result outlives the run.

**Layer 1, the deterministic suite.** One definition, `just check`, which is
what correct means. It runs on a contributor's machine, in a pinned container,
and in CI. Every check in it obeys two rules:

- **Fail on absence.** A check whose tool is missing fails and says so. Silence
  is never a pass.
- **Report the denominator.** Every check states what it examined: files
  scanned, rules evaluated, pages checked. "Exited 0" is not evidence.

The remote runner adds only what a local machine cannot do: the platform
matrix, wheel builds across architectures, registry propagation, environment
approvals and provenance attestation. That set is named in the workflow, and a
test asserts the two lists agree, so drift fails on the pull request that
causes it rather than in an audit months later.

**Layer 2, durable facts.** Anything the suite computes that a later reader
needs goes into the repository as data: the advisory snapshot with its
timestamp and tool versions, the public API snapshot, coverage, the scan
counts. Facts carry their own freshness, and stale is a finding rather than a
silence.

**Layer 3, the steward.** The agent reads layers 1 and 2 as data rather than
re-deriving them, and spends judgment on the three things a script cannot do:
whether a finding matters, what to do when two rules conflict, and what no rule
covers. Refusal belongs here too: the rules that are currently prose a session
must remember (`rip` rather than `rm`, never `cargo test --all-features`, never
a publish path that skips the gates) become hooks that decline, because a rule
enforced by memory is enforced by luck.

**The self-description rule.** Every claim the harness makes about itself is
checkable, and checked. If a script says it runs in CI, a test finds it in a
workflow. If a comment names an environment variable, a test finds it set. All
three false claims the audit found were of this kind: nobody lied, the claims
stopped being true, and nothing noticed. For a project meant to be an exemplar,
the distinguishing property is not that an agent maintains it, but that its
claims about itself fail loudly when they stop holding.

**The container's boundary.** The image carries the deterministic suite, and
never the platform matrix. Models are baked only if their licence permits
redistribution, which is checked before the image is built, and one lane always
starts cold so the provisioning path stays exercised.

## Consequences

- Positive: a green suite means the checks ran, which is not true today.
- Positive: the local loop needs no network and no vendor. A clone plus the
  image is the whole environment.
- Positive: the steward's orientation becomes a file read rather than a dozen
  API calls, which also means a human, or a session five years from now, can
  reconstruct the project's state from the repository alone.
- Positive: boundary rules 1, 2, 5 and 7 become assertions in a test that runs
  in an already-required lane, rather than review discipline.
- Negative: a container image and its digest join the release checklist, beside
  the maturin image ADR-0014 already records as unwatched.
- Negative: making the four skippable lanes required means a flaky lane blocks
  merges. That is the intent, and it will be felt.
- Neutral: the hook gets smaller, not larger. Its unique contribution costs 40
  milliseconds; the eight cargo invocations it currently runs duplicate
  required CI jobs and are why it cannot be mandatory in its present shape.
- Neutral: "every check reports its denominator" is a convention, and
  conventions decay. It is cheap to add to a shared shell preamble and cheap to
  assert in the self-description test.

## Validation

This decision is right if, a release or two from now, a green run means the
checks examined something, the repository's statements about its own harness
are all executable, and a session can orient offline from committed facts.

It is falsified by any of:

- A check that passes with its tool absent, after this lands. The survey's
  reproduction command is the regression test.
- The suite becoming slow enough that contributors stop running it locally,
  which would return us to CI as the only real gate and make the local-first
  commitment a fiction.
- The container swallowing the platform matrix, visible as a class of bug that
  reaches a release because everything ran on one Linux image.
- The durable facts going stale without anyone noticing, which would mean the
  freshness reporting is decorative.
- The refusal hooks proving so noisy that they are disabled, which would be
  evidence they encode the wrong rules rather than that enforcement is wrong.

## References

- [`docs/surveys/2026-09-22-harness-enforcement.md`](../surveys/2026-09-22-harness-enforcement.md),
  the evidence for every claim in the Context section.
- ADR-0005 (supply-chain hardening: SHA pinning, least privilege, the scanners
  this decision would finally run), ADR-0014 (the pinned image precedent and
  the unwatched-digest consequence), ADR-0016 (release automation, whose
  standing release PR is the same shape: compute mechanically, decide humanly).
- `scripts/check-boundaries.sh`, `scripts/check-docsite-floor.sh`,
  `scripts/pre-commit-hook.sh`, `justfile`, `.github/workflows/ci.yml`.
