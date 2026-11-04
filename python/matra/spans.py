"""Reading a span against the text it points into."""

from __future__ import annotations

from matra.types import Span


def span_text(source: str | bytes, span: Span) -> str:
    """The stretch of ``source`` that ``span`` covers.

    A span counts UTF-8 bytes, not characters, so slicing a ``str`` by it
    directly goes wrong as soon as the text holds a character outside
    ASCII. ``source`` is the analysed text: the input itself for markdown
    and plain text, and the blocks' texts joined by a blank line for block
    input.

    Raises:
        UnicodeDecodeError: the span does not fall on character
            boundaries of ``source``, so it was not taken from it.
    """
    data = source.encode() if isinstance(source, str) else source
    return data[span["start"] : span["end"]].decode()


__all__ = ["span_text"]
