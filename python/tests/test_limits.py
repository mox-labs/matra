"""``matra.limits``: Rust's ``domain::limits``, name for name and value for
value. Needs no model."""

from __future__ import annotations

import re
from pathlib import Path

import pytest

matra = pytest.importorskip("matra", reason="matra wheel not built")

ROOT = Path(__file__).resolve().parents[2]


def rust_limits() -> set[str]:
    """The ``pub const`` names inside ``pub mod limits`` in the domain."""
    source = (ROOT / "src" / "domain.rs").read_text()
    body = source.split("pub mod limits {", 1)[1].split("\n}\n", 1)[0]
    return set(re.findall(r"pub const ([A-Z_]+): usize", body))


def test_python_names_every_rust_limit() -> None:
    names = rust_limits()
    assert len(names) == 11, names
    assert set(matra.limits.__all__) == names


def test_the_limits_hold_their_published_values() -> None:
    """The same pins as the domain's own test: a change is made on
    purpose, with the docs that state it."""
    assert {name: getattr(matra.limits, name) for name in matra.limits.__all__} == {
        "MAX_INPUT_BYTES": 8_388_608,
        "MAX_CONFIG_FILE_BYTES": 65_536,
        "MAX_DOWNLOAD_BYTES": 67_108_864,
        "MAX_TFIDF_SENTENCES": 2000,
        "MAX_TEXTRANK_SENTENCES": 2000,
        "MAX_CLUSTER_SENTENCES": 2000,
        "MAX_RAKE_TOKENS": 200_000,
        "MAX_YAKE_TOKENS": 200_000,
        "MAX_COMPRESSION_PARAGRAPH_BYTES": 262_144,
        "MIN_READABILITY_WORDS": 11,
        "MIN_COMPRESSION_WORDS": 51,
    }
