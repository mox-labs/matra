"""Type stubs for the PyO3 extension module.

These stubs describe the surface exposed by `_core` (built from `src/lib.rs`).
The TypedDict return types are imported from `matra.types`, where they are
defined as runtime modules so they're available at runtime (not just at
type-check time).

Stubs are versioned alongside the Rust code; keep them in lockstep with
`#[pyclass]`/`#[pymethods]` signatures and with `matra.types`.
"""

from __future__ import annotations

from matra.types import Document, Keyphrase, ScoredSentence, SemanticClusters

class Matra:
    """Loaded NLP engine. Create once, reuse across calls.

    The underlying UDPipe model holds C-side state that is not thread-safe;
    the Rust binding is `#[pyclass(unsendable)]`, so cross-thread access
    panics at runtime. Multi-process Python (e.g. `ProcessPoolExecutor`)
    is fine; multi-thread is not.
    """

    @staticmethod
    def from_path(model_path: str) -> Matra:
        """Load a UDPipe model from a local file.

        Raises:
            FileNotFoundError: the model path does not exist.
            RuntimeError: the model file is corrupt or wrong format.
        """
        ...

    @staticmethod
    def english(model_dir: str | None = None) -> Matra:
        """Download (if absent) and load the English UDPipe model.

        With no argument the directory is resolved the way every matra
        surface resolves it: `MATRA_MODEL_DIR`, else the `models`
        subdirectory of the data root (`MATRA_DATA_DIR`, else
        `$XDG_DATA_HOME/matra`, else `~/.local/share/matra`), except that
        a pre-existing `~/.matra/models` wins when the new location does
        not exist yet.

        The download is atomic and SHA-256-verified against a pinned
        hash. Concurrent processes calling this with the same `model_dir`
        cannot corrupt each other's downloads.

        Raises:
            ValueError: the config file is malformed, or the environment
                names no home directory at all.
            OSError: the config file could not be read.
            RuntimeError: download or load failed.
        """
        ...

    def analyze(self, text: str) -> Document:
        """Analyze plain text.

        Raises:
            ValueError: input exceeds the size cap (8 MiB).
            RuntimeError: NLP parsing failed.
        """
        ...

    def analyze_markdown(self, text: str) -> Document:
        """Analyze markdown text with section awareness.

        Raises:
            ValueError: input exceeds the size cap (8 MiB).
            RuntimeError: NLP parsing failed.
        """
        ...

    def tfidf_summarize(self, text: str, n: int) -> list[ScoredSentence]:
        """TF-IDF extractive summary. Returns the top-`n` sentences.

        Raises:
            ValueError: input exceeds the size cap (8 MiB) or the
                per-algorithm sentence cap.
            RuntimeError: NLP parsing failed.
        """
        ...

    def textrank_summarize(self, text: str, n: int) -> list[ScoredSentence]:
        """TextRank extractive summary. Returns the top-`n` sentences.

        Raises:
            ValueError: input exceeds the size cap (8 MiB) or the
                per-algorithm sentence cap.
            RuntimeError: NLP parsing failed.
        """
        ...

    def rake_keyphrases(self, text: str, max_phrases: int) -> list[Keyphrase]:
        """RAKE keyphrase extraction. Returns up to `max_phrases` ranked phrases.

        Raises:
            ValueError: input exceeds the size cap (8 MiB) or the
                per-algorithm token cap.
            RuntimeError: NLP parsing failed.
        """
        ...

    def yake_keyphrases(self, text: str, max_phrases: int) -> list[Keyphrase]:
        """YAKE keyphrase extraction. Returns up to `max_phrases` ranked phrases.

        Raises:
            ValueError: input exceeds the size cap (8 MiB) or the
                per-algorithm token cap.
            RuntimeError: NLP parsing failed.
        """
        ...

    def semantic_clusters(self, text: str, threshold: float, model: Model2Vec) -> SemanticClusters:
        """Parse plain text, embed its sentences, cluster at `threshold`.

        Tier 2 output: the clusters reflect `model`'s geometry, and the
        result carries its identity. See `matra.types.SemanticClusters`
        for what co-membership does and does not mean.

        Raises:
            ValueError: input exceeds the size cap (8 MiB), the sentence
                cap (2,000), or the threshold is not finite.
            RuntimeError: NLP parsing or embedding failed.
        """
        ...

class Model2Vec:
    """A loaded static embedding model (model2vec artifact format).

    Tier 2: its vectors are model opinion; everything derived from them
    carries this model's identity.
    """

    @staticmethod
    def from_dir(dir: str) -> Model2Vec:  # noqa: A002 - the published keyword name
        """Load from a directory holding model.safetensors,
        tokenizer.json, and config.json. No network is touched.

        Raises:
            FileNotFoundError: an artifact file is absent.
            RuntimeError: the artifact does not parse or is malformed.
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

def cli_main(argv: list[str]) -> int:
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

def semantic_clusters(
    embeddings: list[list[float]], threshold: float, model_hash: str
) -> SemanticClusters:
    """Cluster caller-supplied embedding vectors at `threshold`.

    The vectors-in twin of `Matra.semantic_clusters` for consumers who
    already hold embeddings; indices in the result are positions in
    `embeddings`, and the scores are attributed to `model_hash`.

    Raises:
        ValueError: vectors disagree on dimension, contain a non-finite
            value, exceed the 2,000-vector cap, or the threshold is not
            finite.
    """
    ...
