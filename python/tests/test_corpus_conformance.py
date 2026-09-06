"""The corpus item fixture (i10 M5): the Python crust's ``analyze_path``
items must carry the shapes and the kind vocabulary
``spec/tests/corpus/items.json`` pins.

``DocumentError`` has no serde form, so each binding materializes its
fields by hand. That is the drift this lane exists to catch: a renamed
key or a kind spelled differently in one crust than in the other.

The vocabulary and the declared shapes are checked with no model. The
directory expectation needs one, because ``analyze_path`` is a method on
a loaded engine; the ``model`` marker says so.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import pytest
from matra import ERROR_KINDS, CorpusEntry, DocumentError, ErrorInfo, Matra

SPEC = Path(__file__).resolve().parents[2] / "spec" / "tests" / "corpus" / "items.json"
FIXTURE = json.loads(SPEC.read_text())


def _model_dir() -> str:
    return os.environ.get("MATRA_MODEL_DIR") or str(Path.home() / ".matra" / "models")


@pytest.fixture(scope="module")
def engine() -> Matra:
    try:
        return Matra.english(_model_dir())
    except Exception as exc:
        pytest.skip(f"UDPipe model unavailable: {exc}")


def test_the_kind_vocabulary_is_the_one_the_fixture_pins() -> None:
    """One string per Rust variant, in declaration order. A kind added on
    the Rust side and not here fails at this assertion rather than at a
    consumer's ``if kind == ...`` that silently never matches."""
    assert list(ERROR_KINDS) == FIXTURE["error_kinds"]


def test_the_declared_item_shapes_match_the_fixture() -> None:
    shapes = FIXTURE["item_shapes"]
    assert set(CorpusEntry.__annotations__) == set(shapes["entry"])
    assert set(DocumentError.__annotations__) == set(shapes["error"])
    assert set(ErrorInfo.__annotations__) == set(shapes["error_object"])


@pytest.mark.model
def test_a_directory_walk_yields_the_items_the_fixture_expects(
    engine: Matra, tmp_path: Path
) -> None:
    directory = FIXTURE["directory"]
    for spec in directory["files"]:
        path = tmp_path / spec["name"]
        if "text" in spec:
            path.write_text(spec["text"])
        else:
            path.write_bytes(bytes(spec["bytes"]))

    items = engine.analyze_path(tmp_path)

    assert len(items) == len(directory["expect"])
    for item, expect in zip(items, directory["expect"], strict=True):
        assert Path(item["path"]).name == expect["name"]
        if expect["shape"] == "entry":
            assert set(item) == set(FIXTURE["item_shapes"]["entry"])
            assert "sections" in item["analysis"]
        else:
            assert set(item) == set(FIXTURE["item_shapes"]["error"])
            assert set(item["error"]) == set(FIXTURE["item_shapes"]["error_object"])
            assert item["error"]["kind"] == expect["kind"]
            assert item["error"]["message"]
