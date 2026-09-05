"""The FFI shape fixture (i9 M5): the Python crust's vectors-in
``semantic_clusters`` must produce exactly the serialized value in
``spec/tests/semantic/clusters.json``.

No model is required. Score comparison happens in f32 space (both sides
cast through ``struct.pack('f', ...)``), because the fixture's JSON
shortest-repr floats and the f32-to-f64 widening the binding performs
denote the same f32 while differing as f64.
"""

import json
import struct
from pathlib import Path

from matra import semantic_clusters

SPEC = Path(__file__).resolve().parents[2] / "spec" / "tests" / "semantic" / "clusters.json"


def as_f32(x: float) -> float:
    return float(struct.unpack("f", struct.pack("f", x))[0])


def test_semantic_clusters_matches_the_shape_fixture() -> None:
    fixture = json.loads(SPEC.read_text())
    got = semantic_clusters(
        fixture["embeddings"], fixture["threshold"], fixture["model_hash"]
    )
    expect = fixture["expect"]

    assert got["model_hash"] == expect["model_hash"]
    assert as_f32(got["threshold"]) == as_f32(expect["threshold"])
    assert len(got["clusters"]) == len(expect["clusters"])
    for g, e in zip(got["clusters"], expect["clusters"]):
        assert g["members"] == e["members"]
        assert len(g["edges"]) == len(e["edges"])
        for ge, ee in zip(g["edges"], e["edges"]):
            assert ge["a"] == ee["a"]
            assert ge["b"] == ee["b"]
            assert as_f32(ge["score"]) == as_f32(ee["score"])
