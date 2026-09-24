# 0012. The agent surface: a skill the binary prints

- **Status:** accepted
- **Date:** 2026-09-06
- **Decider(s):** project owner (direction), maintainer (shape)

## Context

matra is built to be run by agents at least as often as by people.
An agent that is about to run matra needs its semantics: what it is
for, when to reach for it, the exact commands and their JSON shapes,
how to read the numbers, and the limits. Today that lives only in the
docsite, which an agent reaches by link and by prior knowledge, and in
the CLI's `--help`, which is a reference for humans and carries none
of the semantics.

The owner's direction (2026-09-05): a `--skill` flag that prints a
self-contained description an agent can use directly, with progressive
disclosure (`--skill` for the top level, `--skill -r <name>` for one
deeper reference), so that sharing `uvx matra --skill` with an agent
is enough. The docsite stays the human door, optimized for
comprehension, with citations and readable benchmarks; the skill is
the agent door. Neither derives from the other; both derive from the
code and are tested against it.

The conventions survey (`docs/surveys/2026-09-05-conventions.md`)
found one precedent for a CLI printing its own agent instructions with
a second tier of detail (Vercel Labs' `agent-browser`: `skills list`,
`skills get <name>`, `--full`). Its stated reason is version
coherence: instructions served from the installed binary always match
it, where a cached copy may not. Everything else in the survey is a
static file (`llms.txt`, `AGENTS.md`, a skill on disk), a package
manager for other tools' skills, or an MCP server.

## Options considered

### Option A: docs only, plus `llms.txt`

**Pros:** nothing new in the binary. **Cons:** the agent needs a link
and a fetch; the text can lag the installed version; nothing verifies
that the commands in it run.

### Option B: a skill file in the repository only

A `skills/matra/SKILL.md` an agent installs by hand or via a plugin.
**Pros:** the standard shape; marketplaces distribute it. **Cons:** the
installed skill and the installed binary drift independently; the
human still has to know the file exists.

### Option C: the binary prints its skill, and the same files ship as the plugin

`matra --skill` prints a `SKILL.md` embedded in the binary with
`include_str!`; `--skill -r` lists references and `--skill -r <name>`
prints one. The source files live at `skills/matra/` in the standard
plugin layout, so the plugin and the flag are one set of files.
**Pros:** version coherence for free; one source; the human's
hand-off is one command. **Cons:** binary size grows by the text
(tens of kilobytes); the content becomes public surface with the
crate's semver.

### Option D: an MCP server

**Pros:** structured tool calls. **Cons:** a second protocol surface
to keep in lockstep; the CLI plus JSON already is the tool interface;
revisit when a caller asks.

## Decision

We choose Option C.

**The flag.** `--skill` is a global flag on the CLI. Alone it prints
`SKILL.md`. With `-r` (`--reference`) and no name it prints the list of
references, one per line with a one-line summary; with `-r <name>` it
prints that reference. `--json` keeps the one envelope every `--json`
invocation emits (`format_version`, `command`, `input`, `result`):
`command` is `"skill"`, `input` is `null` because no document is read,
and `result` is `{"name": "SKILL" | "<reference>", "body": "<text>"}`
for the top level or one reference and `{"references": [{"name",
"summary"}, ...]}` for the list. `--help` is untouched and stays the
human reference clap generates. Two behaviors are pinned: bare `matra`
with no flag and no subcommand keeps its usage error and exit code 2,
and when `--skill` is combined with a subcommand the flag wins and the
subcommand is ignored, since the flag is a property of the program. A
subcommand (`matra skill get <name>`) was considered and rejected
because the owner's hand-off is `uvx matra --skill`, and a flag reads
as a property of the program rather than an action.

**The content.** `skills/matra/SKILL.md` (frontmatter `name`,
`description`, `version` equal to the crate version, body under 150
lines) and `skills/matra/references/*.md`, one file per reference.
Both are embedded with `include_str!` and shipped in the crate and
the wheel, so the text always matches the installed binary. The body
covers: what matra is in three lines; when to reach for it; install and
first run; every command with its `--json` envelope; how to read the
numbers and what they do not mean; limits and errors; how to spot
where matra applies in a user's work and what to propose; the list of
references. References: `json` (the envelope and the `Document`
shape), `structure` (tokens, arcs, structural primitives),
`metrics` (formulas and limitations), `semantic` (clusters, threshold,
model provisioning), `python` (the API and the `Embedder` protocol),
`errors` (every error, its Python exception, what to do).

**The test.** `tests/skill.rs` extracts every fenced `console` or
`bash` block whose first line starts with `matra ` from `SKILL.md` and
every reference, runs it through `cli::run` against a fixture input,
and asserts the exit code the block annotates (default 0) and, for
`--json` blocks, that the output parses and carries `format_version`.
Blocks that need a model are annotated and run in the model-gated
lane. The docsite floor's type-name gate extends to `skills/`. A Python
test asserts `--skill` output is byte-identical between the Rust
binary and the Python launcher. A skill whose commands do not run is
a defect, not a docs nit.

**Alongside the flag.** `llms.txt` at the docsite root, generated from
`SUMMARY.md` by a script and checked current by a docs-floor gate.
`AGENTS.md` at the repository root for contributing agents: build,
gates, boundary rules, pointing at `CLAUDE.md` rather than restating
it. `.claude-plugin/plugin.json` so the repository installs as a plugin
(`--plugin-dir`), with a marketplace entry as a later, owner-decided
step. `CITATION.cff` so the research behind each measure is citable
from the repository page; it lands once the owner settles the
canonical author and copyright form, because it names the author.

## Consequences

- Positive: the agent hand-off is one command and never stale; the
  skill's commands are executed in CI; plugin and flag share one
  source.
- Positive: the JSON envelope's `format_version` is now referenced by
  the skill, which makes the stability statement in the CLI guide a
  contract an agent depends on.
- Negative: the skill text is public surface; wording changes that
  alter an incantation are breaking for agents and go through the
  changelog like an API change.
- Negative: binary and wheel grow by the embedded text.
- Neutral: `--help` is unchanged.

## Validation

Right if, at 0.3.0, the skill test has caught at least one drift
between the skill and the CLI before release, and no agent-facing
issue reports a command in the skill that does not run. Falsified if a
caller needs structured tool calls the CLI-plus-JSON cannot express;
that reopens Option D as its own ADR.

## References

- ROADMAP.md, "Agent surface" (trigger fired 2026-09-05)
- [ADR-0011](0011-out-of-the-box.md): the one CLI and the JSON envelope
  this surface documents
- [Conventions survey](../surveys/2026-09-05-conventions.md), section 3
- `book/src/plans/i11-agent-surface.md`: the execution plan
