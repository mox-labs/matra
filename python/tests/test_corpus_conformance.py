"""The corpus item fixture (EP-0010 M5): the Python crust's ``analyze_path``
items must carry the shapes and the kind vocabulary
``spec/tests/corpus/items.json`` pins.

A failure item is ``DocumentError``'s serde form, with its path put back
through ``os.fsdecode``. This lane catches what a binding could still get
wrong on the way across: a renamed key, a kind spelled differently in one
crust than in the other, a detail lost.

The vocabulary and the declared shapes are checked with no model. The
directory expectation needs one, because ``analyze_path`` is a method on
a loaded engine; the ``model`` marker says so.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import matra
import pytest
from document_schema import assert_valid
from matra import ERROR_KINDS, Document, DocumentError, ErrorInfo, Matra

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
    assert set(Document.__annotations__) == set(shapes["document"])
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
        if expect["shape"] == "document":
            assert set(item) == set(FIXTURE["item_shapes"]["document"])
            assert Path(item["provenance"]["source"]["path"]).name == expect["name"]
            assert_valid("document", item, expect["name"])
        else:
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


def test_a_model_directory_that_cannot_be_created_is_an_os_error(tmp_path: Path) -> None:
    """The provisioning classification RFC-0015 records, on the one
    failure a runner can produce with no model and no network:
    ``create_dir_all`` fails first, and ``io`` crosses into Python as
    ``matra.Io``, an ``OSError``, carrying the directory as a detail.

    Not itself the contract change. ``create_dir_all`` already converted
    through ``Error::Io`` and routed to ``OSError`` before RFC-0015; the
    reclassification is DNS, TLS and a full disk, which no runner
    exercises."""
    expect = FIXTURE["provisioning"]["unwritable_model_dir"]["expect"]
    assert expect["kind"] == "io"

    blocked = tmp_path / "not-a-directory"
    blocked.write_bytes(b"x")

    with pytest.raises(OSError) as caught:
        Matra.english(str(blocked / "models"))

    error = caught.value
    assert isinstance(error, matra.Io)
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
