# One core, three install routes

matra is one Rust core published three ways: a library on crates.io, a command-line binary built from the same crate, and a Python package on PyPI. The routes differ in what they compile and what they print about themselves, and not in what an analysis returns. This page explains why the install commands look as they do. [Install matra](../tutorials/installation.md) has the steps and [Platforms and models](../reference/platforms.md) the facts.

## One program behind every `matra` command

The `matra` command the Python package installs is not a Python program. From 0.2.0 it is the Rust command line, compiled into the library and reached through the extension module, so `pip install` and `cargo install` give you the same program with two launchers. 0.1.0 had a second command line written in Python, with a `--json` shape of its own and a model directory hardcoded to the pre-0.2.0 location. Two implementations of one interface drifted apart, which is why the command line now lives in the library: a binary target cannot be reached from Python, and a library module can.

## Why every install line names a version

An unpinned install line takes whatever the registry hands out. Today that is 0.2.x on both registries, and it will not stay that way. 0.1.0 is also still published, and its `matra` is the separate Python implementation above. The failure an unpinned line risks is not an error: the install succeeds and gives you a program these pages do not describe, so its version banner and its output do not match theirs.

The Python lines take a floor, `'matra>=0.2'`, because what they claim holds for 0.2.0 and every release after it, and an exact pin would still name 0.2.0 after a later release ships. The cargo lines take a caret requirement, `matra@0.2` and `--version '^0.2'`, which is how cargo states a version. Cargo reads a change in the first non-zero component as breaking, so the caret accepts 0.2.0 and later 0.2 releases and stops short of 0.3.0, which a floor would not.

## Same program, different builds

The two banners differ, and both are correct. The CLI binary from `cargo install` is built with the features the command line needs, `udpipe` and `cli`. The wheel is built with `python` and `model2vec` on top, because the Python package also carries the library, embedding included. `matra --version` prints the version and then the features of the build that is running, so a report quoting it says which build produced a result. Every analysis command behaves the same in both.

Free-threaded CPython is the one interpreter the wheels do not reach, and the reason is a tag, not a platform. The wheels are built once per platform against the CPython stable ABI and tagged `abi3`, which every GIL-enabled CPython from 3.12 accepts. A free-threaded interpreter accepts `abi3t` instead, so it falls back to building from source even where a wheel exists.

## Why the first run is quiet in the library

The first call that needs the parsing model downloads it, about 16 MB from a university server, and that can take from a few seconds to over half a minute. The command line says so on stderr before it starts, because a blank terminal for that long is indistinguishable from a hung process. The library says nothing, because a program embedding matra chooses its own reporting: the constructors that take a notice callback hand it the same facts, and the program decides what to do with them.
