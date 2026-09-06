# matra 0.2.0, clean-Linux install and first-run verification

> **Point-in-time record.** This is the report of one exploratory pass against
> commit `7197ca0`, kept as the evidence behind
> `.claude/skills/e2e-validation/SKILL.md`. It describes what matra was on
> 2026-09-06, not what it is now, and several findings below were fixed before
> 0.2.0 shipped. The `[0.2.0]` section of `CHANGELOG.md` records what was done
> about them. Nothing here should be read as a description of current
> behaviour.

**Subject:** worktree at commit `7197ca0` (main), `<worktree>`
**Date:** 2026-09-06
**Scope:** Linux only. Nothing was published anywhere. No repo file was modified. `cargo` was never run on the host and nothing was written to the host `target/`.
**Host:** Apple Silicon, OrbStack, Docker server 29.4.0, `docker info` architecture `aarch64 linux`. **Every container in this report ran `linux/arm64` natively.** No `linux/amd64` emulation was used anywhere, so no timing here is distorted by QEMU. The consequences of that choice for the findings are stated where they matter.

---

## 1. The wheel

Built in the maturin manylinux image, no publish step.

**Command**

```
docker build --progress=plain -f .../Dockerfile.wheel -t matra-wheel-build <worktree>
```

with the Dockerfile

```dockerfile
FROM ghcr.io/pyo3/maturin:latest
ENV RUSTUP_PERMIT_COPY_RENAME=1
COPY . /work
WORKDIR /work
ENV CARGO_TARGET_DIR=/build-target
RUN date +%s > /t0 && maturin build --release --interpreter python3.12 --out /dist && date +%s > /t1
RUN ls -la /dist && echo "seconds:" && expr $(cat /t1) - $(cat /t0)
```

**Result**

```
#8 99.60     Finished `release` profile [optimized] target(s) in 1m 22s
#8 100.0 📦 Built wheel for CPython 3.12 to /dist/matra-0.2.0-cp312-cp312-manylinux_2_17_aarch64.manylinux2014_aarch64.whl
#9 0.127 -rw-r--r-- 1 root root 4773689 Sep  6 14:43 matra-0.2.0-cp312-cp312-manylinux_2_17_aarch64.manylinux2014_aarch64.whl
#9 0.131 100
```

- **Platform tag:** `cp312-cp312-manylinux_2_17_aarch64.manylinux2014_aarch64` (glibc floor 2.17)
- **Size:** 4,773,689 bytes (4.55 MiB); the extension module inside is 12,209,792 bytes uncompressed
- **Build time:** 100 s wall in the container
- **`WHEEL` metadata:** `Generator: maturin (1.15.0)`, two tags, `Root-Is-Purelib: false`
- The wheel carries an SBOM (`matra-0.2.0.dist-info/sboms/matra.cyclonedx.json`, 232 KB) and the licence.

**Does the standard maturin image have a C++ toolchain?** Yes.

```
#5 0.123 NAME="CentOS Linux"   VERSION="7 (AltArch)"
#5 0.124 /opt/rh/devtoolset-10/root/usr/bin/g++
#5 0.127 gcc (GCC) 10.2.1 20210130 (Red Hat 10.2.1-11)
#5 0.183 rustc 1.98.0
```

`ghcr.io/pyo3/maturin:latest` builds matra with no additions. **One thing a release pipeline must add**: the repo's `rust-toolchain.toml` asks for `channel = "stable"` plus `rustfmt, clippy, llvm-tools-preview`, which makes rustup re-sync the channel inside the build. Under a `docker build` overlay that fails:

```
#8 0.435 info: syncing channel updates for stable-aarch64-unknown-linux-gnu
#8 0.435 error: could not rename 'component' file from '/root/.rustup/toolchains/.../share/zsh/site-functions'
         to '/root/.rustup/tmp/...': Invalid cross-device link (os error 18)
#8 0.435 💥 maturin failed
```

Setting `RUSTUP_PERMIT_COPY_RENAME=1` fixes it; that is the whole delta.

---

## 2. Install into a genuinely clean container and use it

Container: `python:3.12-slim`, nothing else, wheel bind-mounted read-only.

```
docker run -d --name matra-a -v .../out:/wheel:ro -v .../scripts:/scripts:ro python:3.12-slim sleep infinity
```

Baseline: `Python 3.12.14`, `pip 25.0.1`, `HOME=/root`, uid 0, `ca-certificates 20250419` present.

**Install**, `pip install --no-cache-dir /wheel/matra-...whl`

```
Processing /wheel/matra-0.2.0-cp312-cp312-manylinux_2_17_aarch64.manylinux2014_aarch64.whl
Installing collected packages: matra
Successfully installed matra-0.2.0
rc=0
```

**`matra --version`**

```
/usr/local/bin/matra
matra 0.2.0
features: udpipe model2vec python cli
rc=0
```

**First (cold) analysis.** Run under a pty so every byte the user would see is timestamped. `/work/essay.md` is a 598-byte markdown file I wrote in the container.

```
$ docker exec -w /work matra-a python /scripts/ptytime.py matra analyze essay.md
[ 17.562s] '\x1b[1messay.md\x1b[0m\r\n  sentences          8\r\n  words              91\r\n  mean sentence len  11.4\r\n  sentence len sd    6.9\r\n  passive ratio      0.375\r\n'
[ 17.567s] EXIT=0
```

**What is on the screen during that wait: nothing.** The first byte of output arrives at t = 17.562 s. Before it, zero bytes on stdout and zero on stderr, no banner, no "downloading", no path, no progress bar, no spinner, no dots. The cursor sits on a blank line and the terminal is indistinguishable from a hung process. Then the whole result appears at once and the process exits.

Cold-start time is not stable. Five separate cold runs, each in a container with an empty model directory:

| run | seconds |
|---|---|
| `matra-a`, first ever run | 17.56 |
| fresh container | 3.40 |
| fresh container | 34.51 |
| fresh container | 22.34 |
| fresh container, shared volume | 23.33 |

The variance is the LINDAT endpoint's throughput, not local work: an interrupted run had already written 16,252,074 of 16,309,608 bytes after 2 seconds on a good draw. Warm runs are flat at **1.07 s / 1.10 s / 1.09 s**.

**JSON output**, `matra analyze essay.md --json` returned a 49.7 KB well-formed envelope: `format_version: 1`, `command: "analyze"`, `input: "essay.md"`, then `result.sections[].paragraphs[].sentences[].tokens[]` with the full CoNLL-U column set (`id, text, lemma, pos, xpos, feats, head, dep, deps, misc, is_punct`). rc=0.

**`matra config show`**

```
data_dir = "/root/.local/share/matra" # default
model_dir = "/root/.local/share/matra/models" # default
models.udpipe = "english-ewt-ud-2.5-191206" # default
models.embedding = "potion-base-8M" # default
semantic.threshold = 0.85 # default
summarize.n = 3 # default
summarize.algorithm = "tfidf" # default
keyphrases.n = 10 # default
keyphrases.algorithm = "rake" # default
rc=0
```

**The documented verify snippet** (`book/src/tutorials/installation.md:72-89`) reproduces its documented output byte for byte:

```
$ docker exec matra-a python -c 'from matra import Matra
v = Matra.english()
result = v.analyze("The committee approved the proposal without debate.")
print("sections:", len(result["sections"]))
print("vocabulary_ttr:", result["vocabulary_ttr"])'
sections: 1
vocabulary_ttr: 0.8571428571428571
```

---

## 3. The failure hunt

### 3.1 No CA certificates, nothing found, and the reason matters

```
$ docker run --rm matra-clean sh -c 'rm -rf /etc/ssl/certs /usr/share/ca-certificates /usr/lib/ssl/certs /etc/ca-certificates; matra analyze essay.md; echo "rc=$?"'
certs removed
essay.md
  sentences          8
  ...
rc=0
```

And the `cargo install`-built binary in `debian:bookworm-slim`, where the `ca-certificates` package was never installed at all:

```
$ docker run --rm matra-rust-bin sh -c 'dpkg -l ca-certificates 2>&1 | tail -1; ls /etc/ssl/certs | wc -l; matra analyze README.md | head -8'
un  ca-certificates <none>       <none>       (no description available)
1
README.md
  sentences          49
  ...
rc=0
```

matra does not read the system trust store. `Cargo.toml:59` declares `ureq = { version = "3.3", optional = true }` with default features, and `udpipe-rs-0.2.0/Cargo.toml:67-68` declares `ureq = "3.1"` likewise; `ureq-3.3.0/Cargo.toml:74-77` defaults to `["rustls", "gzip"]` and `:101-105` expands `rustls` to include `rustls-webpki-roots`, i.e. `webpki-roots`. The build log confirms the crate is linked (`#11 10.25 Compiling webpki-roots v1.0.9`).

That is a *good* outcome for a bare container and a *bad* one behind a corporate TLS proxy, see 3.2.

### 3.2 TLS interception, a leaked Rust `Debug` string, no URL, wrong category

Simulated a corporate MITM: a container serving HTTPS on 443 with a self-signed cert for `CN=lindat.mff.cuni.cz`, reached via `--add-host`.

```
$ docker run --rm --network matra-tls-net --add-host lindat.mff.cuni.cz:192.168.97.2 matra-clean sh -c 'matra analyze essay.md; echo "rc=$?"'
matra: invalid model: UDPipe error: Failed to download: io: invalid peer certificate: Other(OtherError(CaUsedAsEndEntity))
rc=2
```

Four things wrong in one line: the failure is labelled **"invalid model"** when no model was ever fetched (`src/nlp/udpipe.rs:169` maps every download failure to `Error::ModelInvalid`, `src/domain.rs:110-111`); it is sub-labelled **`io:`** when it is a trust failure; it ends in a raw `rustls` enum `Debug` rendering, `Other(OtherError(CaUsedAsEndEntity))`, which means nothing to a person; and it names neither the host nor the URL it was talking to. Nothing tells the user that matra carries its own trust roots and will never see their proxy CA. `grep` over `Cargo.toml` and `src/nlp/udpipe.rs` finds no `SSL_CERT_FILE`, `SSL_CERT_DIR`, `platform-verifier` or `native-tls` (exit 1, no matches), and `grep -rn "proxy|certificat|SSL_CERT|firewall|offline|air.gap" book/src/` returns only unrelated prose. There is no escape hatch and no documented manual-placement path.

The exit code (2) is right, and matches `book/src/guides/cli.md:238`.

### 3.3 No network at all, comprehensible, but mislabelled and anonymous

```
$ docker run --rm --network none matra-clean sh -c 'matra analyze essay.md; echo "rc=$?"'
matra: invalid model: UDPipe error: Failed to download: io: failed to lookup address information: Temporary failure in name resolution
rc=2
```

Same with `--json`: the error is plain text on stderr, stdout is empty, no JSON envelope. Exit 2 is correct. A user can guess "no network" from the wording, but the message names no host, no URL, and no directory, and again calls it an "invalid model".

### 3.4 No timeout anywhere, an unbounded silent hang

Pointed the hostname at a non-routable address so the TCP connect never completes:

```
$ docker run --rm --add-host lindat.mff.cuni.cz:10.255.255.1 ... matra-clean python /scripts/hang.py 90
STILL RUNNING after 90.1s, no output produced, no message on screen
bytes on stdout: 0  bytes on stderr: 0
```

`udpipe-rs-0.2.0/src/lib.rs:627-632` calls `ureq::get(url).call()` with the default config, and `ureq-3.3.0/src/config.rs:894-908` sets every timeout to `None` except `await_100`. matra sets none of its own. A stalled or throttled LINDAT connection therefore hangs the user's terminal indefinitely with a blank screen and no way to tell a slow download from a dead one.

### 3.5 HOME unset, nothing found, the message is exactly right

```
$ docker run --rm matra-clean env -u HOME matra config show
matra: invalid input: cannot locate the data directory: set MATRA_DATA_DIR, XDG_DATA_HOME, or HOME
rc=2
```

Identical for `matra analyze`. This is generated by `src/config.rs:514-516` and documented at `book/src/reference/errors.md:80`. Clean, specific, actionable.

### 3.6 Unwritable HOME, the error names nothing

Running as `nobody` (`HOME=/nonexistent`):

```
$ docker run --rm --user 65534:65534 matra-clean sh -c 'echo "HOME=[$HOME] uid=$(id -u)"; matra config show; matra analyze essay.md; echo "rc=$?"'
HOME=[/nonexistent] uid=65534
data_dir = "/nonexistent/.local/share/matra" # default
model_dir = "/nonexistent/.local/share/matra/models" # default
...
matra: io error: Permission denied (os error 13)
rc=2
```

`config show` works and does tell you the resolved directory, which is the recovery `book/src/tutorials/installation.md:93` prescribes, that recovery path is real and it works. But the failure itself is a bare `Permission denied (os error 13)` from `std::fs::create_dir_all` at `src/nlp/udpipe.rs:78`, with no path and no verb. The user is not told what matra tried to create.

### 3.7 Read-only filesystem, same shape

```
$ docker run --rm --read-only --tmpfs /tmp matra-clean sh -c 'matra config show | head -2; matra analyze essay.md; echo "rc=$?"'
data_dir = "/root/.local/share/matra" # default
model_dir = "/root/.local/share/matra/models" # default
matra: io error: Read-only file system (os error 30)
rc=2
```

Correct exit code, no path in the message.

A read-only model directory that **already holds a valid model** works, which is the deployment shape that matters for containers:

```
$ docker run --rm -v matra-models:/models:ro --env MATRA_MODEL_DIR=/models matra-clean sh -c 'matra analyze essay.md; echo "rc=$?"'
essay.md
  sentences          8
  ...
rc=0
```

### 3.8 Full disk, partial-download cleanup works exactly as claimed

8 MiB tmpfs as the model directory, model is 15.6 MiB:

```
$ docker run --rm --tmpfs /models:size=8m --env MATRA_MODEL_DIR=/models matra-clean sh -c 'matra analyze essay.md; echo "rc=$?"; echo "--- leftovers ---"; ls -laR /models'
matra: invalid model: UDPipe error: No space left on device (os error 28)
rc=2
--- leftovers ---
/models:
total 0
drwxrwxrwt 2 root root 40 Sep  6 14:47 .
drwxr-xr-x 1 root root 24 Sep  6 14:47 ..
```

**Nothing left behind.** The `Drop`-based cleanup at `src/nlp/udpipe.rs:132-150` did its job: no truncated model, no `.tmp.download.*` directory. The claim in `book/src/tutorials/installation.md:64` holds under a disk-full failure. (The message once more says "invalid model" for a disk condition.)

### 3.9 Ctrl-C during the silent wait, a 15.5 MB orphan that nothing reclaims

This is the case a real user hits, because the wait is silent and long enough to look hung. Interrupted a cold run 2 seconds in:

```
$ docker run --rm ... --env MATRA_MODEL_DIR=/models matra-clean python /scripts/interrupt.py SIGINT
sending SIGINT
rc= -9
stdout:
stderr:
--- tree of /models
  DIR  /models/.tmp.download.6
  FILE /models/.tmp.download.6/english-ewt-ud-2.5-191206.udpipe 16252074
```

A 15.5 MB partial file survives in `.tmp.download.<pid>/`. `src/nlp/udpipe.rs:143-146` only reclaims a stale temp directory whose name matches **the current process's own pid**, so on a real machine (where pids do not repeat any time soon) that file is permanent. There is no `matra clean` and nothing else ever looks in there. The source comment at `:161-164` acknowledges the leak; what it does not say is that the recovery it describes effectively never fires outside a container.

The same run also shows the process did not die on SIGINT within 20 s (`rc = -9` is my harness's follow-up `kill`), consistent with 3.4: a blocking `ureq` read is not interruptible.

### 3.10 Second container on a shared model volume, fast, no re-download

```
# container 1, empty volume
seconds: 23.33  rc=0
# container 2, same volume
seconds: 1.07  rc=0
# volume afterwards
-rw-r--r-- 1 root root 16309608 Sep  6 14:48 english-ewt-ud-2.5-191206.udpipe
```

One file, correct size, second start 22× faster. Confirmed.

### 3.11 Concurrent first runs, safe

Three `matra analyze` processes racing on one empty model directory:

```
proc 0 rc=0 stderr=''
proc 1 rc=0 stderr=''
proc 2 rc=0 stderr=''
elapsed 27.4s
--- model dir
  FILE /models/english-ewt-ud-2.5-191206.udpipe 16309608
```

All three succeed, one correct file, no temp directories left. The per-pid-temp-dir-plus-rename design at `src/nlp/udpipe.rs:152-173` does what its doc comment claims.

### 3.12 Corrupted cache, self-heals silently

Replaced the cached model with 1000 random bytes:

```
seconds: 3.48  rc=0
# after
-rw-r--r-- 1 root root 16309608 english-ewt-ud-2.5-191206.udpipe
```

Size mismatch is caught by `read_and_verify` (`src/nlp/udpipe.rs:189-197`), the bad file is removed and re-fetched, and the run succeeds. Correct, though the user is told nothing about the 16 MB re-download that just happened.

### 3.13 Everything written outside the documented locations, nothing found

`docker diff matra-a` after: pip install, `--version`, `--help`, three analyses, `--json`, `config show`, `config init`, the Python verify snippet, and a `Model2Vec.potion_base_8m()` fetch. Everything except pip's own `site-packages` and `__pycache__` writes:

```
A /scripts                                    # my read-only mount
C /tmp ; A /tmp/t0 ; A /tmp/t1                # my own timing files
C /etc/ssl/certs ; A .../orbstack-root.crt    # OrbStack's cert injection, not matra
A /usr/local/bin/matra                        # pip entry point
A /wheel ; A /work ; A /work/essay.md         # my mounts and test file
A /root/.config/matra
A /root/.config/matra/config.toml             # only after explicit `matra config init`
A /root/.local/share
A /root/.local/share/matra
A /root/.local/share/matra/models
A /root/.local/share/matra/models/english-ewt-ud-2.5-191206.udpipe
```

**matra created exactly two paths.** No `~/.matra` (matching the promise at `installation.md:62`), no `~/.cache`, no dotfile in the working directory, no stray temp. The embedding artifacts also land inside the model directory as documented (`book/src/guides/semantic-clusters.md:37`):

```
$ docker exec matra-a sh -c 'ls -la /root/.local/share/matra/models/'
-rw-r--r-- 1 root root 16309608 english-ewt-ud-2.5-191206.udpipe
drwxr-xr-x 1 root root       84 potion-base-8M
```

The model's SHA-256 on disk equals the constant pinned at `src/nlp/udpipe.rs:18-19`:

```
784bd0fa85e3d831fd02a55290d0acfd05c953159dc38cc33d52e1b28add9957
```

and the embedding model reports the digest documented at `book/src/guides/semantic-clusters.md:41`:

```
model_hash 81c3592150873b1c5a8c4262850f795bff4fd568fbde80ac69889d087f16a0b4
```

---

## 4. The Rust-only path

Container with a Rust toolchain and **no Python at all** (`rust:slim-bookworm`, verified: `NO python3`, `NO python`, `NO g++`).

**As documented, it fails.** `book/src/tutorials/installation.md:9` lists exactly one requirement, "Rust 1.85 or later (MSRV)". That is not enough:

```
$ cargo install --path . --features cli,udpipe
info: syncing channel updates for stable-aarch64-unknown-linux-gnu
info: downloading 6 components
...
  cargo:warning=Compiler family detection failed due to error: ToolNotFound: failed to find tool "c++": No such file or directory (os error 2)
  --- stderr
  error occurred in cc-rs: failed to find tool "c++": No such file or directory (os error 2)
error: failed to run custom build command for `udpipe-rs v0.2.0`
error: failed to compile `matra v0.2.0 (/build)`
exit code: 101
```

Two things to note: the `rust-toolchain.toml` in the crate makes rustup pull a whole second toolchain plus `rustfmt`, `clippy` and `llvm-tools-preview` before compiling a line ("downloading 6 components"), and then the build dies on a missing C++ compiler that no document mentions.

**With `g++` added, it builds and the binary stands alone.**

```
$ apt-get install -y g++ && cargo install --path . --features cli,udpipe --root /out
    Finished `release` profile [optimized] target(s) in 1m 12s
  Installing /out/bin/matra
-rwxr-xr-x 1 root root 6868840 matra
```

`ldd` in the builder and again after copying the binary into a bare `debian:bookworm-slim`:

```
	linux-vdso.so.1
	libstdc++.so.6 => /lib/aarch64-linux-gnu/libstdc++.so.6
	libc.so.6      => /lib/aarch64-linux-gnu/libc.so.6
	libm.so.6      => /lib/aarch64-linux-gnu/libm.so.6
	libgcc_s.so.1  => /lib/aarch64-linux-gnu/libgcc_s.so.1
	/lib/ld-linux-aarch64.so.1
```

**Every one resolves in `debian:bookworm-slim` with no extra package.** `libstdc++.so.6` is the only non-obvious dependency (it comes from the UDPipe C++), and it is present in the base image; it would be absent from a static-distroless or musl base, which is worth knowing but is not the documented target. The binary runs:

```
$ docker run --rm matra-rust-bin sh -c 'matra --version'
matra 0.2.0
features: udpipe cli
```

which is byte-identical to the expected output at `book/src/tutorials/installation.md:41-44`. It also completed a cold analysis of `README.md` in that bare image with `ca-certificates` never installed (see 3.1).

One incidental note: `cargo install --path .` resolves dependencies fresh rather than from the tracked `Cargo.lock` (which pins `ureq 3.3.0`); the build log shows `Compiling ureq v3.4.1`. Expected cargo behaviour, not a defect, but it means the version set a `cargo install` user gets is not the version set CI tested.

---

## 5. Are the documented Linux locations true?

Every path claim checked against where a file actually landed.

| Claim | Cited at | Verified how | Result |
|---|---|---|---|
| data root defaults to `~/.local/share/matra` | `book/src/tutorials/installation.md:62`, `book/src/guides/cli.md:30`, `book/src/explanation/programming-model.md:51` | `matra config show` printed `data_dir = "/root/.local/share/matra"`; `docker diff` shows `A /root/.local/share/matra` | **True** |
| models live in the `models` subdirectory of the data root | same lines | `docker diff` shows `A /root/.local/share/matra/models/english-ewt-ud-2.5-191206.udpipe` | **True** |
| config file is `~/.config/matra/config.toml` | `book/src/guides/cli.md:29`, `book/src/explanation/programming-model.md:50` | `matra config init` printed `/root/.config/matra/config.toml`; the file exists there, 348 bytes | **True** |
| `MATRA_MODEL_DIR` overrides the model directory | `installation.md:62` | every scenario using `--env MATRA_MODEL_DIR=/models` wrote into `/models` and nowhere else | **True** |
| `XDG_DATA_HOME`/`HOME` resolution order, error when none is set | `book/src/reference/errors.md:80`, `src/config.rs:497-516` | `env -u HOME` produced exactly the documented `InvalidInput` | **True** |
| matra never creates `~/.matra` | `installation.md:62`, `programming-model.md:54` | `docker diff matra-a` contains no `/root/.matra` entry after a full session | **True** (see warrant) |
| download goes to a temp location, then moves into place, hash-checked before load | `installation.md:64` | disk-full run left an empty model directory; concurrent run left one correct file; SHA-256 on disk equals `src/nlp/udpipe.rs:18-19` | **True** |
| a failed check deletes the file and re-downloads once | `installation.md:64` | 1000-byte corrupt cache was replaced with the correct 16,309,608-byte file, rc=0 | **True** |
| `Matra.english()` verify snippet output | `installation.md:72-89` | reproduced byte for byte | **True** |
| `matra --version` prints `matra 0.2.0` / `features: udpipe cli` | `installation.md:41-44` | true for the `cargo install` binary; the pip-installed CLI prints `features: udpipe model2vec python cli` | **Partly** (see Cosmetic 2) |
| embedding artifacts land in the configured model directory | `book/src/guides/semantic-clusters.md:37` | `potion-base-8M/` appeared under `/root/.local/share/matra/models/` | **True** |
| the pinned embedding digest | `book/src/guides/semantic-clusters.md:41` | `model_hash` returned the same 64 hex characters | **True** |
| exit codes: 0 found, 2 error, message on stderr prefixed `matra:` | `book/src/guides/cli.md:238` | every failure scenario above returned 2 with a `matra: ` prefix on stderr | **True** |

---

## Findings

### Blocks release

**B1. No Linux `aarch64` wheel is built, and the fallback path is broken.**
`.github/workflows/publish-pypi.yml:97-99` builds three wheels: `linux-x86_64`, `macos-x86_64`, `macos-aarch64`. The comment at `:79-81` states the omission plainly: *"no linux aarch64 wheel ships for 0.1.x"*. Every Linux ARM user therefore falls to the sdist, Graviton and Ampere instances, Raspberry Pi, and, most commonly, **anyone running a Linux container on an Apple Silicon Mac, where `linux/arm64` is the default platform**. That fallback then fails (B2). This is the first command a user types.

**B2. The source build fails with only the documented prerequisites, because a C++ compiler is required and never mentioned.**
`installation.md:9` lists "Rust 1.85 or later (MSRV)" and `installation.md:11` says the sdist "additionally needs the Rust toolchain". Neither names a C++ toolchain. In `python:3.12-slim` with `gcc`, `libc6-dev` and a full rustup stable toolchain installed and no `g++`:

```
$ pip install --no-cache-dir /src
  error occurred in cc-rs: failed to find tool "c++": No such file or directory (os error 2)
  error: failed to run custom build command for `udpipe-rs v0.2.0`
  💥 maturin failed
ERROR: Failed to build installable wheels for some pyproject.toml based projects (matra)
```

Identical failure for `cargo install --path . --features cli,udpipe` (section 4), which is the exact command `installation.md:28` recommends. A user who follows the page precisely still cannot install. Fix is one sentence in the requirements plus, ideally, a named package (`g++` / `gcc-c++` / `build-essential`).

**B3. The wheel the release workflow actually produces has a glibc 2.34 floor, so it is rejected on still-supported distros.**
Replicating the `wheels` job exactly (ubuntu 24.04, `ldd (Ubuntu GLIBC 2.39-0ubuntu8.8) 2.39`, rustup stable, `pip install maturin==1.14.1`, `maturin build --release --out dist`, **no manylinux container**) yields:

```
📦 Built wheel for CPython 3.12 to /dist/matra-0.2.0-cp312-cp312-manylinux_2_34_aarch64.whl
```

On glibc 2.31:

```
$ docker run --rm ... python:3.12-slim-bullseye sh -c 'ldd --version | head -1; pip install /w/matra-0.2.0-cp312-cp312-manylinux_2_34_aarch64.whl'
ldd (Debian GLIBC 2.31-13+deb11u13) 2.31
ERROR: matra-0.2.0-cp312-cp312-manylinux_2_34_aarch64.whl is not a supported wheel on this platform.
rc=1
```

The maturin-image wheel from section 1 installs there without complaint:

```
$ docker run --rm ... python:3.12-slim-bullseye sh -c 'pip install -q /w/matra-...manylinux2014_aarch64.whl; matra --version'
ldd (Debian GLIBC 2.31-13+deb11u13) 2.31
matra 0.2.0
```

glibc 2.34 excludes Debian 11 (2.31), Ubuntu 20.04 (2.31), RHEL/Rocky/Alma 8 (2.28) and Amazon Linux 2 (2.26). Those users land on the sdist and then on B2. Building in `ghcr.io/pyo3/maturin` (plus `RUSTUP_PERMIT_COPY_RENAME=1`) fixes B1 and B3 together and costs 100 s of CI time.

### Hurts first impression

Ranked by how early a user meets them.

**H1. The first run is completely silent for as long as it takes.**
17.6 s in the primary run, 3.4-34.5 s across five cold starts, **zero bytes written to either stream before the result**. No indication that a 16 MB download is happening, where it is going, or that the command is alive. Every user meets this on their very first command. `installation.md:91` sets the expectation as "several seconds", which is true of the best draw and not of the median. One line to stderr naming the artifact, the size and the destination directory would remove the whole problem.

**H2. There is no timeout, so a bad network turns H1 into an unbounded hang.**
90 seconds against an unreachable host produced zero bytes and the process was still running. `udpipe-rs-0.2.0/src/lib.rs:627-632` uses `ureq::get(url).call()` with the default config, and `ureq-3.3.0/src/config.rs:894-908` leaves `global`, `connect`, `resolve` and `recv_body` all `None`. matra sets nothing. Combined with H1 the user cannot distinguish "slow" from "dead", and (H3) the natural response makes it worse.

**H3. Ctrl-C during that silence leaves a 15.5 MB orphan that nothing ever reclaims.**
`/models/.tmp.download.6/english-ewt-ud-2.5-191206.udpipe`, 16,252,074 bytes, after a SIGINT 2 s into a cold run. The reclaim at `src/nlp/udpipe.rs:143-146` keys on the current process's own pid, which on a real machine will not recur. There is no `matra clean`. Repeated interruptions accumulate.

**H4. Network, TLS and disk failures are all reported as "invalid model".**
`src/nlp/udpipe.rs:169` funnels every `download_model_from_url` failure into `Error::ModelInvalid` (`src/domain.rs:110-111`, `kind()` = `model_invalid` at `:158`). Observed:

- no network → `matra: invalid model: UDPipe error: Failed to download: io: failed to lookup address information: Temporary failure in name resolution`
- TLS interception → `matra: invalid model: UDPipe error: Failed to download: io: invalid peer certificate: Other(OtherError(CaUsedAsEndEntity))`
- full disk → `matra: invalid model: UDPipe error: No space left on device (os error 28)`

None of the three is an invalid model. The TLS one additionally leaks a `rustls` `Debug` rendering at a user-facing surface. None names the URL or the host it was reaching for. A consumer branching on the stable `kind` string gets `model_invalid` for a DNS failure, which is a contract problem, not only a cosmetic one.

**H5. matra ignores the system trust store, with no escape hatch and no documentation.**
Section 3.2. This is what makes matra unusable behind a TLS-intercepting corporate proxy: there is no `SSL_CERT_FILE` path, no `platform-verifier` feature, no proxy guidance anywhere in `book/src/`, and no documented way to hand-place the UDPipe model (the semantic-clusters page does document hand-placement for the embedding model at `book/src/guides/semantic-clusters.md:60-66`; the UDPipe model has no equivalent). The user gets `CaUsedAsEndEntity` and nothing else.

**H6. Filesystem errors name neither the path nor the operation.**
`matra: io error: Permission denied (os error 13)` and `matra: io error: Read-only file system (os error 30)`. The originating call is `std::fs::create_dir_all(dir)` at `src/nlp/udpipe.rs:78`, and the directory is right there in `dir`. `installation.md:93` compensates by telling the user to run `matra config show`, and that does work, but the message itself should not need a documentation lookup.

### Cosmetic

**C1. `installation.md:91` says the first run "can take several seconds"; the observed range is 3.4-34.5 s** across five cold starts on a fast connection, unbounded on a bad one (H2).

**C2. The pip-installed CLI's feature line differs from the documented one.** `installation.md:41-44` shows `features: udpipe cli`; `installation.md:54` says the pip command "is the same Rust CLI `cargo install` gives you". The pip-installed binary prints `features: udpipe model2vec python cli`. Both are correct for their build; the page reads as though one output covers both.

**C3. `matra config show --json` reports a config file that does not exist.** `"input": "/root/.config/matra/config.toml"` in a container where that file had never been created. The human-readable form does not make the claim.

**C4. `--json` errors are not JSON.** With `--json`, a failure writes `matra: ...` plain text to stderr and nothing to stdout. Nothing in `book/src/guides/cli.md` promises an error envelope, so this is a gap rather than a mismatch, but a JSON consumer has to shell out to text parsing for every failure.

**C5. `rust-toolchain.toml` costs every from-source user a second toolchain.** `channel = "stable"` plus three components means rustup downloads 6 components before compiling, even on a machine with a perfectly good stable toolchain under a different name. In a `docker build` it fails outright without `RUSTUP_PERMIT_COPY_RENAME=1` (`Invalid cross-device link (os error 18)`). This does not affect `cargo install matra` from crates.io, where the toolchain file is not in the invocation's cwd.

### Nothing found, stated explicitly

Each of these came back clean and is worth recording as such:

- **CA certificates.** Not required, on either the wheel or the `cargo install` path. Verified twice (3.1).
- **HOME unset.** Produces a precise, actionable, documented error (3.5).
- **Partial-download cleanup on a full disk.** Works. The model directory was left completely empty (3.8).
- **Corrupted cache.** Detected on size, removed, re-downloaded, run succeeded (3.12).
- **Concurrent cold starts.** Three racing processes, all rc=0, one correct file, no residue (3.11).
- **Read-only model directory holding a valid model.** Works (3.7).
- **Shared model volume across containers.** Second start 1.07 s vs 23.33 s, no re-download (3.10).
- **Files written outside documented locations.** None. `docker diff` over a full session shows exactly two matra-created paths, both documented (3.13).
- **The `cargo install` binary's runtime dependencies.** All five resolve in a bare `debian:bookworm-slim` (section 4).
- **Every documented Linux path.** All true (section 5).
- **The pinned hashes.** UDPipe model SHA-256 and `potion-base-8M` `model_hash` both match the constants in the source and the docs.
- **Exit codes.** Every failure returned 2 on stderr with the `matra:` prefix, as `book/src/guides/cli.md:238` states.

---

## Warrant

**What I verified directly, with the command and its real output in this report:** wheel build in the maturin manylinux image; wheel platform tag, size and contents; clean-container install and first use; cold-start timing across five runs, measured through a pty so silence is evidence rather than inference; JSON output; `config show` and `config init`; the documented Python verify snippet; behaviour with certificates deleted, with no network, with an unreachable host, with a self-signed TLS interceptor, with HOME unset, as an unprivileged user with an unwritable HOME, on a read-only root, on a disk too small for the model, on a shared volume, under SIGINT, under three-way concurrency, and with a corrupted cache; the full filesystem diff; the `cargo install` path with and without `g++`, and `ldd` on the resulting binary in a bare Debian; the release workflow's wheel tag reproduced from the workflow's own steps; and that tag's rejection on glibc 2.31 alongside the manylinux2014 wheel's acceptance there.

**What I could not verify, and why:**

- **`pip install matra` from PyPI.** Nothing is published; forbidden and pointless. B1's consequence (ARM Linux falls to sdist) is inferred from the workflow matrix at `publish-pypi.yml:97-99` plus the *directly observed* sdist failure. The inference is pip's documented wheel-selection behaviour, not a guess, but it is an inference.
- **x86_64 specifically.** Every container ran `linux/arm64` natively; I deliberately did not use emulation, since its timings would misrepresent a user's experience and it would not change any conclusion. The glibc floor a build host imposes is set by the host's glibc, not the architecture, so B3's `manylinux_2_34` figure carries over to `x86_64`; the wheel *filename* on the real runner would read `..._x86_64.whl`. The C++-toolchain requirement (B2) and every message-quality finding are architecture-independent.
- **The legacy `~/.matra/models` fallback.** Not exercised. I confirmed only the negative half, matra never created `~/.matra`, which is what a fresh install can show.
- **The behaviour of a genuinely trusted-but-wrong response** (valid TLS, HTTP 200, wrong bytes, twice in a row, producing `SHA-256 mismatch after re-download` from `src/nlp/udpipe.rs:95-98`). I verified the single-mismatch self-heal but not the double-mismatch terminal error, which would need control over a trusted endpoint.
- **Real slow-network behaviour.** I observed 3.4-34.5 s of genuine variance from LINDAT, and demonstrated the unbounded case by routing to a black hole. I did not shape bandwidth to characterise the distribution.

**What I am deliberately not claiming:**

- I am not claiming the download is slow. It is *variable*, and the code says nothing about it while it varies; that is the finding.
- I am not claiming the `Error::ModelInvalid` funnelling is a correctness bug. It is a labelling and contract-vocabulary problem, and I have said only that.
- I am not claiming `libstdc++.so.6` is a problem. It resolved in every image I tried; I recorded it because a static-distroless or musl base would not have it.
- I am not claiming B3 affects a majority of users. glibc ≥ 2.34 covers current mainstream distros. It affects the long-support tail, and it is cheap to eliminate.
- I did not review code quality, architecture, or anything outside install-and-first-run on Linux.

---

## If this wheel went to PyPI tonight, what would the first Linux user's experience be?

It depends entirely on their machine, and the split is not graceful.

**On x86_64 with glibc 2.34 or newer**, a current Ubuntu, Debian 12, RHEL 9, most CI runners, it works, and works well. `pip install matra` lands in seconds. `matra --version` is correct. Then they type `matra analyze essay.md` and the terminal goes completely blank for somewhere between 3 and 35 seconds with no explanation, while 16 MB comes down from a university server in Prague. If they wait, they get a clean, correct table and every run after that takes about a second. If they assume it hung and press Ctrl-C, which is the reasonable read of a silent, unbounded pause, they leave a 15 MB orphan in `~/.local/share/matra/models/.tmp.download.<pid>/` that nothing will ever clean up, and they try again. On an unlucky network they wait forever, because there is no timeout at any layer. If they are behind a corporate TLS proxy they get `invalid peer certificate: Other(OtherError(CaUsedAsEndEntity))` and there is nothing in the product or the documentation that will get them past it.

**On Linux ARM, and that now means Graviton, Ampere, Pi, and every Linux container a developer runs on an Apple Silicon Mac, where `linux/arm64` is the default**, there is no wheel. pip downloads the sdist, tries to build it, and fails on a missing C++ compiler that no page in the documentation mentions. The user follows `installation.md`, installs Rust exactly as told, tries again, and fails again in exactly the same way. There is no path from the documentation to a working install.

**On Debian 11, Ubuntu 20.04, RHEL 8 or Amazon Linux 2**, pip says `not a supported wheel on this platform`, falls to the sdist, and lands in the same wall.

Three changes close all of it and none is large: build the Linux wheels in `ghcr.io/pyo3/maturin` for both architectures (fixes the ARM gap and lowers the glibc floor from 2.34 to 2.17 in one move, proven, 100 s of CI); add the C++ toolchain to the documented requirements; and print one line to stderr before the download saying what is being fetched, how big it is, and where it is going. The last one is a handful of characters and it is the difference between "this is broken" and "this is working".
