# Security policy

## Supported versions

matra is pre-1.0 and ships from `main`. Security fixes are released as
patches against the latest tagged version. Older versions are not
supported.

| Version | Supported |
|---|---|
| `main` (HEAD) | yes |
| latest 0.x.y release | yes |
| older 0.x.y | no; upgrade to the latest patch |

## Verifying a release

From 0.2.1, every release artifact carries a build provenance
attestation: the `.crate`, each wheel and the sdist. crates.io cannot
host one, so the attestation lives on GitHub and you check it with the
GitHub CLI against the file you downloaded.

A crate, straight from crates.io:

```bash
curl -sSfLO https://static.crates.io/crates/matra/matra-X.Y.Z.crate
gh attestation verify matra-X.Y.Z.crate --repo mox-labs/matra
```

A wheel from PyPI, for your platform:

```bash
pip download matra==X.Y.Z --no-deps --only-binary=:all:
gh attestation verify matra-X.Y.Z-*.whl --repo mox-labs/matra
```

PyPI also carries a PEP 740 attestation for each file, shown with the
file's details on pypi.org and naming `mox-labs/matra` and
`release.yml` as the publisher. The same record is served at
`https://pypi.org/integrity/matra/X.Y.Z/<filename>/provenance`.

`gh attestation verify` exits 0 when the file's SHA-256 matches a
signed attestation from `mox-labs/matra`, and non-zero otherwise. On a
terminal it prints a summary; piped, it prints nothing, so in a script
add `--format json` and check the exit status. Adding
`--signer-workflow mox-labs/matra/.github/workflows/release.yml`
narrows the check from any workflow in the repository to the release
workflow.

What a pass proves: the file was built by `.github/workflows/release.yml`
in `mox-labs/matra`, on a GitHub-hosted runner, from the commit the
attestation names. That is SLSA Build Level 2.

What it does not prove: it is not Level 3, which needs the build to run
in an isolated reusable workflow the verifier can identify, and the
release workflow is not one. It also says nothing about whether the
source is correct or safe. It tells you where the file came from, not
that the code in it is free of bugs or vulnerabilities.

## Reporting a vulnerability

**Do not file a public issue.** Public disclosure before a fix is
available puts every consumer at risk.

Use one of these private channels:

1. **GitHub Security Advisories** (preferred):
   <https://github.com/mox-labs/matra/security/advisories/new>
2. **Email:** open a draft advisory via the link above and the
   maintainer will follow up by the channel you prefer.

Please include:

- A description of the vulnerability and its impact.
- Steps to reproduce (or a proof-of-concept).
- The version (or commit SHA) where you observed it.
- Whether you intend to publish independently and on what timeline.

## What to expect

| Stage | Target time |
|---|---|
| Acknowledgment of report | within 3 business days |
| Initial assessment | within 7 business days |
| Fix released (or workaround documented) | within 30 days for high-severity; longer for low-severity |
| Public disclosure | coordinated with the reporter, typically after the fix ships |

For critical issues (active exploitation, RCE, credential exposure) we
will move faster than the targets above.

## Scope

In scope:

- Vulnerabilities in the `matra` crate or any of its published artifacts
  (crates.io, PyPI wheels).
- Vulnerabilities in `scripts/` that can affect a clone of this repo.
- Vulnerabilities in CI or release tooling visible to PR authors.

Out of scope (open a regular issue or discussion):

- Bugs that do not have a security impact.
- Issues in dependencies (report upstream; we will track but cannot fix).
- Compatibility issues with consumer code.

## Disclosure approach

We follow coordinated disclosure. After a fix is available and consumers
have had a reasonable window to upgrade, we publish an advisory naming
the issue, the reporter (if they wish), and the fix.

The reporter is welcome to publish their own write-up after the
coordinated date. Pre-disclosure, we ask reporters not to publish exploit
details that would let third parties reproduce the issue.

## Acknowledgments

Reporters who follow this policy will be credited in the advisory and in
release notes (unless they prefer to remain anonymous).
