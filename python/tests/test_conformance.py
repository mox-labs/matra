"""Conformance tests: the Python crust against the shared spec.

Every fixture in ``spec/tests/`` runs through the Python API and is checked
against the same expectations the Rust crust checks. A difference between
crusts is a binding defect (a renamed field, a lost value, a rounded number),
not a difference in behaviour.

Requires the UDPipe model:

    uv run pytest python/tests/test_conformance.py
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

import pytest

# The extension module is built by maturin. Without a built wheel there is
# nothing to conform, so skip rather than fail collection.
Matra = pytest.importorskip("matra", reason="matra wheel not built").Matra

TOLERANCE = 1e-6

SPEC_DIR = Path(__file__).resolve().parents[2] / "spec" / "tests"


def model_dir() -> str:
    override = os.environ.get("MATRA_MODEL_DIR")
    if override:
        return override
    return str(Path.home() / ".matra" / "models")


def load_fixtures() -> list[dict[str, Any]]:
    fixtures = [json.loads(path.read_text()) for path in sorted(SPEC_DIR.glob("*.json"))]
    assert fixtures, f"no fixtures found in {SPEC_DIR}"
    return fixtures


def sentences_of(doc: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        sentence
        for section in doc["sections"]
        for paragraph in section["paragraphs"]
        for sentence in paragraph["sentences"]
    ]


def paragraph_count(doc: dict[str, Any]) -> int:
    return sum(len(section["paragraphs"]) for section in doc["sections"])


@pytest.fixture(scope="module")
def matra() -> Matra:
    return Matra.english(model_dir())


@pytest.mark.model
@pytest.mark.parametrize("fixture", load_fixtures(), ids=lambda f: f["name"])
def test_python_crust_conforms_to_spec(matra: Matra, fixture: dict[str, Any]) -> None:
    text = fixture["input"]
    if fixture["format"] == "markdown":
        doc = matra.analyze_markdown(text)
    elif fixture["format"] == "plain":
        doc = matra.analyze(text)
    else:
        pytest.fail(f"unknown format {fixture['format']}")

    expect = fixture["expect"]
    got_sentences = sentences_of(doc)

    assert len(got_sentences) == expect["total_sentences"], "sentence count"
    words = sum(len([t for t in s["tokens"] if not t["is_punct"]]) for s in got_sentences)
    assert words == expect["total_words"], "word count"
    assert paragraph_count(doc) == expect["paragraph_count"], "paragraph count"

    assert len(got_sentences) == len(expect["sentences"]), "sentences returned"

    for i, (got, want) in enumerate(zip(got_sentences, expect["sentences"], strict=True)):
        assert got["text"] == want["text"], f"sentence {i} text"
        if "negations" in want:
            assert got["negations"] == want["negations"], f"sentence {i} negations"
        if "modals" in want:
            assert got["modals"] == want["modals"], f"sentence {i} modals"
        if "bare_assertion" in want:
            assert got["bare_assertion"] == want["bare_assertion"], f"sentence {i} bare_assertion"
        if "reportings" in want:
            assert got["reportings"] == want["reportings"], f"sentence {i} reportings"
        if "root_adverbials" in want:
            assert got["root_adverbials"] == want["root_adverbials"], f"sentence {i} root_adverbials"
        assert len(got["tokens"]) == want["token_count"], f"sentence {i} token count"
        for j, (token, wanted) in enumerate(zip(got["tokens"], want["tokens"], strict=True)):
            where = f"sentence {i} token {j}"
            assert token["id"] == wanted["id"], f"{where} id"
            assert token["text"] == wanted["text"], f"{where} text"
            assert token["lemma"] == wanted["lemma"], f"{where} lemma"
            assert token["pos"] == wanted["pos"], f"{where} pos"
            assert token["head"] == wanted["head"], f"{where} head"
            assert token["dep"] == wanted["dep"], f"{where} dep"

    if expect.get("passive_ratio") is not None:
        assert abs(doc["passive_ratio"] - expect["passive_ratio"]) < TOLERANCE
    if expect.get("vocabulary_ttr") is not None:
        assert abs(doc["vocabulary_ttr"] - expect["vocabulary_ttr"]) < TOLERANCE
    if expect.get("nominalization_ratio") is not None:
        assert abs(doc["nominalization_ratio"] - expect["nominalization_ratio"]) < TOLERANCE
