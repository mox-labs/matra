"""The FFI shape fixture (i9 M5): the Python crust's vectors-in
``semantic_clusters`` must produce exactly the serialized value in
``spec/tests/semantic/clusters.json``.

No model is required. Score comparison happens in f32 space (both sides
cast through ``struct.pack('f', ...)``), because the fixture's JSON
shortest-repr floats and the f32-to-f64 widening the binding performs
denote the same f32 while differing as f64.
"""

import hashlib
import json
import os
import struct
from pathlib import Path

import pytest
from matra import Model2Vec, semantic_clusters

SPEC = Path(__file__).resolve().parents[2] / "spec" / "tests" / "semantic" / "clusters.json"


def as_f32(x: float) -> float:
    return float(struct.unpack("f", struct.pack("f", x))[0])


def test_semantic_clusters_matches_the_shape_fixture() -> None:
    fixture = json.loads(SPEC.read_text())
    got = semantic_clusters(fixture["embeddings"], fixture["threshold"], fixture["model_hash"])
    expect = fixture["expect"]

    assert got["model_hash"] == expect["model_hash"]
    assert as_f32(got["threshold"]) == as_f32(expect["threshold"])
    assert len(got["clusters"]) == len(expect["clusters"])
    for g, e in zip(got["clusters"], expect["clusters"], strict=True):
        assert g["members"] == e["members"]
        assert len(g["edges"]) == len(e["edges"])
        for ge, ee in zip(g["edges"], e["edges"], strict=True):
            assert ge["a"] == ee["a"]
            assert ge["b"] == ee["b"]
            assert as_f32(ge["score"]) == as_f32(ee["score"])


REF = SPEC.parent / "reference-model.json"


@pytest.mark.model
def test_reference_model_vectors_and_clusters_are_exact() -> None:
    """Reference-model conformance (i9 M6): the Python crust reproduces
    the pinned potion-base-8M expectations exactly. The adapter is
    bit-deterministic, so cluster scores compare exactly in f32 space.
    Requires the model at ~/.matra/models/potion-base-8M (or
    MATRA_MODEL2VEC_DIR).
    """
    fixture = json.loads(REF.read_text())
    model_dir = os.environ.get(
        "MATRA_MODEL2VEC_DIR",
        str(Path.home() / ".matra" / "models" / "potion-base-8M"),
    )
    model = Model2Vec.from_dir(model_dir)
    assert model.model_hash == fixture["model"]["artifact_hash"]
    assert model.dimensions == fixture["model"]["dimensions"]

    vectors = model.embed(fixture["inputs"])
    assert all(len(v) == model.dimensions for v in vectors)

    # The bit-determinism pin, same bytes the Rust runner hashes:
    # f32-to-f64 widening is exact, so packing back to "<f" is lossless.
    digest = hashlib.sha256()
    for vec in vectors:
        for v in vec:
            digest.update(struct.pack("<f", v))
    assert digest.hexdigest() == fixture["vectors_sha256"]

    for case in fixture["cases"]:
        got = semantic_clusters(vectors, case["threshold"], model.model_hash)
        expect = case["clusters"]
        assert len(got["clusters"]) == len(expect)
        for g, e in zip(got["clusters"], expect, strict=True):
            assert g["members"] == e["members"]
            assert len(g["edges"]) == len(e["edges"])
            for ge, ee in zip(g["edges"], e["edges"], strict=True):
                assert (ge["a"], ge["b"]) == (ee["a"], ee["b"])
                assert as_f32(ge["score"]) == as_f32(ee["score"])
