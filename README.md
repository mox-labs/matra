# matra

[![CI](https://github.com/mox-labs/matra/actions/workflows/ci.yml/badge.svg)](https://github.com/mox-labs/matra/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue?logo=rust)](https://github.com/mox-labs/matra/blob/main/Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/mox-labs/matra/blob/main/LICENSE)

NLP library. Text in, structured analysis out.

UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

A pure, performant, ACE-aligned NLP library: **A**daptable, **C**omposable, **E**xtensible. Hex architecture, domain has zero internal dependencies, public enums and structs with public fields are `#[non_exhaustive]`. The substrate is small and stable; opinions live in consumer code.

## Install

Not yet published to crates.io or PyPI. Build from source:

```bash
# Rust
git clone https://github.com/mox-labs/matra && cd matra
cargo build

# Python (requires maturin)
maturin develop
```

## Usage (Rust)

```rust,ignore
use matra::{analyze_markdown, nlp::udpipe::Udpipe};

// Downloads the English model on first call (~16MB)
let nlp = Udpipe::english("./models").unwrap();

let text = std::fs::read_to_string("essay.md").unwrap();
let analysis = analyze_markdown(&text, &nlp).unwrap();

println!("Sentences: {}", analysis.total_sentences());
println!("Mean length: {:.1}", analysis.mean_sentence_length());
println!("Passive: {:.1}%", analysis.passive_ratio() * 100.0);
```

## Usage (Python)

```python
from pathlib import Path
from matra import Matra

# Downloads the English model on first call (~16MB)
model_dir = str(Path.home() / ".matra" / "models")
v = Matra.english(model_dir)

result = v.analyze_markdown(Path("essay.md").read_text())
```

## Usage (CLI)

```bash
# Auto-downloads model on first use
matra analyze essay.md
matra analyze essay.md --json
matra analyze essay.md -s    # section breakdown
```

## Metrics

**Per sentence:** word count, POS tags, dependency labels, passive voice, tree depth.

**Per paragraph:** sentence count, readability grade, lexical density, compression ratio.

**Per document:** passive ratio, mean sentence length, vocabulary TTR, nominalization ratio.

## Architecture

Hex architecture. Domain depends on port traits (`Source`, `Decomposer`, `NlpProvider`), not on adapters directly. UDPipe is the default NLP adapter, behind the `udpipe` feature flag.

```text
src/
  domain.rs              # types (zero internal deps)
  source/                # Source port + File/Directory adapters
  decompose/             # Decomposer port + Markdown/Plain adapters
  nlp/                   # NlpProvider port
    udpipe.rs            # UDPipe adapter (only file importing udpipe_rs)
  metrics/               # readability, lexical, compression, document
  extraction/            # TF-IDF, TextRank, RAKE, YAKE
  lib.rs                 # composition root + PyO3 bindings
```

Deeper docs in `.claude/arch/`: ports, adapters, domain model, evolution.

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
participate without re-deriving the substrate.

| Where to look | What's there |
|---|---|
| [`.claude/arch/`](.claude/arch/) | Architecture docs |
| [`.claude/plans/`](.claude/plans/) | Iteration plans (current + future) |
| [`docs/decisions/`](docs/decisions/) | Architecture Decision Records (ADRs) |
| [`CHANGELOG.md`](CHANGELOG.md) | What changed and why, per release |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How to participate, commit conventions, decision flow |
| [`SECURITY.md`](SECURITY.md) | Vulnerability disclosure policy |

## License

MIT
