"""The semantic lanes in Python: ``matra.clusters`` over a parsed document
must produce exactly what ``spec/tests/semantic/`` pins.

The shape fixture (EP-0009 M5) runs its hand-built vectors through an
embedder that returns them, so what is pinned is the clustering and its
crossing, not a model. Since 0.3.0 ``clusters`` takes a ``Document``, the
one route in every language, so the lane parses a document with as many
sentences as the fixture has vectors, and needs the UDPipe model.

Score comparison happens in f32 space (both sides cast through
``struct.pack('f', ...)``), because the fixture's JSON shortest-repr
floats and the f32-to-f64 widening the binding performs denote the same
f32 while differing as f64.
"""

from __future__ import annotations

import hashlib
import json
import os
import struct
from pathlib import Path
from typing import TYPE_CHECKING, Any

import matra
import pytest
from matra import Model2Vec

if TYPE_CHECKING:
    from matra.types import Identity

SPEC = Path(__file__).resolve().parents[2] / "spec" / "tests" / "semantic" / "clusters.json"
REF = SPEC.parent / "reference-model.json"

# One sentence per vector of the shape fixture, each its own sentence to
# the parser.
SENTENCES = [
    "The river rose overnight.",
    "The bridge was closed at dawn.",
    "Engineers inspected the piers.",
    "The council met in the afternoon.",
    "Rain is forecast for the weekend.",
]


def as_f32(x: float) -> float:
    return float(struct.unpack("f", struct.pack("f", x))[0])


def assert_same_edges(got: list[dict[str, Any]], expect: list[dict[str, Any]]) -> None:
    assert len(got) == len(expect)
    for g, e in zip(got, expect, strict=True):
        assert (g["a"], g["b"]) == (e["a"], e["b"])
        assert as_f32(g["score"]) == as_f32(e["score"])


class Vectors:
    """Returns the vectors it was built with, one per sentence."""

    def __init__(self, vectors: list[list[float]], identity: Identity) -> None:
        self.vectors = vectors
        self._identity = identity

    def identity(self) -> Identity:
        return self._identity

    def embed(self, texts: list[str]) -> list[list[float]]:
        assert len(texts) == len(self.vectors), "one vector per sentence"
        return self.vectors


def sentences_of(doc: matra.Document) -> list[str]:
    return [
        sentence["text"]
        for section in doc["sections"]
        for paragraph in section["paragraphs"]
        for sentence in paragraph["sentences"]
    ]


@pytest.mark.model
def test_clusters_matches_the_shape_fixture(engine: Any) -> None:
    fixture = json.loads(SPEC.read_text())
    assert len(SENTENCES) == len(fixture["embeddings"])
    doc = engine.analyze(" ".join(SENTENCES))
    assert sentences_of(doc) == SENTENCES

    got = matra.clusters(
        doc, Vectors(fixture["embeddings"], fixture["embedder"]), fixture["threshold"]
    )
    expect = fixture["expect"]

    assert got["provenance"]["method"] == "cosine"
    assert got["provenance"]["parameters"] == expect["parameters"]
    assert got["clusters"] == expect["clusters"]
    assert_same_edges(got["edges"], expect["edges"])


@pytest.mark.model
def test_reference_model_vectors_and_clusters_are_exact(engine: Any) -> None:
    """Reference-model conformance (EP-0009 M6): the Python crust reproduces
    the pinned potion-base-8M expectations exactly. The adapter is
    bit-deterministic, so cluster scores compare exactly in f32 space.

    Since EP-0010 M4 the model arrives through ``potion_base_8m``, which
    downloads it into the resolved directory on the first run and
    verifies the same digest this fixture asserts. ``MATRA_MODEL2VEC_DIR``
    names a models directory when set, the model landing in its
    ``potion-base-8M`` subdirectory; without it the directory comes from
    the resolved configuration. Marked ``model`` because the first
    run needs a network, and because the clusters run over a parse.
    """
    fixture = json.loads(REF.read_text())
    model = Model2Vec.potion_base_8m(os.environ.get("MATRA_MODEL2VEC_DIR"))
    assert model.model_hash == fixture["model"]["artifact_hash"]
    assert model.identity() == {
        "name": "model2vec",
        "model": "potion-base-8M",
        "sha256": model.model_hash,
        "license": "MIT",
    }
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

    # The same inputs as a document's sentences, through the outcome
    # function a caller uses: the model embeds them there.
    doc = engine.analyze(" ".join(fixture["inputs"]))
    assert sentences_of(doc) == fixture["inputs"]
    for case in fixture["cases"]:
        got = matra.clusters(doc, model, case["threshold"])
        assert got["provenance"]["parameters"]["embedder"] == {
            "name": "model2vec",
            "model": "potion-base-8M",
            "sha256": model.model_hash,
            "license": "MIT",
        }
        assert got["clusters"] == case["clusters"]
        assert_same_edges(got["edges"], case["edges"])
