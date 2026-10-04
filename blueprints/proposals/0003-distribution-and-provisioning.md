# EPR-0003: Distribution and provisioning

- Feature Name: `distribution_and_provisioning`
- Start Date: 2026-10-04
- Proposal PR: [#133](https://github.com/mox-labs/matra/pull/133)
- Tracking issue: none (a baseline proposal describes code that already ships, so there is no milestone to track)
- Status: proposed (only the owner changes it to `accepted`)
- Pinned at: `4fcfb4adc85524243c5e392becd4f490e3e42252`

## Summary

matra ships as one crate on crates.io and one package on PyPI, under one
name, one version and one attribution. PyPI gets four prebuilt wheels per
release, all against CPython's stable ABI from 3.12, plus a source
distribution. The models matra parses and embeds with are not shipped: each
is pinned in the source by URL, size and SHA-256, downloaded on first use
into a directory matra resolves, verified in memory, and loaded from the
bytes that were verified. A failure to fetch is reported as I/O, never as an
invalid model. A release runs from one workflow that a person approves for
each registry, and every artifact carries build provenance a caller can
verify.

This is a baseline proposal. It describes the code at the pinned commit and
carries forward the reasons that still hold from RFC-0005, RFC-0010
(decision 6), RFC-0011, RFC-0013, RFC-0014 and RFC-0015. A later proposal
that changes any of it says so. Unmarked statements about the code are
observed at the pinned commit through the link beside them; a claim that is
inferred or assumed says so.

## Motivation

A caller meets this foundation before any other: `pip install matra`,
`cargo add matra`, and the first run that has no model yet. Each decision
here was made after that first meeting went wrong.

**Wheels for the platforms people actually run.** Two clean-room installs
before 0.2.0 found that the wheels missed Linux on arm64, required glibc
2.34, and were tied to CPython 3.12, so most first installs fell to a source
build, which then failed for want of a C++ compiler
([RFC-0014](../legacy/rfcs/0014-distribution-matrix.md)). The stable ABI and
a manylinux container fixed all three at once.

**A model is pinned, not named.** matra downloads exactly one artifact set
per model, the one whose digest is a constant in its source. That is what
makes a download acceptable in a library at all
([RFC-0010](../legacy/rfcs/0010-embeddings-adapter.md) decision 6, as
amended by [RFC-0011](../legacy/rfcs/0011-out-of-the-box.md)): what matra
refuses is unpinned network access, fetching whatever a name resolves to
today.

**A failed fetch must say what failed.** A clean-room pass measured a first
run that wrote nothing for up to 35 seconds, had no timeout at all, left a
15 MB orphan on Ctrl-C, and reported DNS, TLS and full-disk failures as
"invalid model" ([RFC-0015](../legacy/rfcs/0015-provisioning-failures.md)).
The provisioning code today answers each of those.

**Publishing is irreversible.** Every downstream build inherits matra's
supply-chain posture, so publishing runs from a least-privilege workflow
with no stored registry token and waits for a person
([RFC-0005](../legacy/rfcs/0005-supply-chain-hardening.md)).

## Guide-level explanation

### Installing

- `pip install matra` or `uvx matra` gets a prebuilt wheel on Linux x86_64
  and aarch64 (glibc 2.17 and later), and macOS x86_64 and arm64, for every
  GIL-enabled CPython from 3.12 on. The wheel carries the library, the
  Python bindings and the command line, and has no runtime dependencies
  ([`pyproject.toml`][pyproject]).
- `cargo add matra` gets the library with UDPipe; `cargo install matra
  --features cli` gets the binary.
- Building from source, by either route, needs Rust 1.88 or later and a C++
  compiler, because UDPipe is C++ ([`rust-version`][cargo-package]).
- Windows has no wheel.

### The first run

The first call that needs the parsing model downloads 16 MB from LINDAT
into the model directory (by default `~/.local/share/matra/models`;
[EPR-0004](0004-command-line-and-configuration.md) gives the resolution
order). The command line says so on stderr, naming the file, its size, the
directory and the model's licence; the library says nothing itself and
hands a caller who wants to say something a `ProvisionNotice`
([`ProvisionNotice`][notice]):

```rust
let engine = matra::Engine::from_config_with_notice(&cfg, |n| {
    eprintln!("fetching {} ({} bytes) into {}", n.artifact, n.bytes, n.destination.display());
})?;
```

The embedding model, 30 MB across three files from Hugging Face, is fetched
the same way the first time `Model2Vec::potion_base_8m` or
`Model2Vec::from_config` runs, and has no notice form.

Both models are separate works with their own licences. The parsing model
is CC BY-NC-SA 4.0, which does not permit commercial use; the notice
carries that licence and its URL ([pin][udpipe-pin]). The embedding model is
MIT per its model card, which the documentation states and the code does
not carry.

### When the first run fails

| Condition | Error | Kind |
|---|---|---|
| No network, unreachable host, refused connection | `Error::Io`, `ErrorKind::NotConnected`, naming the URL | `io` |
| Connect or transfer timeout | `Error::Io`, `ErrorKind::TimedOut` | `io` |
| TLS certificate rejected | `Error::Io`, with a sentence saying why installing a CA does not help | `io` |
| Non-2xx status | `Error::Io` | `io` |
| Response over 64 MiB | `Error::InputTooLarge`, `what` = `udpipe_download` or `embedding_download` | `input_too_large` |
| Model directory cannot be created or written | `Error::Io`, naming the operation and the path | `io` |
| Bytes arrived and failed the digest twice, or verified and did not load | `Error::ModelInvalid` | `model_invalid` |

The rule behind the table: `model_invalid` is about bytes that arrived
([RFC-0015](../legacy/rfcs/0015-provisioning-failures.md)). Behind a proxy
that re-signs TLS, matra cannot be made to trust the proxy; the way through
is to fetch the pinned file by hand and place it in the model directory,
which is exactly as trustworthy because the digest decides.

### Loading a model of your own

`Udpipe::from_path` and `Model2Vec::from_dir` load caller-supplied files and
never touch the network or the pinned digest
([`from_path`][udpipe-from-path], [`from_dir`][m2v-from-dir]).

## Reference-level explanation

### Registries and attribution

| | crates.io | PyPI |
|---|---|---|
| Name and version | `matra` 0.2.1 ([`Cargo.toml`][cargo-package]) | `matra` 0.2.1 ([`pyproject.toml`][pyproject]) |
| Author | `mox labs <yza.v@moxlabs.org>` | `mox labs`, `yza.v@moxlabs.org` |
| Licence | MIT | MIT |

The same organization is the copyright holder in `LICENSE`, the entity
author in `CITATION.cff`, and the author in `.claude-plugin/plugin.json`,
because registry metadata is immutable per version and attribution that
disagrees across files cannot be corrected in place
([RFC-0013](../legacy/rfcs/0013-attribution-and-citation.md)). The crate
excludes `blueprints/`, `site/` and the tests that read `site/`, and keeps
`skills/matra/`, which the binary embeds ([exclude list][cargo-exclude]).

### The wheel matrix

[`build-wheels`][wheel-matrix] builds four wheels: Linux x86_64 on
`ubuntu-latest` and Linux aarch64 on `ubuntu-24.04-arm`, both inside
`ghcr.io/pyo3/maturin` pinned by index digest; macOS arm64 natively and
macOS x86_64 cross-compiled, both on `macos-14`. `pyo3` is built with
`abi3-py312` ([`Cargo.toml`][cargo-pyo3]). The workflow fails a release if a
wheel is not tagged `cp312-abi3` or a Linux wheel is not `manylinux2014`,
and installs each native wheel on CPython 3.13 with `--only-binary :all:`
([assertions][wheel-asserts]). CI asserts the abi3 tag on every push
([`ci.yml`][ci-abi3]).

`RUSTUP_TOOLCHAIN=stable` in the container run is load-bearing: it keeps
rustup from re-syncing the channel inside the image, a rename across the
overlay boundary that fails with `os error 18`
([`build-wheels`][wheel-container]; [RFC-0014](../legacy/rfcs/0014-distribution-matrix.md)).

### Provisioning

The two adapters implement one discipline, each in its own file, as
[RFC-0015](../legacy/rfcs/0015-provisioning-failures.md) decided: a shared
module would add a third file to the wiring to save a few dozen lines.

| | UDPipe ([`nlp/udpipe.rs`][udpipe-pin]) | Model2Vec ([`embed/model2vec.rs`][m2v-pin]) |
|---|---|---|
| Pin | one file: filename, size, SHA-256, URL, licence, licence URL | three files: one SHA-256 over all three in order, three URLs at a fixed Hugging Face commit |
| Caps | 64 MiB per response; 300 s end to end; 30 s to connect | the same |
| Client | `ureq` with `https_only` ([agent][udpipe-agent]) | the same |
| Order | fetch into memory, verify, write through a temporary and one rename, load the verified bytes ([`provision`][udpipe-provision]) | fetch all three into memory, verify the set, write as one transaction, load the verified bytes ([`provision`][m2v-provision]) |
| Retry | one more fetch on a digest mismatch; a transport failure is not retried ([`fetch_verified`][udpipe-fetch-verified]) | the same |
| A file already there that is not the pin | replaced only when verified bytes arrive; never deleted | the directory is refused and nothing is written or removed |
| Leftover temporaries | reclaimed when older than 600 s, twice the fetch budget ([reclaim][udpipe-reclaim]) | the same rule |
| Notice | `_with_notice` forms on `Udpipe` and `Engine` | none |

The asymmetry on a file already present is deliberate. The UDPipe filename
names one release, so a file under it that is not that release is matra's
own stale cache. The three Model2Vec filenames belong to the artifact
format, so a directory holding them may be a caller's own model, and the
provisioner refuses rather than overwrite it.

The digest check and the load use the same bytes, so there is no window
between verifying and loading in which a file could be swapped
([`read_and_verify`][udpipe-read-verify]). Every filesystem failure names
the operation and the path ([`io_at`][udpipe-io-at]). Transport failures are
classified by [`transport_failure`][udpipe-transport], and a rejected
certificate gets the explanation in [`download_message`][udpipe-cert]. The
classification for every binding is stated in
[`spec/tests/corpus/items.json`][items-provisioning].

On `wasm32` the download half of the embedding adapter is not compiled:
every fetch item is behind `cfg(not(target_arch = "wasm32"))`, and `ureq`
is declared under that target predicate ([`Cargo.toml`][cargo-ureq]).

### Releasing

[`release.yml`][release-header] is the one release path, dispatched by hand
from `main` with a version; it verifies, creates an annotated tag, builds
the crate, the four wheels and the sdist, attests them, publishes, and
smoke-tests what the registries serve. What a caller can rely on:

- **A person approved each upload.** `publish-crates` runs in the
  `crates-io` environment and `publish-pypi` in `pypi`; each waits for a
  reviewer ([crates][publish-crates], [PyPI][publish-pypi]). The
  reviewer setting itself lives in the repository settings, not the file;
  the workflow header documents it as a prerequisite. Assumed configured,
  not verifiable from the tree.
- **No registry token is stored.** Both uploads use Trusted Publishing
  through OIDC.
- **Provenance is verifiable.** `actions/attest-build-provenance` attests
  the crate, the wheels and the sdist before publishing; the crate job
  checks the bytes it uploads equal the bytes attested
  ([attest][attest], [check][publish-crates]). Verify with
  `gh attestation verify matra-X.Y.Z.crate --repo mox-labs/matra`. PyPI
  uploads carry PEP 740 attestations.
- **Actions are pinned by commit SHA** in the release workflow, which,
  like CI, CodeQL and Scorecard, defaults to `permissions: read-all` and
  grants more per job. CI runs `cargo-deny` over the dependency tree and
  `cargo-semver-checks` over the public API on every push
  ([`ci.yml`][ci-deny]).

### No longer in force

- **SLSA Build Level 3 through `slsa-github-generator`.**
  [RFC-0005](../legacy/rfcs/0005-supply-chain-hardening.md) item 7.
  Releases attest at SLSA Build Level 2 with
  `actions/attest-build-provenance`, because the generator must be
  referenced by tag and the organization's policy requires SHA pins
  ([attest][attest]).
- **Signed tags.** RFC-0005 item 8. Release tags are annotated and created
  through the API, which cannot carry a maintainer's signature; the
  workflow asserts annotation and reports whether the tag is signed
  ([tag check][tag-signed]).
- **Tag-triggered publishing from `publish.yml` and `publish-pypi.yml`.**
  RFC-0005 and RFC-0014 name both. They were merged into `release.yml`,
  dispatched from `main` because the `crates-io` environment's branch
  policy rejected a tag ref ([header][release-header]).
- **Rust 1.85 as the from-source floor.** RFC-0014 states it. The floor is
  1.88 ([`Cargo.toml`][cargo-package]), checked by the `msrv` job
  ([`ci.yml`][ci-msrv]).
- **"No network" for the embedding model.** RFC-0010 decision 6 as first
  written, and EP-0009's "the caller supplies model files". Amended by
  RFC-0011: a pinned download is allowed, and `Model2Vec::potion_base_8m`
  provisions.
- **The UDPipe fetch through `udpipe_rs::download_model_from_url`.**
  Replaced by matra's own fetch, which is what made the timeouts, the size
  cap and the classification possible (RFC-0015).
- **`book/book.toml` as a place attribution lives.** RFC-0013 lists it; the
  mdBook site it configured has been replaced, and the file does not exist.

## Drawbacks

- **The stable ABI is a ceiling.** Any pyo3 feature outside it is
  unavailable, and leaving it would multiply the wheel matrix by the
  CPython versions (RFC-0014).
- **Free-threaded CPython falls to the sdist.** A free-threaded interpreter
  accepts no `abi3` tag, and pyo3's free-threaded stable ABI begins at 3.15
  (RFC-0014; not re-verified against the current pyo3).
- **Nothing watches the maturin image digest.** Dependabot cannot see a
  digest written inline in a `run:` step, so bumping it is a person's job
  on the release checklist ([comment][wheel-matrix]).
- **A proxy that re-signs TLS blocks the download.** matra verifies against
  root certificates compiled into it and never reads the system trust store;
  the documented way through is placing the file by hand.
- **The embedding model's first run is silent**, and the code carries
  neither its size in a notice nor its licence.
- **The parsing model's licence is non-commercial.** matra is MIT; its
  default parsing model is not, and a commercial user must read and decide.

## Rationale and alternatives

- **Document the old wheel matrix honestly instead of fixing it**: rejected
  in [RFC-0014](../legacy/rfcs/0014-distribution-matrix.md); it truthfully
  documents a bad first install.
- **One wheel per CPython version**: twelve jobs growing with each CPython
  release, and an interpreter released after a publish still falls to the
  sdist. Rejected in RFC-0014.
- **cibuildwheel**: a build orchestrator with QEMU for arm64, for a matrix
  four explicit jobs cover. Rejected in RFC-0014.
- **`RUSTUP_PERMIT_COPY_RENAME=1`**: works, but rustup documents it as
  unstable and removable, and it lets the calendar choose the compiler.
  Rejected in RFC-0014.
- **Keep the upstream UDPipe fetch and patch around it**: cannot add a
  timeout or a size cap to a fetch matra does not perform. Rejected in
  [RFC-0015](../legacy/rfcs/0015-provisioning-failures.md).
- **A new `Error` variant for transport failures**: grows the kind
  vocabulary with every place I/O happens, for a distinction
  `io::ErrorKind` already carries. Rejected in RFC-0015.
- **An escape hatch to the system trust store**: widens what can authorize a
  model download, when hand placement costs one `curl` and is exactly as
  trustworthy. Rejected in RFC-0015 pending a threat model.
- **Attribution to an individual, or to both an individual and the
  organization**: contradicts the repository and homepage, or reintroduces
  the disagreement in a form that looks intentional. Rejected in
  [RFC-0013](../legacy/rfcs/0013-attribution-and-citation.md).

## Prior art

- ruff and uv ship abi3 wheels built with maturin and pin a single build
  CPython because of it (RFC-0014's release-validation survey).
- Trusted Publishing on crates.io and PyPI, and GitHub artifact
  attestations, are the registry and platform mechanisms used here.
- [RFC-0005](../legacy/rfcs/0005-supply-chain-hardening.md),
  [RFC-0013](../legacy/rfcs/0013-attribution-and-citation.md),
  [RFC-0014](../legacy/rfcs/0014-distribution-matrix.md) and
  [RFC-0015](../legacy/rfcs/0015-provisioning-failures.md) hold the full
  history, including the clean-room measurements.

## Unresolved questions

- **Prebuilt command-line binaries.** RFC-0005 deferred binary releases
  "if matra ever ships a `matra` Rust CLI binary". It now does, behind the
  `cli` feature ([`[[bin]]`][cargo-bin]); a Rust user gets it with
  `cargo install`, and a Python user through the wheel's launcher, but no
  release publishes a standalone binary. Is that trigger fired, and is a
  standalone binary wanted?
- **The embedding model's licence and notice.** The parsing model's licence
  is pinned beside its URL and travels in the notice; the embedding
  model's is stated only in the documentation, and its provisioning has no
  notice form. Should the embedding pin carry its licence and a notice, as
  the parsing pin does?

## Future possibilities

- A Windows wheel, once the UDPipe C++ build under MSVC is verified.
- A free-threaded wheel when pyo3's free-threaded stable ABI covers a
  CPython matra supports.
- Signed release tags, if the release is ever tagged where a maintainer's
  key is available.

[cargo-package]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/Cargo.toml#L1-L14
[cargo-exclude]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/Cargo.toml#L15-L37
[cargo-bin]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/Cargo.toml#L47-L50
[cargo-pyo3]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/Cargo.toml#L53-L57
[cargo-ureq]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/Cargo.toml#L76-L84
[pyproject]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/pyproject.toml#L5-L42
[notice]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L34-L72
[udpipe-pin]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L18-L108
[udpipe-from-path]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L122-L138
[udpipe-provision]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L276-L324
[udpipe-fetch-verified]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L326-L366
[udpipe-agent]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L434-L450
[udpipe-transport]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L469-L489
[udpipe-cert]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L491-L515
[udpipe-io-at]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L544-L554
[udpipe-reclaim]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L571-L663
[udpipe-read-verify]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/nlp/udpipe.rs#L665-L694
[m2v-pin]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/model2vec.rs#L40-L113
[m2v-from-dir]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/model2vec.rs#L143-L174
[m2v-provision]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/model2vec.rs#L291-L359
[items-provisioning]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/spec/tests/corpus/items.json#L64-L81
[release-header]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/release.yml#L1-L66
[tag-signed]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/release.yml#L257-L277
[wheel-matrix]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/release.yml#L311-L351
[wheel-container]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/release.yml#L360-L384
[wheel-asserts]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/release.yml#L402-L441
[attest]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/release.yml#L475-L520
[publish-crates]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/release.yml#L522-L575
[publish-pypi]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/release.yml#L577-L628
[ci-msrv]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/ci.yml#L70-L100
[ci-deny]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/ci.yml#L287-L316
[ci-abi3]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/ci.yml#L359-L387
