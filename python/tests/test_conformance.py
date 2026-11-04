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
import os
from pathlib import Path
from typing import Any

import pytest
from document_schema import assert_valid_document

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


@pytest.fixture(scope="module")
def matra() -> Matra:
    return Matra.english(model_dir())


@pytest.mark.model
@pytest.mark.parametrize("fixture", load_fixtures(), ids=lambda f: f["name"])
def test_python_crust_conforms_to_spec(matra: Matra, fixture: dict[str, Any]) -> None:
    text = fixture["input"]
    if fixture["format"] == "markdown":
        doc = matra.analyze_markdown(text)
    elif fixture["format"] == "plain_text":
        doc = matra.analyze(text)
    else:
        pytest.fail(f"unknown format {fixture['format']}")

    assert_valid_document(doc, fixture["name"])

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

    assert metrics["total_sentences"] == expect["metrics"]["total_sentences"], "sentence count"
    assert len(got_sentences) == expect["metrics"]["total_sentences"], "sentences attached"
    assert metrics["total_words"] == expect["metrics"]["total_words"], "word count"
    assert metrics["paragraph_count"] == expect["metrics"]["paragraph_count"], "paragraph count"

    assert len(got_sentences) == len(expect["sentences"]), "sentences returned"

    for i, (got, want) in enumerate(zip(got_sentences, expect["sentences"], strict=True)):
        assert got["text"] == want["text"], f"sentence {i} text"
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
            assert token["text"] == wanted["text"], f"{where} text"
            assert token["lemma"] == wanted["lemma"], f"{where} lemma"
            assert token["pos"] == wanted["pos"], f"{where} pos"
            assert token["head"] == wanted["head"], f"{where} head"
            assert token["dep"] == wanted["dep"], f"{where} dep"

    for ratio in ("passive_ratio", "vocabulary_ttr", "nominalization_ratio"):
        want = expect["metrics"].get(ratio)
        if want is not None:
            assert abs(metrics[ratio] - want) < TOLERANCE, ratio
