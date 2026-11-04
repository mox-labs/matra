"""The corpus item fixture (EP-0010 M5): the items ``Engine.analyze_all``
yields must carry the shapes and the kind vocabulary
``spec/tests/corpus/items.json`` pins.

A document item is a ``matra.Document``, whose ``to_dict()`` is the
document's serde form; a failure item is ``DocumentError``'s serde form,
with its path put back through ``os.fsdecode``. This lane catches what a
binding could still get wrong on the way across: a renamed key, a kind
spelled differently in one crust than in the other, a detail lost.

The vocabulary and the declared shapes are checked with no model. The
directory expectation needs one, because ``analyze_all`` is a method on
a loaded engine; the ``model`` marker says so.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import matra
import pytest
from document_schema import assert_valid
from matra import ERROR_KINDS, DocumentDict, DocumentError, ErrorInfo

SPEC = Path(__file__).resolve().parents[2] / "spec" / "tests" / "corpus" / "items.json"
FIXTURE = json.loads(SPEC.read_text())


def test_the_kind_vocabulary_is_the_one_the_fixture_pins() -> None:
    """One string per Rust variant, in declaration order. A kind added on
    the Rust side and not here fails at this assertion rather than at a
    consumer's ``if kind == ...`` that silently never matches."""
    assert list(ERROR_KINDS) == FIXTURE["error_kinds"]


def test_the_declared_item_shapes_match_the_fixture() -> None:
    shapes = FIXTURE["item_shapes"]
    assert set(DocumentDict.__annotations__) == set(shapes["document"])
    assert set(DocumentError.__annotations__) == set(shapes["error"])
    assert set(ErrorInfo.__annotations__) == set(shapes["error_object"])


@pytest.mark.model
def test_a_directory_walk_yields_the_items_the_fixture_expects(engine: Any, tmp_path: Path) -> None:
    directory = FIXTURE["directory"]
    for spec in directory["files"]:
        path = tmp_path / spec["name"]
        if "text" in spec:
            path.write_text(spec["text"])
        else:
            path.write_bytes(bytes(spec["bytes"]))

    items = list(engine.analyze_all(dir=tmp_path))

    assert len(items) == len(directory["expect"])
    for item, expect in zip(items, directory["expect"], strict=True):
        if expect["shape"] == "document":
            assert isinstance(item, matra.Document)
            doc = item.to_dict()
            assert set(doc) == set(FIXTURE["item_shapes"]["document"])
            assert set(item) == set(doc)
            assert Path(doc["provenance"]["source"]["path"]).name == expect["name"]
            assert_valid("document", doc, expect["name"])
        else:
            assert not isinstance(item, matra.Document)
            assert Path(item["path"]).name == expect["name"]
            assert set(item) == set(FIXTURE["item_shapes"]["error"])
            assert item["kind"] == expect["kind"]
            assert item["message"]
            assert item["details"] == expect["details"]
            assert_valid("error", item, expect["name"])


def test_every_provisioning_kind_is_in_the_vocabulary() -> None:
    """A row naming a string no variant reports would pin a contract
    nothing can satisfy."""
    for condition, kind in FIXTURE["provisioning"]["kinds"].items():
        assert kind in FIXTURE["error_kinds"], f"{condition} names {kind}"


def test_a_model_directory_that_cannot_be_created_is_an_os_error(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """The provisioning classification RFC-0015 records, on the one
    failure a runner can produce with no model and no network:
    ``create_dir_all`` fails first, and ``io`` crosses into Python as
    ``matra.IoError``, an ``OSError``, carrying the directory as a detail.

    ``Engine()`` reads the model directory from the environment alone, so
    the test points ``MATRA_MODEL_DIR`` below a regular file."""
    expect = FIXTURE["provisioning"]["unwritable_model_dir"]["expect"]
    assert expect["kind"] == "io"

    blocked = tmp_path / "not-a-directory"
    blocked.write_bytes(b"x")
    monkeypatch.setenv("MATRA_MODEL_DIR", str(blocked / "models"))

    with pytest.raises(OSError) as caught:
        matra.Engine()

    error = caught.value
    assert isinstance(error, matra.IoError)
    assert error.kind == expect["kind"]
    message = str(error)
    for fragment in expect["message_contains"]:
        assert fragment in message, f"{fragment!r} missing from {message!r}"
    assert str(blocked) in message
    assert list(error.details) == expect["detail_keys"]
    assert error.details["path"] == str(blocked / "models")
    assert_valid(
        "error",
        {"kind": error.kind, "message": message, "details": error.details},
        "unwritable model directory",
    )
