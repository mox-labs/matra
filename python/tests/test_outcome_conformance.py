"""The outcome lane in Python: ``summarize``, ``keyphrases`` and
``clusters`` over the document ``spec/tests/outcomes/outcomes.json`` pins.

Rust reads the fixture's committed ``Document`` directly and needs no
parser. Python has no constructor from data in 0.3.0, so this runner
parses the fixture's ``input`` and first checks the document it gets is
the committed one; every case then runs over it. ``clusters`` takes the
fixture's own vectors through an embedder that returns them, so what is
pinned is the clustering, not a model.

Requires the UDPipe model:

    uv run pytest python/tests/test_outcome_conformance.py
"""

from __future__ import annotations

import json
import struct
from pathlib import Path
from typing import Any

import pytest
from document_schema import assert_valid

matra = pytest.importorskip("matra", reason="matra wheel not built")

FIXTURE = json.loads(
    (
        Path(__file__).resolve().parents[2] / "spec" / "tests" / "outcomes" / "outcomes.json"
    ).read_text()
)
TOLERANCE = 1e-6


def as_f32(x: float) -> float:
    return float(struct.unpack("f", struct.pack("f", x))[0])


class Vectors:
    """Returns the fixture's vectors, one per sentence."""

    def identity(self) -> str:
        identity: str = FIXTURE["embedder"]
        return identity

    def embed(self, texts: list[str]) -> list[list[float]]:
        vectors: list[list[float]] = FIXTURE["embeddings"]
        assert len(texts) == len(vectors), "one vector per sentence"
        return vectors


@pytest.fixture(scope="module")
def doc(engine: Any) -> Any:
    analysed = engine.analyze(FIXTURE["input"], format=FIXTURE["format"])
    got = analysed.to_dict()
    # The matra version moves with every release; the parse does not.
    got["provenance"]["matra"] = FIXTURE["document"]["provenance"]["matra"]
    assert got == FIXTURE["document"], "Python's parse is not the committed document"
    return analysed


def assert_derived(got: dict[str, Any], doc: Any, method: str, parameters: dict[str, Any]) -> None:
    provenance = dict(got["provenance"])
    assert provenance.pop("method") == method
    assert provenance.pop("parameters") == parameters
    assert provenance == doc["provenance"]


@pytest.mark.model
@pytest.mark.parametrize("case", FIXTURE["summaries"], ids=lambda c: f"{c['method']}-{c['n']}")
def test_summaries_match_the_fixture(doc: Any, case: dict[str, Any]) -> None:
    got = matra.summarize(doc, case["n"], method=case["method"])
    assert_derived(got, doc, case["method"], {"n": case["n"]})
    assert_valid("summary", got, f"summarize {case}")
    assert len(got["sentences"]) == len(case["expect"])
    for g, e in zip(got["sentences"], case["expect"], strict=True):
        assert (g["index"], g["span"]) == (e["index"], e["span"])
        assert abs(g["score"] - e["score"]) < TOLERANCE
        assert matra.span_text(FIXTURE["input"], g["span"]) == g["text"]


@pytest.mark.model
@pytest.mark.parametrize("case", FIXTURE["keyphrases"], ids=lambda c: f"{c['method']}-{c['n']}")
def test_keyphrases_match_the_fixture(doc: Any, case: dict[str, Any]) -> None:
    got = matra.keyphrases(doc, case["n"], method=case["method"])
    assert_derived(got, doc, case["method"], {"n": case["n"]})
    assert_valid("keyphrases", got, f"keyphrases {case}")
    assert len(got["phrases"]) == len(case["expect"])
    for g, e in zip(got["phrases"], case["expect"], strict=True):
        assert g["phrase"] == e["phrase"]
        assert abs(g["score"] - e["score"]) < TOLERANCE
        assert g["occurrences"] == e["occurrences"]


@pytest.mark.model
@pytest.mark.parametrize("case", FIXTURE["clusters"], ids=lambda c: str(c["threshold"]))
def test_clusters_match_the_fixture(doc: Any, case: dict[str, Any]) -> None:
    got = matra.clusters(doc, Vectors(), case["threshold"])
    assert_derived(
        got, doc, "cosine", {"threshold": case["threshold"], "embedder": FIXTURE["embedder"]}
    )
    assert_valid("clusters", got, f"clusters {case['threshold']}")
    assert got["clusters"] == case["expect"]["clusters"]
    assert len(got["edges"]) == len(case["expect"]["edges"])
    for g, e in zip(got["edges"], case["expect"]["edges"], strict=True):
        assert (g["a"], g["b"]) == (e["a"], e["b"])
        assert as_f32(g["score"]) == as_f32(e["score"])
