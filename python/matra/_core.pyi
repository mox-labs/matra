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
    def english(model_dir: str) -> Matra:
        """Download (if absent) and load the English UDPipe model.

        The download is atomic and SHA-256-verified against a pinned
        hash. Concurrent processes calling this with the same `model_dir`
        cannot corrupt each other's downloads.

        Raises:
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

    def semantic_clusters(
        self, text: str, threshold: float, model: Model2Vec
    ) -> SemanticClusters:
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
    def from_dir(dir: str) -> Model2Vec:
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
