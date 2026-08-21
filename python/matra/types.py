"""Runtime-available TypedDict shapes for matra's domain types.

These mirror the Rust types in `src/domain.rs` and the wire shapes
produced by `pythonize`. Defined here (not in `_core.pyi`) so they
can be imported at runtime — TypedDicts in a stub file would only
exist at type-check time.

Keep in lockstep with `python/matra/_core.pyi`.
"""

from __future__ import annotations

from typing import TypedDict


class Token(TypedDict):
    """One CoNLL-U token. Mirrors `matra::domain::Token`."""

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


class Negation(TypedDict):
    """One negation cue, referenced by token id. Mirrors `matra::domain::Negation`.

    Reports structure only: which token carries the cue, its lemma,
    and the head it attaches to. What the negation means is the
    consumer's reading.
    """

    cue_id: int
    cue_lemma: str
    head_id: int


class Modal(TypedDict):
    """One modal auxiliary, referenced by token id. Mirrors `matra::domain::Modal`.

    Reports structure only: which token carries the modal, its lemma
    (closed class: can, could, may, might, must, ought, shall, should,
    will, would), and the head it attaches to. The epistemic, deontic
    or dynamic reading is the consumer's, not matra's.
    """

    aux_id: int
    aux_lemma: str
    head_id: int


class Sentence(TypedDict):
    """One parsed sentence. Mirrors `matra::domain::Sentence`."""

    text: str
    tokens: list[Token]
    negations: list[Negation]
    modals: list[Modal]
    bare_assertion: bool


class Paragraph(TypedDict):
    """One paragraph with optional metric slots. Mirrors `matra::domain::Paragraph`."""

    text: str
    in_blockquote: bool
    sentences: list[Sentence]
    readability_grade: float | None
    lexical_density: float | None
    compression_ratio: float | None


class Section(TypedDict):
    """One section (heading + paragraphs). Mirrors `matra::domain::Section`."""

    heading: str | None
    level: int
    paragraphs: list[Paragraph]


class Document(TypedDict):
    """Top-level analysis output. Mirrors `matra::domain::Document`.

    Document-level aggregates that cross the FFI boundary do so as
    fields filled by the metric suite (ADR-0008): `passive_ratio`
    arrives materialized, like `vocabulary_ttr`. Remaining aggregate
    methods on the Rust `Document` (`mean_sentence_length`,
    `total_words`, etc.) do not cross; consumers compute them from
    `sections` if needed.
    """

    sections: list[Section]
    vocabulary_ttr: float | None
    nominalization_ratio: float | None
    passive_ratio: float | None


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
    "Document",
    "Keyphrase",
    "Modal",
    "Negation",
    "Paragraph",
    "ScoredSentence",
    "Section",
    "Sentence",
    "Token",
]
