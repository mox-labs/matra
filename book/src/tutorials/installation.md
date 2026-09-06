# Installation

matra ships three ways from one Rust core: a Rust library on [crates.io](https://crates.io/crates/matra), a command-line binary installed from the same crate, and a Python package on [PyPI](https://pypi.org/project/matra/).

---

## Requirements

**Rust 1.85 or later (MSRV)** for the Rust library and the CLI. Check with `rustc --version`.

**Python 3.12 or later** for the Python package. Check with `python --version`. Wheels ship for Linux x86_64 and macOS (Intel and Apple Silicon); on any other platform `pip` builds the sdist, which additionally needs the Rust toolchain.

---

## The Rust library

```bash
cargo add matra
```

The default feature set includes `udpipe`, so `matra::nlp::udpipe::Udpipe` is available without enabling any extra feature flags.

---

## The CLI binary

```bash
cargo install matra --features cli
```

matra's binary target is gated behind the `cli` feature, so `--features cli` is required. This compiles from source and places the binary in your cargo bin directory; the build can take a minute or more.

Confirm it landed:

```bash
matra --version
```

Expected output:

```
matra 0.1.0
```

---

## The Python package

```bash
pip install matra    # or: uv add matra
```

On the platforms with wheels this downloads a prebuilt binary and installs in seconds. The verify step further down this page doubles as the check: if `from matra import Matra` fails there, the package did not install correctly.

---

## The English model

matra parses through UDPipe. None of the three install paths bundle the English model: the library, the binary, and the Python package each download it independently on first use and cache it on disk. The Rust and Python APIs take the model directory as an explicit argument or resolve one themselves when you leave it out: `Engine::with_defaults()` and `Matra.english()` use `MATRA_MODEL_DIR`, else the `models` subdirectory of `$XDG_DATA_HOME/matra`, which defaults to `~/.local/share/matra`. The CLI's own default is still `~/.matra/models`, which is also the fallback the resolved path uses when that directory already exists and is not empty.

matra writes the download (about 16 MB) to a temporary location first, then moves it into place, and checks the bytes against a fixed hash before loading them. If a file fails that check, matra deletes it and re-downloads once; if the second attempt still does not match, matra returns an error instead of loading an unverified file.

---

## Verify the install

Run this once. It downloads and caches the model on this first run, then parses a sentence through it and prints two results:

```python
from pathlib import Path
from matra import Matra

model_dir = str(Path.home() / ".matra" / "models")
v = Matra.english(model_dir)

result = v.analyze("The committee approved the proposal without debate.")
print("sections:", len(result["sections"]))
print("vocabulary_ttr:", result["vocabulary_ttr"])
```

Expected output:

```
sections: 1
vocabulary_ttr: 0.8571428571428571
```

That first run downloads about 16 MB and can take several seconds depending on your connection. Every run after that loads the cached file and touches no network.

If `Matra.english()` raises `RuntimeError`, the download or the hash check failed; check your network connection and run the snippet again. If it raises `OSError`, matra could not create or write to the model directory; check permissions on the path you passed.

---

## What you have

- matra installed as a Rust crate, a CLI binary, a Python package, or some combination, all from the same published 0.1.0 core.
- The English UDPipe model cached at `~/.matra/models`, verified against a pinned hash.
- A confirmed working call from `Matra.english()` through `analyze()` to a result.

Next: [Rust](../guides/rust.md), [Python](../guides/python.md), or [CLI](../guides/cli.md), depending on which surface you are calling from.
