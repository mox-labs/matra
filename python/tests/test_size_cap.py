"""The input size cap applies to every method that reaches the parser.

Regression test. Four extraction methods on the Python class called
``nlp.parse`` directly with no size gate, while ``analyze`` and
``analyze_markdown`` enforced it. So ``analyze(huge)`` raised while
``tfidf_summarize(huge, 3)`` handed unbounded text to UDPipe.

``MAX_INPUT_BYTES`` is the bound past which UDPipe's per-token allocations
cross roughly a gigabyte resident, so the gap was a memory-exhaustion path
reachable from Python and not from Rust.

The cap is a property of the parser, not of any one entry point. Every method
that takes text and reaches the parser enforces it, or the bound is not a
bound.

Requires the UDPipe model:

    uv run pytest python/tests/test_size_cap.py
"""

from __future__ import annotations

import os
from pathlib import Path

import pytest

Matra = pytest.importorskip("matra", reason="matra wheel not built").Matra

# Mirrors domain::MAX_INPUT_BYTES. If the Rust constant moves, this test
# starts failing loudly rather than silently passing on a smaller input.
MAX_INPUT_BYTES = 8 * 1024 * 1024


def _model_dir() -> str:
    return os.environ.get("MATRA_MODEL_DIR") or str(Path.home() / ".matra" / "models")


@pytest.fixture(scope="module")
def engine() -> Matra:
    try:
        return Matra.english(_model_dir())
    except Exception as exc:  # noqa: BLE001 - the model is an external artifact
        pytest.skip(f"UDPipe model unavailable: {exc}")


@pytest.fixture(scope="module")
def oversized() -> str:
    """One byte past the cap. Cheap to build, never reaches the parser."""
    return "a" * (MAX_INPUT_BYTES + 1)


@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda m, t: m.analyze(t), id="analyze"),
        pytest.param(lambda m, t: m.analyze_markdown(t), id="analyze_markdown"),
        pytest.param(lambda m, t: m.tfidf_summarize(t, 3), id="tfidf_summarize"),
        pytest.param(lambda m, t: m.textrank_summarize(t, 3), id="textrank_summarize"),
        pytest.param(lambda m, t: m.rake_keyphrases(t, 10), id="rake_keyphrases"),
        pytest.param(lambda m, t: m.yake_keyphrases(t, 10), id="yake_keyphrases"),
    ],
)
def test_every_text_method_enforces_the_cap(engine, oversized, call) -> None:
    """No method that takes text may reach the parser with oversized input.

    ``Error::InputTooLarge`` routes to ``ValueError`` through the exhaustive
    PyErr mapping, so the assertion is on the Python exception class a caller
    would actually write ``except`` for.
    """
    with pytest.raises(ValueError):
        call(engine, oversized)
