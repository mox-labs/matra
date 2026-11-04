"""matra: structured parse, text metrics, summarization, and keyphrase extraction.

import matra

engine = matra.Engine()
doc = engine.analyze(file="README.md")
summary = matra.summarize(doc, n=3)
print(doc["metrics"]["total_words"])
"""

# The crust requires the extension built with the `model2vec` feature
# (pyproject.toml pins it for official wheels). A feature-tailored build
# without it fails here by design; the message names the missing feature
# instead of a bare symbol.
try:
    from matra._core import (
        Document,
        Engine,
        InputTooLargeError,
        InvalidInputError,
        IoError,
        IoNotFoundError,
        IoPermissionError,
        MatraError,
        Model2Vec,
        ModelInvalidError,
        ModelNotFoundError,
        ParseFailedError,
        UnsupportedFormatError,
        clusters,
        keyphrases,
        summarize,
    )
except ImportError as _exc:  # pragma: no cover
    raise ImportError(
        "matra's Python package requires the extension to be built with "
        "the 'model2vec' cargo feature (official wheels include it; for "
        "source builds use: maturin develop --features python,udpipe,model2vec)"
    ) from _exc
from matra.spans import span_text
from matra.types import (
    ERROR_KINDS,
    BlockDict,
    Cluster,
    Clusters,
    DocumentDict,
    DocumentError,
    Embedder,
    ErrorDetails,
    ErrorInfo,
    FormatName,
    HeadingBlock,
    Keyphrase,
    KeyphraseMethodName,
    Keyphrases,
    Paragraph,
    ParagraphBlock,
    Parameters,
    Section,
    Sentence,
    SimilarityEdge,
    Skipped,
    Span,
    Summary,
    SummaryMethodName,
    SummarySentence,
    Token,
)

__all__ = [
    "ERROR_KINDS",
    "BlockDict",
    "Cluster",
    "Clusters",
    "Document",
    "DocumentDict",
    "DocumentError",
    "Embedder",
    "Engine",
    "ErrorDetails",
    "ErrorInfo",
    "FormatName",
    "HeadingBlock",
    "InputTooLargeError",
    "InvalidInputError",
    "IoError",
    "IoNotFoundError",
    "IoPermissionError",
    "Keyphrase",
    "KeyphraseMethodName",
    "Keyphrases",
    "MatraError",
    "Model2Vec",
    "ModelInvalidError",
    "ModelNotFoundError",
    "Paragraph",
    "ParagraphBlock",
    "Parameters",
    "ParseFailedError",
    "Section",
    "Sentence",
    "SimilarityEdge",
    "Skipped",
    "Span",
    "Summary",
    "SummaryMethodName",
    "SummarySentence",
    "Token",
    "UnsupportedFormatError",
    "clusters",
    "keyphrases",
    "span_text",
    "summarize",
]
