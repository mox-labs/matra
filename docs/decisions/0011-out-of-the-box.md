# 0011. Out of the box: configuration, paths, and one CLI

- **Status:** accepted
- **Date:** 2026-09-05
- **Decider(s):** project owner (direction), maintainer (shape)

## Context

matra 0.1.0 runs from the command line with no setup: the CLI defaults
the model directory to `~/.matra/models` and downloads the UDPipe model
on first use. Three things stop that from being true of the package as a
whole.

1. **The library requires what the CLI defaults.** `Udpipe::english(dir)`
   and `Matra.english(dir)` take the model directory as a required
   argument (`src/nlp/udpipe.rs:75`, `python/matra/_core.pyi:36`). Every
   caller re-derives the same path, and `~/.matra` is not where
   developer tools keep data on Linux or macOS.
2. **Two CLIs, and they have drifted.** `src/bin/matra.rs` is the CLI.
   `python/matra/cli.py` re-implements it in click and rich so that
   `uvx matra` works, and it has grown a `--sections` flag the Rust CLI
   does not have while lacking `--model-dir`. The 0.1.0 changelog
   already records the same failure once: the Python CLI re-implemented
   passive detection and was wrong. The roadmap's stated rule is that
   the library returns typed data and the binary decides presentation;
   two binaries deciding presentation is the drift.
3. **The semantic tier is not out of the box.** ADR-0010 decision 6
   forbids the library from downloading embedding models, so
   `semantic_clusters` needs a hand-placed directory while the UDPipe
   model, one call away, downloads itself with a pinned hash.

The roadmap entry "Configuration-driven invocation" names agent-driven
use as the trigger: an agent benefits from declaring intent once. That
trigger fired on 2026-09-05 with the owner's direction that matra work
out of the box, follow developer-tool conventions for config and
paths, keep Rust as the core with Python and TypeScript as thin reach
layers, and be consumable by an agent. The roadmap entry also names the
real question: is configuration a library concern or an application
concern? A config format that reaches into the library puts policy where
the composition root belongs.

## Options considered

### Option A: application-tier only

Configuration lives in the CLI. The library keeps explicit arguments
everywhere.

**Pros:** no policy in the library; nothing new in the public surface.
**Cons:** every Python caller and every Rust caller keeps writing the
path line; "works out of the box" is true only of the binary; an agent
driving the Python API gets no defaults.

### Option B: a resolver in the library, policy in the application

The library gains one composition-root value, `Config`, that resolves
*where things are and what the defaults are* (model directories, the
semantic threshold, extractor counts) from explicit argument, then
environment, then a config file, then defaults compiled into the crate.
Constructors gain forms that consult it. *Which metrics run* and *how
output is shaped* stay where they are: the metric suite is already a
`Vec<Metric>` the caller chooses (`src/metrics/mod.rs:31-43`), and
rendering is the binary's.

**Pros:** out of the box for every surface; one resolution order for
all of them; the library still never renders and never selects
behavior on the caller's behalf beyond defaults it documents.
**Cons:** one new module and one new dependency (`toml`); the config
file's schema becomes public surface.

### Option C: a full configuration schema driving the pipeline

Config selects metrics, extractors, thresholds, and output shape, and
`Engine` is built from it.

**Pros:** declare once, run anywhere.
**Cons:** puts pipeline policy in a file the library parses; couples
the library to a presentation vocabulary; the roadmap's own warning.

### Option D: keep two CLIs, add a parity test

**Pros:** no restructuring.
**Cons:** the test catches drift after it happens; the second
implementation still exists for a reason (`uvx`) that a launcher
serves equally well.

## Decision

We choose Option B for configuration and collapse the CLIs to one
implementation with two launchers.

**Paths follow developer-tool conventions on Linux and macOS.** Config
at `$XDG_CONFIG_HOME/matra/config.toml`, defaulting to
`~/.config/matra/config.toml`; data at `$XDG_DATA_HOME/matra`,
defaulting to `~/.local/share/matra`, with models under `models/`.
The same layout on macOS: uv documents that it "follows the XDG
conventions on Linux and macOS" and gh resolves `$XDG_CONFIG_HOME/gh`
before `~/.config/gh` on both; matra is driven from terminals and
agents, not from Finder. `~/.matra/models` stays readable as a
fallback so an existing cache keeps working. Environment overrides
name the thing they override, as `UV_CONFIG_FILE`, `UV_CACHE_DIR`, and
`OLLAMA_MODELS` do: `MATRA_CONFIG_FILE` (the file), `MATRA_DATA_DIR`
(the data root), and the existing `MATRA_MODEL_DIR` (the UDPipe model
directory, already honored by the Rust CLI). A directory passed
explicitly wins over all three.

**Defaults ship inside the crate.** `config/default.toml` is embedded
with `include_str!` and parsed at resolution; a user file overrides it
key by key. `Config::resolve()` records the source of every value so
`matra config show` can print where each came from.

**Constructors that consult the resolver are additive, and share one
name.** Existing signatures do not change (0.1.0 froze them). Every
adapter gains `from_config(&Config)`: `Udpipe::from_config` loads or
downloads the pinned English model into the resolved directory,
`Model2Vec::from_config` does the same for the pinned reference
embedding model, and `Engine::from_config` assembles the standard
pipeline. `Engine::with_defaults()` is the one-liner
(`Config::resolve()` then `from_config`), the only place a second name
is worth its cost. In Python the argument goes optional:
`Matra.english(model_dir=None)`, `Model2Vec.potion_base_8m(dir=None)`.

**Pinned downloads are the same discipline for every model.** ADR-0010
decision 6 is amended: the library performs no *unpinned* network
access. A download whose target digest is a constant in the source,
verified before load with no second disk read, is the UDPipe
discipline already in the tree (`src/nlp/udpipe.rs:60-100`), and the
reference embedding model gets the same treatment. Model identity
(the three-file digest) is unchanged; what changes is that the library
can fetch the pinned artifact when asked.

**The launcher shape is the one maturin recommends.** ruff and uv
ship their Rust CLI to `uvx` users with `bindings = "bin"`, which
packages the binary as the wheel's script. matra also ships a Python
extension, and maturin's own guidance for that case is to expose a CLI
function through the library and use a Python entry point rather than
double the wheel with both. So the Python launcher is not a compromise;
it is the documented shape for a crate that is both a library and a
tool.

**One CLI, two launchers.** The CLI moves into the library as
`src/cli/` behind the `cli` feature, with one entry point
`cli::run(args, stdout, stderr) -> ExitCode`. `src/bin/matra.rs`
becomes a launcher. The Python extension exposes the same function as
`_core.cli_main(argv) -> int`, and `python/matra/cli.py` becomes a
launcher of a few lines: pass `sys.argv`, exit with the code. `click`
and `rich` leave the Python dependencies. `--sections` is ported to
Rust so nothing a user could do yesterday stops working. The `python`
feature enables `cli`.

**Configuration does not select behavior.** The file carries defaults
(paths, the semantic threshold, summary and keyphrase counts, the
default algorithms) and nothing that changes what a call computes
beyond what the caller could pass as an argument. Suite selection and
output shape remain application-tier.

## Consequences

- Positive: `uvx matra analyze file.md`, `cargo install matra
  --features cli && matra analyze file.md`, and
  `Matra.english().analyze(text)` all work on a fresh machine with no
  flags; there is one CLI implementation to test and one JSON contract
  to pin; the Python binding stops carrying logic.
- Positive: the stack rule is now written down as a boundary: Rust is
  the core and every CLI; Python and TypeScript are reach layers that
  expose the API and the extension points and carry no behavior of
  their own.
- Negative: `toml` joins the dependency tree (pure Rust, compiles for
  wasm32); the config file schema is public surface and versioned with
  the crate; the `python` feature now builds `clap`.
- Negative: two more constructors per adapter to keep in lockstep with
  the Python stubs.
- Neutral: `~/.matra` remains readable; nothing is migrated or deleted.

## Validation

Right if, at 0.3.0, the Python CLI file is still a launcher (no
rendering code, no click), no CLI-drift bug has been filed, and no
caller has asked for configuration that selects metrics or output
shape. Falsified if a consumer needs per-project config that changes
what a call computes; that would reopen Option C as its own ADR rather
than growing this file's schema.

## References

- ROADMAP.md, "Configuration-driven invocation" (trigger fired
  2026-09-05)
- [ADR-0007](0007-one-pipeline.md): the surface these constructors
  extend
- [ADR-0010](0010-embeddings-adapter.md): decision 6, amended here
- `book/src/plans/i10-foundations.md`: the execution plan
- [Conventions survey, 2026-09-05](../surveys/2026-09-05-conventions.md):
  the exemplar evidence behind the path, naming, and launcher decisions
- maturin, "Bindings": https://www.maturin.rs/bindings
- uv, "Storage": https://docs.astral.sh/uv/reference/storage/
