# 0015. Provisioning is matra's own, and a failure to fetch is not an invalid model

- **Status:** accepted
- **Date:** 2026-09-06
- **Decider(s):** project maintainer; measured by a clean-room container pass on 2026-09-06

## Context

ADR-0011 settled that pinned downloads are one discipline rather than an
exception per model, and ADR-0010's amendment applied it to the reference
embedding model. One exception remained. `Model2Vec::potion_base_8m`
fetched through matra's own code, with a size cap, a global and a connect
timeout, a transactional temporary that cleaned up after itself, and
transport failures reported as `Error::Io` naming the URL.
`Udpipe::english` fetched through `udpipe_rs::download_model_from_url`,
which calls `ureq::get(url).call()` with the default configuration, and
matra caught whatever came back with a single `map_err` to
`Error::ModelInvalid`.

A clean-room pass on 2026-09-06 measured what that cost, in containers,
with the command and the output recorded for each condition.

1. **The first run wrote zero bytes for 3.4 to 34.5 seconds.** Measured
   through a pty across five cold starts. No banner, no destination, no
   progress, no spinner. The terminal is indistinguishable from a hung
   process, and the README section the user is following is titled "No
   setup".
2. **No timeout at any layer.** `ureq`'s default configuration sets every
   timeout to `None`. 90 seconds against a black-holed host produced zero
   bytes with the process still running. Unbounded.
3. **Ctrl-C during that silence left a 15.5 MB orphan.** The temporary
   directory carried the process id, and the reclaim matched only the
   current process's own pid, which on a real machine does not recur.
   `Drop` does not run on `SIGINT`, so cleanup on scope exit could not be
   the whole answer.
4. **Network, TLS and disk-full failures were all reported as "invalid
   model".** Observed: a DNS failure, a TLS certificate rejection, and
   `No space left on device` all arriving as `invalid model`. None of the
   three is an invalid model, and none named the URL or the host.
5. **A raw `rustls` `Debug` string reached the user.** `invalid peer
   certificate: Other(OtherError(CaUsedAsEndEntity))` is what someone
   behind a TLS-intercepting corporate proxy saw.
6. **Filesystem errors named neither the path nor the operation.**
   `matra: io error: Permission denied (os error 13)`, from a
   `create_dir_all` whose directory was in a variable one line away.

Item 4 is a contract problem and not only a cosmetic one. `Error::kind()`
is a documented stable string, pinned in `spec/tests/corpus/items.json`
with a runner per crust, and a consumer that branches on it was told
`model_invalid` for a DNS failure.

The forces in tension are two. The classification the embedding adapter
already uses is the right one, and adopting it changes a published
string. Leaving the string alone preserves a promise whose content is
false.

## Options considered

### Option A: fix the six findings where they are, keeping the upstream fetch

Add a notice at the call site, wrap the `map_err` in a classifier that
inspects the upstream error's message, sweep stale temporary directories.

**Pros:** no change to the dependency's role; the smallest diff.
**Cons:** the timeout cannot be fixed at all, because the configuration
belongs to `udpipe-rs` and it passes none; the size cap cannot be applied,
because the bytes never pass through matra before they are on disk; and
the classifier would have to recover the failure's nature by reading a
string another crate formatted, which is the sin the finding names.
Two of the six findings are simply not reachable from here.

### Option B: move the fetch into matra, and classify failures the way the embedding adapter already does

`nlp/udpipe.rs` fetches with its own `ureq::Agent`, under the same 300
second global and 30 second connect budgets, capped at the same 64 MiB,
into memory. The bytes are verified there and only then written.
`udpipe_rs` keeps the job only it can do, `Model::load_from_memory`, so
boundary rule 4 is untouched: this file is still the only importer.

**Pros:** every one of the six findings closes at the same seam; the two
provisioning paths become one discipline described once; the size cap,
the timeouts and the classification live in matra, where they can be
tested with an injected fetcher and no network.
**Cons:** `ureq` joins the `udpipe` feature's dependency set explicitly.
It was already in the tree through `udpipe-rs`, so no crate is added.
The classification change is a published-contract change.

### Option C: Option B, plus a new `Error` variant for transport failures

A `Network` or `Download` variant with its own kind string.

**Pros:** a consumer can branch on "the download failed" without reading
an `io::ErrorKind`.
**Cons:** it contradicts the embedding adapter, which has reported
transport failures as `Error::Io` since 0.2.0 and documented why; it adds
a string every crust must learn, for a distinction `io::ErrorKind`
already carries; and it makes the vocabulary grow with the number of
places I/O can happen, which is the wrong axis.

### Option D: Option B, plus an escape hatch for the system trust store

matra verifies TLS against roots compiled into the binary
(`webpki-roots`, via `ureq`'s default features). That is why it needs no
`ca-certificates` package, which the container pass confirmed twice and
recorded as a genuine strength. It is also why a TLS-intercepting
corporate proxy cannot be trusted at all: there is no `SSL_CERT_FILE`
path, no `platform-verifier` feature, and no way through.

**Pros:** unblocks users behind such a proxy.
**Cons:** it is a security decision, not an ergonomics one. Reading a
trust anchor from the environment widens what can authorize a model
download, and the download's whole trust story today is the pinned digest
plus a fixed root set. It also has a free alternative that costs the user
one `curl`: the artifact is pinned by name, size and SHA-256, so placing
it by hand is exactly as trustworthy as fetching it.

## Decision

**We choose Option B.** The UDPipe fetch moves into `nlp/udpipe.rs`, and
the classification becomes the one the embedding adapter already
documents.

The vocabulary does not change. What changes is which kind a given
failure reports:

| Condition | Before | Now |
|---|---|---|
| DNS failure, unreachable host, refused connection | `model_invalid` | `io`, `ErrorKind::NotConnected` |
| Timeout, connect or transfer | never fired (no timeout existed) | `io`, `ErrorKind::TimedOut` |
| TLS certificate rejected | `model_invalid` | `io` |
| Non-2xx status | `model_invalid` | `io` |
| Response past the size cap | never fired (no cap existed) | `input_too_large`, `what = "udpipe_download"` |
| Model directory cannot be created or written, disk full | `model_invalid` or a pathless `io` | `io`, naming the operation and the path |
| Bytes arrived and failed the pinned digest | `model_invalid` | `model_invalid` |
| Bytes arrived, passed the digest, and did not load | `model_invalid` | `model_invalid` |

The rule behind the table, stated once: **`model_invalid` is about bytes
that arrived.** A download that never arrived, a server that answered
about the request rather than with a model, and a filesystem that refused
the write are all `io`. The `io::ErrorKind` is preserved where `ureq`
knows it, so a caller separates a timeout from an unreachable host
without reading a message, and the message names the URL.

**In Python the change is larger than a kind string.** `Error::kind` is
not the only thing keyed off the variant: `From<MatraError> for PyErr`
routes `Io` to `PyOSError` and `ModelInvalid` to `PyRuntimeError`, and
neither the routing nor `kind` is touched by this decision. The
consequence is that a DNS failure, a rejected certificate, a timeout or
a non-2xx status now raises `OSError` from `Matra.english()` where it
raised `RuntimeError`. That is not a caller reading a different string.
A bootstrap wrapped in `except RuntimeError`, which is exactly the shape
`book/src/guides/python.md` taught, silently stops catching network
failures: the handler goes quiet and the `OSError` propagates past it.
The migration is to catch both. Every page that named an exception class
for a provisioning failure is corrected with this decision, and
`tests/error_tables.rs` now holds those tables to the routing in
`src/lib.rs` so the next reclassification cannot leave them behind.

Preserving it takes one deliberate step on the body-read path. `ureq`'s
body reader converts its own failure with `Error::into_io`, which returns
the inner error only for `Error::Io` and wraps everything else, a
`Timeout` included, in `io::Error::other`; reading the kind straight off
that yields `Other`. Both adapters pass the read error back through
`From<io::Error> for ureq::Error`, which unwraps it, before classifying.
Without that step the `TimedOut` row above is false for a transfer that
stalls after the response has begun, and a certificate rejected
mid-stream misses the sentence the connect phase gets.

**We reject Option D, and document hand-placement instead.** The pinned
UDPipe model gets the documented manual path
`book/src/guides/semantic-clusters.md` already gives the embedding model,
in `book/src/reference/errors.md`. The certificate message names the host
and says why installing a CA changes nothing, so a reader is pointed at
the way out rather than left with a `rustls` enum. If a trust-store
escape hatch is ever wanted, it is a separate ADR with a threat model,
not a flag added on the way past.

Three smaller decisions follow from the same seam and are recorded here
because a reader of this file will ask about them.

**The notice is data, not a sentence.** `domain::ProvisionNotice` carries
the artifact, its pinned size and the destination directory.
`Udpipe::english_with_notice`, `Udpipe::from_config_with_notice` and
`Engine::from_config_with_notice` are additive forms that call it once
per fetch and never when the model is on disk. The library renders
nothing, so the wording, the stream and the decision to say anything at
all stay with the application. The command line writes one line to
stderr, which keeps `--json` stdout a single object, and `--quiet`
silences it.

The three forms are the UDPipe path's. `Model2Vec::potion_base_8m`
provisions 30.2 MB across three artifacts and has no notice form, so a
semantic first run is still silent. Giving it one is additive and
unblocked; it is not part of this decision, and the docsite says so
rather than implying the notice is universal.

**Staleness is measured in time, not in process identity.** A temporary
download directory older than twice the fetch budget cannot belong to a
live call, so it is reclaimed. Anything younger is left alone, which is
what keeps concurrent cold starts safe: three racing processes see each
other's directories as seconds old. Fetching into memory before writing
shrinks the exposure that produced the orphan from the length of a
download to the length of one write.

Process identity is not usable for naming either. The temporary used to
carry the process id alone and be removed by name just before it was
created, which is safe only where a pid is unique. In a container every
process is pid 1, so two cold starts sharing a bind-mounted model
directory pick the same name, and the removal deletes a live peer's
transfer. The name now carries the pid, a nanosecond timestamp and a
counter, and nothing is removed for looking like this call's own: age is
the only rule that reclaims anything.

**The two adapters still do not share a provisioning module.** They share
a discipline, described here, and each implements it. That is the call
`embed/model2vec.rs` already recorded for the temp-then-rename pattern,
and the reason is unchanged: a utility module both imported would put a
third file into the wiring to save a few dozen lines, and the two
adapters differ in the ways that matter (one artifact against three, one
file against a directory).

**One divergence survives, and it is deliberate.** `Udpipe::english`
removes a cached file that fails verification; `Model2Vec::potion_base_8m`
removes nothing at all. The asymmetry follows from what the two names
mean. `english-ewt-ud-2.5-191206.udpipe` names one pinned release, so a
file sitting under it that is not that release is matra's own stale
cache and nothing else it could plausibly be. `model.safetensors`,
`tokenizer.json` and `config.json` are the artifact format's names rather
than this model's, so a directory holding them may be a caller's own
model, which is why the embedding provisioner refuses a directory it did
not fill instead of clearing it. Removing where the name is unambiguous
and refusing where it is not is one rule, not two.

The ordering is a second thing, and the review of #77 corrected it. The
UDPipe removal used to run before the refetch, on the reasoning that it
kept the "download only into a directory that does not hold this name"
shape. It bought nothing: `install` lands through `fs::rename`, which
replaces an existing destination, so the write never needed the name
free. And it cost a user offline with a corrupt cache the only copy they
had, leaving an empty directory and a transport error.

A later review took that one step further and it was right. Moving the
removal after the fetch left it strictly redundant, and a redundant
unlink is not free. It falsified the guarantee stated on `install` that
the rename is the only operation touching the final path. It let a
failed removal, reached with `?`, abort a run whose download had already
succeeded. And it opened a window in which the file did not exist, so a
concurrent process saw no cache and paid a redundant 16 MB fetch. There
is no removal on this path now. A cached file that is not the pinned
model is simply replaced when a verified one arrives, and left alone
when none does.

Sharing a discipline meant bringing `embed/model2vec.rs` up to it, which
the first version of this change did not do. It fetched each artifact
into a temporary, renamed all three onto the artifact names, and only
then read them back to check the digest, so unverified bytes reached the
model directory under their real names. And its temporaries,
`.tmp.<name>.<pid>` opened with `create_new`, had no reclaim at all, so
one interrupt wedged every later embedding provision until somebody
deleted the file by hand, which in a container is every interrupt because
every process is pid 1. Both are closed here: the set is verified in
memory before anything is written, and a temporary older than twice the
fetch budget is reclaimed by the next install. That is the same defect
class as the UDPipe orphan this ADR already answers, on the path the
docsite now recommends.

Finding 6 above needed the same treatment and did not get it the first
time. `nlp/udpipe.rs` routes every filesystem failure through `io_at`,
which names the operation and the path; `embed/model2vec.rs` propagated
four of its own bare, so a full disk during an embedding install still
produced `io error: No space left on device (os error 28)` while this
decision and `book/src/reference/errors.md` both said the two adapters
classify a failure the same way. It now has an `io_at` of its own, which
is the discipline implemented twice rather than a helper imported twice,
exactly as the temp-then-rename pattern is.

**The sweep's directory is validated where it is configured.** The
embedding provisioner sweeps aged temporaries in
`cfg.model_dir().join(cfg.embedding_model())`, and `models.embedding` was
a free string, so `"../.."` pointed a delete-by-pattern operation at a
directory the operator did not name. The sweep only unlinks `.tmp.`
entries older than ten minutes and the configuration is the operator's
own, so this is hardening rather than a hole; it is also the first
delete-by-pattern operation matra runs in a directory configuration
chooses, and refusing a name that is not a single ordinary path component
costs one function. The check lives in `config.rs` rather than in the
adapter, because that is where a bad value can be reported once, against
the key and the file it came from, in the shape the resolver already uses
for an unknown algorithm name. `models.udpipe` gets no such check: the
UDPipe filename is pinned in the adapter beside its digest and nothing
joins the configured value to a path, so validating it would refuse
configurations that harm nobody.

## Consequences

- Positive: the first run says what it is fetching, how big it is, and
  where it is going. A stalled transfer fails in a bounded time instead
  of holding the terminal. An interrupted one leaves nothing behind, and
  an orphan from an older version is reclaimed on the next download. A
  failure names the URL or the path, and a proxy user gets a sentence.
- Negative: a consumer that branched on `kind == "model_invalid"` to mean
  "the model could not be obtained" now sees `io` for every failure that
  is not about the bytes. The migration is one line: branch on `io` for
  transport and filesystem failures, and keep `model_invalid` for a
  digest or loader verdict. Both kinds already existed, so no consumer
  meets a string it has never seen; a consumer that matched exhaustively
  still matches.
- Positive: the embedding provisioner verifies before writing, reclaims
  aged temporaries, and names the operation and the path on a filesystem
  failure, so the two adapters implement one discipline rather than one
  and a half. An interrupted embedding provision no longer wedges every
  later one.
- Negative: a Python caller catching `RuntimeError` around a first run
  catches nothing when the network is the problem, because that failure
  is now `OSError`. The exception classes are unchanged and no new one
  appears, so a caller catching `Exception` is unaffected; a caller
  catching the narrower class adds `OSError` beside it.
- Neutral: `tests/error_tables.rs` reads the routing out of `src/lib.rs`
  and the kind strings out of `src/domain.rs` and holds three documented
  tables to them. It pins which arm a variant sits in, which is what a
  reclassification changes; it cannot pin prose, so a page that describes
  a failure wrongly in a sentence is still a review finding.
- Neutral: the download client sets `https_only`. `ureq` follows up to
  ten redirects, so an `https` URL that redirected to `http` would
  otherwise be fetched in cleartext. The digest pin means integrity was
  never at stake here; this is confidentiality.
- Negative: `ureq` is now named in the `udpipe` feature. It was already
  in the tree through `udpipe-rs`, so the dependency graph is unchanged
  and only the declaration is new.
- Neutral: `udpipe_rs::download_model_from_url` is no longer called.
  `nlp/udpipe.rs` remains the only file importing `udpipe_rs`, for
  `Model::load` and `Model::load_from_memory`, and the panic boundary
  stays where it is.
- Neutral: `spec/tests/corpus/items.json` gains a `provisioning` block
  stating which kind each failure class reports, with runners in
  `tests/corpus_conformance.rs` and
  `python/tests/test_corpus_conformance.py`. The filesystem row is the
  one a runner can produce with no network, so it is the one asserted;
  the other two rows state the contract.

### Deferred: a JSON error envelope

Under `--json` a failure writes plain text to stderr and nothing to
stdout, so a JSON consumer parses text for every failure. Nothing
promises an error envelope, so this is a gap rather than a broken
promise, and it is left open deliberately.

The envelope ADR-0011 pinned is `format_version`, `command`, `input`,
`result`. An error envelope would swap `result` for an error object, and
that object needs a kind. The command line's own refusals ("no such
file", "x is a directory", a bad algorithm name) are
`Box<dyn std::error::Error>` strings with no kind at all, so shipping one
means either inventing a kind vocabulary for the application tier or
guessing a domain kind from a message. Guessing a kind from a message is
precisely the defect this ADR exists to remove, and doing it properly
means moving `src/cli/` onto `domain::Error`, which is a change of its
own with its own consequences for the exit-code contract.

It is worth doing. It is not worth doing as a side effect of a
provisioning fix. When it happens it uses the same `format_version`
envelope, is pinned in `spec/tests/cli/envelope.json` with both runners
updated, and is documented in `book/src/guides/cli.md`.

## Validation

- **The classification is falsifiable per condition**, and each was
  reproduced in a container: no network, an unreachable host, a
  TLS-intercepting proxy, a full disk, an unwritable HOME, a read-only
  filesystem, `SIGINT` mid-download, and three concurrent cold starts.
  A condition that reports a kind other than the table's falsifies the
  decision.
- **The concurrency constraint is the one to watch.** Three concurrent
  cold starts must still produce one correct file and no residue. If a
  reclaim ever deletes a live peer's temporary directory, the
  time-based rule is wrong and the answer is a longer threshold or a
  lock file, not a narrower pattern.
- **The notice must not become noise.** It fires once per fetch and never
  on a warm run, pinned by unit tests with an injected fetcher. A notice
  on every run would be filtered by every user, which is the failure mode
  that makes a progress line worthless.
- **Revisit trigger for Option D:** a user who cannot use matra at all
  behind a proxy and for whom hand-placement is not available (an
  automated build, say). The answer is a separate ADR with a threat
  model, not a flag.
- **Revisit trigger for the shared-module question:** a third adapter
  that provisions. Two implementations of a discipline is duplication a
  reader can hold; three is a module.

## References

- Clean-room container pass, 2026-09-06 (sections 3.2, 3.3, 3.4, 3.6,
  3.7, 3.9, and findings H1 to H6): the measurements this ADR answers
- [ADR-0010](0010-embeddings-adapter.md): decision 6 and its amendment,
  the pinned-download discipline this extends
- [ADR-0011](0011-out-of-the-box.md): pinned downloads as one discipline,
  and the `--json` envelope this defers changing
- [ADR-0012](0012-agent-surface.md): the envelope's `format_version` as
  an agent-facing contract
- `book/src/reference/errors.md`: the variant, kind and message tables
  this changes, and the hand-placement path for a blocked download
