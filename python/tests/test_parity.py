"""The cross-surface parity lane, Python's third (EPR-0006).

``spec/tests/parity/parity.json`` pins what Rust returns for a few files:
the whole ``Document``, summaries and keyphrases, and the error a file
that cannot be analysed raises. ``tests/parity.rs`` holds the library
and the command line to it; this holds the Python surface to the same
values, so the three cannot drift apart without one runner failing.

A case with no ``n`` or ``method`` calls the Python function with its
own defaults, which must be the ones the command line applies.

Requires the UDPipe model:

    uv run pytest python/tests/test_parity.py
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

matra = pytest.importorskip("matra", reason="matra wheel not built")

FIXTURE = json.loads(
    (Path(__file__).resolve().parents[2] / "spec" / "tests" / "parity" / "parity.json").read_text()
)


def normalized(value: dict[str, Any], passed: Path) -> dict[str, Any]:
    """The JSON with the matra version and the run's own path set aside,
    after checking the path is the one passed."""
    provenance = value["provenance"]
    assert provenance["source"]["path"] == str(passed)
    provenance["source"]["path"] = None
    provenance["matra"] = "fixture"
    return value


def kwargs(call: dict[str, Any]) -> dict[str, Any]:
    return {key: call[key] for key in ("n", "method") if key in call}


@pytest.mark.model
@pytest.mark.parametrize("case", FIXTURE["documents"], ids=lambda c: c["file"])
def test_python_gives_what_rust_gives(engine: Any, tmp_path: Path, case: dict[str, Any]) -> None:
    path = tmp_path / case["file"]
    path.write_text(case["input"])
    expect = case["expect"]

    doc = engine.analyze(file=path)

    assert normalized(doc.to_dict(), path) == expect["document"]
    for call, want in zip(case["summaries"], expect["summaries"], strict=True):
        got = matra.summarize(doc, **kwargs(call))
        assert normalized(got, path) == want, f"summarize {call}"
    for call, want in zip(case["keyphrases"], expect["keyphrases"], strict=True):
        got = matra.keyphrases(doc, **kwargs(call))
        assert normalized(got, path) == want, f"keyphrases {call}"


@pytest.mark.model
@pytest.mark.parametrize("case", FIXTURE["errors"], ids=lambda c: c["file"])
def test_python_raises_what_rust_returns(engine: Any, tmp_path: Path, case: dict[str, Any]) -> None:
    path = tmp_path / case["file"]
    if "bytes" in case:
        path.write_bytes(bytes(case["bytes"]))
    else:
        path.write_text(case["fill"] * case["count"])
    expect = case["expect"]

    with pytest.raises(matra.MatraError) as caught:
        engine.analyze(file=path)

    error = caught.value
    assert error.kind == expect["kind"]
    assert str(error) == expect["message"]
    assert error.details == expect["details"]
