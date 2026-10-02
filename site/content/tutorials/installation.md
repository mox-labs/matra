# Install matra

Install matra as a Rust library, a command-line binary or a Python package, then check that it works. Each route is one command; take the one for the language you call matra from, or more than one. [Platforms and models](../reference/platforms.md) lists what each route needs and installs, and [One core, three install routes](../explanation/install-routes.md) explains why the commands look as they do.

To check a release file before you install it, follow [Verifying a release](https://github.com/mox-labs/matra/blob/main/SECURITY.md#verifying-a-release) in the security policy.

---

## Requirements

For the Rust library and the CLI, and for the Python package on a platform with no wheel, install Rust 1.88 or later (check with `rustc --version`) and a C++ compiler. A C compiler on its own is not enough: without C++ the build stops with `error occurred in cc-rs: failed to find tool "c++"`.

| Platform | Package |
| --- | --- |
| Debian, Ubuntu | `apt install build-essential` |
| Fedora, RHEL, Rocky, Alma | `dnf install gcc-c++` |
| Alpine | `apk add g++` |
| Arch | `pacman -S base-devel` |
| macOS | `xcode-select --install` |

For the Python package, use Python 3.12 or later (check with `python --version`). [Python wheels](../reference/platforms.md#python-wheels) lists the platforms with a prebuilt wheel; on any other, and on a free-threaded interpreter, `pip` builds from source and needs the Rust toolchain and the C++ compiler above.

---

## The Rust library

```bash
cargo add matra@0.2
```

The default features include `udpipe`, so `matra::nlp::udpipe::Udpipe` is available without a feature flag.

---

## The CLI binary

```bash
cargo install matra --version '^0.2' --features cli
```

`--features cli` is required: the binary is gated behind it. This compiles from source and places the binary in your cargo bin directory, which can take a minute or more. Confirm it landed:

```bash
matra --version
```

Expected output:

```
matra 0.2.1
features: udpipe cli
```

The second line names the features this build was compiled with, so the Python package's banner below is longer.

---

## The Python package

```bash
pip install 'matra>=0.2'    # or: uv add 'matra>=0.2'
```

This installs the library and the `matra` command together; the command is the same program as the CLI binary above, so `uvx 'matra>=0.2' analyze essay.md` and the installed binary do the same thing. Keep the version floor in the line: [why the install lines name a version](../explanation/install-routes.md#why-every-install-line-names-a-version) says what an unpinned one can give you.

On a platform with a wheel this installs in seconds. Its version banner reads:

```
matra 0.2.1
features: udpipe model2vec python cli
```

The verify step below doubles as the check: if `from matra import Matra` fails there, the package did not install correctly.

---

## The English model

There is nothing to install. The library, the CLI and the Python package each download the English UDPipe model (about 16 MB) on first use, verify it against a pinned hash, and cache it in the model directory, by default `~/.local/share/matra/models`.

To keep it somewhere else, set `MATRA_MODEL_DIR`, pass `--model-dir` to the CLI, or pass the directory to `Udpipe::english` in Rust or `Matra.english` in Python. [The model directory](../reference/platforms.md#the-model-directory) gives the full resolution order, and [the model download](../reference/platforms.md#the-model-download) what happens when a download fails its check.

---

## Model licenses

matra's code is MIT, and the two models it downloads are separate works with licenses of their own. The English parsing model comes from [LINDAT](https://hdl.handle.net/11234/1-3131) under CC BY-NC-SA 4.0, which does not permit commercial use; read it before you use matra commercially. The embedding model, potion-base-8M, comes from [Hugging Face](https://huggingface.co/minishlab/potion-base-8M) under MIT per its model card, and is used for semantic clusters only. [Model licenses](../reference/platforms.md#model-licenses) has both in a table, with links to the license texts.

The command line and the config file always use this model. To parse with a different UDPipe model file, load it by path from the library: `Udpipe::from_path` in [Rust](../guides/rust.md#construct-a-provider) or `Matra.from_path` in [Python](../guides/python.md#load-the-model-from-a-directory-you-choose).

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

`Matra.english("/some/directory")` is the same call with the directory named explicitly. Pass a real path: the string goes straight to Rust's `create_dir_all`, which does not expand `~`.

Expected output:

```
sections: 1
vocabulary_ttr: 0.8571428571428571
```

That first run fetches the model from a university server in Prague and prints nothing while it does. Cold starts measured on a fast connection ranged from 3 to 35 seconds; on a slow network it takes longer, and the command is waiting on the network rather than working. Every run after that loads the cached file and touches no network, in about a second.

If `Matra.english()` raises `OSError`, either the download never arrived or the model directory could not be written, and the message says which by naming the URL or the path. Check your network connection first; then run `matra config show` to see which directory matra resolved and check the permissions on it. If it raises `RuntimeError`, bytes did arrive and then failed the pinned hash check; run the snippet again.

---

## What you have

- matra installed as a Rust crate, a CLI binary, a Python package, or some combination, all from the same 0.2.x core.
- The English UDPipe model cached in the resolved model directory, verified against a pinned hash.
- A confirmed working call from `Matra.english()` through `analyze()` to a result.

Next: [Your first analysis](first-analysis.md) walks through what that result holds, or go to [Rust](../guides/rust.md), [Python](../guides/python.md) or [CLI](../guides/cli.md) for the surface you are calling from.
