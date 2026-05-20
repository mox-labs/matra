# 0005. Supply-chain hardening posture

- **Status:** Accepted
- **Date:** 2026-05-20
- **Decider(s):** project maintainer

## Context

Vaani is a substrate library. Downstream consumers (alif, cancan, radix, third-party Rust + Python projects) inherit vaani's supply-chain posture transitively. If vaani publishes from a workflow with a long-lived API token, every downstream depends on that token never leaking. If vaani's actions are pinned to floating tags, a hostile force-push to one of those tags can be injected into every downstream build through vaani's CI cache. The substrate's trust posture is load-bearing.

The rust-mastery research surfaced `sbom-tool/gh-guard` (https://github.com/sbom-tool/gh-guard), a Claude Code plugin that codifies current best practices for Rust supply-chain hardening: OpenSSF Scorecard, CodeQL, action SHA-pinning, Trusted Publishing for crates.io, SLSA L3 provenance attestation, signed tags. The plugin's `templates/workflows/` and `templates/versions.json` give concrete workflow yaml and pin set.

This ADR records vaani's adoption of the gh-guard posture, the manual setup the maintainer performs on GitHub.com and crates.io, and the items deliberately deferred.

## Decision

Adopt the following hardening posture in this iteration (i9):

### 1. Action SHA-pinning

Every `uses:` line in every workflow pins to a 40-character commit SHA with a `# version` comment. Floating tags (e.g., `@v4`, `@stable`) are forbidden; Scorecard's `Pinned-Dependencies` check requires this. Dependabot is configured to update the SHA pins on its weekly run (the cargo + github-actions ecosystems both updated).

Initial pin set (gh-guard v0.2.1's `templates/versions.json`, last-verified 2026-03-10):

| Action | SHA | Version |
|---|---|---|
| `actions/checkout` | `de0fac2e4500dabe0009e67214ff5f5447ce83dd` | v6.0.2 |
| `dtolnay/rust-toolchain` | `efa25f7f19611383d5b0ccf2d1c8914531636bf9` | master (no tagged release) |
| `Swatinem/rust-cache` | `779680da715d629ac1d338a641029a2f4372abb5` | v2 |
| `actions/setup-python` | `a309ff8b426b58ec0e2a45f0f869d46889d02405` | v6.2.0 |
| `EmbarkStudios/cargo-deny-action` | `3fd3802e88374d3fe9159b834c7714ec57d6c979` | v2 |
| `obi1kenobi/cargo-semver-checks-action` | `6b69fcf40e9b5fb17adeb57e4b6ecd020649a239` | v2.9 |
| `actions/upload-pages-artifact` | `fc324d3547104276b827a68afc52ff2a11cc49c9` | v5.0.0 |
| `actions/deploy-pages` | `cd2ce8fcbc39b97be8ca5fce6e763baed58fa128` | v5.0.0 |
| `actions/upload-artifact` | `bbbca2ddaa5d8feaa63e36b76fdaad77386f024f` | v7.0.0 |
| `actions/download-artifact` | `d3f86a106a0bac45b974a628896c90dbdf5c8093` | v4 |
| `ossf/scorecard-action` | `4eaacf0543bb3f2c246792bd56e8cdeffafb205a` | v2.4.3 |
| `github/codeql-action` | `0d579ffd059c29b07949a3cce3983f0780820c98` | v4 |
| `rust-lang/crates-io-auth-action` | `b7e9a28eded4986ec6b1fa40eeee8f8f165559ec` | v1 |
| `slsa-framework/slsa-github-generator` | `@v2.1.0` | (reusable workflow; tag required, not SHA) |

### 2. Least-privilege GITHUB_TOKEN

Every workflow declares `permissions: read-all` at workflow level. Individual jobs escalate only what they need (`security-events: write` for Scorecard SARIF upload, `id-token: write` for OIDC, `contents: write` for release artifacts). Every `actions/checkout` step sets `persist-credentials: false` so a compromised step cannot push back to the repo using the runner's token.

### 3. OpenSSF Scorecard

`.github/workflows/scorecard.yml` runs weekly (Mondays 06:30 UTC), on push to `main`, on branch-protection-rule changes, and on manual dispatch. Results upload as SARIF to the Security tab and publish publicly to `scorecard.dev`. Target: score ≥ 7.5/10.

### 4. CodeQL static analysis

`.github/workflows/codeql.yml` analyzes Rust + Python on push to main, on PR, and weekly (Tuesdays 03:12 UTC). `build-mode: none` for both — sufficient for what CodeQL extracts from a Rust crate and a Python package.

**Manual prerequisite:** the maintainer must disable the "default setup" for CodeQL in Settings → Code security → Code scanning *before* this workflow runs, otherwise GitHub rejects the custom workflow.

### 5. Trusted Publishing (OIDC) for crates.io

`.github/workflows/publish.yml` mints a crates.io scoped token just-in-time via `rust-lang/crates-io-auth-action` (OIDC). No `CARGO_REGISTRY_TOKEN` secret is stored in the repo. The crate-side configuration at `crates.io/crates/vaani/settings → Trusted Publishing` binds the OIDC subject (repo + workflow + environment) to the right to publish vaani.

### 6. Manual approval gate via GitHub environment

The `publish` job in `publish.yml` declares `environment: crates-io`. GitHub pauses the job at this declaration and requires the configured reviewer (the maintainer) to click "Approve" in the Actions UI before `cargo publish` runs. This is the canonical per-publish approval point under the project's "never publish without explicit approval" memory rule.

The gate enforces the approval even if a hostile commit is tagged and pushed: the workflow cannot fire `cargo publish` without the human click.

### 7. SLSA L3 provenance

After successful publish, `slsa-framework/slsa-github-generator@v2.1.0` (reusable workflow) generates an `intoto.jsonl` attestation for the published `.crate` artifact's SHA-256. The release job attaches the attestation to the GitHub Release alongside auto-generated release notes.

Downstream verification:

```bash
slsa-verifier verify-artifact vaani-X.Y.Z.crate \
  --provenance-path vaani.intoto.jsonl \
  --source-uri github.com/mox-labs/vaani \
  --source-tag vX.Y.Z
```

### 8. Signed tags

Tags use SSH-signature signing via `git tag -s`. The `justfile`'s `release` recipe documents the signed-tag command. Local config:

```bash
git config --global gpg.format ssh
git config --global user.signingkey ~/.ssh/id_ed25519.pub
git config --global tag.gpgSign true
```

Past tags do not need re-signing; future ones do.

## Consequences

**Positive:**

- Scorecard "Pinned-Dependencies" check passes (all actions SHA-pinned).
- Scorecard "Token-Permissions" check passes (workflow-level `read-all`).
- Scorecard "Dangerous-Workflow" check passes (no `pull_request_target` with checkout of head ref; `persist-credentials: false`).
- Scorecard "SAST" check passes (CodeQL configured).
- Scorecard "Signed-Releases" check eventually passes (after first SLSA-provenanced release ships).
- Scorecard "Token Leakage" risk eliminated (Trusted Publishing; no `CARGO_REGISTRY_TOKEN` in repo or org secrets).
- Per-publish approval gate moves from "human runs cargo publish locally" to "human approves a workflow deployment that runs cargo publish in a least-privilege ephemeral runner" — same human intent, stronger enforcement.
- Downstream consumers can verify any vaani release's provenance via `slsa-verifier`.

**Negative:**

- Dependabot churn: 13 action pins to keep current, weekly PRs.
- One-time manual setup: GitHub environment, crates.io Trusted Publishing config, CodeQL default-setup disable, SSH signing config.
- The `crates-io` environment requires a reviewer to be available at publish time. Solo maintainer = maintainer is the bottleneck. Acceptable; the discipline is the point.

**Neutral:**

- The `justfile`'s `release` recipe no longer prints `cargo publish ...`; it prints the tag-push command. The publish step lives in the workflow. The audit trail moves from "I ran cargo publish on my laptop" to "I approved deployment NNNN in the Actions UI", which is more auditable.

## Re-open conditions

This decision is reversible if any of:

1. **Trusted Publishing becomes unavailable.** crates.io revokes the feature or the OIDC issuer fails. Fallback: traditional `CARGO_REGISTRY_TOKEN` in the `crates-io` environment's secrets (still gated by required reviewer).
2. **SLSA L3 generator deprecates the reusable workflow.** slsa-github-generator could rev to v3 with breaking changes. Update the `@v2.1.0` reference.
3. **The maintainer set grows beyond a single human.** The required-reviewer set in the `crates-io` environment must update to cover the new maintainers (and ideally enforce N-of-M approval).

Any of these conditions, write a new ADR.

## Validation

This decision is correct if:

- The first SLSA-provenanced release ships and `slsa-verifier` validates the attestation.
- Scorecard scores 7.5+/10 within one week of merge.
- The next `cargo publish` runs through the workflow (not from a laptop) and the human-approval gate logs the approver and timestamp.

Falsified if:

- The workflow can be made to publish without the human-approval click (security regression).
- Trusted Publishing fails to mint a token and the workflow falls back to a leakable secret in the workflow yaml (config regression).
- Scorecard drops below 6.0/10 (hardening regression).

## What is deferred

| Item | Reason | Trigger to revisit |
|---|---|---|
| `cargo-vet` | Heavy review burden for a solo OSS substrate; cargo-deny already covers advisory / license / source policy. | A downstream consumer (alif, cancan) requires explicit audit attestation, or a CVE class emerges that cargo-deny misses. |
| Fuzz testing (`fuzz/`, `fuzz.yml`) | Vaani has no `fuzz/` crate; the parser surface (`ingest`, `decompose`) is still evolving. | When a parser ships a bug class that fuzz would have caught, or when `ingest`/`decompose` stabilize for v1. |
| `osv-scanner.toml` | No SBOM fixtures producing PURL false positives; cargo-deny's advisory check covers the same ground. | If Scorecard's `Vulnerabilities` check scores low after first analysis. |
| Binary releases via `cargo-dist` | Vaani is a library crate; no CLI binary ships from the Rust side. Python wheels are handled by maturin. | If vaani ever ships a `vaani` Rust CLI binary. |
| CII / OpenSSF Best Practices badge | Cosmetic for pre-1.0; nice-to-have post-1.0. | Approaching v1.0 release. |

## References

- [sbom-tool/gh-guard](https://github.com/sbom-tool/gh-guard) — the Claude Code plugin whose patterns this ADR adopts.
- [OpenSSF Scorecard](https://scorecard.dev/) — the analysis tool and its 18-check rubric.
- [crates.io Trusted Publishing announcement](https://blog.rust-lang.org/) — the OIDC-based publishing model.
- [SLSA spec](https://slsa.dev/) — the supply-chain levels framework.
- [slsa-github-generator](https://github.com/slsa-framework/slsa-github-generator) — the reusable workflow generating provenance.
- ADR-0001 (record-architectural-decisions) — the ADR template.
- `~/radix-workspaces/rust-mastery/` — the corpus that grounded the gh-guard discovery via the deep research pass on 2026-05-20.
