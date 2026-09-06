# Installation

matra ships three ways from one Rust core: a Rust library on [crates.io](https://crates.io/crates/matra), a command-line binary installed from the same crate, and a Python package on [PyPI](https://pypi.org/project/matra/).

---

## Requirements

**Rust 1.85 or later (MSRV)** for the Rust library and the CLI. Check with `rustc --version`.

**A C++ compiler** for anything that compiles from source: the Rust library, the CLI, and the Python package on a platform with no wheel. matra parses through UDPipe, which is a C++ library that `udpipe-rs` builds during the cargo build. A C compiler on its own is not enough. Without a C++ compiler the build stops with `error occurred in cc-rs: failed to find tool "c++"`, whichever route you took.

| Platform | Package |
| --- | --- |
| Debian, Ubuntu | `apt install build-essential` |
| Fedora, RHEL, Rocky, Alma | `dnf install gcc-c++` |
| Alpine | `apk add g++` |
| Arch | `pacman -S base-devel` |
| macOS | `xcode-select --install` |

**Python 3.12 or later** for the Python package. Check with `python --version`. Wheels ship for Linux x86_64, Linux aarch64, macOS x86_64 and macOS arm64. They are built against the CPython stable ABI, so one wheel per platform serves 3.12 and every later 3.x on GIL-enabled CPython, rather than only the version it was compiled against. The Linux wheels are manylinux2014, which asks for glibc 2.17 or newer and so reaches back through Debian 11, Ubuntu 20.04, RHEL 8 and Amazon Linux 2. On anything else `pip` builds the sdist, which needs the Rust toolchain and the C++ compiler above.

Free-threaded CPython is the exception to "every later 3.x", and it is a real one. A free-threaded interpreter accepts a different ABI tag, `abi3t`, and matra's wheels are tagged `abi3`, so `python3.14t` gets no wheel even on a platform this page promises one for and falls to the sdist. pyo3 gains the free-threaded stable ABI at 3.15, so this closes when matra can build against it; until then the prerequisites above apply to a free-threaded install.

Windows is not a target yet. No wheel ships for it and the UDPipe build under MSVC is unverified, so the sdist route there is untested rather than known to work.

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
matra 0.2.0
features: udpipe cli
```

The second line names the features *this* build was compiled with, so it is not the same on every install route. `cargo install matra --features cli` prints the line above. The Python package is compiled with more features and prints a longer one; see below.

---

## The Python package

```bash
pip install matra    # or: uv add matra
```

This installs the library and the `matra` command together. From 0.2.0 the command is the same Rust CLI, reached through the extension module rather than reimplemented in Python, so `uvx 'matra>=0.2' analyze essay.md` and the installed binary do the same thing.

The version in that line is deliberate. The claim holds for 0.2.0 and later, not for what an unpinned `uvx matra` resolves to today: 0.1.0 shipped a second CLI written in Python, with a `--json` shape of its own and a model directory hardcoded to the pre-0.2.0 location. Pin the floor until 0.2.0 is the release `uvx` picks. A floor rather than an exact pin, because the claim holds for every release from 0.2.0 onward and an exact pin would still be naming 0.2.0 after 0.3.0 ships.

It is not, however, the same *build*. The wheel is compiled with the Python and embedding features on top of the CLI, so its version banner reads:

```
matra 0.2.0
features: udpipe model2vec python cli
```

Both banners are correct for their own build, and every analysis command behaves identically across the two. The Python package has no runtime dependencies of its own.

On the platforms with wheels this downloads a prebuilt binary and installs in seconds, on any GIL-enabled CPython from 3.12 up. A free-threaded interpreter builds from source instead, for the reason given under Requirements. The verify step further down this page doubles as the check: if `from matra import Matra` fails there, the package did not install correctly.

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

That first run fetches about 16 MB from a university server in Prague, and prints nothing while it does. Cold starts measured on a fast connection ranged from 3 to 35 seconds. A slow or throttled network can take longer, and the command is waiting on the network rather than working. Every run after that loads the cached file and touches no network, in about a second.

If `Matra.english()` raises `OSError`, either the download never arrived or the model directory could not be written, and the message says which by naming the URL or the path. Check your network connection first; then run `matra config show` to see which directory matra resolved and check the permissions on it. If it raises `RuntimeError`, bytes did arrive and then failed the pinned hash check; run the snippet again.

---

## What you have

- matra installed as a Rust crate, a CLI binary, a Python package, or some combination, all from the same published 0.2.0 core.
- The English UDPipe model cached under the resolved model directory, verified against a pinned hash.
- A confirmed working call from `Matra.english()` through `analyze()` to a result.

Next: [Rust](../guides/rust.md), [Python](../guides/python.md), or [CLI](../guides/cli.md), depending on which surface you are calling from.
