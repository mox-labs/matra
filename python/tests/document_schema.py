"""The published schemas, as the Python runners check against them.

The Python twin of ``tests/document_schema/mod.rs``, and stricter than the
published schemas in the same two ways. Every object definition that lists
its properties is closed (``additionalProperties: false``) before
validating: the published schemas leave objects open, because a minor
version adds fields a reader of the older minor must accept, and matra's
own output must not carry a field the schema for its version does not
list. And the three keys the schema lets ``matra analyze --json --depth``
remove (``sections``, a paragraph's ``sentences``, a sentence's
``tokens``) are required again, because Python always returns all three.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import jsonschema
from referencing import Registry, Resource

SCHEMAS = Path(__file__).resolve().parents[2] / "spec" / "schema" / "v2"


def _close(node: Any) -> None:
    if isinstance(node, dict):
        if "properties" in node and "additionalProperties" not in node:
            node["additionalProperties"] = False
        for value in node.values():
            _close(value)
    elif isinstance(node, list):
        for value in node:
            _close(value)


def _require_every_level(document: Any) -> None:
    for node, key in (
        (document, "sections"),
        (document["$defs"]["Paragraph"], "sentences"),
        (document["$defs"]["Sentence"], "tokens"),
    ):
        if key not in node["required"]:
            node["required"].append(key)


def _closed(name: str) -> Any:
    schema = json.loads((SCHEMAS / f"{name}.schema.json").read_text())
    _close(schema)
    if name == "document":
        _require_every_level(schema)
    return schema


def assert_valid(name: str, value: Any, context: str) -> None:
    """Validate ``value`` against ``spec/schema/v2/{name}.schema.json``,
    closed, naming every violation.

    ``name`` is ``document``, ``summary``, ``keyphrases``, ``clusters`` or ``error``.
    The outcome schemas refer to the document schema's definitions by its
    ``$id``, so the document schema, closed the same way, is registered
    under it.
    """
    document = _closed("document")
    registry = Registry().with_resource(document["$id"], Resource.from_contents(document))
    validator = jsonschema.Draft202012Validator(_closed(name), registry=registry)
    errors = [
        f"{error.message} at /{'/'.join(str(p) for p in error.absolute_path)}"
        for error in validator.iter_errors(value)
    ]
    assert not errors, f"{context}: the {name} does not match schema 2:\n  " + "\n  ".join(errors)
