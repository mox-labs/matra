# I10: Out of the box

**Boundary:** post-publish, additive to the 0.1.0 surface. No existing signature changes. The Python CLI's implementation is replaced, its command line is not.

**Origin:** the roadmap's "Configuration-driven invocation" trigger fired on 2026-09-05 with the owner's direction: matra works with no setup on every surface, follows developer-tool conventions for config and paths, keeps Rust as the core with Python and TypeScript as thin reach layers, and is consumable by an agent. [ADR-0011](https://github.com/mox-labs/matra/blob/main/docs/decisions/0011-out-of-the-box.md) records the decisions; this plan is how they land.

This is the first of three iterations: foundations (this plan), the agent surface (`--skill`, planned once the CLI contract here is final), and later a terminal UI for the Rust CLI.

---

## Why this shape and not another

### The boundary test, applied to the language layers

Each layer is checked against one question: does it reduce the total surface a caller integrates against, or add to it? The Rust core passes. The Python binding half passes: fields cross, methods do not, one `Matra` class over the engine. It fails twice, and both failures are the same mistake. `python/matra/cli.py` re-implements rendering and argument parsing that `src/bin/matra.rs` already has, so there are two surfaces where there should be one launcher. And a Python caller who needs a custom embedder, a metric suite, or a directory walk has no extension point and writes the logic in Python, so the binding accumulates behavior the core should own.

The rule that falls out, now written into ADR-0011: every CLI is Rust; a reach layer exposes the API and the extension points and carries no behavior of its own.

### Config resolves locations and defaults, never behavior

The roadmap's warning is that a config format reaching into the library puts policy where the composition root belongs. So the config carries only what a caller could already pass as an argument: where models live, the semantic threshold, the default counts and algorithms. Suite selection stays `Vec<Metric>`; output shape stays the binary's. `Config` is a composition-root value, in the same layer as `Engine`, importing `domain` and nothing from the ports.

### Pinned downloads are one discipline, not two

The UDPipe adapter already downloads a pinned artifact, verifies it against a constant, and loads the verified bytes with no second read. Making the reference embedding model do the same is not a new capability, it is the removal of an exception. ADR-0010 decision 6 said "no network"; it meant "no unpinned network", and the amendment says so.

## The surface

```rust
// composition root
pub struct Config { /* non_exhaustive */ }
impl Config {
    pub fn resolve() -> Result<Config>;              // arg > env > file > built-in
    pub fn data_dir(&self) -> &Path;                 // $XDG_DATA_HOME/matra or ~/.local/share/matra
    pub fn model_dir(&self) -> PathBuf;              // data_dir/models, or ~/.matra/models if only that exists
    pub fn semantic_threshold(&self) -> f32;
    pub fn sources(&self) -> impl Iterator<Item = (&str, ValueSource)>; // key -> Argument | Environment | File | Default
}
impl Engine {
    pub fn from_config(cfg: &Config) -> Result<Engine>;
    pub fn with_defaults() -> Result<Engine>;        // Config::resolve() then from_config
}
impl Udpipe    { pub fn from_config(cfg: &Config) -> Result<Self>; }   // pinned English model, downloaded if absent
impl Model2Vec { pub fn from_config(cfg: &Config) -> Result<Self>;     // pinned reference embedding model, same
                 pub fn potion_base_8m(dir: impl AsRef<Path>) -> Result<Self>; }  // explicit directory

// cli feature
pub mod cli { pub fn run(args: impl IntoIterator<Item = OsString>, out: &mut dyn Write, err: &mut dyn Write) -> ExitCode; }
```

```python
Matra.english(model_dir: str | None = None)
Model2Vec.potion_base_8m(dir: str | None = None)
class Embedder(Protocol):                      # any object with these two methods is accepted
    def embed(self, texts: list[str]) -> list[list[float]]: ...
    def identity(self) -> str: ...
Matra.semantic_clusters(text, threshold, model: Model2Vec | Embedder)
Matra.analyze_path(path: str) -> list[CorpusEntry | DocumentError]
_core.cli_main(argv: list[str]) -> int
```

Config file, shipped inside the crate as `config/default.toml` and written out by `matra config init`:

```toml
[models]
# data_dir defaults to $XDG_DATA_HOME/matra; override with MATRA_DATA_DIR
udpipe = "english-ewt-ud-2.5-191206"
embedding = "potion-base-8M"

[semantic]
threshold = 0.85

[summarize]
n = 3
algorithm = "tfidf"

[keyphrases]
n = 10
algorithm = "rake"
```

Names are forever, and M1 settled the two that were open with the [conventions survey](https://github.com/mox-labs/matra/blob/main/docs/surveys/2026-09-05-conventions.md). Constructors share one name across adapters, `from_config`, with `Engine::with_defaults` as the single one-liner. Environment variables name the thing they override, as `UV_CONFIG_FILE`, `UV_CACHE_DIR`, and `OLLAMA_MODELS` do: `MATRA_CONFIG_FILE`, `MATRA_DATA_DIR`, and the existing `MATRA_MODEL_DIR`.

## Milestones

Each milestone is one PR, review-hardened by the CI harness before merge. Strict order.

### M1: the ADR, the roadmap, the names

ADR-0011 accepted; roadmap entry marked fired with a pointer here; the survey filed in `docs/surveys/` and the two open names settled with its evidence; plans index and `SUMMARY.md` carry this page.

**Rubric.** `just docs-floor` passes. ADR-0011 names every new public item that M2 to M5 add, so a later reviewer can diff the surface against the decision.

### M2: `Config` and the default constructors

`src/config.rs` at the composition-root layer. Resolution order: explicit argument, `MATRA_*` environment, the config file, `include_str!("../config/default.toml")`. Path resolution honors `XDG_CONFIG_HOME` and `XDG_DATA_HOME`, defaults to `~/.config` and `~/.local/share`, and falls back to `~/.matra/models` for the model directory when that exists and the new location does not. `Udpipe::from_config`, `Engine::from_config`, `Engine::with_defaults`. Python `Matra.english(model_dir=None)`.

**Rubric.** Every resolved value knows its source, and a test asserts each rung of the order with the environment isolated (no test reads the developer's real home). A malformed config file is `Error::InvalidInput` naming the key, never a panic and never silently ignored. The file is a new input surface, so it is capped before it is read: its size is checked against metadata first, and past the cap it is `InputTooLarge` with its own `what` label (`config_file`). `cargo check --no-default-features` and the wasm32 job still pass with `toml` in the tree. `python/matra/_core.pyi` and `types.py` updated in the same PR.

### M3: one CLI, two launchers

`src/cli/` (behind `cli`): the clap definition, the renderers, exit codes, and `run(args, out, err)`. `src/bin/matra.rs` calls it. `--sections` ported from the Python CLI. New `config show` (effective values with sources, as `cargo config get --show-origin` does) and `config init` (atomic write, refuses to overwrite without `--force`). The survey's remaining CLI gaps close here because clap makes them cheap: `NO_COLOR` and `--color`, `--quiet`, `-` for stdin with `--stdin-filename`, `completions <shell>` via `clap_complete`, and `--version` listing the compiled features. The JSON payload gets a one-line stability statement in the CLI guide and a `format_version` field on the envelope, cargo's precedent over a published schema. `_core.cli_main(argv)`; `python/matra/cli.py` becomes a launcher; `click` and `rich` removed from `pyproject.toml`; the `python` feature enables `cli`. `book/src/reference/boundary-rules.md` rule 7 states what `src/cli/` may import (the public surface, `domain`, and `config`; never a port or adapter directly), since rendering now lives inside the library crate.

**Rubric.** `tests/cli.rs` drives `cli::run` directly with captured output, so the CLI's tests no longer need a built binary. The JSON emitted by `--json` is the serde form of the domain types and a conformance fixture pins it. `python/matra/cli.py` is under ten lines. `uvx --from . matra analyze README.md` and the Rust binary produce byte-identical output for the same input and flags, asserted by a test.

### M4: the pinned embedding download

`Model2Vec::potion_base_8m(dir)` downloads `model.safetensors`, `tokenizer.json`, and `config.json` from the pinned release, verifies the three-file digest against the constant already in `spec/tests/semantic/reference-model.json`, and loads from the verified bytes. `Model2Vec::from_config` resolves the directory through `Config` and calls it. ADR-0010 decision 6 amended in place with a dated note. Python `Model2Vec.potion_base_8m(dir=None)`. The `just conformance` semantic lane stops needing a hand-placed model.

**Rubric.** A digest mismatch removes the files and retries once, then fails with `Error::ModelInvalid`; no second disk read between verify and load (the resilience skill's TOCTOU rule). The download is behind the `model2vec` feature and is never triggered by `from_dir`.

### M5: the Python extension points

A Python object with `embed` and `identity` is accepted wherever an `Embedder` is, wrapped by a PyO3 adapter that holds the GIL only during the call and converts a Python exception into `Error::InvalidInput`. `Matra.analyze_path(path)` exposes `Ingest::path` and the corpus types cross as dicts, typed in `types.py`.

**Rubric.** A Python `Embedder` returning the wrong count or dimension surfaces as `ValueError` with the same message a Rust implementor would get, asserted by a test. `analyze_path` on a directory with one unreadable file returns one `DocumentError` entry and the rest analyzed, matching `Engine::analyze` law by law.

### M6: docs, changelog, attribution

Installation, the Rust, Python, and CLI guides, pragmatics, and the semantic clusters guide describe the no-setup path first and the explicit path second. `CHANGELOG.md` records every added item under `[Unreleased]`. `Cargo.toml`, `pyproject.toml`, `LICENSE`, and the README state the same author and copyright holder (the owner supplies the canonical form).

**Rubric.** `just docs-floor` and `just check` pass. Every new public item in M2 to M5 appears in CHANGELOG, in the Python stubs where it crosses, and in exactly one docs page.

## Costs, named

- `toml` and its serde integration enter the default build. Pure Rust; verified on wasm32 by the existing CI job.
- The `python` feature now compiles `clap`. Wheel size grows by the CLI, which was previously shipped as Python source.
- The config schema is public surface from M2 on and follows the crate's semver.
- Two constructors per adapter to keep in lockstep with the Python stubs.

## Risks

- **Reading the developer's real home in tests.** Every config test sets `XDG_*`, `MATRA_*`, and `HOME` to a temp dir. A test that forgets is a flaky test on someone else's machine.
- **Atomic writes for `config init`.** Write to a temp file in the same directory, then rename; refuse if the target exists unless forced.
- **The launcher shim and `sys.argv[0]`.** Pass `argv[1:]` and set the program name in clap explicitly, so `--help` reads the same from both launchers.
- **Wheel build with `cli`.** maturin must build with `--features python,udpipe,model2vec` implying `cli`; the CI wheel matrix is the check.

## Acceptance gate

On a machine with no matra state: `uvx matra analyze README.md`, `cargo install matra --features cli && matra analyze README.md`, and `python -c "from matra import Matra; print(Matra.english().analyze('The report was filed.')['passive_ratio'])"` all succeed with no flags and no environment; `matra config show` prints every value with its source; `python/matra/cli.py` contains no rendering; `Matra.semantic_clusters` accepts a Python object with `embed` and `identity`; every rubric above holds.
