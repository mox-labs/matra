# Installation

## Rust

```toml
[dependencies]
vaani = "0.1"
```

The default feature set (`udpipe`) pulls in the UDPipe NLP backend. To use vaani without UDPipe (for example to plug in your own `NlpProvider` adapter):

```toml
[dependencies]
vaani = { version = "0.1", default-features = false }
```

Vaani targets Rust **1.85** or later (the MSRV is pinned in `Cargo.toml`).

### The English UDPipe model

Vaani downloads and verifies the English UDPipe 2.5 model on first use, into `~/.vaani/models/` by default. The download is atomic (per-process temp + rename), SHA-256-verified against a pinned hash, and resistant to TOCTOU races between verify and load.

To pre-populate the model directory:

```rust
use vaani::nlp::udpipe::Udpipe;
let _nlp = Udpipe::english("./models")?;
```

To use a model from an existing local path:

```rust
let nlp = Udpipe::from_path("/path/to/english-ewt.udpipe")?;
```

If you need non-English models, point `Udpipe::from_path` at the relevant UDPipe model file. Adding language-specific download helpers is a planned addition gated on consumer need.

## Python

```bash
pip install vaani
```

The Python wheel is built via `maturin` and ships with PyO3 bindings. Python **3.12** or later is required.

The wheel includes type stubs (`py.typed` + `_core.pyi`), so type checkers like `mypy` and `pyright` will pick up the full typed surface automatically.

## CLI

The Python wheel installs a `vaani` CLI command (see [CLI usage](../usage/cli.md)):

```bash
vaani analyze README.md
vaani summarize essay.md
vaani keyphrases paper.md --method yake
```
