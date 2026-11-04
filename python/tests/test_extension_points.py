"""The two Python extension points (EP-0010 M5).

A Python object with ``embed`` and ``identity`` is accepted wherever an
embedding model is, and ``Engine.analyze_all(dir=...)`` puts directory
ingestion on the Python surface, each failure crossing as a dict.

The embedder tests use a deterministic fake rather than a real model, so
what they pin is the adapter: the conversion, the contract check, and the
error mapping. They still need the UDPipe model, because clustering runs
over parsed sentences; the ``model`` marker says so.

    uv run pytest python/tests/test_extension_points.py
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

import pytest

matra = pytest.importorskip("matra", reason="matra wheel not built")


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
def test_a_python_object_is_an_embedder(engine: Any) -> None:
    doc = engine.analyze(f"{PARAPHRASE_A} {PARAPHRASE_B} {UNRELATED}")
    result = matra.clusters(doc, FakeEmbedder(), 0.9)

    # The embedder's own identity travels into the result, so the scores
    # cannot be attributed to a model that did not produce them.
    assert result["provenance"]["parameters"]["embedder"] == "fake-embedder-v1"
    assert result["provenance"]["parameters"]["threshold"] == 0.9
    assert result["provenance"]["method"] == "cosine"
    assert len(result["clusters"]) == 1

    assert result["clusters"][0]["members"] == [0, 1]
    assert len(result["edges"]) == 1
    assert result["edges"][0]["score"] > 0.99


@pytest.mark.model
def test_wrong_vector_count_raises_the_same_message_a_rust_caller_gets(
    engine: Any,
) -> None:
    """The contract check lives in `matra::clusters`, once, for every
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
        matra.clusters(engine.analyze(f"{PARAPHRASE_A} {UNRELATED}"), ShortEmbedder(), 0.9)

    # Two sentences, one vector short: the count is singular where it is one.
    assert seen == [2]
    assert str(excinfo.value) == (
        "invalid input: embedder returned 1 vector for 2 sentences, violating its contract"
    )


@pytest.mark.model
def test_an_exception_inside_embed_surfaces_with_its_own_text(engine: Any) -> None:
    class BoomEmbedder:
        def identity(self) -> str:
            return "boom-embedder"

        def embed(self, texts: list[str]) -> list[list[float]]:
            raise KeyError("no vectors cached for this batch")

    with pytest.raises(ValueError) as excinfo:
        matra.clusters(engine.analyze(PARAPHRASE_A), BoomEmbedder(), 0.9)

    message = str(excinfo.value)
    assert "no vectors cached for this batch" in message
    assert "embed()" in message


@pytest.mark.model
def test_a_shape_violation_names_the_level_that_was_wrong(engine: Any) -> None:
    class ScalarEmbedder:
        def identity(self) -> str:
            return "scalar-embedder"

        def embed(self, texts: list[str]) -> list[list[float]]:
            return 42  # type: ignore[return-value]

    with pytest.raises(ValueError) as excinfo:
        matra.clusters(engine.analyze(PARAPHRASE_A), ScalarEmbedder(), 0.9)

    assert "must return a sequence of vectors" in str(excinfo.value)


@pytest.mark.model
def test_an_embedder_without_identity_is_refused_before_any_work(
    engine: Any,
) -> None:
    """Before any work means before anything is embedded: the embedder is
    settled first, so an object that cannot name its geometry costs one
    method call."""

    class Nameless:
        def embed(self, texts: list[str]) -> list[list[float]]:
            raise AssertionError("embed must not be called")

    with pytest.raises(ValueError) as excinfo:
        matra.clusters(engine.analyze(PARAPHRASE_A), Nameless(), 0.9)

    assert "identity()" in str(excinfo.value)


# ---------------------------------------------------------------------------
# analyze_all(dir=...) and analyze(file=...)
# ---------------------------------------------------------------------------


def item_path(item: Any) -> str | None:
    """The path an item names: a document's own provenance, or an error's."""
    if isinstance(item, matra.Document):
        path: str | None = item["provenance"]["source"]["path"]
        return path
    path = item["path"]
    return path


@pytest.mark.model
def test_a_directory_yields_one_item_per_file_in_order(engine: Any, tmp_path: Path) -> None:
    """One unreadable file costs one item, not the walk. The bad file is
    invalid UTF-8, which is an unsupported format: a symlink or a
    subdirectory would not appear at all, because the directory listing
    filters both out before reading."""
    (tmp_path / "a.txt").write_text("The committee approved the proposal.")
    (tmp_path / "b.txt").write_bytes(b"\xff\xfe not valid utf-8")
    (tmp_path / "c.txt").write_text("The report was filed without comment.")

    items = list(engine.analyze_all(dir=str(tmp_path)))

    assert len(items) == 3
    assert [Path(item_path(item) or "").name for item in items] == ["a.txt", "b.txt", "c.txt"]

    assert isinstance(items[0], matra.Document)
    assert items[0]["sections"][0]["paragraphs"][0]["sentences"]

    assert not isinstance(items[1], matra.Document)
    assert items[1]["kind"] == "unsupported_format"
    assert items[1]["message"]
    assert items[1]["details"] == {}

    assert isinstance(items[2], matra.Document)
    assert items[2]["metrics"]["passive_ratio"] is not None


@pytest.mark.model
def test_a_markdown_file_is_read_as_markdown(engine: Any, tmp_path: Path) -> None:
    """A `.md` file is read as markdown: its heading opens a section and
    is not parsed as prose."""
    path = tmp_path / "notes.md"
    path.write_text("# Field notes\n\nThe river rose overnight.\n")

    doc = engine.analyze(file=path)

    assert doc["provenance"]["format"] == "markdown"
    assert doc["sections"][0]["heading"] == "Field notes"
    assert doc["metrics"]["total_sentences"] == 1


@pytest.mark.model
def test_a_file_names_itself_in_its_provenance(engine: Any, tmp_path: Path) -> None:
    path = tmp_path / "only.txt"
    path.write_text("The committee approved the proposal.")

    doc = engine.analyze(file=str(path))

    assert Path(item_path(doc) or "") == path


@pytest.mark.model
@pytest.mark.skipif(os.name != "posix", reason="only posix filenames carry raw bytes")
def test_a_file_whose_name_is_not_utf_8_is_analyzed_and_round_trips(
    engine: Any, tmp_path: Path
) -> None:
    """The document's path is replaced rather than left as serde wrote
    it, because serde writes a non-UTF-8 path lossily. The name crosses
    through `os.fsdecode`, so `os.fsencode` hands back the bytes it came
    from and a caller can open what the walk named.

    Not every filesystem can hold such a name: APFS rejects a filename
    that is not valid UTF-8 outright, so this skips on macOS and runs on
    the Linux side of the matrix."""
    name = b"weird-\xff.txt"
    raw = os.path.join(os.fsencode(str(tmp_path)), name)
    try:
        with open(raw, "wb") as f:
            f.write(b"The committee approved the proposal.")
    except OSError as exc:
        pytest.skip(f"this filesystem refuses a non-UTF-8 filename: {exc}")

    items = list(engine.analyze_all(dir=tmp_path))

    assert len(items) == 1
    assert isinstance(items[0], matra.Document)
    # The document's own provenance names the same bytes, not the lossy
    # string serde writes for a path JSON cannot carry.
    assert os.fsencode(item_path(items[0]) or "").endswith(name)
    assert items[0]["sections"][0]["paragraphs"][0]["sentences"]


@pytest.mark.model
def test_a_non_ascii_filename_round_trips(engine: Any, tmp_path: Path) -> None:
    """The same decoding on a name every filesystem accepts, so the route
    through `os.fsdecode` is exercised where APFS will not hold the
    undecodable case above."""
    name = "rapport-financiér-ü.txt"
    (tmp_path / name).write_text("The committee approved the proposal.")

    items = list(engine.analyze_all(dir=tmp_path))

    assert len(items) == 1
    path = item_path(items[0]) or ""
    assert path.endswith(name)
    assert os.fsencode(path) == os.fsencode(str(tmp_path / name))


@pytest.mark.model
def test_a_file_takes_a_pathlib_path(engine: Any, tmp_path: Path) -> None:
    """Every path argument on the surface extracts as a `PathBuf`, which
    pyo3 fills from a `str` or from anything implementing `os.PathLike`.
    A caller holding a `pathlib.Path` does not have to stringify it."""
    (tmp_path / "only.txt").write_text("The committee approved the proposal.")

    doc = engine.analyze(file=tmp_path / "only.txt")

    assert Path(item_path(doc) or "").name == "only.txt"


def test_model2vec_takes_a_pathlib_path(tmp_path: Path) -> None:
    """The same extraction on the embedding adapter. No model needed: a
    missing directory is `Error::ModelNotFound`, raised as
    `matra.ModelNotFoundError`, a `FileNotFoundError`, and reaching that error
    at all proves the argument was accepted."""
    with pytest.raises(matra.ModelNotFoundError):
        matra.Model2Vec.from_dir(tmp_path / "no-such-model")


@pytest.mark.model
def test_potion_base_8m_takes_the_models_directory(
    tmp_path: Path, capfd: pytest.CaptureFixture[str]
) -> None:
    """Regression: `potion_base_8m(dir)` wrote the three files at the root
    of `dir`, so a caller who passed the model directory, as
    `MATRA_MODEL_DIR` names it, got a second copy beside the parsing
    model. `dir` is the models directory: the set is read from
    `dir/potion-base-8M`, and a set already there loads with no download
    and nothing on stderr."""
    models = os.environ.get("MATRA_MODEL_DIR")
    source = Path(models or "") / "potion-base-8M"
    if not models or not (source / "model.safetensors").is_file():
        pytest.skip("needs MATRA_MODEL_DIR holding potion-base-8M")
    target = tmp_path / "models" / "potion-base-8M"
    target.mkdir(parents=True)
    for name in ("model.safetensors", "tokenizer.json", "config.json"):
        (target / name).write_bytes((source / name).read_bytes())
    capfd.readouterr()

    model = matra.Model2Vec.potion_base_8m(tmp_path / "models")

    assert model.model_hash == matra.Model2Vec.from_dir(source).model_hash
    assert sorted(p.name for p in (tmp_path / "models").iterdir()) == ["potion-base-8M"]
    assert capfd.readouterr().err == ""


@pytest.mark.model
def test_a_missing_directory_raises(engine: Any, tmp_path: Path) -> None:
    """A listing failure has no per-document result to travel in, so it
    is raised at the call rather than yielded. `Error::Io` routes on the I/O kind it
    wraps (defect 8 of EPR-0006): a missing path is `matra.IoNotFoundError`, a
    `FileNotFoundError`, where it used to be a bare `OSError`."""
    with pytest.raises(FileNotFoundError) as excinfo:
        engine.analyze_all(dir=str(tmp_path / "no-such-directory"))

    assert isinstance(excinfo.value, matra.IoNotFoundError)
    assert excinfo.value.kind == "io"
    assert excinfo.value.details["io_kind"] == "not_found"
    assert "No such file or directory" in str(excinfo.value)


# ---------------------------------------------------------------------------
# The typed surface
# ---------------------------------------------------------------------------


def test_the_new_shapes_are_importable_from_the_package_root() -> None:
    """No model needed: the crust either exports these names or it does
    not, and a consumer's `from matra import ...` is the check."""
    from matra import BlockDict, DocumentDict, DocumentError, Embedder, ErrorInfo, Skipped, Span

    assert set(DocumentError.__annotations__) == {"path", "kind", "message", "details"}
    assert set(DocumentDict.__annotations__) == {"provenance", "sections", "metrics", "skipped"}
    assert set(Span.__annotations__) == {"start", "end"}
    assert set(Skipped.__annotations__) == {"kind", "span"}
    assert set(ErrorInfo.__annotations__) == {"kind", "message", "details"}
    assert callable(Embedder.embed) and callable(Embedder.identity)
    assert BlockDict is not None


def test_the_builtin_adapter_satisfies_the_embedder_protocol() -> None:
    """The docs say `Model2Vec` is an `Embedder`, and the annotation on
    `clusters` is the protocol alone. Both are only true while
    the class carries both methods; no model is needed to check that."""
    assert callable(matra.Model2Vec.embed)
    assert callable(matra.Model2Vec.identity)
