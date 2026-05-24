# Installation

## Rust

```toml
[dependencies]
vaani = "0.0"
```

The current alpha line is `0.0.x` (pre-publication; expect breaking changes between releases). The first version that downstream code should consider taking a long-lived dependency on is `0.1.0`. Until then, pin the minor: `vaani = "0.0"` resolves to the latest 0.0.x and refuses to silently jump past the alpha cycle.

The default feature set (`udpipe`) pulls in the UDPipe NLP backend. To use vaani without UDPipe (for example to plug in your own `NlpProvider` adapter):

```toml
[dependencies]
vaani = { version = "0.0", default-features = false }
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

The wheel tracks the same `0.0.x` line as the Rust crate. If your project needs to stay inside the alpha line explicitly, pin to it:

```bash
pip install "vaani>=0.0,<0.1"
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
