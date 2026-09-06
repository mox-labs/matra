# 0014. The Distribution Matrix

- **Status:** accepted
- **Date:** 2026-09-06
- **Decider(s):** owner decision on the abi3 question, recorded and verified by
  the maintainer role

## Context

matra 0.2.0 is cut and unpublished. Two clean-room install passes, one on macOS
and one on Linux under OrbStack, ran against `main` before this decision and
found that the shipping distribution surface does not match what the
documentation promises, in three independent dimensions.

**Architecture.** `publish-pypi.yml` built three wheels: `linux-x86_64`,
`macos-x86_64`, `macos-aarch64`. Linux aarch64 was absent, and the workflow
comment said so plainly. That is not a niche gap. It is Graviton, Ampere, every
Raspberry Pi, and, far more commonly, any Linux container a developer runs on an
Apple Silicon Mac, where `linux/arm64` is the default platform. Those users fall
to the sdist.

**glibc floor.** The Linux job ran `maturin build` directly on the GitHub runner
rather than inside a manylinux container, so the wheel inherited the runner's
glibc and was tagged `manylinux_2_34`. The Linux pass reproduced the workflow's
own steps and got exactly that, then confirmed pip rejects it on glibc 2.31 with
`not a supported wheel on this platform`. glibc 2.34 excludes Debian 11, Ubuntu
20.04, RHEL/Rocky/Alma 8 and Amazon Linux 2. Those users also fall to the sdist.

**Python version.** No `abi3` feature was declared anywhere, so pyo3 built
against the version-specific CPython ABI and the wheel was tagged `cp312`.
`publish-pypi.yml` pinned `python-version: "3.12"` for every wheel target, so
`pip install matra` on 3.13 or 3.14 falls to the sdist even on a platform that
does ship a wheel. This was not a prediction: the macOS pass ran `uvx matra`
against the already-published 0.1.0 on macOS arm64 and watched it print
`Building matra==0.1.0`, because the machine's interpreter was 3.14. Homebrew's
default `python3` is 3.14 today.

Every one of the three lands the user on the sdist, and the sdist did not work
either. `book/src/tutorials/installation.md` named exactly one prerequisite,
"Rust 1.85 or later". matra parses through UDPipe, which `udpipe-rs` compiles
from C++ during the cargo build, so the build needs a C++ compiler. In a
container with `gcc`, `libc6-dev` and a full rustup stable toolchain but no
`g++`, both `pip install` from the sdist and
`cargo install --path . --features cli,udpipe` failed with
`error occurred in cc-rs: failed to find tool "c++"`. A reader following the page
literally could not install matra on any of the platforms above.

The constraint shaping the options is the org Actions policy: every `uses` ref is
validated at workflow start and must be a commit SHA, which is why the wheel jobs
were written with `actions/*` plus shell rather than a third-party build action
in the first place. That constraint is real and this decision keeps it.

## Options considered

### Option A: leave the matrix, fix only the documentation

Say honestly that wheels cover x86_64 Linux with glibc 2.34+, macOS, and CPython
3.12 exactly, and that everyone else builds from source with Rust and a C++
compiler.

**Pros:**
- No workflow change, no build-time cost, nothing new to maintain.
- Truthful, which the current page is not.

**Cons:**
- Truthfully documents a bad product. The single most common first command,
  `pip install matra` inside a Linux container on a Mac laptop, still ends in a
  compiler error.
- Pushes a Rust plus C++ toolchain requirement onto Python users, which is
  exactly the cost a wheel exists to remove.
- The published metadata for 0.2.0 would then be immutable and wrong for the
  next several years of interpreter releases.

### Option B: a build matrix across Python versions

Keep the version-specific ABI and multiply the matrix by the supported
interpreters: 3.12, 3.13, 3.14, on each of four platforms.

**Pros:**
- No constraint on which pyo3 or CPython APIs the extension may use, now or
  later.
- Marginally faster at the C level, since the version-specific ABI inlines what
  the stable ABI reaches through function calls.

**Cons:**
- Twelve wheel jobs instead of four, growing by four with every CPython release,
  and each new interpreter is a workflow edit rather than a no-op.
- A user on an interpreter released after matra's last publish still falls to the
  sdist. The matrix does not fix the problem, it chases it.
- Twelve artifacts per release to upload, three times the surface to smoke test,
  and three times the wheels on the project page for a reader to reason about.

### Option C: abi3, one wheel per platform, built in a manylinux container

Declare pyo3's `abi3-py312` feature so the extension builds against the CPython
stable ABI, build the Linux wheels inside `ghcr.io/pyo3/maturin`, and add a
native Linux aarch64 runner.

**Pros:**
- One wheel per platform covers 3.12 and every later 3.x, including interpreters
  that do not exist yet. Four jobs, and the count does not grow with CPython.
- The container fixes the glibc floor and the architecture gap in the same move:
  the image is manylinux2014-based and multi-arch, so the same command on an
  arm64 runner produces `manylinux_2_17_aarch64`.
- It is the shape ruff and uv already ship. The release-validation survey records
  that ruff fixes its build `PYTHON_VERSION` at 3.13 precisely because its wheels
  are abi3, and matches on OS and architecture only.
- Reachable with plain `docker run` and an image pinned by index digest, so the
  Actions policy constraint holds without a third-party action.

**Cons:**
- The stable ABI is a real ceiling. Anything pyo3 offers that requires
  version-specific CPython internals is now foreclosed, and the failure arrives
  as a compile error at whatever future moment someone reaches for it.
- Small per-call cost at the C boundary. Not measured here; asserted by the
  mechanism, not by a benchmark on matra.
- Free-threaded CPython is not covered. A free-threaded interpreter accepts
  `abi3t` tags and no plain `abi3` tag, and pyo3 0.29.2's free-threaded stable
  ABI features are `abi3t` and `abi3t-py315`, so it begins at 3.15. Until matra
  can build against it, `python3.14t` falls to the sdist on a platform that does
  ship a wheel.

### Option D: cibuildwheel

Hand the whole matrix to `cibuildwheel`, which orchestrates manylinux containers
and QEMU for foreign architectures.

**Pros:**
- One well-maintained tool covering every target, including musl and Windows.
- Its manylinux handling is the reference implementation everyone else copies.

**Cons:**
- A build orchestrator with its own configuration surface, added to solve a
  problem that four explicit jobs already solve. matra publishes to four targets,
  not thirty.
- Its Linux aarch64 path is QEMU by default, which is slow enough to matter
  against a 45-minute job timeout, where a native arm64 runner is free for public
  repositories.
- Buys generality this project has no use for and pays for it in opacity: the
  reason a wheel has a given tag moves from four lines of shell into a tool's
  defaults.

## Decision

We choose Option C. matra publishes exactly four wheels per release, all abi3:

| Wheel | Runner | Built in | Tag shape |
|---|---|---|---|
| Linux x86_64 | `ubuntu-latest` | `ghcr.io/pyo3/maturin` | `cp312-abi3-manylinux_2_17_x86_64` |
| Linux aarch64 | `ubuntu-24.04-arm` | `ghcr.io/pyo3/maturin` | `cp312-abi3-manylinux_2_17_aarch64` |
| macOS x86_64 | `macos-14`, cross | on the runner | `cp312-abi3-macosx_*_x86_64` |
| macOS arm64 | `macos-14` | on the runner | `cp312-abi3-macosx_*_arm64` |

The reason is that Option C is the only one where the fix and the ongoing cost
both shrink. B and D both answer "more platforms" with "more jobs"; C answers it
once, at the ABI, and the matrix stops growing. The glibc floor and the
architecture gap turn out to be the same fix, because the image that gives broad
manylinux is also the image that runs natively on an arm64 runner.

Windows stays absent. The UDPipe C++ build under MSVC is unverified, and shipping
an untested wheel is worse than shipping none.

The from-source path stays supported and is now documented honestly. Building
matra from source, by any route, requires a Rust toolchain at 1.85 or later
**and** a C++ compiler, because UDPipe is C++.
`book/src/tutorials/installation.md` names the package per platform.

### Getting rustup through the container

Running the build inside the image costs one thing that running it on the runner
did not. `rust-toolchain.toml` names components (`rustfmt`, `clippy`,
`llvm-tools-preview`) the image does not carry, so rustup re-syncs the channel
before cargo runs, and the rename it performs to swap a component crosses the
overlay boundary between the image layer and the container. It fails with
`Invalid cross-device link (os error 18)` and the build stops before it starts.

Two variables can get past it, and they are not equivalent.

`RUSTUP_PERMIT_COPY_RENAME=1` tells rustup to fall back to copying when the
rename fails. It works, and it was the first answer here. But rustup's own
environment-variable documentation marks it *unstable*, says the feature
"sacrifices some transactions protections", and says it "may be removed at any
point"; it is Linux only. That is not a workaround with an unknown lifetime, it
is one with a vendor-declared expiry, sitting on the release path.

`RUSTUP_TOOLCHAIN=stable` sits above `rust-toolchain.toml` in rustup's override
precedence, so rustup resolves to the toolchain already installed in the image
and never syncs a channel at all. The failing operation is not permitted, it is
never reached, and a second toolchain download is avoided along with it. This is
what the workflow uses.

The choice was tested rather than reasoned. On native `linux/arm64`, inside the
pinned `ghcr.io/pyo3/maturin:v1.14.1` image with the repository mounted at
`/io`: with neither variable set, `cargo --version` fails with `Invalid
cross-device link (os error 18)`; with `RUSTUP_TOOLCHAIN=stable` and no permit
flag, `rustup toolchain list` reports `stable-aarch64-unknown-linux-gnu (active,
default)` with no sync line, `cargo --version` returns cleanly, and the full
`maturin build --release --out dist` produces
`matra-0.2.0-cp312-abi3-manylinux_2_17_aarch64.manylinux2014_aarch64.whl`, which
installs under CPython 3.13.15 with `--only-binary :all:` and runs.

Two things are asserted mechanically rather than trusted. `publish-pypi.yml`
fails the release if a wheel is not tagged `cp312-abi3`, or if a Linux wheel is
not `manylinux2014`; and it smoke tests each native wheel under CPython 3.13,
one version newer than it was built against, with `--only-binary :all:` so pip
fails rather than quietly building from source. `ci.yml` asserts the abi3 tag on
every push, so losing the pyo3 feature is caught long before a release.

## Consequences

- Positive: `pip install matra` gets a prebuilt wheel on four platforms and on
  every GIL-enabled CPython from 3.12 up, including releases that postdate the
  publish. Free-threaded builds are the exception, below.
- Positive: the glibc floor drops from 2.34 to 2.17, which brings Debian 11,
  Ubuntu 20.04, RHEL 8 and Amazon Linux 2 back onto the wheel path.
- Positive: the wheel count per release is four and stays four. A new CPython
  release is a no-op for this project.
- Negative: `abi3-py312` is now a constraint on the Python bindings. Any pyo3
  feature outside the stable ABI is unavailable, and dropping abi3 to get one
  back would be a distribution regression, not a local change. That is a
  superseding-ADR decision, and the CI assertion exists so it cannot happen by
  accident.
- Negative: `RUSTUP_TOOLCHAIN=stable` is now load-bearing in the release
  workflow. `rust-toolchain.toml` asks for components the image does not carry,
  so rustup re-syncs the channel inside the container, and the rename it
  performs crosses the overlay boundary between the image layer and the
  container, failing with `Invalid cross-device link (os error 18)`.
  `RUSTUP_TOOLCHAIN` sits above `rust-toolchain.toml` in rustup's override
  precedence, so setting it means the build uses the toolchain already in the
  image and rustup never re-syncs. The workflow carries a comment saying so; a
  future contributor who tidies away the env var breaks every Linux wheel.
- Neutral: the consequence of that override is that the Linux wheels are built
  on whatever stable the pinned maturin image ships, not on the newest stable,
  and without the `rustfmt`, `clippy` and `llvm-tools-preview` components
  `rust-toolchain.toml` requests. A `maturin build` needs none of the three, and
  the compiler version becomes a property of the image digest rather than of the
  day the release ran, which is the more reproducible of the two. If the image
  ever ships a stable below the MSRV the build fails at compile time, loudly.
- Negative: the release now depends on a container image and on
  `ubuntu-24.04-arm` being available. The image is pinned by index digest, so
  moving to a newer maturin is a deliberate edit; the runner label is a GitHub
  product commitment for public repositories, and if it went away the arm64 wheel
  would need QEMU and a longer timeout.
- Neutral: `requires-python = ">=3.12"` in `pyproject.toml` and the abi3 floor
  now say the same thing in two places. They must move together if the floor ever
  rises.
- Neutral: macOS wheels are still built on the runner rather than in a container,
  because macOS deployment targets are set by the SDK, not by a base image. Only
  the Linux half needed to move.

## Validation

This decision is right if, after publishing 0.2.0, a reader on any of the four
platforms and any GIL-enabled CPython from 3.12 up gets a prebuilt wheel, and
the next CPython release requires no change to `publish-pypi.yml`. It is right about the
documentation if a container carrying exactly the prerequisites the installation
page names can build matra from source without adding anything.

It is falsified by any of:

- The Python bindings needing a pyo3 or CPython API the stable ABI does not
  expose. The compile error is the signal, and the answer is a superseding ADR
  choosing Option B, not a quiet feature removal.
- Free-threaded CPython becoming a target matra must serve before the stable ABI
  covers it. This is a live gap, not a hypothetical one: a free-threaded
  interpreter accepts `abi3t` tags and no `abi3` tag at all, and pyo3 0.29.2
  offers only `abi3t` and `abi3t-py315`, so the free-threaded stable ABI starts
  at 3.15. Until then a `python3.14t` user on a platform with a wheel falls to
  the sdist. `book/src/tutorials/installation.md` says so.
- `RUSTUP_TOOLCHAIN` ceasing to outrank `rust-toolchain.toml` in rustup's
  override precedence, which would put the cross-device rename back on the
  release path with the unstable permit flag as the only remaining answer. The
  signal is the same `os error 18` the container originally failed with.
- A measured, user-visible cost from the abi3 call indirection. Asserted as a
  mechanism above and never measured on matra; a benchmark showing it matters
  would reopen the choice.
- Windows becoming a supported target, which adds a fifth wheel and a build this
  ADR explicitly declined to guess about.

## References

- `.github/workflows/publish-pypi.yml`, `.github/workflows/ci.yml`,
  `Cargo.toml` (the pyo3 dependency), `book/src/tutorials/installation.md`.
- The two clean-room install passes of 2026-09-06, macOS and Linux, which
  produced findings B1 through B3 on Linux and B2 on macOS. The Linux pass is the
  source of the reproduced `manylinux_2_34` tag, its rejection on glibc 2.31, and
  the `failed to find tool "c++"` failure; the macOS pass is the source of the
  observed `Building matra==0.1.0` under CPython 3.14.
- `docs/surveys/2026-09-06-release-validation.md`, for how ruff, uv, maturin and
  polars build and smoke test their wheels, and specifically for ruff pinning a
  single build Python because its wheels are abi3.
- ADR-0010, which set the wasm32-open constraint on the embedding adapter and is
  the other place the crate's target surface is decided.
