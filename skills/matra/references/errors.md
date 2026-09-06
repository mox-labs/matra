---
name: errors
summary: Every failure kind, the size gate that produced it, its Python exception, and what to do about it.
---

# Errors

Every fallible call returns a typed error, never a string and never a panic. A panic raised inside the UDPipe C++ boundary is caught at the adapter and converted, so it cannot abort the host process.

Seven kinds exist. The kind string is the stable key a consumer branches on; the message is for a human and is not a contract.

| Kind | Display | Python exception | Means |
|---|---|---|---|
| `model_not_found` | `model not found: {path}` | `FileNotFoundError` | A model file does not exist at the path given |
| `model_invalid` | `invalid model: {s}` | `RuntimeError` | Bytes that arrived could not be verified against the pin, or could not be loaded |
| `parse_failed` | `parse failed: {s}` | `RuntimeError` | The parser failed on the input, panicked, or produced an unusable token id |
| `input_too_large` | `{what} input too large: {actual} > limit {limit}` | `ValueError` | A size gate rejected the input. `what` names the gate |
| `unsupported_format` | `unsupported format: {format}` | `ValueError` | The document's format has no decomposer in this build |
| `invalid_input` | `invalid input: {s}` | `ValueError` | A caller violated a documented contract |
| `io` | `io error: {e}` | `OSError` | A filesystem or network operation failed |

## What to do about each

**`model_not_found`.** You named a path that is not there. The constructors that download (the no-argument English engine, the pinned embedding model) never produce this: they fetch instead. Check the path, or switch to the downloading constructor.

**`model_invalid`.** Either the bytes do not load, or they do not match the pinned digest. A download that fails the digest is fetched once more, in memory, and a second failure produces this. Nothing that failed verification is ever written, so a failed provision leaves the model directory as it found it. A download that never arrived is `io`, not this. For the embedding model this also fires when the directory already holds three artifacts that are not the pinned set, or holds only some of them, in which case nothing there was downloaded over or removed. Do not work around it by loading the file anyway. A model whose hash does not match is untrusted, and every number downstream inherits it.

**`parse_failed`.** The parser failed on this text. Retrying the same bytes will fail the same way. Reduce the input to find the failing paragraph if it matters; otherwise skip the document. In a directory walk it arrives as one item and the rest of the walk continues.

**`input_too_large`.** Read the `what` label and act on the gate that fired, not on the size in general.

| `what` | Gate | Limit | Measured over |
|---|---|---|---|
| `input` | The only route from text to the parser, and the command line's stdin read | 8 MiB (8,388,608) | UTF-8 byte length. The command line counts bytes before decoding them |
| `file_source` | Reading a file from disk | 8,388,608 bytes | File size from the filesystem, checked before any read |
| `tfidf` | TF-IDF summarization | 2,000 | Sentences in the slice |
| `textrank` | TextRank summarization | 2,000 | Sentences in the slice |
| `rake` | RAKE keyphrases | 200,000 | Tokens across the slice, punctuation included |
| `yake` | YAKE keyphrases | 200,000 | Tokens across the slice, punctuation included |
| `semantic_clusters` | Clustering | 2,000 | Sentences in the slice |
| `embedding_download` | One embedding artifact fetch | 64 MiB (67,108,864) | Bytes read from the response, which stops one past the cap |
| `config_file` | Reading the config file | 64 KiB (65,536) | File size from the filesystem, checked before any read |

The value carries both `limit` and `actual`, so a message can be built without hardcoding the constants. The caps bound worst-case memory and time: TextRank's dense matrix reaches roughly 32 MB at its cap, and the keyphrase caps are stated in tokens because their maps grow with token count rather than sentence count. The download cap is the one gate whose input is not the caller's; it bounds what a misbehaving server can make the process hold.

A document from disk crosses two text gates in sequence, and `file_source` fires first, since both carry the same 8 MiB limit and the file size is checked before the read.

The four ranking extractors check their caps after their empty-result checks, so asking for zero results returns an empty list whatever the size of the slice.

**`unsupported_format`.** PDF and DOCX are reserved names with no decomposer in this build. Convert to markdown or plain text first, or read the text yourself and pass it in.

**`invalid_input`.** This one means the call site is wrong, not the document. Nothing about the analyzed text produces it. It fires for embeddings that disagree on dimension, an embedding containing a non-finite value, a non-finite threshold, an embedder that broke its own length contract, and for a configuration that cannot be honored: a config file that does not parse, an unknown key, an algorithm name the build does not know, a config file that is not valid text, or an environment naming no home directory at all. Fix the call or the config. A config file that is simply absent is not an error; the built-in defaults stand.

**`io`.** A read, listing, directory creation, removal, or rename failed, or a download failed at the transport or answered with a non-2xx status. A filesystem failure names the operation and the path; a transport failure names the URL. It is also what a refused input looks like: a symlink is refused rather than followed, and a path that is not a regular file is refused. Bytes that arrived and then failed their digest are `model_invalid` instead; this kind is for the ones that never arrived.

In Python this kind is `OSError`. From 0.2.0 that includes every failed download, which raised `RuntimeError` before, so a caller catching `RuntimeError` around a first run needs `OSError` as well.

## Per-document failures

A stream of documents never lets one bad file abort the rest. Each document yields either its analysis or a failure paired with the path it happened at, and collecting the stream partitions the two, with successes plus failures equal to documents consumed. Nothing is silently dropped.

Two kinds of entry appear on neither side: the directory listing skips symlinks and skips anything that is not a regular file, and the walk is one level deep, so a subdirectory is skipped the same way. Neither produces a document nor an error.

## At the command line

<!-- expect: exit 2 -->

```console
$ matra analyze missing.md
```

Exit 2, with `matra: ` and the display string on stderr. Nothing is written to stdout, so a consumer parsing JSON sees an empty stream rather than a half object. The model is not touched: a missing path and a directory are both refused before the engine is built, so a bad argument never costs a 16 MB download.

Exit 1 is not a failure. It is success with nothing found, and under `--json` the envelope is still emitted with an empty result.

<!-- needs: model -->
<!-- expect: exit 1 -->

```console
$ matra summarize draft.txt -n 0 --json
```

The exception classes above belong to the library surface, not to the command line. A failure reaching the command line is rendered and turned into an exit code; it never surfaces as a Python exception, whichever launcher started it.

## One failure with no kind behind it

The Python engine class is unsendable: the loaded model holds C-side state that is not thread-safe, so accessing one instance from a thread other than the one that created it fails at runtime with no library error variant behind it. Multi-process use is unaffected.
