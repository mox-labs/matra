"""Conformance tests: the Python crust against the shared spec.

Every fixture in ``spec/tests/`` runs through the Python API and is checked
against the same expectations the Rust crust checks. A difference between
crusts is a binding defect (a renamed field, a lost value, a rounded number),
not a difference in behaviour.

Requires the UDPipe model:

    uv run pytest python/tests/test_conformance.py
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

import pytest
from document_schema import assert_valid

# The extension module is built by maturin. Without a built wheel there is
# nothing to conform, so skip rather than fail collection.
matra = pytest.importorskip("matra", reason="matra wheel not built")

TOLERANCE = 1e-6

SPEC_DIR = Path(__file__).resolve().parents[2] / "spec" / "tests"


def load_fixtures() -> list[dict[str, Any]]:
    fixtures = [json.loads(path.read_text()) for path in sorted(SPEC_DIR.glob("*.json"))]
    assert fixtures, f"no fixtures found in {SPEC_DIR}"
    return fixtures


def paragraphs_of(doc: dict[str, Any]) -> list[dict[str, Any]]:
    return [paragraph for section in doc["sections"] for paragraph in section["paragraphs"]]


def check_spans(source: bytes, doc: dict[str, Any]) -> None:
    """Every token's span slices the input to its form; a sentence runs
    from its first token to its last, inside its paragraph."""
    for paragraph in paragraphs_of(doc):
        outer = paragraph["span"]
        for sentence in paragraph["sentences"]:
            span = sentence["span"]
            assert outer["start"] <= span["start"] <= span["end"] <= outer["end"]
            if sentence["tokens"]:
                assert span["start"] == sentence["tokens"][0]["span"]["start"]
                assert span["end"] == sentence["tokens"][-1]["span"]["end"]
            for token in sentence["tokens"]:
                piece = source[token["span"]["start"] : token["span"]["end"]]
                assert piece.decode() == token["text"], f"token {token['id']} span"


def check_accounting(text: str, doc: dict[str, Any]) -> None:
    """L8: every byte of the input that is not whitespace lies in exactly
    one heading, paragraph or skipped span."""
    source = text.encode()
    owners = [0] * len(source)
    spans = [s["span"] for s in doc["sections"] if s["span"] is not None]
    spans += [p["span"] for p in paragraphs_of(doc)]
    spans += [s["span"] for s in doc["skipped"]]
    for span in spans:
        for at in range(span["start"], span["end"]):
            owners[at] += 1
    at = 0
    for char in text:
        width = len(char.encode())
        if not char.isspace():
            assert owners[at] == 1, f"byte {at} ({char!r}) lies in {owners[at]} spans"
        at += width


def sentences_of(doc: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        sentence
        for section in doc["sections"]
        for paragraph in section["paragraphs"]
        for sentence in paragraph["sentences"]
    ]


@pytest.mark.model
@pytest.mark.parametrize("fixture", load_fixtures(), ids=lambda f: f["name"])
def test_python_crust_conforms_to_spec(engine: Any, fixture: dict[str, Any]) -> None:
    text = fixture["input"]
    assert fixture["format"] in ("markdown", "plain_text"), fixture["format"]
    analysed = engine.analyze(text, format=fixture["format"])
    doc = analysed.to_dict()

    assert_valid("document", doc, fixture["name"])
    # Every outcome's result matches its schema, with the method and the
    # count it ran with recorded.
    for summary_method in ("tfidf", "textrank"):
        summary = matra.summarize(analysed, 3, method=summary_method)
        assert_valid("summary", summary, f"{fixture['name']} {summary_method}")
        assert summary["provenance"]["method"] == summary_method
        assert summary["provenance"]["parameters"] == {"n": 3}
    for keyphrase_method in ("rake", "yake"):
        found = matra.keyphrases(analysed, 10, method=keyphrase_method)
        assert_valid("keyphrases", found, f"{fixture['name']} {keyphrase_method}")
        assert found["provenance"]["method"] == keyphrase_method

    # Provenance: this build's schema, the parser's own identity, and the
    # input as it was handed in.
    provenance = doc["provenance"]
    assert provenance["schema"] == "2.0", "schema"
    assert provenance["format"] == fixture["format"], "format"
    assert provenance["parser"]["name"] == "udpipe", "parser name"
    assert provenance["parser"]["model"] == "english-ewt-ud-2.5-191206", "parser model"
    assert provenance["parser"]["license"], "parser licence"
    assert provenance["source"]["path"] is None, "in-memory input"
    assert provenance["source"]["bytes"] == len(text.encode()), "input bytes"
    assert provenance["source"]["sha256"] == hashlib.sha256(text.encode()).hexdigest()

    expect = fixture["expect"]
    metrics = doc["metrics"]
    got_sentences = sentences_of(doc)

    check_spans(text.encode(), doc)
    check_accounting(text, doc)
    if "sections" in expect:
        got = [
            {"heading": s["heading"], "level": s["level"], "span": s["span"]}
            for s in doc["sections"]
        ]
        assert got == expect["sections"], "sections"
    if "paragraphs" in expect:
        got = [{"kind": p["kind"], "span": p["span"]} for p in paragraphs_of(doc)]
        assert got == expect["paragraphs"], "paragraphs"
    if "skipped" in expect:
        assert doc["skipped"] == expect["skipped"], "skipped"

    assert metrics["total_sentences"] == expect["metrics"]["total_sentences"], "sentence count"
    assert len(got_sentences) == expect["metrics"]["total_sentences"], "sentences attached"
    assert metrics["total_words"] == expect["metrics"]["total_words"], "word count"
    assert metrics["paragraph_count"] == expect["metrics"]["paragraph_count"], "paragraph count"

    assert len(got_sentences) == len(expect["sentences"]), "sentences returned"

    for i, (got, want) in enumerate(zip(got_sentences, expect["sentences"], strict=True)):
        assert got["text"] == want["text"], f"sentence {i} text"
        if "span" in want:
            assert got["span"] == want["span"], f"sentence {i} span"
        if "negations" in want:
            assert got["negations"] == want["negations"], f"sentence {i} negations"
        if "modals" in want:
            assert got["modals"] == want["modals"], f"sentence {i} modals"
        if "root_mood" in want:
            assert got["root_mood"] == want["root_mood"], f"sentence {i} root_mood"
        if "clausal_complements" in want:
            assert got["clausal_complements"] == want["clausal_complements"], (
                f"sentence {i} clausal_complements"
            )
        if "root_adverbials" in want:
            assert got["root_adverbials"] == want["root_adverbials"], (
                f"sentence {i} root_adverbials"
            )
        if "hearst_pairs" in want:
            assert got["hearst_pairs"] == want["hearst_pairs"], f"sentence {i} hearst_pairs"
        assert len(got["tokens"]) == want["token_count"], f"sentence {i} token count"
        for j, (token, wanted) in enumerate(zip(got["tokens"], want["tokens"], strict=True)):
            where = f"sentence {i} token {j}"
            assert token["id"] == wanted["id"], f"{where} id"
            if "span" in wanted:
                assert token["span"] == wanted["span"], f"{where} span"
            assert token["text"] == wanted["text"], f"{where} text"
            assert token["lemma"] == wanted["lemma"], f"{where} lemma"
            assert token["pos"] == wanted["pos"], f"{where} pos"
            assert token["head"] == wanted["head"], f"{where} head"
            assert token["dep"] == wanted["dep"], f"{where} dep"

    for ratio in ("passive_ratio", "vocabulary_ttr", "nominalization_ratio"):
        want = expect["metrics"].get(ratio)
        if want is not None:
            assert abs(metrics[ratio] - want) < TOLERANCE, ratio
