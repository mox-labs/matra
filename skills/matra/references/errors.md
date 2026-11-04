---
name: errors
summary: Every failure kind, its details, the size gate that produced it, its Python exception, and what to do about it.
---

# Errors

Every fallible call returns a typed error, never a string and never a panic. A panic raised inside the UDPipe C++ boundary is caught at the adapter and converted, so it cannot abort the host process.

Seven kinds exist. The kind string is the stable key a consumer branches on; the message is for a human and is not a contract. Every dynamic value in the message is also in the error's details, by name, so nothing has to be parsed out of the message.

| Kind | Display | Python class | Also a | Means |
|---|---|---|---|---|
| `model_not_found` | `model not found: {path}` | `ModelNotFoundError` | `FileNotFoundError` | A model file does not exist at the path given |
| `model_invalid` | `invalid model: {s}` | `ModelInvalidError` | `RuntimeError` | Bytes that arrived could not be verified against the pin, or could not be loaded |
| `parse_failed` | `parse failed: {s}` | `ParseFailedError` | `RuntimeError` | The parser failed on the input, panicked, or produced an unusable token id |
| `input_too_large` | `{what} input too large: {actual} > limit {limit}`, or `input too large: ...` for the main gate, whose `what` is `"input"` | `InputTooLargeError` | `ValueError` | A size gate rejected the input. `what` names the gate |
| `unsupported_format` | `unsupported format: {format}; ...`, or `unsupported format: the input is not UTF-8 text ...` | `UnsupportedFormatError` | `ValueError` | The input is a PDF or DOCX, which no reader in this build reads, or is not UTF-8 text |
| `invalid_input` | `invalid input: {s}` | `InvalidInputError` | `ValueError` | A caller violated a documented contract |
| `io` | `io error: {e}` | `IoError` | `OSError` | A filesystem or network operation failed |

Each Python class lives in `matra` and derives from `matra.MatraError` and from the builtin beside it, so an `except ValueError` still catches an `InputTooLargeError`. An `io` failure whose I/O kind is not found is an `IoNotFoundError`, also a `FileNotFoundError`, and one whose kind is permission denied is an `IoPermissionError`, also a `PermissionError`; both derive from `IoError`, which is not the builtin `IOError`.

## Details

The serde form of an error is `{kind, message, details}`, and a Python exception carries the same three as `kind`, `str(exc)` and `details`. `details` holds every dynamic value in the message, by name:

| Kind | Details |
|---|---|
| `model_not_found` | `path` |
| `model_invalid`, `parse_failed`, `invalid_input` | `reason`, the variant's own text |
| `input_too_large` | `what`, `limit`, `actual` |
| `unsupported_format` | `format` when one was recognised; `feature` when a feature would read it |
| `io` | `io_kind` (`not_found`, `permission_denied`, `timed_out`, ...) and `reason`; `path` when the failure concerns a path; `url` and `retryable` when a download did not arrive |

A detail that does not apply is absent. A key a kind carries keeps appearing; new keys may be added, so read the ones you know. `retryable` is `true` when the same call can be made again as it is and may succeed, and appears only on a download. The JSON Schema is `spec/schema/v2/error.schema.json`.

## What to do about each

**`model_not_found`.** You named a path that is not there. The constructors that download (the no-argument English engine, the pinned embedding model) never produce this: they fetch instead. Check the path, or switch to the downloading constructor.

**`model_invalid`.** Either the bytes do not load, or they do not match the pinned digest. A download that fails the digest is fetched once more, in memory, and a second failure produces this. Nothing that failed verification is ever written, so a failed provision leaves the model directory as it found it. A download that never arrived is `io`, not this. For the embedding model this also fires when the directory already holds three artifacts that are not the pinned set, or holds only some of them, in which case nothing there was downloaded over or removed. Do not work around it by loading the file anyway. A model whose hash does not match is untrusted, and every number downstream inherits it.

**`parse_failed`.** The parser failed on this text. Retrying the same bytes will fail the same way. Reduce the input to find the failing paragraph if it matters; otherwise skip the document. In a directory walk it arrives as one item and the rest of the walk continues.

**`input_too_large`.** Read `details.what` and act on the gate that fired, not on the size in general.

| `what` | Gate | Limit | Measured over |
|---|---|---|---|
| `input` | The only route from text to the parser, and the command line's stdin read | 8 MiB (8,388,608) | UTF-8 byte length. The command line counts bytes before decoding them |
| `file_source` | Reading a file from disk | 8,388,608 bytes | File size from the filesystem, checked before any read |
| `tfidf` | TF-IDF summarization | 2,000 | Sentences in the slice |
| `textrank` | TextRank summarization | 2,000 | Sentences in the slice |
| `rake` | RAKE keyphrases | 200,000 | Tokens across the slice, punctuation included |
| `yake` | YAKE keyphrases | 200,000 | Tokens across the slice, punctuation included |
| `clusters` | Clustering | 2,000 | Sentences in the document, or vectors passed in |
| `udpipe_download` | The UDPipe model fetch | 64 MiB (67,108,864) | Bytes read from the response, which stops one past the cap |
| `embedding_download` | One embedding artifact fetch | 64 MiB (67,108,864) | Bytes read from the response, which stops one past the cap |
| `config_file` | Reading the config file | 64 KiB (65,536) | File size from the filesystem, checked before any read |

The details carry both `limit` and `actual`, so a message can be built without hardcoding the constants. The caps bound worst-case memory and time: TextRank's dense matrix reaches roughly 32 MB at its cap, and the keyphrase caps are stated in tokens because their maps grow with token count rather than sentence count. The two download caps are the gates whose input is not the caller's; they bound what a misbehaving server can make the process hold.

A document from disk crosses two text gates in sequence, and `file_source` fires first, since both carry the same 8 MiB limit and the file size is checked before the read.

The four ranking extractors check their caps after their empty-result checks, so asking for zero results returns an empty list whatever the size of the slice.

**`unsupported_format`.** PDF and DOCX are recognised, by extension and by their bytes, and no reader for either ships, so the message says no feature reads them, and `details.format` names the format with no `feature` beside it. Bytes that are not UTF-8 text fail the same way. Convert to markdown or plain text first, or extract the text yourself and pass it in. A UTF-8 file with an extension matra does not know is not refused: it reads as plain text.

**`invalid_input`.** This one means the call site is wrong, not the document. Nothing about the analyzed text produces it. It fires for embeddings that disagree on dimension, an embedding containing a non-finite value, a non-finite threshold, an embedder that broke its own length contract, and for a configuration that cannot be honored: a config file that does not parse, an unknown key, a key 0.3.0 renamed or removed (the message names what replaced it), a method name the build does not know, a config file that is not valid text, or an environment naming no home directory at all. Fix the call or the config. A config file that is simply absent is not an error; the built-in defaults stand.

**`io`.** A read, listing, directory creation, write, or rename failed, or a download failed at the transport or answered with a non-2xx status. A filesystem failure names the operation and the path, and carries the path in `details.path`; a transport failure names the URL, carries it in `details.url`, and sets `details.retryable`, which is `true` unless the certificate was rejected. Read `details.io_kind` for the class of failure rather than the message. It is also what a refused input looks like: a symlink is refused rather than followed, and a path that is not a regular file is refused. A file whose bytes are not text is `unsupported_format`, not `io`. Bytes that arrived and then failed their digest are `model_invalid` instead; this kind is for the ones that never arrived.

In Python this kind is `matra.IoError`, an `OSError`. From 0.2.0 that includes every failed download, which raised `RuntimeError` before, so a caller catching `RuntimeError` around a first run needs `OSError` as well, or catches `matra.MatraError` and branches on `kind`.

## Per-document failures

A stream of documents never lets one bad file abort the rest. Each document yields either its analysis or a failure paired with the path it happened at, and collecting the stream partitions the two, with successes plus failures equal to documents consumed. Nothing is silently dropped.

Two kinds of entry appear on neither side: the directory listing skips symlinks and skips anything that is not a regular file, and the walk is one level deep, so a subdirectory is skipped the same way. Neither produces a document nor an error.

## At the command line

<!-- expect: exit 2 -->

```console
$ matra analyze missing.md --json
```

Exit 2, with `matra: ` and the display string on stderr. Under `--json` the envelope goes to stdout with `result: null` and `error` holding the `kind`, `message` and `details` Rust serializes and Python raises, so a consumer branches on `kind` on every surface. Without `--json`, nothing goes to stdout. The model is not touched: a missing path, and a directory given to a command that reads one document, are refused before the engine is built, so a bad argument never costs a 16 MB download.

The command line's own refusals carry the kind whose remedy matches, and each is a call to change, so each is `invalid_input`: a directory given to `summarize`, `keyphrases` or `clusters`, `--format` with a directory, `-r` without `--skill`, an unknown `--skill -r` or `--schema` name, `config init` over an existing file without `--force`, `clusters` in a build without the `model2vec` feature, and a usage error under `--json`.

`analyze DIR` reports a file that fails where it happened: on stderr, and under `--json` as that file's own line with its `error`. The walk goes on, and the exit code is 2 if any file failed.

Exit 1 is not a failure. It is success with nothing found, and under `--json` the envelope is still emitted with an empty result.

<!-- needs: model -->
<!-- expect: exit 1 -->

```console
$ matra summarize draft.txt -n 0 --json
```

The exception classes above belong to the library surface, not to the command line. A failure reaching the command line is rendered and turned into an exit code and, under `--json`, an envelope; it never surfaces as a Python exception, whichever launcher started it.

## One failure with no kind behind it

The Python engine class is unsendable: the loaded model holds C-side state that is not thread-safe, so accessing one instance from a thread other than the one that created it fails at runtime with no library error variant behind it. Multi-process use is unaffected.
