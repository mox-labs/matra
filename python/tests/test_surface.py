"""The Python surface of 0.3.0 (EPR-0006 milestone 5): one input per call,
a read-only ``Document``, and outcome functions that take only it.

Most of these need the UDPipe model, because an ``Engine`` loads it; the
``model`` marker says so. The checks that need no engine run everywhere.
"""

from __future__ import annotations

import pickle
from pathlib import Path
from typing import Any

import pytest

matra = pytest.importorskip("matra", reason="matra wheel not built")

PROSE = "The committee approved the proposal. The chair adjourned the meeting."


# ---------------------------------------------------------------------------
# Exactly one input
# ---------------------------------------------------------------------------


@pytest.mark.model
@pytest.mark.parametrize(
    ("kwargs", "named"),
    [
        ({}, "got none"),
        ({"text": PROSE, "file": "x.txt"}, "got text and file"),
        ({"bytes": b"x", "blocks": []}, "got bytes and blocks"),
        ({"text": PROSE, "file": "x", "bytes": b"x"}, "got text, file and bytes"),
    ],
)
def test_analyze_takes_exactly_one_input(engine: Any, kwargs: dict[str, Any], named: str) -> None:
    with pytest.raises(TypeError, match=named):
        engine.analyze(**kwargs)


@pytest.mark.model
def test_analyze_all_takes_exactly_one_input(engine: Any, tmp_path: Path) -> None:
    with pytest.raises(TypeError, match="got none"):
        engine.analyze_all()
    with pytest.raises(TypeError, match="got dir and texts"):
        engine.analyze_all(dir=tmp_path, texts=[PROSE])


@pytest.mark.model
def test_a_str_is_always_text_never_a_path(engine: Any, tmp_path: Path) -> None:
    """A string that names an existing file is still analysed as text."""
    path = tmp_path / "notes.md"
    path.write_text("# Heading\n\nBody text.\n")

    doc = engine.analyze(str(path))

    assert doc["provenance"]["source"]["path"] is None
    assert doc["provenance"]["format"] == "plain_text"
    assert doc["sections"][0]["paragraphs"][0]["text"] == str(path)


@pytest.mark.model
def test_a_path_object_is_not_text(engine: Any, tmp_path: Path) -> None:
    with pytest.raises(TypeError):
        engine.analyze(tmp_path / "notes.md")


@pytest.mark.model
def test_texts_refuses_a_bare_str(engine: Any) -> None:
    """A ``str`` is iterable, one character at a time; refusing it here
    keeps a forgotten list from becoming a stream of letters."""
    with pytest.raises(TypeError, match="a str alone is one text"):
        engine.analyze_all(texts=PROSE)


@pytest.mark.model
def test_format_applies_to_text_and_bytes_only(engine: Any, tmp_path: Path) -> None:
    path = tmp_path / "notes.txt"
    path.write_text(PROSE)
    with pytest.raises(TypeError, match="format applies to text and bytes"):
        engine.analyze(file=path, format="markdown")
    with pytest.raises(TypeError, match="format applies to text and bytes"):
        engine.analyze(blocks=[], format="blocks")
    with pytest.raises(TypeError, match="format applies to texts"):
        engine.analyze_all(dir=tmp_path, format="markdown")


@pytest.mark.model
def test_an_unknown_format_is_invalid_input(engine: Any) -> None:
    with pytest.raises(matra.InvalidInputError, match="html"):
        engine.analyze(PROSE, format="html")


# ---------------------------------------------------------------------------
# Each input keyword
# ---------------------------------------------------------------------------


@pytest.mark.model
def test_text_reads_as_its_format(engine: Any) -> None:
    text = "# Field notes\n\nThe river rose overnight.\n"
    plain = engine.analyze(text)
    markdown = engine.analyze(text, format="markdown")

    assert plain["provenance"]["format"] == "plain_text"
    assert markdown["provenance"]["format"] == "markdown"
    assert markdown["sections"][0]["heading"] == "Field notes"


@pytest.mark.model
def test_bytes_are_detected_or_read_as_named(engine: Any) -> None:
    data = b"# Field notes\n\nThe river rose overnight.\n"
    assert engine.analyze(bytes=data)["provenance"]["format"] == "plain_text"
    assert engine.analyze(bytes=data, format="markdown")["provenance"]["format"] == "markdown"
    with pytest.raises(matra.UnsupportedFormatError):
        engine.analyze(bytes=b"%PDF-1.7\n")


@pytest.mark.model
def test_blocks_are_the_blocks_format(engine: Any) -> None:
    """Python data and the JSON form read the same: one reader defines
    the vocabulary."""
    blocks = [
        {"kind": "heading", "level": 1, "text": "Field notes"},
        {"kind": "paragraph", "text": "The river rose overnight."},
    ]
    doc = engine.analyze(blocks=blocks)
    via_json = engine.analyze(
        '[{"kind": "heading", "level": 1, "text": "Field notes"},'
        ' {"kind": "paragraph", "text": "The river rose overnight."}]',
        format="blocks",
    )

    assert doc["provenance"]["format"] == "blocks"
    assert doc["sections"][0]["heading"] == "Field notes"
    assert doc.to_dict() == via_json.to_dict()


@pytest.mark.model
@pytest.mark.parametrize(
    ("blocks", "named"),
    [
        ([{"kind": "table", "text": "x"}], "table"),
        ([{"kind": "paragraph", "text": "x", "style": "bold"}], "style"),
        ([{"kind": "heading", "level": 0, "text": "x"}], "level 0"),
        ({"kind": "paragraph", "text": "x"}, "block array"),
    ],
)
def test_bad_blocks_are_refused_by_name(engine: Any, blocks: Any, named: str) -> None:
    with pytest.raises(matra.InvalidInputError, match=named):
        engine.analyze(blocks=blocks)


@pytest.mark.model
def test_a_directory_named_as_a_file_names_analyze_all(engine: Any, tmp_path: Path) -> None:
    with pytest.raises(matra.InvalidInputError):
        engine.analyze(file=tmp_path)


@pytest.mark.model
def test_analyze_all_is_lazy_and_in_order(engine: Any) -> None:
    """Nothing is pulled from ``texts`` until the stream is."""
    pulled: list[int] = []

    def texts() -> Any:
        for i, text in enumerate(["The first text.", "The second text."]):
            pulled.append(i)
            yield text

    stream = engine.analyze_all(texts=texts())
    assert pulled == []
    first = next(stream)
    assert pulled == [0]
    assert first["sections"][0]["paragraphs"][0]["text"] == "The first text."
    rest = list(stream)
    assert pulled == [0, 1]
    assert [d["sections"][0]["paragraphs"][0]["text"] for d in rest] == ["The second text."]


@pytest.mark.model
def test_a_stream_of_one_is_one_analysis(engine: Any) -> None:
    """Law L1 across the binding: a stream of one gives what one call gives."""
    (item,) = engine.analyze_all(texts=[PROSE], format="markdown")
    assert item.to_dict() == engine.analyze(PROSE, format="markdown").to_dict()


@pytest.mark.model
def test_a_text_that_is_not_a_str_raises_from_the_stream(engine: Any) -> None:
    stream = engine.analyze_all(texts=[PROSE, 42])
    next(stream)
    with pytest.raises(TypeError, match="texts must hold str"):
        next(stream)


# ---------------------------------------------------------------------------
# The read-only Document
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def doc(engine: Any) -> Any:
    return engine.analyze(PROSE)


@pytest.mark.model
def test_a_document_reads_as_its_json(doc: Any) -> None:
    whole = doc.to_dict()
    assert list(doc) == list(whole) == doc.keys()
    assert len(doc) == len(whole)
    for key in doc:
        assert key in doc
        assert doc[key] == whole[key]
    assert doc["metrics"]["total_words"] == 10
    assert "words" not in doc
    assert 1 not in doc


@pytest.mark.model
def test_a_document_cannot_be_changed(doc: Any) -> None:
    with pytest.raises((TypeError, AttributeError)):
        doc["metrics"] = {}
    with pytest.raises(AttributeError):
        doc.metrics = {}
    # What a read returns is a copy: changing it changes nothing here.
    metrics = doc["metrics"]
    metrics["total_words"] = 0
    doc.to_dict()["sections"].clear()
    assert doc["metrics"]["total_words"] == 10
    assert doc["sections"]


@pytest.mark.model
def test_an_unknown_key_is_a_key_error(doc: Any) -> None:
    with pytest.raises(KeyError):
        doc["words"]
    with pytest.raises(KeyError):
        doc[0]


@pytest.mark.model
def test_a_document_does_not_pickle(doc: Any) -> None:
    """There is no constructor from data in 0.3.0, so nothing could
    rebuild one on the other side; ``to_dict()`` crosses instead."""
    with pytest.raises(TypeError):
        pickle.dumps(doc)
    assert pickle.loads(pickle.dumps(doc.to_dict())) == doc.to_dict()


@pytest.mark.model
def test_the_repr_names_the_document(doc: Any) -> None:
    assert repr(doc).startswith("<matra.Document: ")


# ---------------------------------------------------------------------------
# The outcome functions
# ---------------------------------------------------------------------------


@pytest.mark.model
@pytest.mark.parametrize(
    "call",
    [
        pytest.param(lambda d: matra.summarize(d), id="summarize"),
        pytest.param(lambda d: matra.keyphrases(d), id="keyphrases"),
        pytest.param(lambda d: matra.clusters(d, matra.Model2Vec), id="clusters"),
    ],
)
def test_an_outcome_takes_a_document_and_no_dict(doc: Any, call: Any) -> None:
    """A dict cannot carry provenance into a result no matra run produced
    (decision 6)."""
    with pytest.raises(TypeError, match="Document"):
        call(doc.to_dict())


@pytest.mark.model
def test_the_defaults_are_recorded(doc: Any) -> None:
    summary = matra.summarize(doc)
    found = matra.keyphrases(doc)
    assert summary["provenance"]["method"] == "tfidf"
    assert summary["provenance"]["parameters"] == {"n": 3}
    assert found["provenance"]["method"] == "rake"
    assert found["provenance"]["parameters"] == {"n": 10}


@pytest.mark.model
@pytest.mark.parametrize("method", ["tfidf", "textrank"])
def test_a_summary_method_is_recorded_as_named(doc: Any, method: str) -> None:
    summary = matra.summarize(doc, 1, method=method)
    assert summary["provenance"]["method"] == method
    assert summary["provenance"]["parameters"] == {"n": 1}
    assert summary["provenance"]["source"] == doc["provenance"]["source"]


@pytest.mark.model
@pytest.mark.parametrize("method", ["rake", "yake"])
def test_a_keyphrase_method_is_recorded_as_named(doc: Any, method: str) -> None:
    found = matra.keyphrases(doc, 2, method=method)
    assert found["provenance"]["method"] == method
    assert found["provenance"]["parameters"] == {"n": 2}


@pytest.mark.model
def test_an_unknown_method_is_invalid_input(doc: Any) -> None:
    with pytest.raises(matra.InvalidInputError, match="lsa"):
        matra.summarize(doc, method="lsa")
    with pytest.raises(matra.InvalidInputError, match="tfidf"):
        matra.keyphrases(doc, method="tfidf")


@pytest.mark.model
def test_a_file_path_travels_into_an_outcome(engine: Any, tmp_path: Path) -> None:
    path = tmp_path / "minutes.txt"
    path.write_text(PROSE)
    summary = matra.summarize(engine.analyze(file=path), 1)
    assert summary["provenance"]["source"]["path"] == str(path)


# ---------------------------------------------------------------------------
# Spans
# ---------------------------------------------------------------------------


def test_span_text_counts_bytes_not_characters() -> None:
    text = "Café au lait. Tea."
    assert matra.span_text(text, {"start": 0, "end": 5}) == "Café"
    assert matra.span_text(text.encode(), {"start": 15, "end": 19}) == "Tea."
    with pytest.raises(UnicodeDecodeError):
        matra.span_text(text, {"start": 0, "end": 4})


@pytest.mark.model
def test_every_token_span_slices_its_text(engine: Any) -> None:
    text = "Café owners in Zürich approved the plan."
    doc = engine.analyze(text)
    for section in doc["sections"]:
        for paragraph in section["paragraphs"]:
            for sentence in paragraph["sentences"]:
                for token in sentence["tokens"]:
                    assert matra.span_text(text, token["span"]) == token["text"]
