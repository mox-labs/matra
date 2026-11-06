"""Runtime-available type shapes for matra's domain types.

These mirror the Rust types in `src/domain.rs` and the wire shapes
produced by `pythonize`. Defined here (not in `_core.pyi`) so they
can be imported at runtime — TypedDicts and protocols in a stub file
would only exist at type-check time.

Keep in lockstep with `python/matra/_core.pyi`.
"""

from __future__ import annotations

from typing import Final, Literal, NotRequired, Protocol, TypedDict

ERROR_KINDS: Final[tuple[str, ...]] = (
    "model_not_found",
    "model_invalid",
    "parse_failed",
    "input_too_large",
    "unsupported_format",
    "invalid_input",
    "io",
)
"""Every value ``ErrorInfo["kind"]`` and ``MatraError.kind`` can take, in
the order the Rust enum declares them.

Mirrors ``matra::domain::Error::kind``, the one place the vocabulary is
written on the Rust side. ``spec/tests/corpus/items.json`` pins the list
and both crusts assert against it, so a kind added on one side and not
the other fails the corpus conformance lane.
"""


class Span(TypedDict):
    """A stretch of the analysed text. Mirrors `matra::domain::Span`.

    Counts UTF-8 bytes, ``byte_start`` inclusive and ``byte_end``
    exclusive, not characters:
    ``text.encode()[span["byte_start"]:span["byte_end"]].decode()``, or
    ``matra.span_text(text, span)``, is the stretch. Slicing the ``str``
    itself by these numbers is right on ASCII and silently wrong after the
    first character outside it. For markdown and plain text the analysed
    text is the input; for block input it is the blocks' texts joined by a
    blank line.
    """

    byte_start: int
    byte_end: int


class Token(TypedDict):
    """One CoNLL-U token. Mirrors `matra::domain::Token`.

    ``features`` is CoNLL-U column 6 as a map, ``{"Mood": "Ind"}``; a
    multi-valued feature keeps its value unsplit (``"Nom,Acc"``).
    ``span`` is where the token's form lies in the analysed text.
    """

    id: int
    span: Span
    text: str
    lemma: str
    pos: str
    xpos: str
    features: dict[str, str]
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


class ClausalComplement(TypedDict):
    """One clausal complement. Mirrors `matra::domain::ClausalComplement`.

    Reports structure only: a verb governing a clausal complement
    (`ccomp`), plus its subject when the parse has one in the same
    sentence. Fires for every verb that fills the construction; which
    verb lemmas count as reporting or evidential is the consumer's
    lexicon, and whether the source is credible is the consumer's
    reading. The subject is absent when upstream sentence segmentation
    strands the attribution in a previous sentence ("Smith et al.
    reported ..." splits at the period in "et al.").
    """

    verb_id: int
    verb_lemma: str
    ccomp_id: int
    subject_id: int | None
    subject_lemma: str | None


class RootMood(TypedDict):
    """The root clause's mood. Mirrors `matra::domain::RootMood`.

    ``mood`` is the ``Mood`` feature read off the root, or off its first
    ``cop``, ``aux`` or ``aux:pass`` child when the root carries none.
    The values are an open set (``Ind``, ``Imp``, ...).
    """

    root_id: int
    mood: str


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
    """One parsed sentence. Mirrors `matra::domain::Sentence`.

    ``tree_depth`` is ``None`` when the parse's head references loop.
    ``text`` is normalized; ``span`` locates the sentence in the analysed
    text.
    """

    span: Span
    text: str
    tokens: list[Token]
    word_count: int
    tree_depth: int | None
    passive: bool
    negations: list[Negation]
    modals: list[Modal]
    clausal_complements: list[ClausalComplement]
    root_adverbials: list[RootAdverbial]
    hearst_pairs: list[HearstPair]
    root_mood: RootMood | None


class ParagraphMetrics(TypedDict):
    """A paragraph's metrics. Mirrors `matra::domain::ParagraphMetrics`.

    Each slot is ``None`` when the paragraph did not meet its condition.
    """

    readability_grade: float | None
    lexical_density: float | None
    compression_ratio: float | None


class Paragraph(TypedDict):
    """One paragraph. Mirrors `matra::domain::Paragraph`.

    ``kind`` is ``paragraph``, ``quote`` or ``list_item`` today, an open
    set; a quote is never parsed or measured. ``text`` is normalized (a
    markdown paragraph's inline markup removed); ``span`` locates the
    paragraph in the analysed text.
    """

    span: Span
    text: str
    kind: str
    sentences: list[Sentence]
    metrics: ParagraphMetrics


class Section(TypedDict):
    """One section (heading + paragraphs). Mirrors `matra::domain::Section`.

    ``span`` is the heading's, ``None`` with no heading.
    """

    heading: str | None
    level: int
    span: Span | None
    paragraphs: list[Paragraph]


class Skipped(TypedDict):
    """A construct the reader saw and did not analyse. Mirrors `matra::domain::Skipped`.

    ``kind`` is ``table``, ``code``, ``front_matter``, ``html``,
    ``thematic_break`` or ``link_definition`` today, an open set.
    """

    kind: str
    span: Span


class DocumentMetrics(TypedDict):
    """A document's metrics. Mirrors `matra::domain::DocumentMetrics`.

    The counts are always present. Each ratio is ``None`` when its
    condition was not met: a mean and the passive ratio need a sentence,
    the sample deviation two, the type-token and nominalization ratios a
    word. ``vocabulary_ttr`` has no length correction, so documents of
    different lengths are not comparable on it.
    """

    total_words: int
    total_sentences: int
    paragraph_count: int
    mean_sentence_length: float | None
    sentence_length_std: float | None
    vocabulary_ttr: float | None
    nominalization_ratio: float | None
    passive_ratio: float | None


class Identity(TypedDict):
    """Who produced a result. Mirrors `matra::domain::Identity`.

    ``model``, ``sha256`` and ``license`` are ``None`` when not known,
    as for a model matra does not pin.
    """

    name: str
    model: str | None
    sha256: str | None
    license: str | None


class ProvenanceSource(TypedDict):
    """The analysed input. Mirrors `matra::domain::ProvenanceSource`.

    ``bytes`` and ``sha256`` describe the analysed text every span points
    into.
    """

    path: str | None
    bytes: int
    sha256: str


class Parameters(TypedDict, total=False):
    """The parameters that shaped a derived result, as it ran. Mirrors
    `matra::domain::Parameters`.

    Only the parameters the method takes are present: ``n`` for a
    summary and keyphrases, ``threshold`` and ``embedder`` for clusters.
    """

    n: int
    threshold: float
    embedder: Identity


class Provenance(TypedDict):
    """Where a result came from. Mirrors `matra::domain::Provenance`.

    ``schema`` is the schema version, ``"major.minor"``, and the one
    place it appears; reject a major you do not know. ``format`` is one
    of ``markdown``, ``plain_text`` and ``blocks`` today, an open set.

    A document's provenance has no ``method`` and no ``parameters``. A
    result derived from a document (``Summary``, ``Keyphrases``,
    ``Clusters``) carries the document's provenance with both added:
    ``method`` names what ran (``tfidf``, ``textrank``, ``rake``,
    ``yake``, ``cosine`` today, an open set).
    """

    matra: str
    schema: str
    parser: Identity
    format: str
    source: ProvenanceSource
    method: NotRequired[str]
    parameters: NotRequired[Parameters]


class DocumentDict(TypedDict):
    """A document as the schema's JSON shape: what ``Document.to_dict()``
    returns, and what ``doc[key]`` reads one key of. Mirrors
    `matra::domain::Document`.

    The document's aggregates arrive in ``metrics``, materialized by the
    metric suite, because methods do not cross the FFI boundary.
    ``skipped`` lists what the reader saw and did not analyse.
    """

    provenance: Provenance
    sections: list[Section]
    metrics: DocumentMetrics
    skipped: list[Skipped]


class SummarySentence(TypedDict):
    """One sentence a summary keeps. Mirrors `matra::domain::SummarySentence`.

    ``index`` is its position among the document's sentences, counting
    from 0; ``span`` is where it lies in the analysed text.
    """

    index: int
    span: Span
    text: str
    score: float


class Summary(TypedDict):
    """The sentences that best represent a document. Mirrors
    `matra::domain::Summary`.

    ``provenance`` is the document's, with ``method`` and
    ``parameters["n"]``. Each method scores on its own scale.
    """

    provenance: Provenance
    sentences: list[SummarySentence]


class Keyphrase(TypedDict):
    """One ranked keyphrase and every place it occurs. Mirrors
    `matra::domain::Keyphrase`."""

    phrase: str
    score: float
    occurrences: list[Span]


class Keyphrases(TypedDict):
    """The phrases that best name what a document is about, highest score
    first. Mirrors `matra::domain::Keyphrases`.

    ``provenance`` is the document's, with ``method`` and
    ``parameters["n"]``. Each method scores on its own scale.
    """

    provenance: Provenance
    phrases: list[Keyphrase]


class SimilarityEdge(TypedDict):
    """A pair of sentences whose similarity cleared the threshold,
    ``a < b``. Mirrors `matra::domain::SimilarityEdge`.

    The score is cosine similarity in the embedder's geometry.
    """

    a: int
    b: int
    score: float


class Cluster(TypedDict):
    """One connected component of the above-threshold similarity graph.
    Mirrors `matra::domain::Cluster`.

    Co-membership is transitive: two members can share a cluster without
    sharing an edge, and must never be read as pairwise similar.
    """

    members: list[int]


class Clusters(TypedDict):
    """Sentences grouped by embedding similarity: model opinion, standing
    alone. Mirrors `matra::domain::Clusters`.

    Never attached to ``Document`` (RFC-0010). ``provenance`` is the
    document's, with ``method`` and the ``threshold`` and ``embedder``
    parameters. ``edges`` lists every pair that cleared the threshold. A
    sentence with no edge appears in no cluster.
    """

    provenance: Provenance
    clusters: list[Cluster]
    edges: list[SimilarityEdge]


type ErrorDetails = dict[str, str | int | bool]
"""Every dynamic value in a failure's message, by name. Mirrors
``matra::domain::ErrorDetails``.

The keys per kind: ``path`` for ``model_not_found``; ``reason`` for
``model_invalid``, ``parse_failed`` and ``invalid_input``; ``what``,
``limit`` and ``actual`` for ``input_too_large``; ``format`` (when one was
recognised) and ``feature`` (when one would read it) for
``unsupported_format``; and for ``io``, ``io_kind`` and ``reason``, with
``path`` when the failure concerns a path matra named, and ``url`` and
``retryable`` when a download did not arrive. A key that does not apply
is absent. A key a kind carries keeps appearing in later versions, and
new keys may be added, so read the ones you know and ignore the rest.
"""


class ErrorInfo(TypedDict):
    """One failure as data. Mirrors the serde form of
    ``matra::domain::Error``, and what ``MatraError`` carries.

    ``kind`` is a stable string naming the variant; ``ERROR_KINDS`` holds
    the whole vocabulary. Branch on it. ``message`` is the Rust error's
    own text, meant for a human and not a contract. ``details`` holds
    every dynamic value in the message, so nothing has to be parsed out
    of it.
    """

    kind: str
    message: str
    details: ErrorDetails


class DocumentError(ErrorInfo):
    """One document that failed. Mirrors the serde form of
    ``matra::domain::DocumentError``: the error's own keys plus ``path``.

    A failure reading or parsing one input does not end
    ``Engine.analyze_all``: it arrives as one of these, in the position
    the document would have held. Tell the two apart with
    ``isinstance(item, matra.Document)``. ``path`` is ``None`` only for
    input that never came from disk.
    """

    path: str | None


class HeadingBlock(TypedDict):
    """A heading a caller supplies, opening a section at ``level`` (1 or
    more). The ``blocks`` input format; see ``BlockDict``."""

    kind: Literal["heading"]
    level: int
    text: str


class ParagraphBlock(TypedDict):
    """A paragraph of prose a caller supplies. The ``blocks`` input
    format; see ``BlockDict``."""

    kind: Literal["paragraph"]
    text: str


type BlockDict = HeadingBlock | ParagraphBlock
"""One block of ``Engine.analyze(blocks=...)``: the JSON form
``spec/schema/v2/blocks.schema.json`` publishes, as Python data. A
``kind`` or a key matra does not know is refused by name with
``InvalidInputError``. The analysed text is the blocks' texts joined by a
blank line, in order, and every span in the result points into it.
"""

type FormatName = Literal["markdown", "plain_text", "blocks"]
"""A format matra reads, in the vocabulary every surface shares."""

type SummaryMethodName = Literal["tfidf", "textrank"]
"""A summary method, as ``summarize(method=...)`` names it and
``provenance["method"]`` records it."""

type KeyphraseMethodName = Literal["rake", "yake"]
"""A keyphrase method, as ``keyphrases(method=...)`` names it and
``provenance["method"]`` records it."""


class Embedder(Protocol):
    """Any object matra will accept where an embedding model is asked for.

    Mirrors the Rust ``matra::embed::Embedder`` port, and carries the
    same contract: ``embed`` returns exactly one vector per input text,
    in input order, every vector the same length. Violating it raises
    ``InvalidInputError`` (a ``ValueError``) from ``clusters``, with the
    same message a Rust implementor would get.

    ``identity`` says who produces the vectors, in the shape the parser
    reports itself (``Identity``): a ``name``, and the ``model``, its
    ``sha256`` and its ``license``, each ``None`` when not known. It travels into every result derived from the
    vectors as ``provenance["parameters"]["embedder"]``, so scores stay
    attributable and the model's licence travels with them. It is read
    once, when the object is handed over. Two embedders that can disagree
    must not report the same identity.

    ``Model2Vec`` satisfies this protocol; so does any class you write.
    """

    def embed(self, texts: list[str]) -> list[list[float]]:
        """Embed each text into a vector: one per text, in order."""
        ...

    def identity(self) -> Identity:
        """Who produces these vectors: the component and its model."""
        ...


__all__ = [
    "ERROR_KINDS",
    "BlockDict",
    "ClausalComplement",
    "Cluster",
    "Clusters",
    "DocumentDict",
    "DocumentError",
    "DocumentMetrics",
    "Embedder",
    "ErrorDetails",
    "ErrorInfo",
    "FormatName",
    "HeadingBlock",
    "Identity",
    "Keyphrase",
    "KeyphraseMethodName",
    "Keyphrases",
    "Modal",
    "Negation",
    "Paragraph",
    "ParagraphBlock",
    "ParagraphMetrics",
    "Parameters",
    "Provenance",
    "ProvenanceSource",
    "RootAdverbial",
    "RootMood",
    "Section",
    "Sentence",
    "SimilarityEdge",
    "Skipped",
    "Span",
    "Summary",
    "SummaryMethodName",
    "SummarySentence",
    "Token",
]
