---
name: e2e-validation
description: How matra verifies that a built artifact actually installs and works for a real user. Two layers, deliberately separate: mechanical gates that run in CI and block a release, and an exploratory pass an agent runs before a release that produces a report a human judges. Use before cutting a release, after changing how matra is built or distributed, or when a documentation page makes a claim nobody has executed.
---

# e2e-validation

Everything else in this repository tests matra against its own source tree. This skill covers the one question that cannot be answered there: does the thing we published work for someone who has none of what we have.

The evidence for the whole approach is in `docs/surveys/2026-09-06-release-validation.md`, which read nine comparable projects, and in the two pilot passes at `docs/e2e/2026-09-06-macos-walkthrough.md` and `docs/e2e/2026-09-06-linux-cleanroom.md`. Those reports are kept in the repository because this skill's own evidence rule applies to itself: a method whose grounding has evaporated is a method nobody can check.

## The two layers, and why they must not be confused

| | Mechanical gates | The exploratory pass |
|---|---|---|
| Runs | in CI, every release | before a release, on request |
| Produces | pass or fail | a report |
| Decides | itself | a human |
| Question | does the documented thing still work | what is it actually like |

Conflating them is the failure mode. A gate that depends on judgment is a gate that gets disabled the first time someone disagrees with it. A judgment pass dressed up as a gate teaches people to argue with a red check instead of reading the finding.

What the survey establishes is narrower than it first looks, and the narrow version is the stronger argument.

**None of the nine runs the second layer.** No chartered exploratory pass, no judgment step, anywhere in release engineering. That half is flat.

The first layer is not universal either. mdBook builds a release binary per target and never executes it, not even a version check. cargo-release is a tool other crates use rather than a project shipping its own artifact, so it is barely one of the nine. And by the bar set immediately below, two of the rest only half qualify: ruff checks with `ruff --help` and ripgrep with a version and a completion dump, which prove the entry point resolves and not that the program works. In fairness the survey rates ripgrep's coverage unusually thorough for a CLI, and it does execute the artifact under emulation; the bar being applied here is this skill's own, not a mark against them.

So the accurate reading is that every project which checks at all checks mechanically, in CI, and never asks a person to judge the result, while one checks nothing and two check only that the binary starts.

**Why keep the second layer anyway.** The survey reports the session lengths the method's own literature uses, 45 minutes to two hours, so two charters was most of someone's afternoon, every release. That is the cost that used to make this uneconomic, and it is not the cost any more. The survey explicitly declines to say whether the second layer is worth having, and it is right to decline, because it gathered no data on that question. So this is matra's own call rather than a conclusion borrowed from anyone, and the evidence it rests on is that the two pilots found six blocking defects a fully green CI matrix had passed.

Never let the second layer block a merge, and never make it a CI job. If it becomes one, that is the signal it has been misunderstood.

## Layer one: what CI must prove

The gate installs the built artifact the way a user would and then uses it. Import is not use.

1. **Install from the built artifact, not the source tree.** The wheel that was built in this run, or the binary that was built in this run.
2. **Run real commands, not a version check.** A version string proves the entry point resolves. It does not prove the program works. Run an actual analysis and assert on the output.
3. **Cover the platforms the documentation promises.** Every architecture, every libc floor, and every interpreter version the install page claims. A promise made on a page with no matching lane is a promise nobody checks.
4. **Execute the documented commands.** Rust doctests already do this for the library. `tests/skill.rs` already does it for the agent skill, extracting every `matra` command and running it, with a count law so a command written outside a fence cannot escape the runner. That pattern is the model; the gap is the docsite.

Absences are findings. When a lane does not exist, say so rather than assuming coverage.

**Where matra stands against this.** Point 1 holds for the wheel, and point 4 holds for the library and the agent skill. Point 2 does not: the release smoke test installs the wheel and then runs an import, which is the check point 2 rules out. Point 3 became true only when the wheel matrix was fixed, and the missing Linux ARM wheel is what its absence cost. The docsite's own commands are executed nowhere.

## Layer two: the exploratory pass

### The charter

Every pass starts from a written charter, because an unbounded "try it out" produces anecdotes. A charter names:

- **The artifact.** Which build, which commit, which platform.
- **The environment.** What must be absent for the test to mean anything.
- **The perspective.** Who the tester is pretending to be, and what that person does not know.
- **The question.** What this pass is for. One sentence.

Two charters have earned their place and should be run for every release: a first-time reader following every page on the maintainer's own platform, and a clean-room install on the platform most users are actually on.

### Behave like a person, not a script

This is the part that carries the value, and it is easy to lose.

The single most-felt defect either pilot found was that the first run writes nothing to the terminal for between 3 and 35 seconds while it downloads a model. No assertion catches that. Every existing gate passed. It was found because someone sat and watched a blank terminal and noticed how it felt.

So: run one command at a time, in the order a reader meets it. Write down what you expected before you run it. Do not batch commands into a script and diff the output at the end, because a script has no patience to lose. When a page makes you guess what it meant, stop and record the guess, because the guess is the finding. You may read the source afterwards to explain it, but say plainly that a real user could not have.

Measure what the user actually waits for. Compile time is not it, because a published wheel arrives built.

### Report only

The pass never fixes what it finds. Two reasons. A tester who fixes things stops being able to say what the experience was, and a fix that skips normal review is a fix nobody checked. Findings go to a report; the repair goes through the ordinary pull request ritual.

### Evidence rules

These are not style. A report that breaks them is not usable, because the reader cannot tell a finding from a guess.

- **Every claim cites its source.** A file and line for a documentation claim. The actual command and its real output for a behavior claim. An uncited claim is dropped, not reported.
- **"Nothing found" is a valid answer and must be stated explicitly.** Without that permission the tester fills the gap from its own expectations. Both pilots' clean lists are load-bearing: knowing that matra needs no CA certificates, and that it writes exactly two paths, is worth as much as knowing what broke.
- **Report the warrant.** What was verified, what could not be and why, and what the pass declines to claim.

### The finding taxonomy

Three groups, ranked inside each by how early a user meets it.

- **Blocks release.** A documented statement that is false, a printed command that fails, a platform the docs promise and the build does not serve, data loss, a wrong path.
- **Hurts first impression.** Slow, silent, confusing, or missing a signal. Nothing is broken and the user does not know that.
- **Cosmetic.** True but imprecise.

The middle group is where the value concentrates and where a pass-or-fail gate would have found nothing.

## The sandbox contract

The environment is the experiment. A pass run in a polluted environment proves nothing and is worse than no pass, because it produces a clean report.

- **Scrub the home directory.** Point `HOME`, `XDG_CONFIG_HOME` and `XDG_DATA_HOME` inside a temporary tree and unset every `MATRA_*` variable. `scripts/e2e-sandbox.sh new` does this. Take `scripts/e2e-sandbox.sh snapshot` before the pass and again after it, from outside the sandbox both times, and diff the pair. An identical pair is the evidence that the pass stayed inside, and the report should say so. The command refuses to run inside a sandbox, because in there it would fingerprint the sandbox and report a false all-clear.
- **Use a container for the install claim.** The maintainer's machine has a Rust toolchain, a C++ compiler, uv, certificates and cached models, so it structurally cannot answer whether a clean machine works. The pilot that found the glibc floor and the undocumented C++ requirement found both in containers, and the host could not have found either, because it has a C++ compiler and a glibc far newer than the floor. The missing ARM wheel is a weaker example and worth being honest about, because it was inferable from reading the publish workflow and the host pass read that same file and did not notice. The container is what made it impossible to overlook, not what made it findable.
- **State the architecture.** On Apple Silicon, containers are ARM by default. Emulated timings are not user timings, so say which you ran.
- **Hunt what a developer machine hides.** No certificates, no network, no writable home, a read-only filesystem, a full disk, an interrupt mid-download, a second run sharing a model directory, and a filesystem diff of everything the program created. That list is where the pilots' best findings came from.

## What must not be delegated

The pass gathers evidence. It does not decide whether a finding blocks a release, and it does not decide what to build in response. A confidently wrong summary is indistinguishable from a right one, so the judgment stays with a person reading cited evidence.

## After a release

Two things can only be checked once the artifact is public: that the install command on the front page resolves to the version that has the feature it describes, and that the wheel the registry serves is the one the matrix built. Both belong on the release checklist, not in CI.
