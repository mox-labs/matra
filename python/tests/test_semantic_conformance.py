"""The FFI shape fixture (EP-0009 M5): the Python crust's vectors-in
``semantic_clusters`` must produce exactly the clusters, edges and
parameters in ``spec/tests/semantic/clusters.json``.

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
    got = semantic_clusters(fixture["embeddings"], fixture["threshold"], fixture["embedder"])
    expect = fixture["expect"]

    assert got["parameters"] == expect["parameters"]
    assert got["clusters"] == expect["clusters"]
    assert_same_edges(got["edges"], expect["edges"])


def assert_same_edges(got: list[dict], expect: list[dict]) -> None:
    assert len(got) == len(expect)
    for g, e in zip(got, expect, strict=True):
        assert (g["a"], g["b"]) == (e["a"], e["b"])
        assert as_f32(g["score"]) == as_f32(e["score"])


REF = SPEC.parent / "reference-model.json"


@pytest.mark.model
def test_reference_model_vectors_and_clusters_are_exact() -> None:
    """Reference-model conformance (EP-0009 M6): the Python crust reproduces
    the pinned potion-base-8M expectations exactly. The adapter is
    bit-deterministic, so cluster scores compare exactly in f32 space.

    Since EP-0010 M4 the model arrives through ``potion_base_8m``, which
    downloads it into the resolved directory on the first run and
    verifies the same digest this fixture asserts. ``MATRA_MODEL2VEC_DIR``
    still names the directory when set; without it the directory comes
    from the resolved configuration. Marked ``model`` because the first
    run needs a network.
    """
    fixture = json.loads(REF.read_text())
    model = Model2Vec.potion_base_8m(os.environ.get("MATRA_MODEL2VEC_DIR"))
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
        assert got["parameters"]["embedder"] == model.model_hash
        assert got["clusters"] == case["clusters"]
        assert_same_edges(got["edges"], case["edges"])
