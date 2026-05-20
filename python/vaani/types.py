"""Runtime-available TypedDict shapes for vaani's domain types.

These mirror the Rust types in `src/domain.rs` and the wire shapes
produced by `pythonize`. Defined here (not in `_core.pyi`) so they
can be imported at runtime — TypedDicts in a stub file would only
exist at type-check time.

Keep in lockstep with `python/vaani/_core.pyi`.
"""

from __future__ import annotations

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


__all__ = [
    "Token",
    "Sentence",
    "Paragraph",
    "Section",
    "Analysis",
    "ScoredSentence",
    "Keyphrase",
]
