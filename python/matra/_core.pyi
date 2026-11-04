"""Type stubs for the PyO3 extension module.

These stubs describe the surface exposed by `_core` (built from `src/python/`).
The TypedDict return types are imported from `matra.types`, where they are
defined as runtime modules so they're available at runtime (not just at
type-check time).

Stubs are versioned alongside the Rust code; keep them in lockstep with
`#[pyclass]`/`#[pymethods]` signatures and with `matra.types`. CI checks
the names, parameters, defaults and `@final` markers against the built
module with `mypy.stubtest` (the `pytype` job); return types and the
documented exceptions are read by review.
"""

from __future__ import annotations

import builtins
import os
from collections.abc import Iterable, Iterator, Sequence
from typing import Literal, final, overload

from matra.types import (
    BlockDict,
    Clusters,
    DocumentDict,
    DocumentError,
    DocumentMetrics,
    Embedder,
    ErrorDetails,
    FormatName,
    KeyphraseMethodName,
    Keyphrases,
    Provenance,
    Section,
    Skipped,
    Summary,
    SummaryMethodName,
)

__all__ = [
    "Document",
    "Engine",
    "InputTooLargeError",
    "InvalidInputError",
    "IoError",
    "IoNotFoundError",
    "IoPermissionError",
    "MatraError",
    "Model2Vec",
    "ModelInvalidError",
    "ModelNotFoundError",
    "ParseFailedError",
    "UnsupportedFormatError",
    "cli_main",
    "clusters",
    "keyphrases",
    "summarize",
]

class MatraError(Exception):
    """Every failure matra raises.

    `kind` is the stable string to branch on, one of `ERROR_KINDS`; each
    subclass below carries its own. `details` holds every dynamic value
    in the message, by name (see `matra.ErrorDetails` for the keys per
    kind), so nothing has to be parsed out of `str(exc)`.

    Each subclass also derives from the builtin class matra raised before
    0.3.0, so `except ValueError` still catches an `InputTooLargeError`.
    """

    kind: str
    details: ErrorDetails

class ModelNotFoundError(MatraError, FileNotFoundError):
    """A model file does not exist where it was looked for (an artifact
    missing from `Model2Vec.from_dir`'s directory). `kind` is
    `model_not_found`; `details["path"]` is the path."""

class ModelInvalidError(MatraError, RuntimeError):
    """Bytes that arrived could not be verified against the pin, or could
    not be loaded. `kind` is `model_invalid`."""

class ParseFailedError(MatraError, RuntimeError):
    """The parser failed on the input, or panicked. `kind` is
    `parse_failed`."""

class InputTooLargeError(MatraError, ValueError):
    """A size gate rejected the input. `kind` is `input_too_large`;
    `details` carries `what` (the gate), `limit` and `actual`."""

class UnsupportedFormatError(MatraError, ValueError):
    """The input is in a format this build cannot read, or is not UTF-8
    text. `kind` is `unsupported_format`; `details` carries `format` when
    one was recognised and `feature` when one would read it."""

class InvalidInputError(MatraError, ValueError):
    """A caller broke a documented contract. `kind` is
    `invalid_input`."""

class IoError(MatraError, OSError):
    """A filesystem operation failed, or a download did not arrive. `kind`
    is `io`; `details` carries `io_kind` and `reason`, `path` when the
    failure concerns one, and `url` and `retryable` for a download. Not
    the builtin `IOError`, which is another name for `OSError`."""

class IoNotFoundError(IoError, FileNotFoundError):
    """An `IoError` failure whose I/O kind is `not_found`."""

class IoPermissionError(IoError, PermissionError):
    """An `IoError` failure whose I/O kind is `permission_denied`."""

@final
class Engine:
    """The analysis pipeline: the pinned English parser and the readers
    this build ships. Build one and reuse it; loading the model is the
    slow part, and the first one downloads it.

    Final: the extension class cannot be subclassed at runtime.

    The parser holds C-side state that is not thread-safe, so the class
    is `#[pyclass(unsendable)]`: touching an engine from a thread other
    than the one that built it raises `pyo3_runtime.PanicException`,
    which derives from `BaseException`. Multi-process Python (e.g.
    `ProcessPoolExecutor`) is fine; multi-thread is not.
    """

    def __new__(cls) -> Engine:
        """The model located from the environment alone:
        `MATRA_MODEL_DIR`, else the `models` subdirectory of the data root
        (`MATRA_DATA_DIR`, else `$XDG_DATA_HOME/matra`, else
        `~/.local/share/matra`), except that a pre-existing
        `~/.matra/models` wins when the new location does not exist yet.
        No configuration file is read.

        The download is atomic and SHA-256-verified against a pinned
        hash. Concurrent processes cannot corrupt each other's downloads.

        Raises:
            InvalidInputError: the environment names no home directory
                at all.
            InputTooLargeError: the download exceeded its 64 MiB cap.
            IoError: the model directory could not be created or written,
                or the download did not arrive (DNS, TLS, a timeout, or a
                non-2xx status).
            ModelInvalidError: bytes arrived and then failed the pinned
                hash, or the verified bytes did not load.
        """
        ...

    @staticmethod
    def from_config(config_file: str | os.PathLike[str] | None = None) -> Engine:
        """`Engine()`, with a configuration file read and checked: the one
        at `config_file`, which must exist, or with no argument the one
        the environment names (`MATRA_CONFIG_FILE`, else
        `$XDG_CONFIG_HOME/matra/config.toml`, else
        `~/.config/matra/config.toml`), where a missing file is no error.

        Raises:
            InvalidInputError: the config file is malformed or not UTF-8.
            InputTooLargeError: the config file exceeds 64 KiB.
            IoError: `config_file` does not exist or cannot be read; and
                whatever `Engine()` raises.
        """
        ...

    def analyze(
        self,
        text: str | None = None,
        *,
        file: str | os.PathLike[str] | None = None,
        bytes: builtins.bytes | None = None,  # noqa: A002 - the published keyword name
        blocks: Sequence[BlockDict] | None = None,
        format: FormatName | None = None,  # noqa: A002 - the published keyword name
    ) -> Document:
        """Analyze one document: read it, parse each paragraph, measure it.

        Exactly one of `text`, `file`, `bytes` and `blocks`. A `str` is
        always text, never a path: name a file with `file=`.

        - `text` is read as `format`, plain text when none is given.
        - `file` is read at the call, its format detected from its name
          and bytes (`.md` as markdown, PDF and DOCX recognised and
          refused, anything else that is UTF-8 as plain text).
        - `bytes` is read as `format`, or detected when none is given.
        - `blocks` is headings and paragraphs you supply (`BlockDict`).

        `format` applies to `text` and `bytes` only.

        Raises:
            TypeError: not exactly one input, or `format` with `file` or
                `blocks`.
            InputTooLargeError: the input exceeds the size cap (8 MiB).
            UnsupportedFormatError: a format no reader ships for, or
                bytes that are not UTF-8.
            InvalidInputError: an unknown `format`, a directory named as
                a file, or block input that is not a block array.
            IoError: the file cannot be read (`IoNotFoundError` when it
                does not exist).
            ParseFailedError: the parser failed.
        """
        ...

    def analyze_all(
        self,
        *,
        dir: str | os.PathLike[str] | None = None,  # noqa: A002 - the published keyword name
        texts: Iterable[str] | None = None,
        format: FormatName | None = None,  # noqa: A002 - the published keyword name
    ) -> Iterator[Document | DocumentError]:
        """Analyze many documents, lazily and in input order.

        Exactly one of `dir` (every regular file in it, in path order;
        symlinks and subdirectories are skipped, not followed) and
        `texts` (each read as `format`, plain text when none is given).
        Nothing is read or parsed until the iterator is pulled.

        Each item is the `Document` for an input that analyzed, or a
        `DocumentError` dict for one that did not, so one unreadable file
        costs one item rather than the stream. Tell them apart with
        `isinstance(item, Document)`. On Unix a returned path is decoded
        with `os.fsdecode`, so `os.fsencode` on it names the same file
        even when the name is not valid UTF-8.

        Raises:
            TypeError: not exactly one of `dir` and `texts`, `texts` is a
                `str`, or `format` with `dir`.
            IoError: `dir` does not exist or cannot be listed.
            InvalidInputError: `dir` is not a directory, or an unknown
                `format`.
        """
        ...

@final
class Document:
    """One analysed document: read-only, holding the Rust value.

    Read it as the JSON reads: `doc["metrics"]["total_words"]`, one key
    per top-level field, and `to_dict()` for the whole. Each read builds
    a new value, so changing what a read returned changes nothing here.
    There is nothing to assign to.

    The outcome functions (`summarize`, `keyphrases`, `clusters`) take a
    `Document` and no dict, so a result cannot carry provenance no matra
    run produced. Final: the extension class cannot be subclassed.
    """

    @overload
    def __getitem__(self, key: Literal["provenance"], /) -> Provenance: ...
    @overload
    def __getitem__(self, key: Literal["sections"], /) -> list[Section]: ...
    @overload
    def __getitem__(self, key: Literal["metrics"], /) -> DocumentMetrics: ...
    @overload
    def __getitem__(self, key: Literal["skipped"], /) -> list[Skipped]: ...
    def __contains__(self, key: object, /) -> bool: ...
    def __iter__(self) -> Iterator[str]: ...
    def __len__(self) -> int: ...
    def keys(self) -> list[str]:
        """The top-level keys, in the order the JSON writes them."""
        ...

    def to_dict(self) -> DocumentDict:
        """The whole document as the schema's JSON shape: a new dict,
        yours to change."""
        ...

@final
class Model2Vec:
    """A loaded static embedding model (model2vec artifact format).

    Tier 2: its vectors are model opinion; everything derived from them
    carries this model's identity. Final: the extension class cannot be
    subclassed at runtime.
    """

    @staticmethod
    def from_dir(
        dir: str | os.PathLike[str],  # noqa: A002 - the published keyword name
    ) -> Model2Vec:
        """Load from a directory holding model.safetensors,
        tokenizer.json, and config.json. No network is touched.

        Raises:
            FileNotFoundError: an artifact file is absent.
            OSError: an artifact is present but cannot be read. The
                message names the operation and the path.
            RuntimeError: the artifact does not parse or is malformed.
        """
        ...

    @staticmethod
    def potion_base_8m(
        dir: str | os.PathLike[str] | None = None,  # noqa: A002 - the published keyword name
    ) -> Model2Vec:
        """Download (if absent) and load the pinned reference model.

        The three artifact files are verified against a digest compiled
        into the library before anything is loaded, so exactly one
        artifact set can arrive this way. Downloading happens only into
        a directory holding none of the three, and the set is verified in
        memory before anything is written, so a mismatch retries once and
        leaves the directory as it was found. With no argument the
        directory is resolved through the configuration: the model
        directory joined with the configured embedding model name.

        Raises:
            OSError: the directory cannot be created, read, or written,
                or a download failed at the transport or answered with a
                non-2xx status. A filesystem failure names the operation
                and the path; a transport failure names the URL.
            ValueError: a download exceeded the 64 MiB artifact cap; or,
                with no argument, the config file is malformed or
                exceeds 64 KiB, or the environment names no home
                directory.
            RuntimeError: the directory already holds artifacts that are
                not the pinned set, the digest still mismatched after one
                retry, or the verified bytes do not parse.
        """
        ...

    @property
    def model_hash(self) -> str:
        """SHA-256 over the three artifact files: the model identity."""
        ...

    @property
    def dimensions(self) -> int:
        """Dimensions of every vector this model produces."""
        ...

    def embed(self, texts: list[str]) -> list[list[float]]:
        """Embed each text into a vector: one per text, in order, all
        of `dimensions` length.

        Raises:
            RuntimeError: embedding failed.
        """
        ...

    def identity(self) -> str:
        """The geometry these vectors live in: the same string
        `model_hash` carries.

        With `embed`, this is what makes a `Model2Vec` an `Embedder`, so
        one annotation covers it and a caller's own object alike.
        """
        ...

def cli_main(argv: Sequence[str | bytes | os.PathLike[str] | os.PathLike[bytes]]) -> int:
    """Run the matra command line and return its exit code.

    `argv` excludes the program name; the Rust side supplies it, so
    `--help` reads the same from this launcher and from the Rust binary.
    Output is rendered in Rust and then written to `sys.stdout` and
    `sys.stderr`, so it interleaves correctly with anything Python has
    already written. A failure writing it out (a broken pipe) is
    swallowed, matching the binary, which exits 0 on one.

    Arguments are passed through with the filesystem encoding
    (`os.fsencode`), so a `sys.argv` entry that is not valid text reaches
    the command line as the bytes it came from. A path Python decoded
    with surrogate escapes names the same file here that it names for the
    Rust binary.

    Exit codes: 0 found, 1 nothing found, 2 error.

    Raises:
        TypeError: an argument is not a string, bytes, or a path.
    """
    ...

def summarize(doc: Document, n: int = 3, *, method: SummaryMethodName | None = None) -> Summary:
    """The `n` sentences that best represent `doc`, in document order.

    `method` names the ranking; with none, matra's default runs (TF-IDF
    today). Either way the result records the method and `n` in its
    provenance. Each method scores on its own scale.

    Raises:
        InvalidInputError: an unknown `method`.
        InputTooLargeError: the document exceeds the method's sentence
            cap.
    """
    ...

def keyphrases(
    doc: Document, n: int = 10, *, method: KeyphraseMethodName | None = None
) -> Keyphrases:
    """At most `n` phrases that best name what `doc` is about, highest
    score first, each with every place it occurs.

    `method` names the ranking; with none, matra's default runs (RAKE
    today). The result records the method and `n` in its provenance.

    Raises:
        InvalidInputError: an unknown `method`.
        InputTooLargeError: the document exceeds the method's token cap.
    """
    ...

def clusters(doc: Document, embedder: Embedder, threshold: float = 0.85) -> Clusters:
    """`doc`'s sentences grouped by the similarity of their embeddings.

    `embedder` is any object satisfying the `Embedder` protocol: two
    methods, `embed` and `identity`. `Model2Vec` is one of them, and it
    takes a fast path that calls no Python. Your own object is asked for
    its identity once, before anything is embedded.

    Tier 2 output: the clusters reflect the embedder's geometry, and the
    result's provenance records its identity and the threshold. See
    `matra.types.Clusters` for what co-membership does and does not mean.

    Raises:
        InputTooLargeError: more than 2,000 sentences.
        InvalidInputError: the threshold is not finite, or the embedder
            broke its contract (raised, returned something other than a
            sequence of vectors, or returned the wrong number of them).
    """
    ...
