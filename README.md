# vaani

[![crates.io](https://img.shields.io/crates/v/vaani.svg)](https://crates.io/crates/vaani)
[![docs.rs](https://img.shields.io/docsrs/vaani)](https://docs.rs/vaani)
[![PyPI](https://img.shields.io/pypi/v/vaani.svg)](https://pypi.org/project/vaani/)
[![CI](https://github.com/mox-labs/vaani/actions/workflows/ci.yml/badge.svg)](https://github.com/mox-labs/vaani/actions/workflows/ci.yml)
[![docs](https://github.com/mox-labs/vaani/actions/workflows/docs.yml/badge.svg)](https://mox-labs.github.io/vaani/)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue?logo=rust)](https://github.com/mox-labs/vaani/blob/main/Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/mox-labs/vaani/blob/main/LICENSE)

NLP library. Text in, structured analysis out.

UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

A pure, performant, ACE-aligned NLP library: **A**daptable, **C**omposable, **E**xtensible. Hex architecture, domain has zero internal dependencies, every public type is `#[non_exhaustive]`. The substrate is small and stable; opinions live in consumer code.

## Install

```bash
# Rust
cargo add vaani

# Python
pip install vaani
```

## Usage (Rust)

```rust,ignore
use vaani::{analyze_markdown, nlp::udpipe::Udpipe};

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
from vaani import Vaani

# Downloads the English model on first call (~16MB)
model_dir = str(Path.home() / ".vaani" / "models")
v = Vaani.english(model_dir)

result = v.analyze_markdown(Path("essay.md").read_text())
```

## Usage (CLI)

```bash
# Auto-downloads model on first use
vaani analyze essay.md
vaani analyze essay.md --json
vaani analyze essay.md -s    # section breakdown
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

vaani is a **Claude-managed** open-source project, intended as an exemplar
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
| [`.claude/implans/`](.claude/implans/) | Iteration plans (current + future) |
| [`docs/decisions/`](docs/decisions/) | Architecture Decision Records (ADRs) |
| [`CHANGELOG.md`](CHANGELOG.md) | What changed and why, per release |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | How to participate, commit conventions, decision flow |
| [`SECURITY.md`](SECURITY.md) | Vulnerability disclosure policy |

## License

MIT
