# Install the model behind a TLS-intercepting proxy

Place the parsing model by hand when a proxy that re-signs TLS stops matra from downloading it.

## Recognise the failure

matra verifies TLS against root certificates compiled into the binary and never reads the system trust store. That is why it needs no `ca-certificates` package, on any platform, and it is also why a proxy that re-signs TLS cannot be trusted by installing its CA anywhere on the machine. The first run that needs the model fails with:

```text
matra: io error: download https://lindat.mff.cuni.cz/...: the TLS certificate offered for
lindat.mff.cuni.cz was rejected. matra verifies TLS against root certificates compiled into
it and never reads the system trust store, so a proxy that re-signs TLS cannot be trusted by
installing its CA. Fetch english-ewt-ud-2.5-191206.udpipe by hand and put it in the model
directory instead. Underlying failure: io: invalid peer certificate: ...
```

In Python the same failure is an `OSError` from `Matra.english()`. [Errors](../reference/errors.md#provisioning-failures) lists every way a download can fail.

## Place the model by hand

The artifact is pinned by name, size and SHA-256, so a hand-placed file is exactly as trustworthy as a fetched one: it goes through the same verification on load. A file that is not the pinned model is never used and never deleted. matra tries to download the pinned model in its place, and where that download cannot get through, which is the reason to place the file by hand, the call fails and the file stays exactly as you left it.

1. Find the model directory this machine resolves, and create it. `matra config show` prints it as `model_dir`.
2. Download the model with a client that trusts the proxy, such as `curl` with the system store.
3. Check its SHA-256 before moving it in, so a truncated or proxy-rewritten download fails here rather than on the next run. The digest must read `784bd0fa85e3d831fd02a55290d0acfd05c953159dc38cc33d52e1b28add9957`.
4. Move it into the model directory.

```bash
mkdir -p "$(matra config show | awk -F\" '/^model_dir/ {print $2}')"
curl -L -o english-ewt-ud-2.5-191206.udpipe \
  "https://lindat.mff.cuni.cz/repository/server/api/core/bitstreams/handle/11234/1-3131/english-ewt-ud-2.5-191206.udpipe?sequence=17&isAllowed=y"
shasum -a 256 english-ewt-ud-2.5-191206.udpipe
# 784bd0fa85e3d831fd02a55290d0acfd05c953159dc38cc33d52e1b28add9957
mv english-ewt-ud-2.5-191206.udpipe "<the model_dir above>/"
```

`MATRA_MODEL_DIR` points at a directory of your own if the resolved one is not writable.

## Check it worked

Run any command that parses. The command line writes `matra: downloading ...` to standard error before a fetch and nothing when the model is already in place, so a run that prints no such line has loaded the file you placed.

## The embedding model

The reference embedding model has the same route: three artifacts into a directory loaded with `Model2Vec::from_dir`, described in [semantic clusters](semantic-clusters.md).
