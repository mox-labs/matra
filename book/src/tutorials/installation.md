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

Expected output, with the second line naming the features this build was compiled with:

```
matra 0.1.0
features: udpipe cli
```

---

## The Python package

```bash
pip install matra    # or: uv add matra
```

This installs the library and the `matra` command together. The command is the same Rust CLI `cargo install` gives you, reached through the extension module rather than reimplemented in Python, so `uvx matra analyze essay.md` and the installed binary do the same thing. The Python package has no runtime dependencies of its own.

On the platforms with wheels this downloads a prebuilt binary and installs in seconds. The verify step further down this page doubles as the check: if `from matra import Matra` fails there, the package did not install correctly.

---

## The English model

matra parses through UDPipe. None of the three install paths bundle the English model: the library, the CLI, and the Python package each download it on first use and cache it on disk. Every surface resolves the directory the same way. `Engine::with_defaults()`, `Matra.english()`, and the CLI all use `MATRA_MODEL_DIR`, else the `models` subdirectory of `$XDG_DATA_HOME/matra`, which defaults to `~/.local/share/matra`, falling back to a pre-existing, non-empty `~/.matra/models` from an older install when the new location does not exist yet (matra never creates `~/.matra`, but a selected legacy cache is used as the model directory, downloads included). The Rust and Python APIs also take the directory as an explicit argument, and the CLI takes `--model-dir`.

matra writes the download (about 16 MB) to a temporary location first, then moves it into place, and checks the bytes against a fixed hash before loading them. If a file fails that check, matra deletes it and re-downloads once; if the second attempt still does not match, matra returns an error instead of loading an unverified file.

---

## Verify the install

Run this once. No arguments and no environment: it resolves the model directory, downloads and caches the model on this first run, then parses a sentence through it and prints two results:

```python
from matra import Matra

v = Matra.english()

result = v.analyze("The committee approved the proposal without debate.")
print("sections:", len(result["sections"]))
print("vocabulary_ttr:", result["vocabulary_ttr"])
```

`Matra.english("/some/directory")` is the same call with the directory named explicitly, which is what you want when the model belongs somewhere specific. Pass a real path: the string goes straight to Rust's `create_dir_all`, which does not expand `~`.

Expected output:

```
sections: 1
vocabulary_ttr: 0.8571428571428571
```

That first run downloads about 16 MB and can take several seconds depending on your connection. Every run after that loads the cached file and touches no network.

If `Matra.english()` raises `RuntimeError`, the download or the hash check failed; check your network connection and run the snippet again. If it raises `OSError`, matra could not create or write to the model directory; run `matra config show` to see which directory it resolved and check the permissions on it.

---

## What you have

- matra installed as a Rust crate, a CLI binary, a Python package, or some combination, all from the same published 0.1.0 core.
- The English UDPipe model cached under the resolved model directory, verified against a pinned hash.
- A confirmed working call from `Matra.english()` through `analyze()` to a result.

Next: [Rust](../guides/rust.md), [Python](../guides/python.md), or [CLI](../guides/cli.md), depending on which surface you are calling from.
