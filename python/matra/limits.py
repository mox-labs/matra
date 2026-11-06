"""Every cap matra's gates apply and every minimum its metrics need, as
``matra::domain::limits`` publishes them in Rust: the same eleven names,
read from the extension this package was built with, so a value cannot
drift from the one the library enforces.

A failure at a cap carries the cap too, as ``details["limit"]`` on an
``InputTooLargeError``, beside ``details["what"]`` naming the gate. These
constants are for deciding before a call: splitting a corpus below
``MAX_TFIDF_SENTENCES``, or skipping a paragraph too short to grade.

>>> import matra
>>> matra.limits.MAX_INPUT_BYTES
8388608
"""

from typing import Final

from matra import _core

MAX_INPUT_BYTES: Final[int] = _core.MAX_INPUT_BYTES
"""The most text one input may hold, in bytes: 8 MiB. The ``input`` gate,
and the ``file_read`` gate on a file's size before it is read."""

MAX_CONFIG_FILE_BYTES: Final[int] = _core.MAX_CONFIG_FILE_BYTES
"""The largest configuration file read, in bytes: 64 KiB. The
``config_file_read`` gate."""

MAX_DOWNLOAD_BYTES: Final[int] = _core.MAX_DOWNLOAD_BYTES
"""The most one model download may deliver, in bytes: 64 MiB. The
``parser_model_download`` and ``embedding_model_download`` gates."""

MAX_TFIDF_SENTENCES: Final[int] = _core.MAX_TFIDF_SENTENCES
"""The most sentences a TF-IDF summary ranks. The ``tfidf`` gate."""

MAX_TEXTRANK_SENTENCES: Final[int] = _core.MAX_TEXTRANK_SENTENCES
"""The most sentences a TextRank summary ranks. The ``textrank`` gate."""

MAX_CLUSTER_SENTENCES: Final[int] = _core.MAX_CLUSTER_SENTENCES
"""The most sentences ``clusters`` compares, TextRank's cap. The
``clusters`` gate."""

MAX_RAKE_TOKENS: Final[int] = _core.MAX_RAKE_TOKENS
"""The most tokens RAKE reads. The ``rake`` gate."""

MAX_YAKE_TOKENS: Final[int] = _core.MAX_YAKE_TOKENS
"""The most tokens YAKE reads. The ``yake`` gate."""

MAX_COMPRESSION_PARAGRAPH_BYTES: Final[int] = _core.MAX_COMPRESSION_PARAGRAPH_BYTES
"""The largest paragraph whose compression ratio is measured, in bytes;
a larger one's ``compression_ratio`` is ``None``."""

MIN_READABILITY_WORDS: Final[int] = _core.MIN_READABILITY_WORDS
"""The fewest words a paragraph needs for a ``readability_grade``."""

MIN_COMPRESSION_WORDS: Final[int] = _core.MIN_COMPRESSION_WORDS
"""The fewest words a paragraph needs for a ``compression_ratio``."""

__all__ = [
    "MAX_CLUSTER_SENTENCES",
    "MAX_COMPRESSION_PARAGRAPH_BYTES",
    "MAX_CONFIG_FILE_BYTES",
    "MAX_DOWNLOAD_BYTES",
    "MAX_INPUT_BYTES",
    "MAX_RAKE_TOKENS",
    "MAX_TEXTRANK_SENTENCES",
    "MAX_TFIDF_SENTENCES",
    "MAX_YAKE_TOKENS",
    "MIN_COMPRESSION_WORDS",
    "MIN_READABILITY_WORDS",
]
