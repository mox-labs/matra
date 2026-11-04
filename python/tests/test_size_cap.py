"""The input size cap applies to every input that reaches the parser.

Regression test. Before 0.3.0 four extraction methods on the Python class
called the parser directly with no size gate, while ``analyze`` enforced
it, so a summary of an unbounded text handed it to UDPipe. Since 0.3.0 the
outcome functions take a ``Document`` and never text, and every input
keyword of ``Engine.analyze`` reaches the parser through the one route
that applies the cap.

``MAX_INPUT_BYTES`` is the bound past which UDPipe's per-token allocations
cross roughly a gigabyte resident, so a gap would be a memory-exhaustion
path reachable from Python and not from Rust.

Requires the UDPipe model:

    uv run pytest python/tests/test_size_cap.py
"""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

matra = pytest.importorskip("matra", reason="matra wheel not built")

# Mirrors domain::limits::MAX_INPUT_BYTES. If the Rust constant moves, this
# test starts failing loudly rather than silently passing on a smaller input.
MAX_INPUT_BYTES = 8 * 1024 * 1024
OVERSIZED = "a" * (MAX_INPUT_BYTES + 1)
GATE = {"what": "input", "limit": MAX_INPUT_BYTES, "actual": MAX_INPUT_BYTES + 1}


def _file(tmp_path: Path) -> Path:
    path = tmp_path / "big.txt"
    path.write_text(OVERSIZED)
    return path


# Each call and the gate that stops it: the file source checks a file's
# size before reading it, under a name of its own.
CALLS: list[Any] = [
    pytest.param(lambda e, _: e.analyze(OVERSIZED), "input", id="text"),
    pytest.param(lambda e, _: e.analyze(OVERSIZED, format="markdown"), "input", id="text-markdown"),
    pytest.param(lambda e, _: e.analyze(bytes=OVERSIZED.encode()), "input", id="bytes"),
    pytest.param(lambda e, p: e.analyze(file=_file(p)), "file_source", id="file"),
    pytest.param(
        lambda e, _: e.analyze(blocks=[{"kind": "paragraph", "text": OVERSIZED}]),
        "input",
        id="blocks",
    ),
]


@pytest.mark.model
@pytest.mark.parametrize(("call", "gate"), CALLS)
def test_every_input_enforces_the_cap(
    engine: Any, tmp_path: Path, call: Callable[[Any, Path], Any], gate: str
) -> None:
    """``Error::InputTooLarge`` is raised as ``matra.InputTooLargeError``,
    which is also a ``ValueError``, so the assertion is on the builtin
    class a caller written before 0.3.0 catches, and then on the gate.

    Block input is read as the ``blocks`` format, so its gate measures
    the JSON the blocks become, a few bytes more than their text."""
    with pytest.raises(ValueError) as caught:
        call(engine, tmp_path)

    error = caught.value
    assert type(error).__name__ == "InputTooLargeError"
    assert error.kind == "input_too_large"
    assert error.details["what"] == gate
    assert error.details["limit"] == MAX_INPUT_BYTES
    assert error.details["actual"] > MAX_INPUT_BYTES


@pytest.mark.model
def test_the_main_gate_reports_its_bound_and_the_size(engine: Any) -> None:
    with pytest.raises(matra.InputTooLargeError) as caught:
        engine.analyze(OVERSIZED)
    assert caught.value.details == GATE


@pytest.mark.model
def test_a_stream_reports_an_oversized_text_as_one_item(engine: Any) -> None:
    """In a stream the cap costs the one oversized input, not the stream."""
    items = list(engine.analyze_all(texts=[OVERSIZED, "The committee met."]))

    assert not isinstance(items[0], matra.Document)
    assert items[0]["kind"] == "input_too_large"
    assert items[0]["details"] == GATE
    assert isinstance(items[1], matra.Document)
