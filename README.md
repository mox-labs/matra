# matra

[![CI](https://github.com/mox-labs/matra/actions/workflows/ci.yml/badge.svg)](https://github.com/mox-labs/matra/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue?logo=rust)](https://github.com/mox-labs/matra/blob/main/Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/mox-labs/matra/blob/main/LICENSE)

NLP library. Text in, structured analysis out.

UDPipe-based structured parse (full CoNLL-U: tokens, lemmas, POS, dependency trees), base text metrics (readability, lexical density, compression), summarization (TF-IDF, TextRank), and keyphrase extraction (RAKE, YAKE). Rust core with Python bindings via PyO3.

If you are an agent, run `uvx 'matra>=0.3' --skill`, which needs nothing installed, or `matra --skill` if you already have the binary. Either way the program prints [`skills/matra/SKILL.md`](https://github.com/mox-labs/matra/blob/main/skills/matra/SKILL.md) out of its own binary, so the text always matches the version you are running. Keep the version floor: 0.2's command line spells its commands and its JSON differently, and 0.1.0's has no `--skill` at all.

A pure, performant NLP library, built to be adaptable, composable, and extensible. Hex architecture, domain has zero internal dependencies, public enums and structs with public fields are `#[non_exhaustive]`. The library is small and stable; the interpretation lives in your code.

## No setup

Nothing installed, nothing configured, no flags. Each of these resolves the model directory and downloads the English model (~16MB) on first use.

```bash
uvx 'matra>=0.3' analyze essay.md
```

```python
import matra

matra.Engine().analyze("The report was filed without comment.")
```

```rust,ignore
let engine = matra::Engine::new()?;
```

The `uvx` line carries a version floor for the same reason the `--skill` line above does. Both registries serve 0.3.x, so a bare `uvx matra` resolves to it today; the floor says what these commands need rather than leaving it to whichever release is newest when you run them. 0.2 is still published too, and its command line and JSON are the ones the [migration guide](https://mox-labs.github.io/matra/guides/migrate-to-0-3.html) moves you off. 0.1.0 is still published, and its `analyze` is a separate Python implementation and not this one missing a flag: `--json` prints a bare document with no envelope, the table is a different renderer, and the model cache is hardcoded to `~/.matra/models`. That line does not fail, it succeeds and hands you something else.

The directory is the one you pass explicitly, else `MATRA_MODEL_DIR`, else the `models` subdirectory of the data root (`MATRA_DATA_DIR`, else `$XDG_DATA_HOME/matra`, else `~/.local/share/matra`); a non-empty `~/.matra/models` from an older install is still used when the new location does not exist yet. The config file does not move the model directory: its `[models] embedding` key names the embedding model's subdirectory inside it,, and nothing selects the parsing model, which is pinned in the source. To put it somewhere specific, pass it explicitly (`--model-dir`, `Config::with_model_dir`), or set `MATRA_MODEL_DIR`, which is how Python names it. `matra config show` prints every resolved value and where it came from.

## Install

```bash
# Rust library
cargo add matra@0.3

# Rust CLI
cargo install matra --version '^0.3' --features cli

# Python library and the matra command
pip install 'matra>=0.3'
```

Each command names a version for the reason the `uvx` line above does. A bare `cargo add matra` or `pip install matra` takes whatever the registry hands out, which is 0.3.x today and will not stay that way, and 0.1.0 and 0.2 are still there for anything already pinned to them: that route does not fail, it succeeds and gives you a different program. The Python routes take a floor, and `uv add 'matra>=0.3'` is that same floor for uv. The cargo routes take a caret requirement, which is how cargo states a version, so they accept 0.3.0 or a later release semver-compatible with it and stop short of 0.4.0. Coming from 0.2, read [Migrate from 0.2 to 0.3](https://mox-labs.github.io/matra/guides/migrate-to-0-3.html) first: nothing keeps its old name.

The Python package's `matra` command is the Rust CLI reached through the extension module, not a second implementation, so the flags, the output and the exit codes are the same either way. Wheels ship for Linux x86_64, Linux aarch64, macOS x86_64 and macOS arm64. They are built against the CPython stable ABI, so one wheel per platform serves 3.12 and every later 3.x on GIL-enabled CPython, and the Linux ones are manylinux2014, which installs on glibc 2.17 or newer and so reaches back through Debian 11, Ubuntu 20.04, RHEL 8 and Amazon Linux 2. Anything else builds from the sdist, which needs a Rust toolchain and a C++ compiler, because UDPipe is C++. On Windows that path is untested rather than known to work, and it needs the MSVC build tools rather than any of the packages the installation page names. Free-threaded CPython also falls here, which takes a different ABI tag than the one these wheels carry.

## Usage (Rust)

```rust,ignore
use matra::{Engine, Input, Inputs};

let engine = Engine::new()?;

// One document: a file, read as its extension and bytes say.
let doc = engine.analyze(Input::file("essay.md"))?;
println!("Sentences: {}", doc.metrics.total_sentences);
println!("Mean length: {:?}", doc.metrics.mean_sentence_length);
println!("Passive: {:?}", doc.metrics.passive_ratio);

// Outcomes are functions of the document: no second parse.
let summary = matra::summarize(&doc, 3)?;
let phrases = matra::keyphrases_with(&doc, 10, matra::KeyphraseMethod::Yake)?;
println!("{:?} kept {} sentences", summary.provenance.method, summary.sentences.len());

// Many documents, lazily and in order; a failure is one item, not an abort.
for item in engine.analyze_all(Inputs::dir("docs/")?) {
    match item {
        Ok(doc) => println!("{:?}: {} words", doc.provenance.source.path, doc.metrics.total_words),
        Err(e) => eprintln!("{e}"),
    }
}
```

`Input` also takes text in a named format (`Input::text`), bytes from anywhere (`Input::bytes`), and headings and paragraphs you supply for a format matra has no reader for (`Input::blocks`). Every heading, paragraph, sentence and token carries a `span` into the analysed text, in UTF-8 bytes, and `skipped` lists what the reader saw and did not analyse (a table, a code block, front matter).

`summarize`, `keyphrases` and `clusters` take the analysed `Document` and need no engine, so they also run in a build without UDPipe. The method is optional (`summarize_with(&doc, n, SummaryMethod::TextRank)`), and every result records the method that ran and its parameters in its `provenance`. `clusters(&doc, &embedder, 0.85)` takes the embedder from you, always. `Engine::new()` reads no configuration file; `Engine::from_config(&Config::resolve()?)` reads one, and `Config::with_model_dir(dir)` puts the model somewhere specific.

## Usage (Python)

```python
import matra

engine = matra.Engine()

doc = engine.analyze(file="essay.md")       # a str alone is text, never a path
print(doc["metrics"]["total_words"])
summary = matra.summarize(doc, n=3)
phrases = matra.keyphrases(doc, n=10, method="yake")

for item in engine.analyze_all(dir="docs/"):   # lazily, one item per file
    if isinstance(item, matra.Document):
        print(item["provenance"]["source"]["path"], item["metrics"]["total_words"])
    else:                                      # a failure: path, kind, message, details
        print(item["path"], item["kind"])
```

A `Document` is read-only and reads as its JSON (`doc.to_dict()` for the whole). Python spells every word Rust spells: `analyze(text=, file=, bytes=, blocks=, format=)` for the `Input` constructors, and `method=` for the `_with` forms.

## Usage (CLI)

```bash
matra analyze essay.md
matra analyze essay.md --json
matra analyze essay.md -s                    # section breakdown
matra analyze docs/ --json --depth document  # one line per file, measures only
matra summarize essay.md -n 3 --method textrank
matra keyphrases essay.md --json
matra clusters essay.md --threshold 0.85     # needs the model2vec feature
matra analyze notes.json --format blocks     # a JSON file of blocks
matra --schema document                      # the JSON Schema of the output
matra config show                            # every resolved value, with its origin
```

Under `--json` every command writes one envelope, `{format_version, command, input, result, error}`, and a failure is an envelope too: `result` null, `error` its `kind`, `message` and `details`, and exit code 2.

## Metrics

**Per sentence:** word count, POS tags, dependency labels, passive voice, tree depth.

**Per paragraph:** sentence count, readability grade, lexical density, compression ratio.

**Per document:** word, sentence and paragraph counts, passive ratio, mean and spread of sentence length, vocabulary TTR, nominalization ratio. Each document also records its provenance: the parser and its model, the format, and the SHA-256 of the input, with the schema version (`2.0`) its JSON follows.

## Architecture

Hex architecture. The pipeline depends on port traits (`Source`, `Decomposer`, `NlpProvider`), not on adapters directly. All three, with detection, the step that turns blocks into sections, the metric suite and the ranking methods, are private to the crate: a caller hands in an `Input` and asks for an outcome. `Embedder` is the one port open to outside implementers. UDPipe is the default NLP adapter, behind the `udpipe` feature flag.

The domain sits at the centre and depends on nothing. Four ports (`Source`, `Decomposer`, `NlpProvider`, `Embedder`) depend only on the domain. Adapters implement one port each, and `nlp/udpipe.rs` is the only file that imports the UDPipe bindings. Metrics and the outcome methods are plain functions over the domain. `lib.rs` wires it together and is the only file that knows the whole shape.

The full walkthrough, with what is resident at each stage and what can fail where, is in [the architecture chapter](https://mox-labs.github.io/matra/architecture/design.html).

## How this project is run

matra is a **Claude-managed** open-source project, intended as an exemplar
for both Claude-managed repositories and human–AI collaborative intelligence.
The owner collaborates with Claude (Anthropic's AI) to plan, implement,
and review changes. Claude reviews and merges a pull request once CI is green
and the review raises no blockers, and leaves a comment with its rationale as
the audit trail. The owner makes the decisions: only the owner merges the
change that accepts an enhancement proposal, and only the owner approves the
release workflow's deployment environments, which is the release decision.
[How matra is maintained](https://mox-labs.github.io/matra/explanation/how-matra-is-maintained.html)
explains the whole arrangement. The working values
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

The agent organization in [`.claude/agents/`](https://github.com/mox-labs/matra/tree/main/.claude/agents)
and skill library in [`.claude/skills/`](https://github.com/mox-labs/matra/tree/main/.claude/skills)
operationalize these disciplines so any contributor, human or AI, can
participate without re-deriving the design.

| Where to look | What's there |
|---|---|
| [`.claude/arch/`](https://github.com/mox-labs/matra/tree/main/.claude/arch) | Rejected-decision history: what was proposed, why it was turned down. The architecture itself is in the docsite's [architecture page](https://mox-labs.github.io/matra/architecture/design.html) |
| [`blueprints/`](https://github.com/mox-labs/matra/blob/main/blueprints/README.md) | Enhancement proposals (substantial changes, accepted by the owner), enhancement plans (work several agents carry out in parallel), and the legacy RFCs and EPs behind the code that exists |
| [`CHANGELOG.md`](https://github.com/mox-labs/matra/blob/main/CHANGELOG.md) | What changed and why, per release |
| [`CONTRIBUTING.md`](https://github.com/mox-labs/matra/blob/main/CONTRIBUTING.md) | How to participate, commit conventions, decision flow |
| [`SECURITY.md`](https://github.com/mox-labs/matra/blob/main/SECURITY.md) | Vulnerability disclosure policy, and how to verify a release with `gh attestation verify` |
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

The models matra downloads are separate files with their own licenses.
The English parsing model, `english-ewt-ud-2.5-191206.udpipe` from
[LINDAT](https://hdl.handle.net/11234/1-3131), is
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/),
which does not permit commercial use. The embedding model,
potion-base-8M, is MIT per its model card. See
[model licenses](https://mox-labs.github.io/matra/reference/platforms.html#model-licenses).
