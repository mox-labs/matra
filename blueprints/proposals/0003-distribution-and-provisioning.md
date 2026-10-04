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

<pragmatics>

<ask>

Accept EPR-0003 as the record of how matra is distributed and how its models are provisioned at `4fcfb4a`, and settle the two decisions under [Unresolved questions](#unresolved-questions). Each can be answered on its own.

</ask>

<will>

- Cite this proposal, rather than the legacy records it carries forward, when a change touches the wheels, the release workflow or model provisioning; a change to what it describes goes in a new proposal that says so.
- Carry out each decision as you rule it, through the record the process names for that change.
- If the cited code moves before you decide, move the `Pinned at` line forward in a revision, so every citation is checked again, and say what changed.

</will>

<needs>

- An answer to the proposal and to each of the two decisions: accept, accept with a reservation, object or redirect.
- For each assumption listed under Unresolved questions: confirm it or strike it. The first, the release environments' reviewers, is the one only you can check.
- Whether the statement marked as not holding at the pinned commit, under Releasing, should be corrected in a revision.

</needs>

<wont>

- Change the status to `accepted`, or merge a pull request that does.
- Change the release workflow, a pin or a provisioning path either decision names before you rule on it.
- Approve a deployment environment, or publish anything.
- Edit a legacy record, or edit this proposal to describe a later change.

</wont>

<silence>

The proposal stays `proposed`. The workflow and the provisioning it describes ship as they are, since a baseline decides nothing new, and Claude lists the open decisions again at the start of the next session that touches distribution or provisioning.

</silence>

</pragmatics>

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
  GIL-enabled CPython from 3.12 on.
  <claim basis="observed">The wheel carries the library, the Python bindings and the command line, and has no runtime dependencies [pyproject.toml:5-42][pyproject]</claim>.
- `cargo add matra` gets the library with UDPipe; `cargo install matra
  --features cli` gets the binary.
- <claim basis="observed">Building from source, by either route, needs Rust 1.88 or later and a C++ compiler, because UDPipe is C++ [Cargo.toml:1-14][cargo-rust-version]</claim>.
- Windows has no wheel.

### The first run

The first call that needs the parsing model downloads 16 MB from LINDAT
into the model directory (by default `~/.local/share/matra/models`;
[EPR-0004](0004-command-line-and-configuration.md) gives the resolution
order). The command line says so on stderr, naming the file, its size, the
directory and the model's licence;
<claim basis="observed">the library says nothing itself and hands a caller who wants to say something a `ProvisionNotice` [domain.rs:34-72][notice]</claim>:

```rust
let engine = matra::Engine::from_config_with_notice(&cfg, |n| {
    eprintln!("fetching {} ({} bytes) into {}", n.artifact, n.bytes, n.destination.display());
})?;
```

<claim basis="observed">The embedding model, 30 MB across three files from Hugging Face, is fetched the same way the first time `Model2Vec::potion_base_8m` or `Model2Vec::from_config` runs, and has no notice form [domain.rs:34-72][notice-m2v]</claim>.

Both models are separate works with their own licences.
<claim basis="observed">The parsing model is CC BY-NC-SA 4.0, which does not permit commercial use; the notice carries that licence and its URL [nlp/udpipe.rs:18-108][udpipe-pin]</claim>.
<claim basis="assumed">The embedding model is MIT per its model card</claim>,
which the documentation states and
<claim basis="observed">the code does not carry [embed/model2vec.rs:40-113][m2v-pin]</claim>.

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

The rule behind the table:
<claim basis="observed">`model_invalid` is about bytes that arrived [nlp/udpipe.rs:469-489][udpipe-model-invalid]</claim>
([RFC-0015](../legacy/rfcs/0015-provisioning-failures.md)). Behind a proxy
that re-signs TLS, matra cannot be made to trust the proxy; the way through
is to fetch the pinned file by hand and place it in the model directory,
which is exactly as trustworthy because the digest decides.

### Loading a model of your own

<claim basis="observed">`Udpipe::from_path` and `Model2Vec::from_dir` load caller-supplied files and never touch the network or the pinned digest [nlp/udpipe.rs:122-138][udpipe-from-path] [embed/model2vec.rs:143-174][m2v-from-dir]</claim>.

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
([RFC-0013](../legacy/rfcs/0013-attribution-and-citation.md)).
<claim basis="observed">The crate excludes `blueprints/`, `site/` and the tests that read `site/`, and keeps `skills/matra/`, which the binary embeds [Cargo.toml:15-37][cargo-exclude]</claim>.

### The wheel matrix

<claim basis="observed">`build-wheels` builds four wheels: Linux x86_64 on `ubuntu-latest` and Linux aarch64 on `ubuntu-24.04-arm`, both inside `ghcr.io/pyo3/maturin` pinned by index digest; macOS arm64 natively and macOS x86_64 cross-compiled, both on `macos-14` [release.yml:311-351][wheel-matrix]</claim>.
<claim basis="observed">`pyo3` is built with `abi3-py312` [Cargo.toml:53-57][cargo-pyo3]</claim>.
<claim basis="observed">The workflow fails a release if a wheel is not tagged `cp312-abi3` or a Linux wheel is not `manylinux2014`, and installs each native wheel on CPython 3.13 with `--only-binary :all:` [release.yml:402-441][wheel-asserts]</claim>.
<claim basis="observed">CI asserts the abi3 tag on every push [ci.yml:359-387][ci-abi3]</claim>.

<claim basis="observed">`RUSTUP_TOOLCHAIN=stable` in the container run is load-bearing: it keeps rustup from re-syncing the channel inside the image, a rename across the overlay boundary that fails with `os error 18` [release.yml:360-384][wheel-container]</claim>
([RFC-0014](../legacy/rfcs/0014-distribution-matrix.md)).

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

<claim basis="observed">The digest check and the load use the same bytes, so there is no window between verifying and loading in which a file could be swapped [nlp/udpipe.rs:665-694][udpipe-read-verify]</claim>.
<claim basis="observed">Every filesystem failure names the operation and the path [nlp/udpipe.rs:544-554][udpipe-io-at]</claim>.
<claim basis="observed">Transport failures are classified by `transport_failure`, and a rejected certificate gets the explanation in `download_message` [nlp/udpipe.rs:469-489][udpipe-transport] [nlp/udpipe.rs:491-515][udpipe-cert]</claim>.
<claim basis="observed">The classification for every binding is stated in `spec/tests/corpus/items.json` [items.json:64-81][items-provisioning]</claim>.

<claim basis="observed">On `wasm32` the download half of the embedding adapter is not compiled: every fetch item is behind `cfg(not(target_arch = "wasm32"))`, and `ureq` is declared under that target predicate [Cargo.toml:76-84][cargo-ureq]</claim>.

### Releasing

<claim basis="observed">`release.yml` is the one release path, dispatched by hand from `main` with a version [release.yml:1-66][release-header]</claim>;
it verifies, creates an annotated tag, builds the crate, the four wheels and
the sdist, attests them, publishes, and smoke-tests what the registries
serve. What a caller can rely on:

- **A person approved each upload.**
  <claim basis="observed">`publish-crates` runs in the `crates-io` environment and `publish-pypi` in `pypi` [release.yml:522-575][publish-crates] [release.yml:577-628][publish-pypi]</claim>;
  <claim basis="assumed">each of the two environments waits for a reviewer before it publishes</claim>.
  The reviewer setting itself lives in the repository settings, not the
  file;
  <claim basis="observed">the workflow header documents it as a prerequisite [release.yml:1-66][release-reviewers]</claim>.
  Assumed configured, not verifiable from the tree.
- **No registry token is stored.**
  <claim basis="observed">Both uploads use Trusted Publishing through OIDC [release.yml:522-575][publish-crates-oidc] [release.yml:577-628][publish-pypi-oidc]</claim>.
- **Provenance is verifiable.**
  <claim basis="observed">`actions/attest-build-provenance` attests the crate, the wheels and the sdist before publishing; the crate job checks the bytes it uploads equal the bytes attested [release.yml:475-520][attest] [release.yml:522-575][publish-crates-check]</claim>.
  Verify with
  `gh attestation verify matra-X.Y.Z.crate --repo mox-labs/matra`.
  <claim basis="observed">PyPI uploads carry PEP 740 attestations [release.yml:577-628][publish-pypi-oidc]</claim>.
- **Actions are pinned by commit SHA** in the release workflow, which,
  like CI, CodeQL and Scorecard,
  <claim basis="observed">defaults to `permissions: read-all` and grants more per job [release.yml:1-66][release-read-all]</claim>.
  CI runs `cargo-deny` over the dependency tree and
  `cargo-semver-checks` over the public API on every push
  ([`ci.yml`][ci-deny]).
  **Does not hold at the pinned commit:**
  <claim basis="observed">the `cargo-semver-checks` job runs on pull requests only, and a push to `main` skips it [ci.yml:287-316][ci-semver]</claim>.
  The sentence before it is left as written for the owner to rule on.

### No longer in force

- **SLSA Build Level 3 through `slsa-github-generator`.**
  [RFC-0005](../legacy/rfcs/0005-supply-chain-hardening.md) item 7.
  Releases attest at SLSA Build Level 2 with
  `actions/attest-build-provenance`, because the generator must be
  referenced by tag and the organization's policy requires SHA pins
  ([attest][attest]).
- **Signed tags.** RFC-0005 item 8.
  <claim basis="observed">Release tags are annotated and created through the API, which cannot carry a maintainer's signature; the workflow asserts annotation and reports whether the tag is signed [release.yml:257-277][tag-signed]</claim>.
- **Tag-triggered publishing from `publish.yml` and `publish-pypi.yml`.**
  RFC-0005 and RFC-0014 name both. They were merged into `release.yml`,
  dispatched from `main` because the `crates-io` environment's branch
  policy rejected a tag ref ([header][release-header]).
- **Rust 1.85 as the from-source floor.** RFC-0014 states it.
  <claim basis="observed">The floor is 1.88, checked by the `msrv` job [Cargo.toml:1-14][cargo-rust-version] [ci.yml:70-100][ci-msrv]</claim>.
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
  accepts no `abi3` tag, and
  <claim basis="assumed">pyo3's free-threaded stable ABI begins at 3.15</claim>
  (RFC-0014; not re-verified against the current pyo3).
- **Nothing watches the maturin image digest.**
  <claim basis="observed">Dependabot cannot see a digest written inline in a `run:` step, so bumping it is a person's job on the release checklist [release.yml:311-351][wheel-digest]</claim>.
- **A proxy that re-signs TLS blocks the download.**
  <claim basis="observed">matra verifies against root certificates compiled into it and never reads the system trust store [nlp/udpipe.rs:491-515][udpipe-trust]</claim>;
  the documented way through is placing the file by hand.
- **The embedding model's first run is silent**, and
  <claim basis="observed">the code carries neither its size in a notice nor its licence [domain.rs:34-72][notice-m2v] [embed/model2vec.rs:40-113][m2v-pin]</claim>.
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

For the owner to settle in this proposal's review. Each is a decision of its
own.

**Prebuilt command-line binaries.** RFC-0005 deferred binary releases "if
matra ever ships a `matra` Rust CLI binary".
<claim basis="observed">It now does, behind the `cli` feature [Cargo.toml:47-50][cargo-bin]</claim>;
a Rust user gets it with `cargo install`, and a Python user through the
wheel's launcher, but no release publishes a standalone binary.

<decision id="standalone-binary" title="1. Does a release publish a standalone command-line binary?">

<choice key="a" title="Not yet; the two routes stay">

`cargo install matra --features cli` and the wheel's launcher remain the ways to get the command line. The trigger is recorded as fired, and a standalone binary as not yet wanted.

</choice>

<choice key="b" title="Yes, from release.yml">

Each release attaches a prebuilt binary per platform, built and attested beside the wheels.

</choice>

<recommendation choice="a">

Each new artifact joins the build, the attestation and the smoke test of a release path that cannot be undone, and nothing at the pinned commit shows a caller who can run neither `cargo install` nor the wheel's launcher.

</recommendation>

<against>

The trigger RFC-0005 wrote has fired. Building from source needs a C++ compiler, the failure the wheel matrix was rebuilt to avoid, so a user with no Python cannot run the command line without one.

</against>

</decision>

**The embedding model's licence and notice.**
<claim basis="observed">The parsing model's licence is pinned beside its URL and travels in the notice [nlp/udpipe.rs:18-108][udpipe-pin]</claim>;
<claim basis="observed">the embedding model's is stated only in the documentation, and its provisioning has no notice form [embed/model2vec.rs:40-113][m2v-pin] [domain.rs:34-72][notice-m2v]</claim>.

<decision id="embedding-licence" title="2. Does the embedding pin carry its licence and a notice, as the parsing pin does?">

<choice key="a" title="Yes, as the parsing pin does">

The licence and its URL sit beside the three pinned URLs, and the provisioning `Model2Vec` constructors gain a notice form that reports the size, the destination and the licence before a fetch.

</choice>

<choice key="b" title="No; the documentation is enough">

The licence stays in the documentation, and the first run stays silent.

</choice>

<recommendation choice="a">

The parsing pin keeps its licence beside its URL because a new model means checking both, a reason that does not depend on which licence it is. A first run of 30 MB with nothing on screen is the state the notice was built to end for the parsing model.

</recommendation>

<against>

The embedding model's licence places no restriction a caller has to decide on, unlike the parsing model's, and a notice form adds constructors to the public surface of an adapter a caller can replace with their own.

</against>

</decision>

What the owner should confirm or strike, collected from the claims above
that stand on no grounds yet:

<assumptions />

## Future possibilities

- A Windows wheel, once the UDPipe C++ build under MSVC is verified.
- A free-threaded wheel when pyo3's free-threaded stable ABI covers a
  CPython matra supports.
- Signed release tags, if the release is ever tagged where a maintainer's
  key is available.

[cargo-package]: ../../Cargo.toml#L1-L14 "[package]"
[cargo-rust-version]: ../../Cargo.toml#L1-L14 'rust-version = "1.88"'
[cargo-exclude]: ../../Cargo.toml#L15-L37 '"blueprints/*",'
[cargo-bin]: ../../Cargo.toml#L47-L50 "[[bin]]"
[cargo-pyo3]: ../../Cargo.toml#L53-L57 '"abi3-py312"'
[cargo-ureq]: ../../Cargo.toml#L76-L84 'ureq = { version = "3.3", optional = true }'
[pyproject]: ../../pyproject.toml#L5-L42 "dependencies = []"
[notice]: ../../src/domain.rs#L34-L72 "pub struct ProvisionNotice {"
[notice-m2v]: ../../src/domain.rs#L34-L72 "and has no notice form, so a semantic first run is still silent."
[udpipe-pin]: ../../src/nlp/udpipe.rs#L18-L108 'const ENGLISH_MODEL_LICENSE: &str = "CC BY-NC-SA 4.0 (non-commercial)";'
[udpipe-from-path]: ../../src/nlp/udpipe.rs#L122-L138 "pub fn from_path(path: impl AsRef<Path>)"
[udpipe-provision]: ../../src/nlp/udpipe.rs#L276-L324 "fn provision("
[udpipe-fetch-verified]: ../../src/nlp/udpipe.rs#L326-L366 "for _ in 0..2 {"
[udpipe-agent]: ../../src/nlp/udpipe.rs#L434-L450 ".https_only(true)"
[udpipe-transport]: ../../src/nlp/udpipe.rs#L469-L489 "fn transport_failure(url: &str, error: &ureq::Error) -> Error {"
[udpipe-model-invalid]: ../../src/nlp/udpipe.rs#L469-L489 "is reserved for bytes that did arrive"
[udpipe-cert]: ../../src/nlp/udpipe.rs#L491-L515 "fn download_message(url: &str, error: &ureq::Error) -> String {"
[udpipe-trust]: ../../src/nlp/udpipe.rs#L491-L515 "compiled into the binary and never reads the system trust store"
[udpipe-io-at]: ../../src/nlp/udpipe.rs#L544-L554 "fn io_at(operation: &str, path: &Path, error: &std::io::Error) -> Error {"
[udpipe-reclaim]: ../../src/nlp/udpipe.rs#L571-L663 "reclaim_stale_temp_dirs(parent, SystemTime::now());"
[udpipe-read-verify]: ../../src/nlp/udpipe.rs#L665-L694 "fn read_and_verify("
[m2v-pin]: ../../src/embed/model2vec.rs#L40-L113 "const POTION_BASE_8M_URLS: [&str; 3] = ["
[m2v-from-dir]: ../../src/embed/model2vec.rs#L143-L174 "No network is touched, ever"
[m2v-provision]: ../../src/embed/model2vec.rs#L291-L359 "fn provision<F>("
[items-provisioning]: ../../spec/tests/corpus/items.json#L64-L81 '"provisioning": {'
[release-header]: ../../.github/workflows/release.yml#L1-L66 "workflow_dispatch:"
[release-reviewers]: ../../.github/workflows/release.yml#L1-L66 "Required reviewers: the maintainer set."
[release-read-all]: ../../.github/workflows/release.yml#L1-L66 "permissions: read-all"
[tag-signed]: ../../.github/workflows/release.yml#L257-L277 "Annotated is asserted; signed is reported."
[wheel-matrix]: ../../.github/workflows/release.yml#L311-L351 "runner: ubuntu-24.04-arm"
[wheel-digest]: ../../.github/workflows/release.yml#L311-L351 "Nothing watches the image digest"
[wheel-container]: ../../.github/workflows/release.yml#L360-L384 "RUSTUP_TOOLCHAIN is not optional."
[wheel-asserts]: ../../.github/workflows/release.yml#L402-L441 "*-cp312-abi3-*) ;;"
[attest]: ../../.github/workflows/release.yml#L475-L520 "uses: actions/attest-build-provenance@"
[publish-crates]: ../../.github/workflows/release.yml#L522-L575 "environment: crates-io"
[publish-crates-check]: ../../.github/workflows/release.yml#L522-L575 "What ships is what was attested"
[publish-crates-oidc]: ../../.github/workflows/release.yml#L522-L575 "Trusted Publishing (OIDC)"
[publish-pypi]: ../../.github/workflows/release.yml#L577-L628 "environment: pypi"
[publish-pypi-oidc]: ../../.github/workflows/release.yml#L577-L628 "Trusted Publishing OIDC, and PEP 740 attestations"
[ci-msrv]: ../../.github/workflows/ci.yml#L70-L100 'RUSTUP_TOOLCHAIN: "1.88"'
[ci-deny]: ../../.github/workflows/ci.yml#L287-L316 "name: cargo-deny"
[ci-semver]: ../../.github/workflows/ci.yml#L287-L316 "if: github.event_name == 'pull_request'"
[ci-abi3]: ../../.github/workflows/ci.yml#L359-L387 "the tag is asserted here on every"
