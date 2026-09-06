"""matra: structured parse, text metrics, summarization, and keyphrase extraction."""

# The crust requires the extension built with the `model2vec` feature
# (pyproject.toml pins it for official wheels). A feature-tailored build
# without it fails here by design; the message names the missing feature
# instead of a bare symbol.
try:
    from matra._core import Matra, Model2Vec, semantic_clusters
except ImportError as _exc:  # pragma: no cover
    raise ImportError(
        "matra's Python package requires the extension to be built with "
        "the 'model2vec' cargo feature (official wheels include it; for "
        "source builds use: maturin develop --features python,udpipe,model2vec)"
    ) from _exc
from matra.types import (
    ERROR_KINDS,
    CorpusEntry,
    CorpusItem,
    Document,
    DocumentError,
    Embedder,
    ErrorInfo,
    Keyphrase,
    Paragraph,
    ScoredSentence,
    Section,
    SemanticCluster,
    SemanticClusters,
    SemanticEdge,
    Sentence,
    Token,
)

__all__ = [
    "ERROR_KINDS",
    "CorpusEntry",
    "CorpusItem",
    "Document",
    "DocumentError",
    "Embedder",
    "ErrorInfo",
    "Keyphrase",
    "Matra",
    "Model2Vec",
    "Paragraph",
    "ScoredSentence",
    "Section",
    "SemanticCluster",
    "SemanticClusters",
    "SemanticEdge",
    "Sentence",
    "Token",
    "semantic_clusters",
]
