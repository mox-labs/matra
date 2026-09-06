# Errors

Every fallible function in matra returns `domain::Result<T>`, which is `std::result::Result<T, domain::Error>`. No function in the library returns `Result<T, String>`. matra signals failure by returning `Error`, not by unwinding: a panic raised inside the UDPipe C++ boundary is caught at the adapter and converted into `Error::ParseFailed`.

`Error` is `#[non_exhaustive]`. Variants can be added in a minor release, so a match on it from another crate needs a catch-all arm. `Error` derives neither `Clone` nor serde; it is moved, not copied. The stream surface wraps it in `DocumentError`, which adds the path the failure occurred at and moves the error into `CorpusResult::errors`.

## The variants

| Variant | Payload | Returned when |
|---|---|---|
| `ModelNotFound` | `PathBuf` | A model file does not exist at the given path |
| `ModelInvalid` | `String` | Bytes that arrived could not be verified against the pin, or could not be loaded |
| `ParseFailed` | `String` | The NLP provider failed on the input, or panicked, or produced an unusable token id |
| `InputTooLarge` | `{ limit: usize, actual: usize, what: &'static str }` | A size gate rejected the input. `what` names the gate |
| `UnsupportedFormat` | `Format` | The document's format has no registered decomposer |
| `InvalidInput` | `String` | A caller violated a documented API contract |
| `Io` | `std::io::Error` | A filesystem operation failed, or a download did not arrive |

`Error` implements `std::error::Error` and `Display` through `thiserror`, and `From<std::io::Error>`, so `?` converts I/O failures automatically.

### ModelNotFound

Returned by `Udpipe::from_path` when the path does not exist, and by `Model2Vec::from_dir` when any of the three artifacts is absent. The payload is the path as given. Neither `Udpipe::english` nor `Model2Vec::potion_base_8m` returns this variant: both download the model when the file is absent.

### ModelInvalid

Returned by:

- `Udpipe::from_path` and `Udpipe::from_bytes` when the loader rejects the bytes. The payload is the loader's message.
- `Udpipe::english` when the bytes it downloaded still fail SHA-256 verification after one refetch. The payload is `SHA-256 mismatch after re-download from <url>`. Nothing that failed verification is written, so the model directory is left as the call found it.
- `Model2Vec::from_dir` when an artifact does not parse, uses an embedding dtype other than f32, or panics the loader.
- `Model2Vec::potion_base_8m` when the directory already holds all three artifacts and their digest is not the pinned one, or holds only some of them. The payload names the directory and the ways out; nothing there is downloaded over or removed.
- `Model2Vec::potion_base_8m` when the artifacts it downloaded still fail the digest after one re-download. Both attempts verify in memory, so neither writes anything, and the directory is left exactly as the call found it. The payload names the directory and the expected digest.

A file whose hash does not match the pinned constant is treated as untrusted and is never loaded.

`ModelInvalid` is about bytes that arrived. A download that never arrived, a server that answered about the request rather than with a model, and a filesystem that refused the write are `Io`, not `ModelInvalid` ([ADR-0015](https://github.com/mox-labs/matra/blob/main/docs/decisions/0015-provisioning-failures.md)). Before 0.2.0 the UDPipe download funnelled all three here, so a DNS failure reported `model_invalid`; a consumer that branched on that reads the [provisioning failures](#provisioning-failures) section below.

### ParseFailed

Returned by the UDPipe adapter's `parse` in three situations:

- The underlying parser returns an error. The payload is that error's message.
- The underlying parser panics. The payload is `udpipe panicked: <message>`. The `catch_unwind` boundary that produces this lives in `nlp/udpipe.rs`, and it exists because an unhandled panic crossing the C++ boundary aborts the host process instead of unwinding.
- A token id or head value cannot be represented as `usize`. The payload names the token and its sentence.

### InputTooLarge

Ten gate labels produce this variant. The `what` field carries the label so a caller can route each gate differently.

| `what` | Gate | Limit | Measured over |
|---|---|---|---|
| `"input"` | `Engine::annotate`, the only route from text to the parser, and the CLI's stdin read, which applies the same cap first | `MAX_INPUT_BYTES`, 8 MiB (8,388,608) | UTF-8 byte length of the text; the CLI counts the bytes as they arrive, before decoding them |
| `"file_source"` | `FileSource::read`, which `Ingest::path` reads through | 8,388,608 bytes | File size reported by the filesystem, checked before any read |
| `"tfidf"` | `tfidf_summarize` | 2,000 | Number of sentences in the slice |
| `"textrank"` | `textrank_summarize` | 2,000 | Number of sentences in the slice |
| `"rake"` | `rake_keyphrases` | 200,000 | Total tokens across the slice, punctuation included |
| `"yake"` | `yake_keyphrases` | 200,000 | Total tokens across the slice, punctuation included |
| `"semantic_clusters"` | `semantic_clusters` | 2,000 | Number of sentences in the slice |
| `"udpipe_download"` | `Udpipe::english` | 64 MiB (67,108,864) | Bytes read from the response, which stops one past the cap |
| `"embedding_download"` | `Model2Vec::potion_base_8m`, per artifact | 64 MiB (67,108,864) | Bytes read from the response, which stops one past the cap |
| `"config_file"` | `Config::resolve`, reading the user's config file | 64 KiB (65,536) | File size reported by the filesystem, checked before any read |

`limit` carries the cap and `actual` carries the measured size, so an error message can be built without hardcoding the constants.

The caps bound worst-case memory and time. TextRank builds a dense similarity matrix that reaches roughly 32 MB of `f64` at 2,000 sentences. RAKE and YAKE build phrase-keyed maps whose size follows token count rather than sentence count, which is why their caps are stated in tokens. The two download caps are the gates whose input is not the caller's: they bound what a redirected or misbehaving server can make the process hold, and the read stops at the bound rather than continuing, so `actual` reports the bound that was breached rather than the response's full length. Both are 64 MiB against pinned artifacts of 16.3 MB and 30.2 MB, which is headroom for a later version and still finite.

A document from disk crosses both text gates in sequence: `"file_source"` when `Ingest` reads it, `"input"` inside `annotate`. In practice `"file_source"` fires first, since both carry the same 8 MiB limit and the file size is checked before the read.

Calling a provider's `parse` directly bypasses the `"input"` gate. The gate belongs to `Engine::annotate`, not to the `NlpProvider` trait.

The label has a second producer. The CLI reads stdin through `read_capped` in `src/cli/mod.rs`, which reads at most one byte past `MAX_INPUT_BYTES` and returns `InputTooLarge` with `what` set to `"input"` before the text ever reaches `annotate`. Same limit, same label, so a caller sees one gate however the text arrived. Two details differ and neither is visible in the value: the CLI counts bytes before decoding them, so an oversized pipe is reported as too large rather than as invalid UTF-8, and the CLI renders the error on stderr and turns it into an exit code rather than returning it.

The four ranking extractors check their caps after their empty-result checks. `tfidf_summarize(sentences, 0)`, `textrank_summarize(sentences, 0)`, `rake_keyphrases(sentences, 0)`, and `yake_keyphrases(sentences, 0)` return an empty vector without evaluating the cap, whatever the size of the slice. The same holds for an empty slice. `semantic_clusters` has no count parameter; its contract checks run first, then its cap, and an empty slice returns empty clusters.

A metric has no gate of its own. The compression ratio skips any paragraph over 262,144 bytes and leaves its metric slot at `None` rather than returning an error.

### InvalidInput

Returned by `semantic_clusters` when a caller breaks its documented contract (embeddings disagreeing on dimension, an embedding containing a non-finite value, a non-finite threshold), and by `embed_and_cluster` when an embedder violates its own length contract. The payload names the violation. This variant means a call site or a provider implementation is wrong, not the input data; nothing about the analyzed text produces it.

`Config::resolve` uses the same variant for a configuration that cannot be honored: a config file that does not parse, an unknown key, an algorithm name this build does not know, a config file that is not valid UTF-8, or an environment with none of `MATRA_DATA_DIR`, `XDG_DATA_HOME` or `HOME` set. The payload names the file and the offending key or line. A config file that is simply absent is not an error; the built-in defaults stand.

### UnsupportedFormat

Returned by `Engine::annotate` when the document's `Format` has no entry in the engine's decomposer table. With `standard_decomposers()` that means `Pdf` or `Docx`; `Markdown` and `PlainText` always have an entry. The payload is the `Format` value.

### Io

Wraps `std::io::Error`, produced by:

- `FileSource` rejecting a symlink, with `ErrorKind::Unsupported` and the message `refusing to read symlink: <path>`.
- `FileSource` rejecting a path that is not a regular file, with `ErrorKind::InvalidInput` and the message `not a regular file: <path>`.
- Any read, directory listing, directory creation, file removal, or rename that fails.
- `Udpipe::english` when a download fails at the transport or answers with a non-2xx status. Same shape as the embedding path below: the message names the URL, and the kind is `TimedOut` past the 300-second fetch budget or the 30-second connect budget, `NotConnected` for an unreachable host, and whatever the socket reported otherwise.
- `Udpipe::english` when the model directory cannot be created, when a cached file that failed verification cannot be removed, or when the verified bytes cannot be written or renamed into place. The message names the operation and the path, so a full disk reads `cannot write the model to <path>: No space left on device (os error 28)` rather than `Permission denied (os error 13)` with nothing to act on.
- `Model2Vec::potion_base_8m` when a download fails at the transport or answers with a non-2xx status. The message names the URL, and the kind is `TimedOut` past the 300-second fetch budget or the 30-second connect budget, `NotConnected` for an unreachable host, and whatever the socket reported otherwise. Bytes that arrived and then failed the digest are `ModelInvalid` instead; this variant is for the ones that never arrived.
- `Model2Vec::potion_base_8m` when the model directory cannot be created, when an artifact cannot be read, or when a verified artifact cannot be written or renamed into place. The message names the operation and the path, exactly as the UDPipe path's does. `Model2Vec::from_dir` reports an unreadable artifact the same way.
- `Model2Vec::potion_base_8m` when the temporary file an artifact lands through cannot be created, with the kind the open reported (`AlreadyExists` when something is already sitting at that path). The temporary is opened exclusively, so a path already there, symlink or not, fails the open rather than being written through.
- `Ingest` when a source yields no document, with `ErrorKind::InvalidData` and the message `source returned no documents`.

## Provisioning failures

`Udpipe::english` and `Model2Vec::potion_base_8m` obtain a pinned artifact over the network. Both fetch through matra's own client, under the same bounds, and both classify a failure the same way.

| Condition | Variant | Kind | What the message carries |
|---|---|---|---|
| DNS failure, unreachable host, refused connection | `Io`, `ErrorKind::NotConnected` | `io` | The URL |
| Connect budget exceeded (30 seconds) | `Io`, `ErrorKind::TimedOut` | `io` | The URL |
| Fetch budget exceeded (300 seconds, lookup through last byte) | `Io`, `ErrorKind::TimedOut` | `io` | The URL |
| TLS certificate rejected | `Io` | `io` | The host, why matra cannot be made to trust it, the way out, then the underlying failure |
| Non-2xx status | `Io` | `io` | The URL and the status |
| Response past 64 MiB | `InputTooLarge` | `input_too_large` | `what` is `"udpipe_download"` or `"embedding_download"` |
| Directory, write, remove or rename failed | `Io` | `io` | The operation and the path |
| Bytes arrived and failed the pinned digest, twice | `ModelInvalid` | `model_invalid` | The URL, or the directory and the expected digest |
| Bytes arrived, passed the digest, and did not load | `ModelInvalid` | `model_invalid` | The loader's message |

The rule behind the table: `model_invalid` is about bytes that arrived. Anything that stopped a fetch from arriving, or a filesystem from accepting it, is `io` ([ADR-0015](https://github.com/mox-labs/matra/blob/main/docs/decisions/0015-provisioning-failures.md)).

The rule has a consequence in Python that is easy to miss, because it changes which `except` clause fires rather than only which string a message carries. `Io` routes to `OSError` and `ModelInvalid` routes to `RuntimeError`, so from 0.2.0 a DNS failure, a rejected certificate, a timeout or a non-2xx status raises `OSError` from `Matra.english()` where it used to raise `RuntimeError`. A caller that wrapped a bootstrap in `except RuntimeError` catches nothing now and the `OSError` propagates past it. Catch both, or catch `Exception` and branch on the message.

Nothing that failed the digest is ever written. Both provisioners fetch into memory, verify there, and write only what verified, so a download that fails adds nothing to the model directory and a run killed mid-transfer leaves nothing at all. The one thing a failed run does remove is a cached file that was already there and had already failed verification: `Udpipe::english` deletes that once the replacement is in hand, because a file under the model's name that is not the pinned model is not a file to keep, and deleting it before the fetch would cost an offline user the only copy they had. `Model2Vec::potion_base_8m` removes nothing at all, because its three filenames belong to the artifact format rather than to this one model and so may be a caller's own ([ADR-0015](https://github.com/mox-labs/matra/blob/main/docs/decisions/0015-provisioning-failures.md)). A temporary left by a killed process is reclaimed by the next download that finds it older than ten minutes, which is twice the fetch budget and therefore older than any transfer that could still be running. `Udpipe::english` leaves a temporary directory and `Model2Vec::potion_base_8m` three temporary files, and each reclaims its own by the same rule.

### Behind a TLS-intercepting proxy

matra verifies TLS against root certificates compiled into the binary and never reads the system trust store. That is why it needs no `ca-certificates` package, on any platform, and it is also why a proxy that re-signs TLS cannot be trusted by installing its CA anywhere on the machine. The failure reads:

```text
matra: io error: download https://lindat.mff.cuni.cz/...: the TLS certificate offered for
lindat.mff.cuni.cz was rejected. matra verifies TLS against root certificates compiled into
it and never reads the system trust store, so a proxy that re-signs TLS cannot be trusted by
installing its CA. Fetch english-ewt-ud-2.5-191206.udpipe by hand and put it in the model
directory instead. Underlying failure: io: invalid peer certificate: ...
```

Place the model by hand. `matra config show` prints the model directory this machine resolves, and the artifact is pinned by name, size and SHA-256, so a hand-placed file is exactly as trustworthy as a fetched one: it goes through the same verification on load, and a file that is not the pinned model is removed rather than used.

```bash
mkdir -p "$(matra config show | awk -F\" '/^model_dir/ {print $2}')"
curl -L -o english-ewt-ud-2.5-191206.udpipe \
  "https://lindat.mff.cuni.cz/repository/server/api/core/bitstreams/handle/11234/1-3131/english-ewt-ud-2.5-191206.udpipe?sequence=17&isAllowed=y"
shasum -a 256 english-ewt-ud-2.5-191206.udpipe
# 784bd0fa85e3d831fd02a55290d0acfd05c953159dc38cc33d52e1b28add9957
mv english-ewt-ud-2.5-191206.udpipe "<the model_dir above>/"
```

`MATRA_MODEL_DIR` points at a directory of your own if the resolved one is not writable. The reference embedding model has the same route, three artifacts into a directory loaded with `Model2Vec::from_dir`, described in [semantic clusters](../guides/semantic-clusters.md).

### The first run says so

A cold run has to fetch 16.3 MB before it can answer, and how long that takes is the link's business rather than matra's. The command line writes one line to standard error before the transfer and nothing at all when the model is already there:

```text
matra: downloading english-ewt-ud-2.5-191206.udpipe (16.3 MB) into /home/u/.local/share/matra/models
```

Standard error, so `--json` output stays a single object on standard output. `--quiet` silences it. A library caller gets the same facts as a `domain::ProvisionNotice` from `Engine::from_config_with_notice`, `Udpipe::english_with_notice` or `Udpipe::from_config_with_notice`, and decides the wording itself.

The notice covers the UDPipe model only. `Model2Vec::potion_base_8m` fetches 30.2 MB across three artifacts with no notice form of its own, so the first `matra semantic` run is silent for the length of that download.

## Per-document failures: `DocumentError` and `CorpusResult`

The stream surface never lets one bad document abort the rest. `Engine::analyze` yields `Result<CorpusEntry, DocumentError>` per document, where `DocumentError` pairs the `Error` with the path it occurred at (`None` for in-memory text, so a path-less document is distinguishable from one at an empty path). Its `Display` prints `path: error` when a path exists and the bare error otherwise, and its `source()` is the wrapped `Error`.

Collecting the stream into `CorpusResult` partitions it: every success into `corpus`, every failure into `errors`, in order, with entries plus errors equal to documents consumed. `Ingest::path` is `Err` only when the listing itself fails; both read failures and analysis failures travel as stream items.

Two kinds of entry never appear on either side. The directory listing skips symlinks, so a symlink produces no document and no error. It also skips anything that is not a regular file, and the walk is one level deep, so a subdirectory is skipped the same way.

Neither `DocumentError` nor `CorpusResult` is `Serialize`, because `Error` wraps `std::io::Error`. Crossing a language boundary therefore needs a projection rather than serde. `Matra.analyze_path` has one: each failed document arrives as `{"path": str | None, "error": {"kind": str, "message": str}}`, where `kind` is the stable string `Error::kind` names for the variant, and the table below carries all seven. `Error::kind` is a match with no wildcard arm, so a new `Error` variant fails to compile until someone names it, and `spec/tests/corpus/items.json` pins the vocabulary with a runner per crust, so every crust spells a failure the same way. Both item shapes are assembled field by field rather than serialized, and a `path` is decoded with `os.fsdecode`, so `os.fsencode` on it names the file the walk read even when that name is not valid UTF-8. `CorpusResult` itself does not cross: the Python call returns the items in order and the caller partitions them, testing `"error" in item`.

## Display strings and kinds

These are the strings `Display` produces, which are also the strings Python's `str(exc)` returns, alongside the kind `Error::kind` reports for each variant. A message is for a human to read; a kind is what code branches on.

| Variant | Format | Kind |
|---|---|---|
| `ModelNotFound(path)` | `model not found: {path}` | `model_not_found` |
| `ModelInvalid(s)` | `invalid model: {s}` | `model_invalid` |
| `ParseFailed(s)` | `parse failed: {s}` | `parse_failed` |
| `InputTooLarge { limit, actual, what }` | `{what} input too large: {actual} > limit {limit}` | `input_too_large` |
| `UnsupportedFormat(format)` | `unsupported format: {format:?}` | `unsupported_format` |
| `InvalidInput(s)` | `invalid input: {s}` | `invalid_input` |
| `Io(e)` | `io error: {e}` | `io` |

`UnsupportedFormat` renders the variant name, so the string reads `unsupported format: Pdf`.

## Matching in Rust

```rust
use matra::Engine;
use matra::domain::{Error, Format, RawDocument};

fn report(text: &str, engine: &Engine) {
    let raw = RawDocument::new(text.to_string(), None, Format::PlainText);
    match engine.analyze_one(raw) {
        Ok(entry) => println!("{} sentences", entry.analysis.total_sentences()),
        Err(doc_err) => match doc_err.error {
            Error::InputTooLarge {
                what,
                actual,
                limit,
            } => eprintln!("{what} gate: {actual} over limit {limit}"),
            Error::ModelNotFound(path) => {
                eprintln!("model missing at {}", path.display())
            }
            other => eprintln!("{other}"),
        },
    }
}
```

## Python exception mapping

The PyO3 binding converts `Error` into a Python exception class. The conversion is a match with no wildcard arm, so adding a variant to `Error` without assigning it an exception class fails to compile.

| Variant | Python exception |
|---|---|
| `ModelNotFound` | `FileNotFoundError` |
| `InputTooLarge` | `ValueError` |
| `UnsupportedFormat` | `ValueError` |
| `InvalidInput` | `ValueError` |
| `Io` | `OSError` |
| `ModelInvalid` | `RuntimeError` |
| `ParseFailed` | `RuntimeError` |

The table is unchanged from 0.1.0, but which row a provisioning failure lands on is not. Transport failures moved from `ModelInvalid` to `Io` in 0.2.0, so a failed download from `Matra.english()` or `Model2Vec.potion_base_8m()` now raises `OSError` rather than `RuntimeError`. See [provisioning failures](#provisioning-failures).

`ModelNotFound` maps to `FileNotFoundError` so that the conventional Python idiom works:

```python
from matra import Matra

try:
    engine = Matra.from_path("models/english-ewt-ud-2.5-191206.udpipe")
except FileNotFoundError as exc:
    print(exc)  # model not found: models/english-ewt-ud-2.5-191206.udpipe
```

The exception message is the `Display` string from the table above. Variant identity beyond the exception class is not carried across the boundary; a caller that needs to distinguish `InputTooLarge` from `UnsupportedFormat` inspects the message.

Every Python method that takes text routes through `Engine::annotate`, so the 8 MiB `"input"` gate applies uniformly: to `analyze` and `analyze_markdown`, and to the four extraction methods, whose per-extractor caps apply on top.

One further failure has no `Error` variant behind it. The `Matra` class is `#[pyclass(unsendable)]`, because the loaded model holds C-side state that is not thread-safe. Accessing one instance from a thread other than the one that created it fails at runtime. Multi-process use is unaffected.

## At the command line

One program is installed under the name `matra`, by either of two routes. The Rust binary comes from `cargo install matra --features cli`; the Python console script comes from the wheel and calls the same `matra::cli::run` through the extension module. The exit-code contract below is the program's, so it holds on both.

| Exit code | Meaning |
|---|---|
| 0 | The command succeeded, and where applicable something was found |
| 1 | The command succeeded and found nothing |
| 2 | An error occurred |

On exit code 2 the command writes `matra: ` followed by the error's `Display` string to standard error. A broken pipe, which is what happens when the reading end of `matra analyze file.md | head` goes away, exits 0 and prints nothing.

The table is the whole contract, and no command narrows it. Two cases are worth naming because they are usage failures rather than failures of the text: `matra --skill -r <name>` with a name no reference answers to exits 2 and names the ones that exist, and `-r` without `--skill` exits 2 and says which flag is missing. `matra --skill` in any of its accepted forms exits 0; it reads no document, so it has nothing to find and never exits 1.

The exception classes in the table above belong to the library surface, not to the command line. A `domain::Error` reaching the command line is rendered and turned into an exit code; it never surfaces as a Python exception, whichever launcher started it.
