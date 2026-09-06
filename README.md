# matra

[![CI](https://github.com/mox-labs/matra/actions/workflows/ci.yml/badge.svg)](https://github.com/mox-labs/matra/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue?logo=rust)](https://github.com/mox-labs/matra/blob/main/Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/mox-labs/matra/blob/main/LICENSE)

NLP library. Text in, structured analysis out.

UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

A pure, performant NLP library, built to be adaptable, composable, and extensible. Hex architecture, domain has zero internal dependencies, public enums and structs with public fields are `#[non_exhaustive]`. The library is small and stable; the interpretation lives in your code.

## No setup

Nothing installed, nothing configured, no flags. Each of these resolves the model directory and downloads the English model (~16MB) on first use.

```bash
uvx matra analyze essay.md
```

```python
from matra import Matra

Matra.english().analyze("The report was filed without comment.")
```

```rust,ignore
let engine = matra::Engine::with_defaults()?;
```

The directory is `MATRA_MODEL_DIR` if set, else your config file, else `$XDG_DATA_HOME/matra/models`. `matra config show` prints every resolved value and where it came from. Every constructor also takes the directory explicitly when you want it somewhere specific.

## Install

```bash
# Rust library
cargo add matra

# Rust CLI
cargo install matra --features cli

# Python library and the matra command
pip install matra
```

The Python package's `matra` command is the Rust CLI reached through the extension module, not a second implementation, so the flags, the output and the exit codes are the same either way. Wheels ship for Linux x86_64 and macOS (Intel and Apple Silicon). Anything else builds from the sdist, which needs a Rust toolchain.

## Usage (Rust)

```rust,ignore
use matra::{Engine, Ingest};

let engine = Engine::with_defaults()?;

let analysis = engine
    .analyze(Ingest::path("essay.md")?)
    .next()
    .unwrap()
    .map_err(|e| e.error)?
    .analysis;

println!("Sentences: {}", analysis.total_sentences());
println!("Mean length: {:.1}", analysis.mean_sentence_length());
println!("Passive: {:.1}%", analysis.passive_ratio() * 100.0);
```

`Engine::new(Box::new(Udpipe::english(dir)?), standard_decomposers())` is the explicit form, and it is what you reach for with your own provider or decomposer table.

## Usage (Python)

```python
from pathlib import Path
from matra import Matra

v = Matra.english()

result = v.analyze_markdown(Path("essay.md").read_text())

for item in v.analyze_path("docs/"):        # a whole directory
    print(item["path"])
```

## Usage (CLI)

```bash
matra analyze essay.md
matra analyze essay.md --json
matra analyze essay.md -s      # section breakdown
matra config show              # every resolved value, with its origin
```

## Metrics

**Per sentence:** word count, POS tags, dependency labels, passive voice, tree depth.

**Per paragraph:** sentence count, readability grade, lexical density, compression ratio.

**Per document:** passive ratio, mean sentence length, vocabulary TTR, nominalization ratio.

## Architecture

Hex architecture. Domain depends on port traits (`Source`, `Decomposer`, `NlpProvider`), not on adapters directly. UDPipe is the default NLP adapter, behind the `udpipe` feature flag.

The domain sits at the centre and depends on nothing. Four ports (`Source`, `Decomposer`, `NlpProvider`, `Embedder`) depend only on the domain. Adapters implement one port each, and `nlp/udpipe.rs` is the only file that imports the UDPipe bindings. Metrics and extractors are plain functions over the domain. `lib.rs` wires it together and is the only file that knows the whole shape.

The full walkthrough, with what is resident at each stage and what can fail where, is in [the architecture chapter](https://mox-labs.github.io/matra/architecture/design.html).

## How this project is run

matra is a **Claude-managed** open-source project, intended as an exemplar
for both Claude-managed repositories and human–AI collaborative intelligence.
The maintainer collaborates with Claude (Anthropic's AI) to plan, implement,
and review changes; humans approve every PR before merge. The working values
are transparency (decisions are visible), auditability (every change has a
trail), and reversibility (every change can be backed out cleanly).

The repository is built on two non-negotiable disciplines:

- **ACES** — Adaptable, Composable, Extensible. The structural design
  philosophy that resists the stasis/drag/opacity decay cycle every long-lived
  library faces. See [`.claude/skills/aces/SKILL.md`](.claude/skills/aces/SKILL.md).
- **Antifragility** — the operational discipline. Size caps at the gate,
  panic boundaries at the C/C++ FFI, atomic file writes, TOCTOU-closed hash
  verification, cycle-safe graph walks. See
  [`.claude/skills/resilience-floor/SKILL.md`](.claude/skills/resilience-floor/SKILL.md).

The agent organization in [`.claude/agents/`](.claude/agents/) (6 practitioner
agents) and skill library in [`.claude/skills/`](.claude/skills/) (7 skills)
operationalize these disciplines so any contributor — human or AI — can
participate without re-deriving the design.

| Where to look | What's there |
|---|---|
| [`.claude/arch/`](.claude/arch/) | Architecture docs |
| [`book/src/plans/`](book/src/plans/) | Iteration plans (current + future) |
| [`docs/decisions/`](docs/decisions/) | Architecture Decision Records (ADRs) |
| [`CHANGELOG.md`](CHANGELOG.md) | What changed and why, per release |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How to participate, commit conventions, decision flow |
| [`SECURITY.md`](SECURITY.md) | Vulnerability disclosure policy |

## License

MIT
