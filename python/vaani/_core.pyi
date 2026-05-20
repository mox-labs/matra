"""Type stubs for the PyO3 extension module.

These stubs describe the surface exposed by `_core` (built from `src/lib.rs`).
The TypedDict return types are imported from `vaani.types`, where they are
defined as runtime modules so they're available at runtime (not just at
type-check time).

Stubs are versioned alongside the Rust code; keep them in lockstep with
`#[pyclass]`/`#[pymethods]` signatures and with `vaani.types`.
"""

from __future__ import annotations

from vaani.types import Analysis, Keyphrase, ScoredSentence


class Vaani:
    """Loaded NLP engine. Create once, reuse across calls.

    The underlying UDPipe model holds C-side state that is not thread-safe;
    the Rust binding is `#[pyclass(unsendable)]`, so cross-thread access
    panics at runtime. Multi-process Python (e.g. `ProcessPoolExecutor`)
    is fine; multi-thread is not.
    """

    @staticmethod
    def from_path(model_path: str) -> Vaani:
        """Load a UDPipe model from a local file.

        Raises:
            FileNotFoundError: the model path does not exist.
            RuntimeError: the model file is corrupt or wrong format.
        """
        ...

    @staticmethod
    def english(model_dir: str) -> Vaani:
        """Download (if absent) and load the English UDPipe model.

        The download is atomic and SHA-256-verified against a pinned
        hash. Concurrent processes calling this with the same `model_dir`
        cannot corrupt each other's downloads.

        Raises:
            RuntimeError: download or load failed.
        """
        ...

    def analyze(self, text: str) -> Analysis:
        """Analyze plain text.

        Raises:
            ValueError: input exceeds the size cap (8 MiB).
            RuntimeError: NLP parsing failed.
        """
        ...

    def analyze_markdown(self, text: str) -> Analysis:
        """Analyze markdown text with section awareness.

        Raises:
            ValueError: input exceeds the size cap (8 MiB).
            RuntimeError: NLP parsing failed.
        """
        ...

    def tfidf_summarize(self, text: str, n: int) -> list[ScoredSentence]:
        """TF-IDF extractive summary. Returns the top-`n` sentences."""
        ...

    def textrank_summarize(self, text: str, n: int) -> list[ScoredSentence]:
        """TextRank extractive summary. Returns the top-`n` sentences."""
        ...

    def rake_keyphrases(self, text: str, max_phrases: int) -> list[Keyphrase]:
        """RAKE keyphrase extraction. Returns up to `max_phrases` ranked phrases."""
        ...

    def yake_keyphrases(self, text: str, max_phrases: int) -> list[Keyphrase]:
        """YAKE keyphrase extraction. Returns up to `max_phrases` ranked phrases."""
        ...
