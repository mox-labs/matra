"""The published document schema, as the Python runners check against it.

The Python twin of ``tests/document_schema/mod.rs``, and stricter than the
published schema in the same one way: every object definition that lists
its properties is closed (``additionalProperties: false``) before
validating. The published schema leaves objects open, because a minor
version adds fields a reader of the older minor must accept; matra's own
output must not carry a field the schema for its version does not list.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import jsonschema

SCHEMA = Path(__file__).resolve().parents[2] / "spec" / "schema" / "v2" / "document.schema.json"


def _close(node: Any) -> None:
    if isinstance(node, dict):
        if "properties" in node and "additionalProperties" not in node:
            node["additionalProperties"] = False
        for value in node.values():
            _close(value)
    elif isinstance(node, list):
        for value in node:
            _close(value)


def assert_valid_document(document: Any, context: str) -> None:
    """Validate ``document`` against schema 2, closed, naming every violation."""
    schema = json.loads(SCHEMA.read_text())
    _close(schema)
    validator = jsonschema.Draft202012Validator(schema)
    errors = [
        f"{error.message} at /{'/'.join(str(p) for p in error.absolute_path)}"
        for error in validator.iter_errors(document)
    ]
    assert not errors, f"{context}: the document does not match schema 2:\n  " + "\n  ".join(errors)
