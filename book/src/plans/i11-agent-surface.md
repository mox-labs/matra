# I11: The agent surface

**Boundary:** additive to the 0.1.0 and I10 surfaces. One new global flag and its references; new files at the repository root and under `skills/`; no existing signature changes.

**Origin:** the roadmap's "Agent surface" trigger fired on 2026-09-05 by owner direction, sequenced after I10 so the skill documents one CLI with a pinned JSON contract. [ADR-0012](https://github.com/mox-labs/matra/blob/main/docs/decisions/0012-agent-surface.md) records the decisions.

---

## Why this shape and not another

### Two doors, both first-class

The docsite is for human comprehension: concept pages, citations, benchmarks a reader can take in at a glance. The skill is for an agent about to run matra: self-contained, exact, and executable. Neither is derived from the other. Both are derived from the code, and each has its own test against it: the docs-floor gates for one, the executed-incantation test for the other.

### The binary is the source of truth for its own instructions

The one precedent the survey found states the reason plainly: instructions served from the installed binary always match it. So the skill text is embedded with `include_str!` and printed by the program it describes. The same files, in the standard plugin layout under `skills/matra/`, are what a marketplace distributes.

### A skill whose commands do not run is a defect

Every `matra ...` command in the skill is executed by the test suite through `cli::run`. Drift between the text and the CLI fails CI, the way a stale type name fails the docsite.

## The surface

```text
matra --skill                 SKILL.md (frontmatter + body under 150 lines)
matra --skill -r              the references, one per line: name, one-line summary
matra --skill -r <name>       one reference
matra --skill --json          {"format_version": 1, "command": "skill", "input": null, "result": {"name": "SKILL", "body": "..."}}
matra --skill -r --json       {"format_version": 1, "command": "skill", "input": null, "result": {"references": [{"name", "summary"}, ...]}}
matra                         unchanged: usage error, exit 2
matra analyze x --skill       the flag wins; the subcommand is ignored
```

```text
skills/matra/SKILL.md                 name, description, version (= crate version); the body
skills/matra/references/json.md       the envelope; the Document shape field by field
skills/matra/references/structure.md  tokens, arcs, structural primitives, the tree example
skills/matra/references/metrics.md    each measure: formula, what it does not mean
skills/matra/references/semantic.md   clusters, the threshold, provisioning, model_hash
skills/matra/references/python.md     the API when the agent writes Python; the Embedder protocol; analyze_path
skills/matra/references/errors.md     every error, its Python exception, what to do
.claude-plugin/plugin.json            the repository as a plugin; skills auto-discovered under skills/
AGENTS.md                             for contributing agents; points at CLAUDE.md
book/src/llms.txt                     generated from SUMMARY.md; gate checks it is current
CITATION.cff                          lands when the author form is settled
```

## Milestones

Each milestone is one PR, review-hardened by the CI harness before merge. Strict order.

### M1: the ADR, the plan, the roadmap

ADR-0012 accepted; roadmap entry points here; plans index and `SUMMARY.md` carry this page.

**Rubric.** `just docs-floor` passes. The ADR names the flag, the reference set, the file layout, and the test, so a later reviewer can diff the surface against the decision.

### M2: the content and its test

`skills/matra/SKILL.md` and the six references, written from the code and the fixtures: every command is one the CLI accepts today, every JSON shape is the serde form the conformance fixtures pin, every number's meaning is the methodology page's, compressed. `tests/skill.rs` extracts every fenced command starting with `matra ` and runs it through `cli::run` against `spec/` fixture inputs; blocks may carry `<!-- expect: exit 1 -->` and `<!-- needs: model -->` annotations; `--json` blocks must parse and carry `format_version`. The docsite floor's type-name gate extends to `skills/`.

**Rubric.** The test runs every incantation in the skill and its references (count asserted against a grep, so an unfenced command cannot hide). SKILL.md body under 150 lines; each reference under 200. No em dashes; plain vocabulary; product-agnostic. A reader with no other document can install matra, run each command, and read every field the envelope carries.

### M3: the flag

`--skill`, `-r` / `--reference [NAME]`, and their `--json` shapes in `src/cli/`, content embedded with `include_str!`; the reference list and summaries come from each reference's frontmatter, not a second list. Unknown reference name: exit 2 naming the known ones. `python/tests/test_cli.py` asserts `--skill` output is byte-identical between the Rust binary and `cli_main`. CLI guide gains the flag; the errors page's exit-code table if anything changes.

**Rubric.** `uvx --from . matra --skill` prints the same bytes as the binary; `--skill -r` lists exactly the files under `references/`; `cargo package --list` shows every file under `skills/matra/` (the manifest uses an `exclude` list, so the check is that nothing excludes them); bare `matra` still exits 2 with the usage error, and `--skill` beside a subcommand wins, both asserted in `tests/cli.rs`; every `--json` shape is the one envelope with the payload under `result` and `input` null.

### M4: alongside the flag

`scripts/gen-llms-txt.sh` writes `book/src/llms.txt` from `SUMMARY.md` (H1, one-paragraph summary, H2 per section with links to the deployed pages); a docs-floor gate fails when the file is stale. The file sits under `book/src/` rather than `book/`, because mdbook copies every non-chapter file there into the built site, which puts it at the site root with no step in the deploy workflow to keep in sync with the script. `AGENTS.md` at the root: build, gates, boundary rules, PR ritual, in under 60 lines, pointing at `CLAUDE.md` for detail. `.claude-plugin/plugin.json` with name, description, version; `claude plugin validate` (or the equivalent check) passes; the plugin installs with `--plugin-dir .` and the skill triggers on a matra question. `CITATION.cff` is written when the owner settles the author form; until then this milestone records it as blocked, not done.

**Rubric.** `just docs-floor` includes the llms.txt gate; `AGENTS.md` restates nothing `CLAUDE.md` says (links instead); the plugin validates.

### M5: lockstep and release readiness

CHANGELOG (Added: the flag, the skill, llms.txt, AGENTS.md, the plugin; Changed: none), the pragmatics page gains "For an agent" (one paragraph: run `matra --skill`), README first screen gains the one line "If you are an agent, run `uvx matra --skill`". `cargo publish --dry-run` and `maturin build` green; a 0.2.0 version bump proposed to the owner with the changelog rolled, publishing itself remaining owner-approved per the project rule.

**Rubric.** `just check` and `just conformance` green; the release dry-runs pass; nothing publishes.

## Costs, named

- The skill text is public surface under semver; an incantation change is a changelog item.
- Binary and wheel grow by the embedded text (tens of kilobytes).
- One more docs-floor gate (llms.txt currency) and one more test lane (the skill runner).

## Risks

- **A skill that reads well but is wrong.** The executed-incantation test covers commands, not prose claims about numbers; the type-name gate covers identifiers. Prose about what a metric means is checked by review against the methodology page, which the reference cites by section.
- **Frontmatter drift.** `version` in SKILL.md must equal the crate version; a test asserts it from `CARGO_PKG_VERSION`.
- **Two launchers, one text.** The parity test exists for exactly this.
- **CITATION.cff blocked on attribution.** Named as blocked; nothing in M4 waits on it.

## Acceptance gate

`uvx matra --skill` prints a skill whose every command the test suite executed against the same build; `--skill -r` lists the references and each prints; output is byte-identical between launchers; `book/llms.txt` is current by gate; `AGENTS.md` and the plugin manifest exist and the plugin installs; every rubric above holds.
