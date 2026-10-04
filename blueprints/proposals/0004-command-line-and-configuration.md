# EPR-0004: The command line, configuration and the agent skill

- Feature Name: `command_line_and_configuration`
- Start Date: 2026-10-04
- Proposal PR: [#133](https://github.com/mox-labs/matra/pull/133)
- Tracking issue: none (a baseline proposal describes code that already ships, so there is no milestone to track)
- Status: proposed (only the owner changes it to `accepted`)
- Pinned at: `4fcfb4adc85524243c5e392becd4f490e3e42252`

## Summary

There is one `matra` command line, written in Rust as the library module
`matra::cli`, whose whole surface is `cli::run(args, out, err) -> u8`. Two
launchers call it: the Rust binary and the Python entry point that `pip` and
`uvx` install. Every `--json` result is one envelope with a version number.
Where things live and what the defaults are come from `Config`, which
resolves each key from an explicit argument, then the environment, then a
TOML file, then defaults compiled into the crate, and records which of those
each value came from. Configuration never decides what a call computes. The
binary prints its own agent instructions with `matra --skill`, embedded from
the same files a plugin distributes, and a test runs every command in them.

This is a baseline proposal. It describes the code at the pinned commit and
carries forward the reasons that still hold from RFC-0011, RFC-0012,
RFC-0020, EP-0010 and EP-0011. A later proposal that changes any of it says
so. Unmarked statements about the code are observed at the pinned commit
through the link beside them; a claim that is inferred or assumed says so.

<pragmatics>

<ask>

Accept EPR-0004 as the record of the command line, configuration and the agent skill as they stand at `4fcfb4a`, and settle the two decisions under [Unresolved questions](#unresolved-questions). Each can be answered on its own.

</ask>

<will>

- Cite this proposal, rather than the legacy records it carries forward, when a change touches the command line, `Config`, the `--json` envelope or the skill; a change to what it describes goes in a new proposal that says so.
- Carry out each decision as you rule it, through the record the process names for that change.
- If the cited code moves before you decide, move the `Pinned at` line forward in a revision, so every citation is checked again, and say what changed.

</will>

<needs>

- An answer to the proposal and to each of the two decisions: accept, accept with a reservation, object or redirect.

</needs>

<wont>

- Change the status to `accepted`, or merge a pull request that does.
- Change the return type of `cli::run` or the shape of a `--json` failure before you rule on it.
- Edit a legacy record, or edit this proposal to describe a later change.

</wont>

<silence>

The proposal stays `proposed`. The command line it describes ships as it is, since a baseline decides nothing new, and Claude lists the open decisions again at the start of the next session that touches the command line or configuration.

</silence>

</pragmatics>

## Motivation

**One program, not two that agree by inspection.** 0.1.0 shipped two command
lines: the Rust binary and a Python re-implementation in click and rich so
that `uvx matra` worked. They drifted: the Python one grew a flag the Rust
one lacked, lacked one the Rust one had, and had earlier re-implemented
passive detection and got it wrong
([RFC-0011](../legacy/rfcs/0011-out-of-the-box.md)). The command line now
lives in the library because a binary target cannot be reached from PyO3,
and maturin's own guidance for a crate that is both a library and a tool is
a Python entry point that calls into the extension.

**Working with no setup, on every surface.** Before 0.2.0 the library
required a model directory every caller re-derived, under a path
(`~/.matra`) no developer tool on Linux or macOS uses
([RFC-0011](../legacy/rfcs/0011-out-of-the-box.md)). `Config` gives every
surface one resolution order and the XDG locations.

**Configuration is where things are, not what happens.** A config format
that reaches into the library would put pipeline policy in a file the
library parses. So the file carries only what a caller could pass as an
argument; which metrics run and how output looks stay with the caller and
the binary (RFC-0011, option B over option C).

**An agent's instructions should match the installed binary.** An agent
reaches the docsite by link and prior knowledge; `--help` carries no
semantics. Instructions printed by the program they describe cannot be
stale against it ([RFC-0012](../legacy/rfcs/0012-agent-surface.md)).

## Guide-level explanation

### Running it

```console
matra analyze notes.md                 # a table of metrics
matra analyze notes.md --json          # the same, as JSON
matra summarize report.txt -n 3 --method textrank
matra keyphrases - --stdin-filename draft.md < draft.md
matra config show                      # every setting and where it came from
matra --skill                          # the agent skill
```

<claim basis="observed">The commands are `analyze` (with `--sections`), `summarize`, `keyphrases`, `config show`, `config init [--force]` and `completions <bash|zsh|fish>` [cli/mod.rs:112-242][cli-commands]</claim>.
<claim basis="observed">The global flags are `--json`, `--model-dir`, `--quiet`, `--color`, `--stdin-filename`, `--skill` and `--reference` [cli/mod.rs:112-242][cli-args]</claim>.
`-` reads stdin, and `--stdin-filename` both labels it and chooses its
decomposer by extension.

<claim basis="observed">Exit codes follow ripgrep: 0 when something was found, 1 when the command succeeded and found nothing, 2 on an error; a broken pipe is 0 [cli/mod.rs:1-19][cli-docs]</claim>.

### Reading `--json`

<claim basis="observed">Every `--json` invocation emits one object with four keys [cli/mod.rs:677-709][envelope-struct]</claim>:

```json
{ "format_version": 1, "command": "analyze", "input": "notes.md", "result": { } }
```

`result` is the serialized domain value unchanged
([EPR-0002](0002-data-model.md)). `input` is `null` for a command that
reads no document.
<claim basis="observed">`format_version` increments on any change to the envelope's shape or to the meaning of a field in `result` [cli/mod.rs:34-39][envelope]</claim>.
<claim basis="observed">Both launchers are held to `spec/tests/cli/envelope.json` [envelope.json:1-19][envelope-fixture]</claim>.

### Configuration

`matra config init` writes the shipped defaults to the config path;
`matra config show` prints every resolved key beside its source. The file:

```toml
[models]
embedding = "potion-base-8M"

[summarize]
n = 3
algorithm = "tfidf"

[keyphrases]
n = 10
algorithm = "rake"
```

([`config/default.toml`][default-toml].) Three environment variables name
what they override: `MATRA_CONFIG_FILE`, `MATRA_DATA_DIR`, `MATRA_MODEL_DIR`.
`--model-dir` on the command line, or `Config::with_model_dir` in Rust,
outranks all three.

A file written by an earlier `config init` may also set `models.udpipe` or
`semantic.threshold`. Both still load, both are ignored, and `config show`
names each one on stderr so the line can be removed.

### For an agent

`uvx matra --skill` prints `SKILL.md`: what matra is for, every command with
its JSON shape, how to read the numbers, and the limits. `--skill -r` lists
the references (`errors`, `json`, `metrics`, `python`, `semantic`,
`structure`) and `--skill -r <name>` prints one. With `--json` each is the
same envelope, with `command` set to `"skill"` and `input` null.

## Reference-level explanation

### One command line, two launchers

<claim basis="observed">`cli::run` parses, dispatches and renders, never calls `std::process::exit`, and never touches the process's own streams: the caller passes both [cli/mod.rs:50-106][cli-run]</claim>.
<claim basis="observed">It returns a `u8` because `ExitCode` cannot be read back for the Python launcher [cli/mod.rs:50-106][cli-run-u8]</claim>.
<claim basis="observed">`src/bin/matra.rs` locks stdout and stderr, calls it, and converts with `ExitCode::from` [bin/matra.rs:1-26][bin]</claim>.
<claim basis="observed">`python/matra/cli.py` passes `sys.argv[1:]` to `_core.cli_main` and exits with the result; `cli_main` passes each argument through `os.fsencode` so a non-UTF-8 path on Unix reaches the command line as the same bytes the Rust binary would see [cli.py:1-10][py-cli] [lib.rs:776-829][cli-main]</claim>.
`pyproject.toml` installs that launcher as the `matra`
script ([scripts][py-scripts]), and the `python` feature enables `cli`,
which enables `udpipe` ([features][cargo-features]).

`src/cli/` is the application tier. Boundary rule 7 holds it to the public
surface (`Engine`, `Ingest`, `extraction`, `config`, `domain`) and never a
port or an adapter; a semgrep rule checks the imports
([EPR-0001](0001-pipeline-and-ports.md)).
<claim basis="observed">It validates an input path before building the engine, so a missing file is reported before a 16 MB download [cli/mod.rs:372-430][execute-validate]</claim>.
<claim basis="observed">The download notice goes to stderr so `--json` stdout stays one object, and `--quiet` silences it [cli/mod.rs:486-530][build-engine]</claim>.

<claim basis="observed">`--skill` outranks a subcommand: `matra analyze x --skill` prints the skill [cli/mod.rs:372-430][execute-skill]</claim>.
<claim basis="observed">Bare `matra` with neither is a usage error, exit 2 [cli/mod.rs:252-288][parse]</claim>.

### Config resolution

[`Config`][config-type] is a composition-root value beside `Engine`,
`#[non_exhaustive]` with private fields. [`Config::resolve`][config-resolve]
reads the process environment and the file; `Config::from_sources` takes
both injected, which is how tests avoid the developer's home.

| Key | Rungs, highest first |
|---|---|
| config file path | `MATRA_CONFIG_FILE`, `$XDG_CONFIG_HOME/matra/config.toml`, `~/.config/matra/config.toml` ([resolver][config-paths]) |
| `data_dir` | `MATRA_DATA_DIR`, `$XDG_DATA_HOME/matra`, `~/.local/share/matra` |
| `model_dir` | `with_model_dir`, `MATRA_MODEL_DIR`, an existing non-empty `~/.matra/models` when `data_dir/models` does not exist, `data_dir/models` ([`resolve_model_dir`][config-model-dir]) |
| `models.embedding`, `summarize.n`, `summarize.algorithm`, `keyphrases.n`, `keyphrases.algorithm` | the file, the compiled default |

matra never creates `~/.matra`; the fallback only keeps an existing cache
working. Environment values are used as paths unchanged, with no `~`
expansion, and an empty variable counts as unset.

<claim basis="observed">The file is capped at 64 KiB by its metadata before it is read (`InputTooLarge`, `what = "config_file"`), and may be a symlink, since dotfiles repositories commonly link it [config.rs:49][config-max] [config.rs:703-720][config-read]</claim>.
<claim basis="observed">Unknown keys are rejected (`deny_unknown_fields`), so a setting a user believes is in force cannot be silently ignored [config.rs:467-503][config-schema]</claim>.
An unknown algorithm name fails at resolve time.
<claim basis="observed">`models.embedding` must be a single ordinary path component, because the embedding provisioner sweeps aged temporaries inside the directory it names [config.rs:588-606][config-component]</claim>.

`Config::sources` yields each resolved key with its `ValueSource`
(`Argument`, `Environment`, `File`, `Default`), which is what `config show`
prints ([accessors][config-accessors]).

### The deprecated keys

[RFC-0020](../legacy/rfcs/0020-deprecate-unread-config-keys.md) deprecated
`models.udpipe` and `semantic.threshold`, because neither selected anything:
the UDPipe model is pinned in the adapter beside its digest, and every
clustering call takes its threshold as an argument. They left
`config/default.toml`; the schema still accepts them so a 0.2.x file loads;
`Config::udpipe_model` and `Config::semantic_threshold` carry
`#[deprecated]` and return the file's value or the former default;
`Config::deprecated_keys` lists which ones the file set; and
`Config::sources` omits them ([accessors][config-accessors]).
<claim basis="observed">`config show` writes one stderr line per deprecated key, naming the file, unless `--quiet` [cli/config_cmd.rs:82-111][deprecation-notes]</claim>.
Removing them is a breaking change and takes its own proposal.

<claim basis="observed">`config init` writes atomically: a temporary file in the same directory, then a hard link when not forced (which fails if the target appeared in the meantime) or a rename when forced [cli/config_cmd.rs:213-293][write-defaults]</claim>.

### The skill

<claim basis="observed">`SKILL.md` and six references under `skills/matra/` are embedded with `include_str!` [cli/skill.rs:1-70][skill-embed]</claim>.
Each reference's summary is read from its own frontmatter rather than kept
in a second list, and a test holds the table of names equal to the
directory.
<claim basis="observed">The frontmatter's `version` matches the crate's [SKILL.md:1-5][skill-md] [Cargo.toml:1-14][cargo-version]</claim>.
<claim basis="observed">`tests/skill.rs` extracts every fenced block whose first line is a `matra` command, runs it through `cli::run` (or as a subprocess with a clean environment, for `config`), and asserts the annotated exit code and, for `--json`, the envelope; the block count is checked against a fence-aware scan so an unfenced command cannot hide [tests/skill.rs:1-37][skill-test]</claim>.
Alongside the flag: `AGENTS.md` for contributing agents,
`.claude-plugin/plugin.json` so the repository installs as a plugin, and
`site/content/llms.txt`, generated from the docsite's `SUMMARY.md` and held
current by a docs gate.

### No longer in force

- **`semantic.threshold` and `models.udpipe` as settings.**
  [RFC-0011](../legacy/rfcs/0011-out-of-the-box.md) and
  [EP-0010](../legacy/eps/0010-foundations.md) list both among the defaults
  `Config` carries; [RFC-0020](../legacy/rfcs/0020-deprecate-unread-config-keys.md)
  deprecated them (above).
- **`book/src/llms.txt`.** [RFC-0012](../legacy/rfcs/0012-agent-surface.md)
  and [EP-0011](../legacy/eps/0011-agent-surface.md) place it under the
  mdBook source; the docsite moved, and the file is
  `site/content/llms.txt`, generated by `scripts/gen-llms-txt.sh`.
- **`CITATION.cff` as blocked.** EP-0011 records it as waiting on the
  attribution decision; [RFC-0013](../legacy/rfcs/0013-attribution-and-citation.md)
  settled it and the file exists ([EPR-0003](0003-distribution-and-provisioning.md)).

## Drawbacks

- **The config schema is public surface**, versioned with the crate, and
  `deny_unknown_fields` means a key can only leave by deprecation.
- **The `python` feature compiles `clap`**, and the wheel carries the whole
  command line.
- **The skill is public surface.** A wording change that alters an
  incantation is breaking for agents and goes through the CHANGELOG like an
  API change.
- **A deprecated key is accepted and ignored** outside `config show`, which
  is the silence `deny_unknown_fields` exists to prevent, narrowed rather
  than closed (RFC-0020).
- **The legacy `~/.matra/models` fallback** keeps capturing downloads for a
  user who has one, until they create the new location or set
  `MATRA_MODEL_DIR`.

## Rationale and alternatives

- **Configuration in the application only**: every library caller keeps
  writing the path, and "works out of the box" is true only of the binary.
  Rejected in [RFC-0011](../legacy/rfcs/0011-out-of-the-box.md) (option A).
- **A configuration schema that drives the pipeline**: puts policy in a file
  the library parses and couples the library to a presentation vocabulary.
  Rejected in RFC-0011 (option C).
- **Two command lines with a parity test**: catches drift after it happens,
  for a second implementation a launcher replaces. Rejected in RFC-0011
  (option D).
- **Docs and `llms.txt` only, or a skill file only, or an MCP server**: the
  first two drift from the installed binary and nothing runs their
  commands; an MCP server is a second protocol surface beside a CLI and JSON
  that already are the tool interface. Rejected in
  [RFC-0012](../legacy/rfcs/0012-agent-surface.md).
- **`matra skill get <name>` as a subcommand**: rejected for a flag, because
  the hand-off to an agent is `uvx matra --skill` and a flag reads as a
  property of the program (RFC-0012).
- **Delete the unread keys, accept them silently, or make them act**: the
  first breaks every 0.2.x file, the second leaves users editing keys that
  do nothing, and the third would load an unpinned model or let
  configuration decide a computation. Rejected in
  [RFC-0020](../legacy/rfcs/0020-deprecate-unread-config-keys.md).

## Prior art

- uv documents that it follows the XDG conventions on Linux and macOS; gh
  resolves `$XDG_CONFIG_HOME/gh` before `~/.config/gh`. `UV_CONFIG_FILE`,
  `UV_CACHE_DIR` and `OLLAMA_MODELS` name the thing they override, as the
  `MATRA_*` variables do.
- ruff and uv ship their Rust command lines to `uvx`; maturin recommends a
  Python entry point into the extension for a crate that is both library
  and tool.
- `cargo metadata --format-version` is the precedent for a versioned
  integer on a JSON output rather than a published schema; `cargo config get
  --show-origin` for printing where each value came from; Cargo's "unused
  manifest key" warning for a deprecated key that still loads.
- ripgrep's exit codes (0 found, 1 nothing found, 2 error), and
  no-color.org for `NO_COLOR`.
- Vercel Labs' `agent-browser` prints its own agent instructions with a
  second tier of detail, for the same reason: they cannot drift from the
  installed binary (RFC-0012's survey).

## Unresolved questions

For the owner to settle in this proposal's review. Each is a decision of its
own.

**The exit code type.**
<claim basis="observed">`cli::run` returns `u8` [cli/mod.rs:50-106][cli-run-u8]</claim>;
whether that is the permanent shape was left open by RFC-0011 and is still
open. Changing it is a breaking change.

<decision id="exit-code" title="1. Is u8 the permanent return type of cli::run?">

<choice key="a" title="Yes; keep u8">

`run` keeps returning `u8`, and its documentation says the shape is permanent and names the three codes.

</choice>

<choice key="b" title="No; a named exit status">

`run` returns a type that names its three codes and converts to both `u8` and `ExitCode`.

</choice>

<recommendation choice="a">

The reason it is a `u8` still holds: `ExitCode` cannot be read back, and the Python launcher needs the number. The three codes are documented beside `run` already, and a second type would wrap one integer to name three values.

</recommendation>

<against>

A bare `u8` admits codes the command line never returns, and only review keeps a new return path to 0, 1 or 2; a type would hold it. The change is breaking whenever it lands, so it costs less before more callers depend on the integer.

</against>

</decision>

**No error envelope under `--json`.**
<claim basis="observed">A failure writes text to stderr and nothing to stdout, so a JSON consumer reads text for every failure [cli/mod.rs:50-106][cli-run-err]</claim>.
The command line's own refusals are untyped strings, so an error envelope
needs a kind vocabulary for the application tier or a move of `src/cli/`
onto `domain::Error`.
[RFC-0015](../legacy/rfcs/0015-provisioning-failures.md) deferred it as
worth doing on its own.

<decision id="json-errors" title="2. Does a --json failure get an envelope, and whose kinds does it carry?">

<choice key="a" title="An envelope, with kinds of the command line's own">

A failure under `--json` writes the envelope with an error in place of `result`. Its `kind` is `Error::kind` for a failure from the library, and a word from a small vocabulary of the command line's own for its refusals.

</choice>

<choice key="b" title="An envelope, on domain::Error">

`src/cli/` reports its refusals as `domain::Error` variants, so every failure carries a library kind.

</choice>

<choice key="c" title="No envelope">

Failures stay text on stderr, as now.

</choice>

<recommendation choice="a">

Failures from the library already have kinds, pinned for every binding, so only the command line's own refusals need new words. Putting those words in `domain::Error` would grow the library's vocabulary with application-tier failures the library never raises.

</recommendation>

<against>

Two vocabularies behind one envelope means a consumer reads two lists, and the library's is the one a conformance fixture already pins. Option B keeps a single list.

</against>

</decision>

<assumptions />

## Future possibilities

- The error envelope above, under the same `format_version`.
- A terminal UI for the Rust command line, which `ROADMAP.md` lists.
- Removing the deprecated keys and accessors in a later breaking release, by
  a new proposal.

[cli-docs]: ../../src/cli/mod.rs#L1-L19 "A broken pipe is exit 0"
[cli-run]: ../../src/cli/mod.rs#L50-L106 "never exits the process"
[cli-run-u8]: ../../src/cli/mod.rs#L50-L106 "It is a `u8` rather than a"
[cli-run-err]: ../../src/cli/mod.rs#L50-L106 'let _ = writeln!(err, "matra: {e}");'
[cli-args]: ../../src/cli/mod.rs#L112-L242 "#[arg(long, global = true)]"
[cli-commands]: ../../src/cli/mod.rs#L112-L242 "enum Command {"
[parse]: ../../src/cli/mod.rs#L252-L288 "the short help on stderr, exit 2"
[execute-validate]: ../../src/cli/mod.rs#L372-L430 "Validate the input before touching the model."
[execute-skill]: ../../src/cli/mod.rs#L372-L430 "so it outranks a subcommand"
[build-engine]: ../../src/cli/mod.rs#L486-L530 "It goes to stderr, which keeps `--json` stdout a single object"
[envelope]: ../../src/cli/mod.rs#L34-L39 "const FORMAT_VERSION: u32 = 1;"
[envelope-struct]: ../../src/cli/mod.rs#L677-L709 "The one shape every `--json` invocation emits."
[skill-embed]: ../../src/cli/skill.rs#L1-L70 'const SKILL: &str = include_str!("../../skills/matra/SKILL.md");'
[deprecation-notes]: ../../src/cli/config_cmd.rs#L82-L111 "One stderr line per deprecated key the config file set"
[write-defaults]: ../../src/cli/config_cmd.rs#L213-L293 "std::fs::hard_link(&temp, path)"
[bin]: ../../src/bin/matra.rs#L1-L26 "ExitCode::from(code)"
[py-cli]: ../../python/matra/cli.py#L1-L10 "raise SystemExit(cli_main(sys.argv[1:]))"
[cli-main]: ../../src/lib.rs#L776-L829 'let fsencode = py.import("os")?.getattr("fsencode")?;'
[py-scripts]: ../../pyproject.toml#L32-L36 'matra = "matra.cli:main"'
[cargo-features]: ../../Cargo.toml#L92-L97 'cli = ["dep:clap", "dep:clap_complete", "dep:serde_json", "udpipe"]'
[cargo-version]: ../../Cargo.toml#L1-L14 'version = "0.2.1"'
[default-toml]: ../../config/default.toml#L1-L12 'embedding = "potion-base-8M"'
[config-type]: ../../src/config.rs#L1-L118 "never behavior"
[config-resolve]: ../../src/config.rs#L120-L135 "Resolve from the process environment and the user's config file."
[config-accessors]: ../../src/config.rs#L342-L461 "pub fn deprecated_keys(&self)"
[config-schema]: ../../src/config.rs#L467-L503 "#[serde(deny_unknown_fields)]"
[config-component]: ../../src/config.rs#L588-L606 "fn check_path_component("
[config-paths]: ../../src/config.rs#L613-L656 'non_empty(env("MATRA_CONFIG_FILE"))'
[config-model-dir]: ../../src/config.rs#L658-L701 "fn resolve_model_dir("
[config-max]: ../../src/config.rs#L49 "const MAX_CONFIG_BYTES: u64 = 64 * 1024;"
[config-read]: ../../src/config.rs#L703-L720 "fn read_config_file(path: &Path) -> domain::Result<Option<String>> {"
[envelope-fixture]: ../../spec/tests/cli/envelope.json#L1-L19 '"name": "envelope",'
[skill-md]: ../../skills/matra/SKILL.md#L1-L5 "version: 0.2.1"
[skill-test]: ../../tests/skill.rs#L1-L37 "The executed-incantation test for the agent skill."
