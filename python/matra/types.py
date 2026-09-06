"""Runtime-available type shapes for matra's domain types.

These mirror the Rust types in `src/domain.rs` and the wire shapes
produced by `pythonize`. Defined here (not in `_core.pyi`) so they
can be imported at runtime — TypedDicts and protocols in a stub file
would only exist at type-check time.

Keep in lockstep with `python/matra/_core.pyi`.
"""

from __future__ import annotations

from typing import Literal, Protocol, TypedDict


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


class Reporting(TypedDict):
    """One reporting construction. Mirrors `matra::domain::Reporting`.

    Reports structure only: a verb governing a clausal complement
    (`ccomp`), plus its subject when the parse has one in the same
    sentence. Fires for every verb that fills the construction; which
    verb lemmas count as evidential is the consumer's lexicon, and
    whether the source is credible is the consumer's reading. The
    subject is absent when upstream sentence segmentation strands the
    attribution in a previous sentence ("Smith et al. reported ..."
    splits at the period in "et al.").
    """

    verb_id: int
    verb_lemma: str
    ccomp_id: int
    subject_id: int | None
    subject_lemma: str | None


class RootAdverbial(TypedDict):
    """One root-attached adverbial. Mirrors `matra::domain::RootAdverbial`.

    Reports structure only: the `advmod` arc into the root, where
    sentence-scope adverbs land. The parse does not distinguish
    sentence scope from manner, so every root-attached adverbial is
    reported and the consumer's lexicon selects the evidential ones.
    """

    adv_id: int
    adv_lemma: str


HearstPattern = Literal[
    "such_as",
    "such_np_as",
    "including",
    "especially",
    "and_other",
    "or_other",
]
"""Which Hearst (1992) construction matched. Mirrors
`matra::domain::HearstPattern` (serde `snake_case` tags).

Each tag names a surface construction, not a semantic verdict; whether
the hypernymy relation actually holds is the consumer's reading.
"""


class HearstSpan(TypedDict):
    """One noun phrase in a Hearst pair. Mirrors `matra::domain::HearstSpan`.

    `head_id` is the syntactic head noun; `first_id..last_id` is the
    contiguous token range of that noun plus its adjacent nominal
    modifiers, with the pattern's own marker words (`such`, `other`)
    excluded. Ids are sentence-scoped token ids.
    """

    head_id: int
    head_lemma: str
    first_id: int
    last_id: int


class HearstPair(TypedDict):
    """One candidate hypernymy pair. Mirrors `matra::domain::HearstPair`.

    Reports the two spans and the construction that connected them. It
    is a candidate by design: matra does not build a taxonomy or assert
    the relation is true, it reports that the sentence used a
    construction which conventionally signals one.
    """

    pattern: HearstPattern
    hypernym: HearstSpan
    hyponym: HearstSpan


class Sentence(TypedDict):
    """One parsed sentence. Mirrors `matra::domain::Sentence`."""

    text: str
    tokens: list[Token]
    negations: list[Negation]
    modals: list[Modal]
    bare_assertion: bool
    reportings: list[Reporting]
    root_adverbials: list[RootAdverbial]
    hearst_pairs: list[HearstPair]


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


class SemanticEdge(TypedDict):
    """An above-threshold similarity between two sentences, ``a < b``.

    Mirrors ``matra::domain::SemanticEdge``. The score is cosine
    similarity in the producing model's geometry.
    """

    a: int
    b: int
    score: float


class SemanticCluster(TypedDict):
    """One connected component of the above-threshold similarity graph.

    Mirrors ``matra::domain::SemanticCluster``. Co-membership is
    transitive: two members can share a cluster without sharing an
    edge, and must never be read as pairwise similar. The edges list
    every pair that actually cleared the threshold.
    """

    members: list[int]
    edges: list[SemanticEdge]


class SemanticClusters(TypedDict):
    """Semantic-similarity clusters: Tier 2 output, standing alone.

    Mirrors ``matra::domain::SemanticClusters``. Never attached to
    ``Document`` (ADR-0010); carries the producing model's identity and
    the caller-supplied threshold as provenance. A sentence with no
    above-threshold edge appears in no cluster.
    """

    model_hash: str
    threshold: float
    clusters: list[SemanticCluster]


class CorpusEntry(TypedDict):
    """One analyzed document from a path. Mirrors ``matra::domain::CorpusEntry``.

    ``path`` is the file the document was read from, or ``None`` for
    text that never came from disk. ``analysis`` is the same
    ``Document`` shape ``analyze`` returns.
    """

    path: str | None
    analysis: Document


class ErrorInfo(TypedDict):
    """One failure, projected for consumers. Mirrors ``matra::domain::Error``.

    ``kind`` is a stable string naming the variant (``model_not_found``,
    ``model_invalid``, ``parse_failed``, ``input_too_large``,
    ``unsupported_format``, ``invalid_input``, ``io``). Branch on it;
    ``message`` is the Rust error's own text, meant for a human and not
    a contract.
    """

    kind: str
    message: str


class DocumentError(TypedDict):
    """One document that failed. Mirrors ``matra::domain::DocumentError``.

    A failure reading or parsing one file does not abort a directory
    walk: it arrives as one of these, in the position the document would
    have held. ``path`` is ``None`` only for input that never came from
    disk.
    """

    path: str | None
    error: ErrorInfo


type CorpusItem = CorpusEntry | DocumentError
"""One item from ``Matra.analyze_path``: a document that analyzed, or one
that failed.

The two are told apart by their keys, and a type checker narrows the
union on that test: ``"error" in item`` selects the failure,
``"analysis" in item`` the success.
"""


class Embedder(Protocol):
    """Any object matra will accept where an embedding model is asked for.

    Mirrors the Rust ``matra::embed::Embedder`` port, and carries the
    same contract: ``embed`` returns exactly one vector per input text,
    in input order, every vector the same length. Violating it raises
    ``ValueError`` from the call that used the embedder, with the same
    message a Rust implementor would get.

    ``identity`` names the geometry the vectors live in, and travels
    into every result derived from them so scores stay attributable. It
    is read once, when the object is handed over. Two embedders that can
    disagree must not return the same string.

    ``Model2Vec`` satisfies this protocol; so does any class you write.
    """

    def embed(self, texts: list[str]) -> list[list[float]]:
        """Embed each text into a vector: one per text, in order."""
        ...

    def identity(self) -> str:
        """A stable identifier for the model behind this embedder."""
        ...


__all__ = [
    "CorpusEntry",
    "CorpusItem",
    "Document",
    "DocumentError",
    "Embedder",
    "ErrorInfo",
    "Keyphrase",
    "Modal",
    "Negation",
    "Paragraph",
    "Reporting",
    "RootAdverbial",
    "ScoredSentence",
    "Section",
    "SemanticCluster",
    "SemanticClusters",
    "SemanticEdge",
    "Sentence",
    "Token",
]
