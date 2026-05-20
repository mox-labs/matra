"""Type stubs for the PyO3 extension module.

These stubs describe the surface exposed by `_core` (built from `src/lib.rs`).
The serialized return types are TypedDicts mirroring the Rust domain types
in `src/domain.rs`. Field names cross the FFI verbatim via `pythonize`.

Stubs are versioned alongside the Rust code; keep them in lockstep with
`#[pyclass]`/`#[pymethods]` signatures.
"""

from typing import TypedDict


class Token(TypedDict):
    """One CoNLL-U token. Mirrors `vaani::domain::Token`."""

    id: int
    text: str
    lemma: str
    pos: str
    xpos: str
    feats: str
    head: int
    dep: str
    deps: str
    misc: str
    is_punct: bool


class Sentence(TypedDict):
    """One parsed sentence. Mirrors `vaani::domain::Sentence`."""

    text: str
    tokens: list[Token]


class Paragraph(TypedDict):
    """One paragraph with optional metric slots. Mirrors `vaani::domain::Paragraph`."""

    text: str
    in_blockquote: bool
    sentences: list[Sentence]
    readability_grade: float | None
    lexical_density: float | None
    compression_ratio: float | None


class Section(TypedDict):
    """One section (heading + paragraphs). Mirrors `vaani::domain::Section`."""

    heading: str | None
    level: int
    paragraphs: list[Paragraph]


class Analysis(TypedDict):
    """Top-level analysis output. Mirrors `vaani::domain::Analysis`.

    Aggregate methods on the Rust `Analysis` (`passive_ratio`,
    `mean_sentence_length`, etc.) do not cross the FFI boundary —
    consumers compute them from `sections` if needed.
    """

    sections: list[Section]
    vocabulary_ttr: float | None
    nominalization_ratio: float | None


class ScoredSentence(TypedDict):
    """One ranked sentence. Output of TF-IDF and TextRank."""

    text: str
    score: float
    position: int


class Keyphrase(TypedDict):
    """One ranked keyphrase. Output of RAKE and YAKE."""

    phrase: str
    score: float


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
