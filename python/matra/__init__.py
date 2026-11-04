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
    Cluster,
    Clusters,
    CorpusItem,
    Document,
    DocumentError,
    Embedder,
    ErrorInfo,
    Keyphrase,
    Keyphrases,
    Paragraph,
    Parameters,
    Section,
    Sentence,
    SimilarityEdge,
    Skipped,
    Span,
    Summary,
    SummarySentence,
    Token,
    VectorClusters,
)

__all__ = [
    "ERROR_KINDS",
    "Cluster",
    "Clusters",
    "CorpusItem",
    "Document",
    "DocumentError",
    "Embedder",
    "ErrorInfo",
    "Keyphrase",
    "Keyphrases",
    "Matra",
    "Model2Vec",
    "Paragraph",
    "Parameters",
    "Section",
    "Sentence",
    "SimilarityEdge",
    "Skipped",
    "Span",
    "Summary",
    "SummarySentence",
    "Token",
    "VectorClusters",
    "semantic_clusters",
]
