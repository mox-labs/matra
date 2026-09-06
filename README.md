# matra

[![CI](https://github.com/mox-labs/matra/actions/workflows/ci.yml/badge.svg)](https://github.com/mox-labs/matra/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue?logo=rust)](https://github.com/mox-labs/matra/blob/main/Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/mox-labs/matra/blob/main/LICENSE)

NLP library. Text in, structured analysis out.

UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

If you are an agent, run `matra --skill`. The program prints [`skills/matra/SKILL.md`](https://github.com/mox-labs/matra/blob/main/skills/matra/SKILL.md) out of its own binary, so the text always matches the version you are running. With nothing installed, `uvx 'matra>=0.2' --skill` does the same. Pin the version: `--skill` arrived in 0.2.0, and a bare `uvx matra` takes whatever release is newest, which before 0.2.0 was a different command line that does not have the flag.

A pure, performant NLP library, built to be adaptable, composable, and extensible. Hex architecture, domain has zero internal dependencies, public enums and structs with public fields are `#[non_exhaustive]`. The library is small and stable; the interpretation lives in your code.

## No setup

Nothing installed, nothing configured, no flags. Each of these resolves the model directory and downloads the English model (~16MB) on first use.

```bash
uvx 'matra>=0.2' analyze essay.md
```

```python
from matra import Matra

Matra.english().analyze("The report was filed without comment.")
```

```rust,ignore
let engine = matra::Engine::with_defaults()?;
```

The `uvx` line carries a version floor for the same reason the `--skill` line above does, and like that one it needs 0.2.0 on PyPI before it resolves. A bare `uvx matra` takes the newest release, which until then is 0.1.0, whose `analyze` is a separate Python implementation and not this one missing a flag: `--json` prints a bare document with no envelope, the table is a different renderer, and the model cache is hardcoded to `~/.matra/models`. That line does not fail, it succeeds and hands you something else.

The directory is the one you pass explicitly, else `MATRA_MODEL_DIR`, else the `models` subdirectory of the data root (`MATRA_DATA_DIR`, else `$XDG_DATA_HOME/matra`, else `~/.local/share/matra`); a non-empty `~/.matra/models` from an older install is still used when the new location does not exist yet. The config file names which models to use (`[models] udpipe`, `embedding`), not where they live. Every constructor takes the directory explicitly when you want it somewhere specific, and `matra config show` prints every resolved value and where it came from.

## Install

```bash
# Rust library
cargo add matra

# Rust CLI
cargo install matra --features cli

# Python library and the matra command
pip install matra
```

The Python package's `matra` command is the Rust CLI reached through the extension module, not a second implementation, so the flags, the output and the exit codes are the same either way. Wheels ship for Linux x86_64, Linux aarch64, macOS x86_64 and macOS arm64. They are built against the CPython stable ABI, so one wheel per platform serves 3.12 and every later 3.x on GIL-enabled CPython, and the Linux ones are manylinux2014, which installs on glibc 2.17 or newer and so reaches back through Debian 11, Ubuntu 20.04, RHEL 8 and Amazon Linux 2. Anything else builds from the sdist, which needs a Rust toolchain and a C++ compiler, because UDPipe is C++. On Windows that path is untested rather than known to work, and it needs the MSVC build tools rather than any of the packages the installation page names. Free-threaded CPython also falls here, which takes a different ABI tag than the one these wheels carry.

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

- **ACES**: Adaptable, Composable, Extensible. The structural design
  philosophy that resists the stasis/drag/opacity decay cycle every long-lived
  library faces. See [`.claude/skills/aces/SKILL.md`](https://github.com/mox-labs/matra/blob/main/.claude/skills/aces/SKILL.md).
- **Antifragility**: the operational discipline. Size caps at the gate,
  panic boundaries at the C/C++ FFI, atomic file writes, TOCTOU-closed hash
  verification, cycle-safe graph walks. See
  [`.claude/skills/resilience-floor/SKILL.md`](https://github.com/mox-labs/matra/blob/main/.claude/skills/resilience-floor/SKILL.md).

The agent organization in [`.claude/agents/`](https://github.com/mox-labs/matra/tree/main/.claude/agents) (6 practitioner
agents) and skill library in [`.claude/skills/`](https://github.com/mox-labs/matra/tree/main/.claude/skills) (7 skills)
operationalize these disciplines so any contributor, human or AI, can
participate without re-deriving the design.

| Where to look | What's there |
|---|---|
| [`.claude/arch/`](https://github.com/mox-labs/matra/tree/main/.claude/arch) | Architecture docs |
| [`book/src/plans/`](https://github.com/mox-labs/matra/tree/main/book/src/plans) | Iteration plans (current + future) |
| [`docs/decisions/`](https://github.com/mox-labs/matra/tree/main/docs/decisions) | Architecture Decision Records (ADRs) |
| [`CHANGELOG.md`](https://github.com/mox-labs/matra/blob/main/CHANGELOG.md) | What changed and why, per release |
| [`CONTRIBUTING.md`](https://github.com/mox-labs/matra/blob/main/CONTRIBUTING.md) | How to participate, commit conventions, decision flow |
| [`SECURITY.md`](https://github.com/mox-labs/matra/blob/main/SECURITY.md) | Vulnerability disclosure policy |
| [`CITATION.cff`](https://github.com/mox-labs/matra/blob/main/CITATION.cff) | How to cite matra |

## Citing matra

If you report matra's numbers in published work, cite two things. Cite the
software with the entry in [`CITATION.cff`](https://github.com/mox-labs/matra/blob/main/CITATION.cff), which identifies
the implementation and its version. Then cite the publication behind each
measure you used, which identifies the method. matra implements published
methods with documented departures, so neither citation substitutes for the
other. The per-measure references, and the departures, are in
[the methodology reference](https://mox-labs.github.io/matra/reference/methodology.html).

To make a result reproducible, record the matra version, the model file name
and its SHA-256, the format the document was analyzed under, and the parameters
you passed. The methodology page gives the exact form.

## License

MIT, Copyright (c) 2026 mox labs. See
[`LICENSE`](https://github.com/mox-labs/matra/blob/main/LICENSE).
