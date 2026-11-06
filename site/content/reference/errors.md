# Errors

Every fallible function in matra returns `domain::Result<T>`, which is `std::result::Result<T, domain::Error>`. No function in the library returns `Result<T, String>`. matra signals failure by returning `Error`, not by unwinding: a panic raised inside the UDPipe C++ boundary is caught at the adapter and converted into `Error::ParseFailed`.

`Error` is `#[non_exhaustive]`. Variants can be added in a minor release, so a match on it from another crate needs a catch-all arm. `Error` does not derive `Clone`; it is moved, not copied. The stream surface wraps it in `DocumentError`, which adds the path the failure occurred at, and yields it as that input's `Err` item.

Every error reads the same way on every channel. It has a kind, a stable string to branch on. It has a message, for a person. And it has details: the values a caller acts on, as named data, so nothing has to parse the message. For every kind but `invalid_input` that is every dynamic value in the message; an `invalid_input` names the check that failed, and its message adds the values that failed it for a person. In Rust these are `Error::kind`, `Display` and `Error::details`. The serde form is the three together, and Python exceptions carry the same three. See [kinds and details](#kinds-and-details).

## The variants

| Variant | Payload | Returned when |
|---|---|---|
| `ModelNotFound` | `PathBuf` | A model file does not exist at the given path |
| `ModelInvalid` | `String` | Bytes that arrived could not be verified against the pin, or could not be loaded |
| `ParseFailed` | `{ reason: Option<&'static str>, cause: String }` | The parser or a reader failed on the input, or returned what its contract forbids. `reason` names the engine's check that failed, when one did |
| `InputTooLarge` | `{ limit: usize, actual: usize, what: &'static str }` | A size gate rejected the input. `what` names the gate |
| `UnsupportedFormat` | `{ format: Option<Format>, feature: Option<&'static str> }` | The input is in a format no reader in this build reads, or is not UTF-8 text |
| `InvalidInput` | `{ reason: &'static str, message: String }` | The call cannot be honored as made. `reason` names the check that failed |
| `Io` | `std::io::Error` | A filesystem operation failed, or a download did not arrive |

`Error` implements `std::error::Error` and `Display` through `thiserror`, and `From<std::io::Error>`, so `?` converts I/O failures automatically.

### ModelNotFound

Returned by `Model2Vec::from_dir` when any of the three artifacts is absent. The payload is the path of the first one missing. Neither the UDPipe provisioner, behind `Engine::new` and `Engine::from_config` in Rust and Python, nor `Model2Vec::potion_base_8m` returns this variant: both download the model when the file is absent.

### ModelInvalid

Returned by:

- The UDPipe provisioner when the bytes it downloaded still fail SHA-256 verification after one refetch. The payload is `SHA-256 mismatch after re-download from <url>`. Nothing that failed verification is written, so the model directory is left as the call found it.
- `Model2Vec::from_dir` when an artifact does not parse, uses an embedding dtype other than f32, or panics the loader.
- `Model2Vec::potion_base_8m` when the directory already holds all three artifacts and their digest is not the pinned one, or holds only some of them. The payload names the directory and the ways out; nothing there is downloaded over or removed.
- `Model2Vec::potion_base_8m` when the artifacts it downloaded still fail the digest after one re-download. Both attempts verify in memory, so neither writes anything, and the directory is left exactly as the call found it. The payload names the directory and the expected digest.

A file whose hash does not match the pinned constant is treated as untrusted and is never loaded.

`ModelInvalid` is about bytes that arrived. A download that never arrived, a server that answered about the request rather than with a model, and a filesystem that refused the write are `Io`, not `ModelInvalid` ([RFC-0015](https://github.com/mox-labs/matra/blob/main/blueprints/legacy/rfcs/0015-provisioning-failures.md)). Before 0.2.0 the UDPipe download funnelled all three here, so a DNS failure reported `model_invalid`; a consumer that branched on that reads the [provisioning failures](#provisioning-failures) section below.

### ParseFailed

The parser or a reader failed on the input, or returned what its contract forbids. The remedy is the component, not the call: the same input through the same parser fails the same way. `cause` is free text for a person, and `reason` is present when the failure is one of the engine's checks.

A parser or a reader reports its own failure with `reason` `None`. The UDPipe adapter's `parse` does so in four situations:

- The underlying parser returns an error. The cause is that error's message.
- The underlying parser panics. The cause is `udpipe panicked: <message>`. The `catch_unwind` boundary that produces this lives in `nlp/udpipe.rs`, and it exists because an unhandled panic crossing the C++ boundary aborts the host process instead of unwinding.
- A token id or head value cannot be represented as `usize`. The cause names the token and its sentence.
- A token's form cannot be found in the paragraph text the parser was given, at or after the previous token, so the token has no span. The cause names the token, its sentence and the byte the search began at. The adapter reports this rather than a guessed span.

The engine trusts no parser and no reader, matra's own included. Before anything a reader returns becomes a section, and before anything the parser returns for a paragraph becomes a sentence, the engine checks it against the contract its trait states (`matra::decompose::Decomposer`, `matra::nlp::NlpProvider`), and a break fails the document with `reason` naming the check and `cause` naming the block, or the paragraph, sentence and token:

| `reason` | Check that failed | Of |
|---|---|---|
| `token_id_sequence` | A sentence's token ids do not run `1..=n` in order | the parser |
| `head_out_of_range` | A head is neither `0` nor the id of a token in the sentence | the parser |
| `root_count` | A sentence has no token whose head is `0`, or more than one; a sentence with no tokens has none | the parser |
| `head_cycle` | Following heads from a token never reaches the root | the parser |
| `token_span_invalid` | A token's span is reversed, runs past the paragraph, or splits a character | the parser |
| `token_span_order` | A token's span starts before the previous token's ends, within a sentence or across two | the parser |
| `token_form_mismatch` | The paragraph text at a token's span is not its form (a run of whitespace in the form matches any run of whitespace) | the parser |
| `block_span_invalid` | A block's or a skipped construct's span is reversed, runs past the analysed text, or splits a character | a reader |
| `block_span_order` | The blocks are out of document order, or two spans among the blocks and skipped constructs overlap | a reader |
| `heading_level` | A heading is at level 0 | a reader |
| `paragraph_text_mismatch` | A paragraph's text is not the analysed text its span covers | a reader |

A conforming parser or reader is one whose output passes these checks on every input it accepts. The checks are a pass over what they read: on the docsite's own pages they cost about 0.014% of UDPipe's parse time, about 3 microseconds a paragraph.

### InputTooLarge

Ten gate labels produce this variant. The `what` field carries the label so a caller can route each gate differently.

| `what` | Gate | Limit | Measured over |
|---|---|---|---|
| `"input"` | The engine's reading stage, the only route from text to the parser, for every `Input` constructor, and the CLI's stdin read, which applies the same cap first | `MAX_INPUT_BYTES`, 8 MiB (8,388,608) | Byte length of the input handed in, and of the analysed text the reader produces (for block input, the blocks joined); the CLI counts the bytes as they arrive, before decoding them |
| `"file_read"` | Reading the file an `Input::file` names | `MAX_INPUT_BYTES`, 8,388,608 bytes | File size reported by the filesystem, checked before any read |
| `"tfidf"` | `summarize`, and `summarize_with` with `SummaryMethod::TfIdf` | `MAX_TFIDF_SENTENCES`, 2,000 | Number of sentences in the document |
| `"textrank"` | `summarize_with` with `SummaryMethod::TextRank` | `MAX_TEXTRANK_SENTENCES`, 2,000 | Number of sentences in the document |
| `"rake"` | `keyphrases`, and `keyphrases_with` with `KeyphraseMethod::Rake` | `MAX_RAKE_TOKENS`, 200,000 | Total tokens across the document's sentences, punctuation included |
| `"yake"` | `keyphrases_with` with `KeyphraseMethod::Yake` | `MAX_YAKE_TOKENS`, 200,000 | Total tokens across the document's sentences, punctuation included |
| `"clusters"` | `clusters`, before anything is embedded | `MAX_CLUSTER_SENTENCES`, 2,000 | Number of sentences in the document |
| `"parser_model_download"` | The UDPipe provisioner | `MAX_DOWNLOAD_BYTES`, 64 MiB (67,108,864) | Bytes read from the response, which stops one past the cap |
| `"embedding_model_download"` | `Model2Vec::potion_base_8m`, per artifact | `MAX_DOWNLOAD_BYTES`, 64 MiB (67,108,864) | Bytes read from the response, which stops one past the cap |
| `"config_file_read"` | `Config::resolve` and `Config::from_file`, reading a config file | `MAX_CONFIG_FILE_BYTES`, 64 KiB (65,536) | File size reported by the filesystem, checked before any read |

`limit` carries the cap and `actual` carries the measured size, so an error message can be built without hardcoding the constants. Each cap is a published constant in `domain::limits`, named in the table; the module's documentation carries the cost model behind each value. No configuration changes any of them.

The caps bound worst-case memory and time. TextRank builds a dense similarity matrix that reaches roughly 32 MB of `f64` at 2,000 sentences. RAKE and YAKE build phrase-keyed maps whose size follows token count rather than sentence count, which is why their caps are stated in tokens. The two download caps are the gates whose input is not the caller's: they bound what a redirected or misbehaving server can make the process hold, and the read stops at the bound rather than continuing, so `actual` reports the bound that was breached rather than the response's full length. Both are 64 MiB per file, against the 16.3 MB parsing model and the embedding model's largest file, 30.2 MB of its 30.9 MB set, which is headroom for a later version and still finite.

A document from disk crosses both gates in sequence: `"file_read"` when the file is read, `"input"` before it is parsed. In practice `"file_read"` fires first, since both carry the same 8 MiB limit and the file size is checked before the read.

Every route from text to a parser, a parser of your own included, runs through the engine, so no text bypasses the `"input"` gate on its way to one.

The label has a second producer. The CLI reads stdin through `read_capped` in `src/cli/mod.rs`, which reads at most one byte past `MAX_INPUT_BYTES` and returns `InputTooLarge` with `what` set to `"input"` before the bytes ever reach the engine. Same limit, same label, so a caller sees one gate however the text arrived. The CLI never decodes what it reads: the bytes go to the engine as `Input::bytes`, so an oversized pipe is reported as too large, and a pipe that is not UTF-8 is `UnsupportedFormat`, as a file's bytes would be.

The four ranking methods check their caps after their empty-result checks. `summarize_with(&doc, 0, ..)` and `keyphrases_with(&doc, 0, ..)` return an empty result without evaluating the cap, whatever the size of the document. The same holds for a document with no sentences. `clusters` has no count parameter; its cap is checked first, before anything is embedded, then its contract, and a document with no sentences returns empty clusters.

A metric has no gate of its own. The compression ratio skips any paragraph over 262,144 bytes (`MAX_COMPRESSION_PARAGRAPH_BYTES`) and leaves its metric slot at `None` rather than returning an error, and a paragraph under a metric's minimum word count (`MIN_READABILITY_WORDS`, `MIN_COMPRESSION_WORDS`) does the same.

### InvalidInput

The call cannot be honored as made, and making it again as it is will fail the same way. The remedy is to change something the caller controls: the arguments, the configuration, the data handed in, or the embedder passed to `clusters`. Nothing about analysed text produces it.

One kind covers every such check, because they share that remedy, and `reason` says which check failed: a stable snake_case name, in `details["reason"]` on every channel, to branch on where the remedy differs. `message` names the values that failed it, for a person. A reader treats the set of reasons as open, since a later version can add a check.

| `reason` | Check that failed | Raised by |
|---|---|---|
| `threshold_not_finite` | The threshold is NaN or infinite | `clusters` |
| `embedder_vector_count` | The embedder returned a different number of vectors than there are sentences | `clusters` |
| `embedder_dimensions_differ` | Two of the embedder's vectors have different lengths | `clusters` |
| `embedder_value_not_finite` | A vector holds a NaN or an infinity | `clusters` |
| `not_a_directory` | The path is not a directory | `Inputs::dir` |
| `is_a_directory` | The path is a directory where one file is read | `Engine::analyze` of an `Input::file` or `Input::file_as` |
| `blocks_malformed` | Block input is not an array of heading and paragraph blocks | `Engine::analyze` of input read as `Format::Blocks`; Python's `Engine.analyze(blocks=...)` |
| `heading_level_zero` | A heading block has level 0 | `Engine::analyze` of block input, from `Input::blocks`, JSON or Python |
| `config_malformed` | The config file does not parse, holds an unknown key, or a value of the wrong type | `Config::resolve`, `Config::from_file` |
| `config_key_renamed` | The config file sets a key 0.3.0 renamed or removed (`[summarize]`, `algorithm`, `[semantic]`, `models.udpipe`); the message names each replacement | `Config::resolve`, `Config::from_file` |
| `config_method_unknown` | `summary.method` or `keyphrases.method` names a method this build does not know | `Config::resolve`, `Config::from_file` |
| `config_threshold_not_finite` | `clusters.threshold` is NaN or infinite | `Config::resolve`, `Config::from_file` |
| `config_not_path_component` | `models.embedding` is not a single directory name | `Config::resolve`, `Config::from_file` |
| `config_not_utf8` | The config file is not valid UTF-8 | `Config::resolve`, `Config::from_file` |
| `config_default_missing` | The built-in defaults lack a key: a defect in this build, to report | `Config::resolve`, `Config::from_file` |
| `no_data_directory` | None of `MATRA_DATA_DIR`, `XDG_DATA_HOME` or `HOME` is set | `Config::resolve`, `Engine::new` |
| `format_unknown` | A `format` name matra does not know | Python's `Engine.analyze` |
| `method_unknown` | A `method` name matra does not know | Python's `summarize` and `keyphrases` |
| `embedder_identity_failed` | The embedder has no `identity()`, or it raised | Python's `clusters` |
| `embedder_identity_shape` | `identity()` returned something other than an `Identity` dict | Python's `clusters` |
| `embedder_raised` | The embedder's `embed()` raised | Python's `clusters` |
| `embedder_return_shape` | `embed()` returned something other than a sequence of number sequences | Python's `clusters` |
| `usage` | The arguments do not parse: an unknown flag, a missing value | the command line, under `--json` |
| `no_command` | No subcommand and neither `--skill` nor `--schema` | the command line |
| `reference_without_skill` | `-r` without `--skill` | the command line |
| `reference_unknown` | `--skill -r` names no reference | the command line |
| `schema_unknown` | `--schema` names no schema | the command line |
| `format_with_directory` | `--format` given with a directory | `matra analyze` |
| `directory_for_one_document` | A directory given to a command that reads one document | `matra summarize`, `keyphrases`, `clusters` |
| `method_unavailable` | The configured method is one this build cannot run | `matra summarize`, `keyphrases` |
| `embedder_unavailable` | The build has no embedder (no `model2vec` feature) | `matra clusters` |
| `config_file_exists` | The config file exists and `--force` was not given | `matra config init` |
| `no_config_file_location` | None of `MATRA_CONFIG_FILE`, `XDG_CONFIG_HOME` or `HOME` is set | `matra config init` |
| `config_key_unrendered` | A resolved key has no renderer: a defect in this build, to report | `matra config show` |
| `embedded_file_malformed` | A schema or skill reference compiled into the binary does not read: a defect in this build, to report | `matra --schema`, `matra --skill` |

A config file that is simply absent is not an error for `Config::resolve`, and the built-in defaults stand; `Config::from_file` refuses one with `Io`, because the caller named it.

### UnsupportedFormat

Returned by `Engine::analyze` in two cases:

- The engine has no reader for the input's format: `Pdf` or `Docx`, whether named by the caller, read off the extension, or recognised from the bytes (`%PDF`, or a zip archive holding `[Content_Types].xml`, which outranks the extension), or a `Format::Other` the caller named and registered no reader for. `format` is that format, and `feature` names the Cargo feature that would read it; no feature of this version reads any of them, so `feature` is `None`. A reader registered with `Engine::with_reader` reads any of them. `Markdown`, `PlainText` and `Blocks` always have a reader.
- The bytes are not UTF-8 text and carry no signature matra recognises. `format` and `feature` are both `None`. This used to be `Io` with kind `InvalidData`; nothing went wrong reading the bytes, so it is now a kind with a remedy.

A UTF-8 file with an extension matra does not know is not this error: it reads as plain text.

### Io

Wraps `std::io::Error`, produced by:

- Reading a file rejecting a symlink, with `ErrorKind::Unsupported` and the message `refusing to read symlink: <path>`; `Inputs::dir` refuses a symlinked directory the same way. The path is also `details.path`.
- Reading a file rejecting a path that is neither a regular file nor a directory, with `ErrorKind::InvalidInput` and the message `not a regular file: <path>`, and the path in `details.path`. A directory is `InvalidInput` instead, naming `Inputs::dir`.
- Any read, directory listing, directory creation, write, or rename that fails.
- The UDPipe provisioner when a download fails at the transport or answers with a non-2xx status. Same shape as the embedding path below: the message names the URL, and the kind is `TimedOut` past the 300-second fetch budget or the 30-second connect budget, `NotConnected` for an unreachable host, and whatever the socket reported otherwise. The URL is also `details.url`, and `details.retryable` is `true`, except for a rejected certificate.
- The UDPipe provisioner when the model directory cannot be created, or when the verified bytes cannot be written or renamed into place. The message names the operation and the path, so a full disk reads `cannot write the model to <path>: No space left on device (os error 28)` rather than `Permission denied (os error 13)` with nothing to act on. The path is also `details.path`.
- `Model2Vec::potion_base_8m` when a download fails at the transport or answers with a non-2xx status. The message names the URL, and the kind is `TimedOut` past the 300-second fetch budget or the 30-second connect budget, `NotConnected` for an unreachable host, and whatever the socket reported otherwise. The URL and `retryable` are details, as on the UDPipe path. Bytes that arrived and then failed the digest are `ModelInvalid` instead; this variant is for the ones that never arrived.
- `Model2Vec::potion_base_8m` when the model directory cannot be created, when an artifact cannot be read, or when a verified artifact cannot be written or renamed into place. The message names the operation and the path, exactly as the UDPipe path's does, and the path is `details.path`. `Model2Vec::from_dir` reports an unreadable artifact the same way.
- `Model2Vec::potion_base_8m` when the temporary file an artifact lands through cannot be created, with the kind the open reported (`AlreadyExists` when something is already sitting at that path). The temporary is opened exclusively, so a path already there, symlink or not, fails the open rather than being written through.
- `Engine::analyze` when reading a file yields no document, with `ErrorKind::InvalidData` and the message `source returned no documents`.

## Provisioning failures

The UDPipe provisioner (behind `Engine::new` and `Engine::from_config`, in Rust and Python) and `Model2Vec::potion_base_8m` obtain a pinned artifact over the network. Both fetch through matra's own client, under the same bounds, and both classify a failure the same way.

| Condition | Variant | Kind | What the message carries | Details |
|---|---|---|---|---|
| DNS failure, unreachable host, refused connection | `Io`, `ErrorKind::NotConnected` | `io` | The URL | `url`, `retryable: true` |
| Connect budget exceeded (30 seconds) | `Io`, `ErrorKind::TimedOut` | `io` | The URL | `url`, `retryable: true` |
| Fetch budget exceeded (300 seconds, lookup through last byte) | `Io`, `ErrorKind::TimedOut` | `io` | The URL | `url`, `retryable: true` |
| TLS certificate rejected | `Io` | `io` | The host, why matra cannot be made to trust it, the way out, then the underlying failure | `url`, `retryable: false` |
| Non-2xx status | `Io` | `io` | The URL and the status | `url`, `retryable: true` |
| Response past 64 MiB | `InputTooLarge` | `input_too_large` | `what` is `"parser_model_download"` or `"embedding_model_download"` | `what`, `limit`, `actual` |
| Directory, write or rename failed | `Io` | `io` | The operation and the path | `path` |
| Bytes arrived and failed the pinned digest, twice | `ModelInvalid` | `model_invalid` | The URL, or the directory and the expected digest | `cause` |
| Bytes arrived, passed the digest, and did not load | `ModelInvalid` | `model_invalid` | The loader's message | `cause` |

The rule behind the table: `model_invalid` is about bytes that arrived. Anything that stopped a fetch from arriving, or a filesystem from accepting it, is `io` ([RFC-0015](https://github.com/mox-labs/matra/blob/main/blueprints/legacy/rfcs/0015-provisioning-failures.md)).

`retryable` is `true` when the same call can be made again as it is and may succeed. A failed fetch writes nothing, so a retry starts from where the first call did. A rejected certificate is `false`: a retry meets the same proxy, and the message names the way out instead.

The rule has a consequence in Python that is easy to miss, because it changes which `except` clause fires rather than only which string a message carries. `IoError` is an `OSError` and `ModelInvalidError` is a `RuntimeError`, so from 0.2.0 a DNS failure, a rejected certificate, a timeout or a non-2xx status raises an `OSError` from the call that builds the engine, where it used to raise `RuntimeError`. A caller that wrapped a bootstrap in `except RuntimeError` catches nothing now and the `OSError` propagates past it. Catch `matra.MatraError`, which both derive from, and branch on `kind`.

Nothing that failed the digest is ever written. Both provisioners fetch into memory, verify there, and write only what verified, so a download that fails adds nothing to the model directory and a run killed mid-transfer leaves nothing at all. A failed run removes nothing either. A cached file that is already there and fails verification is not loaded and not deleted: the UDPipe provisioner downloads again, at most twice, and replaces the file only with bytes that verified, through the same atomic rename every install lands through. A run whose downloads fail, at the network or at the digest, leaves that file byte-identical. `Model2Vec::potion_base_8m` replaces nothing and refuses a directory whose artifacts fail verification, because its three filenames belong to the artifact format rather than to this one model and so may be a caller's own ([RFC-0015](https://github.com/mox-labs/matra/blob/main/blueprints/legacy/rfcs/0015-provisioning-failures.md)). A temporary left by a killed process is reclaimed by the next download that finds it older than ten minutes, which is twice the fetch budget and therefore older than any transfer that could still be running. The UDPipe provisioner leaves a temporary directory and `Model2Vec::potion_base_8m` three temporary files, and each reclaims its own by the same rule.

### Behind a TLS-intercepting proxy

matra verifies TLS against root certificates compiled into the binary and never reads the system trust store, so a proxy that re-signs TLS cannot be made trusted. The fetch fails as `Io`, kind `io`, and the message names the host, says why its certificate cannot be trusted, names the file to fetch by hand, and ends with the underlying failure:

```text
matra: io error: download https://lindat.mff.cuni.cz/...: the TLS certificate offered for
lindat.mff.cuni.cz was rejected. matra verifies TLS against root certificates compiled into
it and never reads the system trust store, so a proxy that re-signs TLS cannot be trusted by
installing its CA. Fetch english-ewt-ud-2.5-191206.udpipe by hand and put it in the model
directory instead. Underlying failure: io: invalid peer certificate: ...
```

[Install the model behind a TLS-intercepting proxy](../guides/tls-proxy.md) has the steps.

### The first run says so

A cold run has to fetch 16.3 MB before it can answer, and how long that takes is the link's business rather than matra's. The command line writes two lines to standard error before the transfer and nothing at all when the model is already there:

```text
matra: downloading english-ewt-ud-2.5-191206.udpipe (16.3 MB) into /home/u/.local/share/matra/models
matra: the model is licensed CC BY-NC-SA 4.0 (non-commercial), separately from matra: https://creativecommons.org/licenses/by-nc-sa/4.0/
```

The second line is there because the model is a separate file with its own license, which is not matra's MIT; [model licenses](../tutorials/installation.md#model-licenses) has both models and their sources.

Standard error, so `--json` output stays a single object on standard output. `--quiet` silences it. A library caller gets the same facts as a `domain::ProvisionNotice` from `Engine::from_config_with_notice`, and decides the wording itself.

The first `matra clusters` says the same for the embedding model, whose three files are fetched as one set of 30.9 MB:

```text
matra: downloading potion-base-8M (30.9 MB) into /home/u/.local/share/matra/models/potion-base-8M
matra: the model is licensed MIT, separately from matra: https://opensource.org/license/mit
```

Python writes the same two lines to `sys.stderr` before `Engine()`, `Engine.from_config()`, `Model2Vec.potion_base_8m()` or `Model2Vec.from_config()` downloads; redirect `sys.stderr` to silence them. A Rust caller gets the facts from `Engine::from_config_with_notice` and `Model2Vec::from_config_with_notice` and chooses the wording, since the library writes to no terminal. The other Rust constructors, `Engine::new`, `Engine::from_config`, `Model2Vec::potion_base_8m` and `Model2Vec::from_config`, take no notice and say nothing for the length of their download.

## Per-document failures: `DocumentError`

The stream surface never lets one bad document abort the rest. `Engine::analyze_all` yields `Result<Document, DocumentError>` per input, where `DocumentError` pairs the `Error` with the path it occurred at (`None` for an input that is not a file, so a path-less document is distinguishable from one at an empty path). Its `Display` prints `path: error` when a path exists and the bare error otherwise, and its `source()` is the wrapped `Error`.

The stream yields one item per input, in order: the `Document`, or the `DocumentError` in its place, so documents plus errors equals inputs consumed, and partitioning them is the caller's. `Inputs::dir` is `Err` only when the listing itself fails; both read failures and analysis failures travel as stream items.

Two kinds of entry never appear on either side. The directory listing skips symlinks, so a symlink produces no document and no error. It also skips anything that is not a regular file, and the walk is one level deep, so a subdirectory is skipped the same way.

`DocumentError` serializes as its error's form plus the path: `{"path", "kind", "message", "details"}`, with `path` null for input that never came from disk. Python's `Engine.analyze_all` yields that form as a dict for each failed document. `kind` is the stable string `Error::kind` names for the variant, and the table below carries all seven. `Error::kind` is a match with no wildcard arm, so a new `Error` variant fails to compile until someone names it, and `spec/tests/corpus/items.json` pins the vocabulary with a runner per crust, so every crust spells a failure the same way. Both item shapes come from serde. Either's path (`provenance.source.path` on a document, `path` on an error) is decoded with `os.fsdecode`, so `os.fsencode` on it names the file the walk read even when that name is not valid UTF-8. The Python call yields the items in order and the caller partitions them, testing `isinstance(item, matra.Document)`.

## Kinds and details

`Error::details` returns an `ErrorDetails`: the values a caller acts on, by name, which for every kind but `invalid_input` is every dynamic value in the message. Each value is a `DetailValue`, which is text, a whole number or a boolean. A caller reads a value instead of parsing the message.

| Kind | Details |
|---|---|
| `model_not_found` | `path` |
| `model_invalid` | `cause` |
| `parse_failed` | `cause`; `reason` when what the parser or a reader returned failed one of the engine's checks, from [the table above](#parsefailed) |
| `input_too_large` | `what`, `limit`, `actual` |
| `unsupported_format` | `format` when one was recognised; `feature` when a Cargo feature would read it |
| `invalid_input` | `reason`, the check that failed, from [the table above](#invalidinput) |
| `io` | `io_kind` and `cause`; `path` when the failure concerns a path matra named; `url` and `retryable` when a download did not arrive |

- One word, one meaning. `reason` is always the name of a check that failed, stable and snake_case, to branch on: on `invalid_input` always, and on `parse_failed` when the engine's checks refused what a parser or a reader returned. `cause` is always free text for a person, like the message: the text the variant holds, or the operating system's account of an I/O failure.
- `io_kind` is the I/O failure's class in snake case: `not_found`, `permission_denied`, `already_exists`, `unsupported`, `invalid_input`, `invalid_data`, `timed_out`, `not_connected`, `connection_refused`, `connection_reset`, `unexpected_eof`, `interrupted`, `not_a_directory`, `is_a_directory` or `storage_full`. Any other class is `other`. Treat the set as open.
- `retryable` appears only on a download that did not arrive. Its absence makes no claim either way.

A detail that does not apply is absent, never empty: an `unsupported_format` for bytes that are not text carries no details at all. A key a kind carries keeps appearing in later versions. A later version may add keys, so read the ones you know and ignore the rest.

The serde form is `{"kind", "message", "details"}`:

```json
{ "kind": "input_too_large", "message": "input too large: 9000000 > limit 8388608",
  "details": { "what": "input", "limit": 8388608, "actual": 9000000 } }
```

`spec/schema/v2/error.schema.json` is the published JSON Schema for this form and for `DocumentError`'s, with the keys each kind carries.

## Display strings and kinds

These are the strings `Display` produces, which are also the strings Python's `str(exc)` returns, alongside the kind `Error::kind` reports for each variant. A message is for a human to read; a kind is what code branches on.

| Variant | Format | Kind |
|---|---|---|
| `ModelNotFound(path)` | `model not found: {path}` | `model_not_found` |
| `ModelInvalid(s)` | `invalid model: {s}` | `model_invalid` |
| `ParseFailed { reason, cause }` | `parse failed: {cause}` | `parse_failed` |
| `InputTooLarge { limit, actual, what }` | `{what} input too large: {actual} > limit {limit}`, or `input too large: {actual} > limit {limit}` when `what` is `"input"` | `input_too_large` |
| `UnsupportedFormat { format, feature }` | `unsupported format: {format}; no feature of this matra version reads it`, `unsupported format: {format}; no reader is registered for it` for a `Format::Other`, `` unsupported format: {format}; enable the `{feature}` feature to read it ``, or `unsupported format: the input is not UTF-8 text and carries no signature matra recognises` when `format` is `None` | `unsupported_format` |
| `InvalidInput { reason, message }` | `invalid input: {message}` | `invalid_input` |
| `Io(e)` | `io error: {e}` | `io` |

`UnsupportedFormat` renders the format's wire name, so the string reads `unsupported format: pdf; no feature of this matra version reads it`.

## Matching in Rust

```rust
use matra::domain::{Error, Format};
use matra::{Engine, Input};

fn report(text: &str, engine: &Engine) {
    match engine.analyze(Input::text(text, Format::PlainText)) {
        Ok(doc) => println!("{} sentences", doc.metrics.total_sentences),
        Err(Error::InputTooLarge {
            what,
            actual,
            limit,
        }) => eprintln!("{what} gate: {actual} over limit {limit}"),
        Err(Error::UnsupportedFormat { format, .. }) => eprintln!("no reader for {format:?}"),
        Err(other) => eprintln!("{}: {}", other.kind(), other),
    }
}
```

The details carry the same values without a match on the variant, which is what a caller forwarding an error to another program wants:

```rust
use matra::domain::{DetailValue, Error};

fn limit_of(err: &Error) -> Option<u64> {
    match err.details().get("limit") {
        Some(DetailValue::Integer(limit)) => Some(*limit),
        _ => None,
    }
}
```

## Python exception mapping

The PyO3 binding raises each `Error` as a class of its own, named for the variant with PEP 8's `Error` suffix. The name a failure shares across languages is its `kind`, not its class. Every class derives from `matra.MatraError` and from the builtin class matra raised before 0.3.0, so `except ValueError` still catches an `InputTooLargeError`. The routing is a match with no wildcard arm over the variants, so adding a variant to `Error` without assigning it a class fails to compile.

| Variant | Python class | Also a |
|---|---|---|
| `ModelNotFound` | `ModelNotFoundError` | `FileNotFoundError` |
| `ModelInvalid` | `ModelInvalidError` | `RuntimeError` |
| `ParseFailed` | `ParseFailedError` | `RuntimeError` |
| `InputTooLarge` | `InputTooLargeError` | `ValueError` |
| `UnsupportedFormat` | `UnsupportedFormatError` | `ValueError` |
| `InvalidInput` | `InvalidInputError` | `ValueError` |
| `Io`, any other kind | `IoError` | `OSError` |
| `Io`, kind `NotFound` | `IoNotFoundError` | `FileNotFoundError` |
| `Io`, kind `PermissionDenied` | `IoPermissionError` | `PermissionError` |

`Io` routes on the I/O kind it wraps. A missing file is an `IoNotFoundError`, which is a `FileNotFoundError`; a refused permission is an `IoPermissionError`, which is a `PermissionError`. Both derive from `IoError`, so `except matra.IoError` catches all three. `matra.IoError` is matra's class, not the builtin `IOError`, which is another name for `OSError`.

Every exception carries the same three things as the serde form: `kind`, the string `Error::kind` names; `details`, a `dict` of the [details](#kinds-and-details); and the `Display` string as its message, which `str(exc)` returns. Branch on the class or on `kind`, and read values from `details`:

```python
import matra

try:
    engine.analyze(huge)
except matra.InputTooLargeError as exc:   # also a ValueError
    print(exc.kind, exc.details["what"], exc.details["limit"], exc.details["actual"])
```

`IoNotFoundError` is a `FileNotFoundError`, so the conventional Python idiom works:

```python
try:
    doc = engine.analyze(file="notes.md")
except FileNotFoundError as exc:
    print(exc.details["path"])  # notes.md
```

The classes name `matra` as their module, so a traceback reads `matra.InputTooLargeError`, and an exception survives `pickle` with its `kind` and `details`.

Every Python input reaches the parser through the engine's reading stage, so the 8 MiB `"input"` gate applies uniformly, to `text`, `bytes` and `blocks` alike, and a `file` meets `"file_read"` first. The outcome functions take a `Document`, never text, and their own caps apply to it.

One further failure has no `Error` variant behind it. Python's `Engine` is `#[pyclass(unsendable)]`, because the loaded model holds C-side state that is not thread-safe. Accessing one engine from a thread other than the one that created it fails at runtime. Multi-process use is unaffected.

## At the command line

One program is installed under the name `matra`, by either of two routes. The Rust binary comes from `cargo install matra --features cli`; the Python console script comes from the wheel and calls the same `matra::cli::run` through the extension module. The exit-code contract below is the program's, so it holds on both.

| Exit code | Meaning |
|---|---|
| 0 | The command succeeded, and where applicable something was found |
| 1 | The command succeeded and found nothing |
| 2 | An error occurred |

On exit code 2 the command writes `matra: ` followed by the error's `Display` string to standard error, prefixed with the path when the failure concerns a file. Under `--json` it also writes the envelope to standard output, with `result` null and `error` holding the serde form above, `{kind, message, details}`: the same three a Rust caller serializes and a Python caller reads off the exception. The envelope's `input` names the file, so the error object carries no `path` of its own.

```json
{ "format_version": 2, "command": "analyze", "input": "scan.pdf", "result": null,
  "error": { "kind": "unsupported_format", "message": "unsupported format: pdf; no feature of this matra version reads it", "details": { "format": "pdf" } } }
```

`spec/tests/parity/parity.json` pins that equality: the same files through Rust, Python and the command line give the same `kind`, `message` and `details`, and a runner per surface fails on a drift.

A broken pipe, which is what happens when the reading end of `matra analyze file.md | head` goes away, exits 0 and prints nothing.

`matra analyze DIR` reads every file in the directory and keeps going past a failure. Each failed file is a line on standard error and, under `--json`, its own envelope line with its `error`; the run exits 2 if any file failed, after the walk has finished. This is the one partial success the command line has: per-item failure in a bulk operation.

The table is the whole contract, and no command narrows it. The command line's own refusals carry the kind whose remedy matches, which for each of them is `InvalidInput`, a call to change: a directory given to `summarize`, `keyphrases` or `clusters`, `--format` with a directory, `matra --skill -r <name>` or `matra --schema <name>` with a name nothing answers to (both name the ones that exist), `-r` without `--skill`, `config init` over an existing file without `--force`, `clusters` in a build without the `model2vec` feature, and a usage error such as an unknown flag. A missing path is not one of them: it is the library's own `Io` failure with `not_found`, raised before the engine is built so it costs no model load. `matra --skill` and `matra --schema` in any of their accepted forms exit 0; they read no document, so they have nothing to find and never exit 1.

The exception classes above belong to the library surface, not to the command line. A `domain::Error` reaching the command line is rendered and turned into an exit code and, under `--json`, an envelope; it never surfaces as a Python exception, whichever launcher started it.
