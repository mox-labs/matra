# Platforms and models

What each of matra's install routes needs and installs, and the models matra downloads. [Install matra](../tutorials/installation.md) has the steps; [One core, three install routes](../explanation/install-routes.md) explains the choices behind them.

## Install routes

| Route | Command | Builds from source | Features in the build |
|---|---|---|---|
| Rust library | `cargo add matra@0.2` | yes | `udpipe` (the default) |
| CLI binary | `cargo install matra --version '^0.2' --features cli` | yes | `udpipe cli` |
| Python package | `pip install 'matra>=0.2'` | only where no wheel fits | `udpipe model2vec python cli` |

The Python package installs the library and the `matra` command, which is the Rust CLI reached through the extension module, and has no runtime dependencies of its own.

## Toolchains

| Needed for | Requirement |
|---|---|
| Every route that builds from source | Rust 1.88 or later, the minimum supported Rust version |
| Every route that builds from source | A C++ compiler: UDPipe is a C++ library, which `udpipe-rs` builds during the cargo build. A C compiler alone is not enough. |
| The Python package | CPython 3.12 or later |

## Python wheels

| Platform | Wheel |
|---|---|
| Linux x86_64, Linux aarch64 | manylinux2014: glibc 2.17 or newer, which covers Debian 11, Ubuntu 20.04, RHEL 8 and Amazon Linux 2 |
| macOS x86_64, macOS arm64 | yes |
| Windows | none; the UDPipe build under MSVC is unverified, so the sdist route there is untested |

The wheels are built against the CPython stable ABI and tagged `abi3`, so one wheel per platform serves 3.12 and every later 3.x on GIL-enabled CPython. A free-threaded interpreter (`python3.14t`, for one) accepts a different tag, `abi3t`, so it gets no wheel on any platform and builds the sdist; pyo3 gains the free-threaded stable ABI at 3.15. Wherever no wheel fits, `pip` builds the sdist, which needs the toolchains above.

Every release file from 0.2.1 on carries a build provenance attestation; [Verifying a release](https://github.com/mox-labs/matra/blob/main/SECURITY.md#verifying-a-release) has the commands.

## Version requirements

| Requirement | Used by | Accepts |
|---|---|---|
| `matra@0.2`, `--version '^0.2'` | cargo | 0.2.0 or a later release semver-compatible with it, so not 0.3.0 |
| `'matra>=0.2'` | pip, uv, uvx | 0.2.0 and every later release |

An install line with no version takes the newest release on the registry. 0.1.0 is still published, and its `matra` command is a separate Python implementation with its own `--json` shape and a model directory fixed at `~/.matra/models`.

## Build features

`matra --version` prints the version, then the features the build was compiled with:

```
matra 0.2.1
features: udpipe cli
```

That is the CLI binary from `cargo install`. The Python package prints `features: udpipe model2vec python cli`. Every analysis command behaves the same in both builds.

## The model directory

The library, the CLI and the Python package resolve the model directory the same way, first match wins:

1. A directory passed explicitly: `--model-dir` on the command line, or the directory argument of `Udpipe::english`, `Matra.english` and the other constructors that take one.
2. `MATRA_MODEL_DIR`.
3. The `models` directory of the data root, which is `MATRA_DATA_DIR`, else `$XDG_DATA_HOME/matra`, else `~/.local/share/matra`.
4. A pre-existing, non-empty `~/.matra/models` from an older install, when the location in 3 does not exist yet. matra never creates `~/.matra`; a selected legacy cache is used as the model directory, downloads included.

[Where matra keeps things](../guides/cli.md#where-matra-keeps-things) has the config file and data root beside it, and `matra config show` prints which rung each value came from.

## The model download

| | Parsing model | Embedding model |
|---|---|---|
| Fetched by | `Engine::with_defaults`, `Udpipe::english`, `Udpipe::from_config`, `Matra.english` and the command line, on first use | `Model2Vec::potion_base_8m`, `Model2Vec::from_config` and their Python forms, on first use |
| Size | about 16 MB | about 30 MB, three files |
| Verified against | a SHA-256 pinned in the source | a SHA-256 over the three files, pinned in the source |
| Download cap | 64 MiB | 64 MiB per file |

The download is held in memory, checked against the pinned hash, and written only if it matches, so nothing unverified reaches the model directory and an interrupted transfer leaves nothing behind. A mismatch downloads once more; a second mismatch is an error, and nothing is loaded. A cached parsing model that fails the check is replaced only when new bytes verify, so a re-download that cannot reach the network leaves the file that was there. The embedding model is never written over files already in its directory; [Provisioning](semantic-clusters.md#provisioning) has what it does instead.

The library prints nothing while it downloads. The command line prints two lines on stderr before the parsing model's download, naming the artifact, its size, where it is going and the model's license; `--quiet` silences them. [Errors](errors.md#provisioning-failures) has what each failure returns.

## Model licenses

matra's code is MIT. The models it downloads are separate files, published by other people under licenses of their own.

| Model | Used for | Source | License |
| --- | --- | --- | --- |
| `english-ewt-ud-2.5-191206.udpipe` | every parse | [LINDAT record 11234/1-3131](https://hdl.handle.net/11234/1-3131), published by ÚFAL, Charles University | [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) (Attribution-NonCommercial-ShareAlike 4.0 International) |
| potion-base-8M | semantic clusters only | [minishlab/potion-base-8M](https://huggingface.co/minishlab/potion-base-8M) on Hugging Face | MIT, per its model card |

The parsing model's license does not permit commercial use. The [UDPipe models page](https://ufal.mff.cuni.cz/udpipe/1/models) states that the UDPipe 1 models it publishes for Universal Dependencies are distributed under CC BY-NC-SA. This page reports what the licenses say and does not interpret them; read the license text for the terms.

The command line and the config file always use this parsing model: `--model-dir`, `MATRA_MODEL_DIR` and the other settings change where it is stored, not which model it is. `models.udpipe`, which earlier releases wrote into the config file, is deprecated and selects nothing. A different UDPipe model file loads by path from the library, through `Udpipe::from_path` in [Rust](../guides/rust.md#construct-a-provider) or `Matra.from_path` in [Python](../guides/python.md#load-the-model-from-a-directory-you-choose).
