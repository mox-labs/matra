# Installation

matra ships three ways from one Rust core: a Rust library you add as a dependency, a command-line binary, and a Python package. The current version is 0.0.1. matra is not yet published to crates.io or PyPI, so every path below builds from source, from the same git repository.

---

## Requirements

**Rust 1.85 or later (MSRV).** Check with `rustc --version`. All three install paths need this, including the Python one: with no published wheel to download, `uv` compiles the Rust core locally.

**Python 3.12 or later**, if you want the Python package. Check with `python --version`.

---

## The Rust library

```bash
cargo add matra --git https://github.com/mox-labs/matra
```

This writes a git dependency into `Cargo.toml`:

```toml
[dependencies]
matra = { git = "https://github.com/mox-labs/matra" }
```

The default feature set includes `udpipe`, so `matra::nlp::udpipe::Udpipe` is available without enabling any extra feature flags. The first `cargo build` in your project compiles matra's Rust core along with the rest of your dependencies.

---

## The CLI binary

```bash
cargo install --git https://github.com/mox-labs/matra matra --features cli
```

matra's binary target is gated behind the `cli` feature, so `--features cli` is required. This command compiles matra and its dependencies from source and places the binary in your cargo bin directory; the first build can take a minute or more, depending on your machine.

Confirm it landed:

```bash
matra --version
```

Expected output:

```
matra 0.0.1
```

---

## The Python package

```bash
uv add matra --git https://github.com/mox-labs/matra
```

`uv` resolves the git source and hands the build to `maturin`, the build backend declared in `pyproject.toml`. `maturin` compiles the Rust core with `cargo`, the same compile step as the two paths above, so this also takes a minute or more the first time. `pyproject.toml` declares `requires-python = ">=3.12"`.

Verify the install, further down this page, doubles as the check for this step: if `from matra import Matra` fails there, the Python package did not install correctly.

---

## The English model

matra parses through UDPipe. None of the three install paths bundle the English model: the library, the binary, and the Python package each download it independently on first use and cache it on disk. The Rust and Python APIs take the model directory as an explicit argument; the CLI is the only surface that supplies a default, `~/.matra/models`.

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

- matra installed as a Rust crate, a CLI binary, a Python package, or some combination, all built from the same git source.
- The English UDPipe model cached at `~/.matra/models`, verified against a pinned hash.
- A confirmed working call from `Matra.english()` through `analyze()` to a result.

Next: [Rust](../guides/rust.md), [Python](../guides/python.md), or [CLI](../guides/cli.md), depending on which surface you are calling from.
