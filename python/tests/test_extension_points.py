"""The two Python extension points (i10 M5).

A Python object with ``embed`` and ``identity`` is accepted wherever an
embedding model is, and ``analyze_path`` puts directory ingestion on the
Python surface with the corpus types crossing as dicts.

The embedder tests use a deterministic fake rather than a real model, so
what they pin is the adapter: the conversion, the contract check, and the
error mapping. They still need the UDPipe model, because clustering runs
over parsed sentences; the ``model`` marker says so.

    uv run pytest python/tests/test_extension_points.py
"""

from __future__ import annotations

import os
from pathlib import Path

import pytest

matra = pytest.importorskip("matra", reason="matra wheel not built")
Matra = matra.Matra


def _model_dir() -> str:
    return os.environ.get("MATRA_MODEL_DIR") or str(Path.home() / ".matra" / "models")


@pytest.fixture(scope="module")
def engine() -> Matra:
    try:
        return Matra.english(_model_dir())
    except Exception as exc:
        pytest.skip(f"UDPipe model unavailable: {exc}")


# ---------------------------------------------------------------------------
# A Python object as an embedder
# ---------------------------------------------------------------------------

PARAPHRASE_A = "The cat sat on the mat."
PARAPHRASE_B = "A cat was sitting on the mat."
UNRELATED = "Quarterly revenue exceeded every forecast."

# Two near-identical vectors and one orthogonal to both. Cosine between
# the first two is about 0.9987; between either and the third, zero.
VECTORS = {
    "sat": [1.0, 0.0, 0.0],
    "sitting": [0.9987, 0.05, 0.0],
    "revenue": [0.0, 0.0, 1.0],
}


class FakeEmbedder:
    """Fixed vectors keyed by a word in the sentence, so the assertion
    does not depend on how the parser segments the text."""

    def identity(self) -> str:
        return "fake-embedder-v1"

    def embed(self, texts: list[str]) -> list[list[float]]:
        out = []
        for text in texts:
            lowered = text.lower()
            match = next((v for k, v in VECTORS.items() if k in lowered), None)
            out.append(match if match is not None else [0.0, 1.0, 0.0])
        return out


@pytest.mark.model
def test_a_python_object_is_an_embedder(engine: Matra) -> None:
    text = f"{PARAPHRASE_A} {PARAPHRASE_B} {UNRELATED}"
    result = engine.semantic_clusters(text, 0.9, FakeEmbedder())

    # The embedder's own identity travels into the result, so the scores
    # cannot be attributed to a model that did not produce them.
    assert result["model_hash"] == "fake-embedder-v1"
    assert len(result["clusters"]) == 1

    cluster = result["clusters"][0]
    assert cluster["members"] == [0, 1]
    assert len(cluster["edges"]) == 1
    assert cluster["edges"][0]["score"] > 0.99


@pytest.mark.model
def test_wrong_vector_count_raises_the_same_message_a_rust_caller_gets(
    engine: Matra,
) -> None:
    """The contract check lives in `embed_and_cluster`, once, for every
    implementor. A Python embedder is held to it identically, and reads
    the same sentence."""
    seen: list[int] = []

    class ShortEmbedder:
        def identity(self) -> str:
            return "short-embedder"

        def embed(self, texts: list[str]) -> list[list[float]]:
            seen.append(len(texts))
            return [[1.0, 0.0, 0.0]] * (len(texts) - 1)

    with pytest.raises(ValueError) as excinfo:
        engine.semantic_clusters(f"{PARAPHRASE_A} {UNRELATED}", 0.9, ShortEmbedder())

    count = seen[0]
    assert str(excinfo.value) == (
        f"invalid input: embedder returned {count - 1} vectors for {count} "
        "sentences, violating its contract"
    )


@pytest.mark.model
def test_an_exception_inside_embed_surfaces_with_its_own_text(engine: Matra) -> None:
    class BoomEmbedder:
        def identity(self) -> str:
            return "boom-embedder"

        def embed(self, texts: list[str]) -> list[list[float]]:
            raise KeyError("no vectors cached for this batch")

    with pytest.raises(ValueError) as excinfo:
        engine.semantic_clusters(PARAPHRASE_A, 0.9, BoomEmbedder())

    message = str(excinfo.value)
    assert "no vectors cached for this batch" in message
    assert "embed()" in message


@pytest.mark.model
def test_a_shape_violation_names_the_level_that_was_wrong(engine: Matra) -> None:
    class ScalarEmbedder:
        def identity(self) -> str:
            return "scalar-embedder"

        def embed(self, texts: list[str]) -> list[list[float]]:
            return 42  # type: ignore[return-value]

    with pytest.raises(ValueError) as excinfo:
        engine.semantic_clusters(PARAPHRASE_A, 0.9, ScalarEmbedder())

    assert "must return a sequence of vectors" in str(excinfo.value)


@pytest.mark.model
def test_an_embedder_without_identity_is_refused_before_any_work(
    engine: Matra,
) -> None:
    class Nameless:
        def embed(self, texts: list[str]) -> list[list[float]]:
            raise AssertionError("embed must not be called")

    with pytest.raises(ValueError) as excinfo:
        engine.semantic_clusters(PARAPHRASE_A, 0.9, Nameless())

    assert "identity()" in str(excinfo.value)


# ---------------------------------------------------------------------------
# analyze_path
# ---------------------------------------------------------------------------


@pytest.mark.model
def test_analyze_path_returns_one_item_per_file_in_order(engine: Matra, tmp_path: Path) -> None:
    """One unreadable file costs one item, not the walk. The bad file is
    invalid UTF-8: a symlink or a subdirectory would not appear at all,
    because the directory listing filters both out before reading."""
    (tmp_path / "a.txt").write_text("The committee approved the proposal.")
    (tmp_path / "b.txt").write_bytes(b"\xff\xfe not valid utf-8")
    (tmp_path / "c.txt").write_text("The report was filed without comment.")

    items = engine.analyze_path(str(tmp_path))

    assert len(items) == 3
    assert [Path(item["path"]).name for item in items] == ["a.txt", "b.txt", "c.txt"]

    assert "analysis" in items[0]
    assert items[0]["analysis"]["sections"][0]["paragraphs"][0]["sentences"]

    assert "error" in items[1]
    assert items[1]["error"]["kind"] == "io"
    assert items[1]["error"]["message"]

    assert "analysis" in items[2]
    assert items[2]["analysis"]["passive_ratio"] is not None


@pytest.mark.model
def test_analyze_path_on_one_file_is_a_stream_of_one(engine: Matra, tmp_path: Path) -> None:
    path = tmp_path / "only.txt"
    path.write_text("The committee approved the proposal.")

    items = engine.analyze_path(str(path))

    assert len(items) == 1
    assert Path(items[0]["path"]) == path


@pytest.mark.model
def test_analyze_path_on_a_missing_directory_raises(engine: Matra, tmp_path: Path) -> None:
    """A listing failure has no per-document result to travel in, so it
    is raised rather than returned. `Error::Io` routes to `OSError`, the
    same mapping every other I/O failure on the surface uses."""
    with pytest.raises(OSError) as excinfo:
        engine.analyze_path(str(tmp_path / "no-such-directory"))

    assert "No such file or directory" in str(excinfo.value)


# ---------------------------------------------------------------------------
# The typed surface
# ---------------------------------------------------------------------------


def test_the_new_shapes_are_importable_from_the_package_root() -> None:
    """No model needed: the crust either exports these names or it does
    not, and a consumer's `from matra import ...` is the check."""
    from matra import CorpusEntry, CorpusItem, DocumentError, Embedder, ErrorInfo

    assert set(DocumentError.__annotations__) == {"path", "error"}
    assert set(CorpusEntry.__annotations__) == {"path", "analysis"}
    assert set(ErrorInfo.__annotations__) == {"kind", "message"}
    assert callable(Embedder.embed) and callable(Embedder.identity)
    assert CorpusItem is not None
