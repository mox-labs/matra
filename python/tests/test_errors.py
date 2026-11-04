"""Every failure crosses into Python as a class of its own (EPR-0006,
milestone 4): ``kind`` to branch on, ``details`` to read values from, and
the builtin class matra raised before 0.3.0 still in its bases.

No model is needed, apart from the one test marked ``model``. Each
failure is one the binding can produce without a parser: a configuration
file too large, malformed or missing, and an embedding directory that is
empty or that matra may not read.
"""

from __future__ import annotations

import os
import pickle
from pathlib import Path
from typing import Any

import matra
import pytest
from document_schema import assert_valid

# The class each kind raises as, and the builtin it also derives from.
# `src/python/errors.rs` (`Class::spec`) is the source; `tests/error_tables.rs` holds
# the documentation to it, and this holds the built module to it.
CLASSES: dict[str, tuple[str, type[BaseException]]] = {
    "ModelNotFoundError": ("model_not_found", FileNotFoundError),
    "ModelInvalidError": ("model_invalid", RuntimeError),
    "ParseFailedError": ("parse_failed", RuntimeError),
    "InputTooLargeError": ("input_too_large", ValueError),
    "UnsupportedFormatError": ("unsupported_format", ValueError),
    "InvalidInputError": ("invalid_input", ValueError),
    "IoError": ("io", OSError),
    "IoNotFoundError": ("io", FileNotFoundError),
    "IoPermissionError": ("io", PermissionError),
}


def _as_data(error: matra.MatraError) -> dict[str, object]:
    """The exception as the serde form of the error it carries."""
    return {"kind": error.kind, "message": str(error), "details": error.details}


@pytest.mark.parametrize("name", sorted(CLASSES))
def test_every_class_is_a_matra_error_and_its_builtin(name: str) -> None:
    kind, builtin = CLASSES[name]
    cls = getattr(matra, name)
    assert issubclass(cls, matra.MatraError)
    assert issubclass(cls, builtin)
    assert cls.kind == kind
    assert cls.__module__ == "matra"
    assert name in matra.__all__


def test_the_classes_cover_every_kind() -> None:
    """A kind with no class would raise as some other kind's class."""
    assert {kind for kind, _ in CLASSES.values()} == set(matra.ERROR_KINDS)


def test_the_io_refinements_are_io() -> None:
    """``except matra.IoError`` catches every I/O failure, refined or not."""
    assert issubclass(matra.IoNotFoundError, matra.IoError)
    assert issubclass(matra.IoPermissionError, matra.IoError)


CONFIG_CAP = 64 * 1024


def _oversized_config(tmp_path: Path) -> Path:
    path = tmp_path / "config.toml"
    path.write_bytes(b"#" * (CONFIG_CAP + 1))
    return path


def test_an_oversized_input_is_input_too_large_with_its_gate(tmp_path: Path) -> None:
    """The configuration file's gate, which ``Engine.from_config`` reaches
    before any model is loaded."""
    with pytest.raises(ValueError) as caught:
        matra.Engine.from_config(_oversized_config(tmp_path))

    error = caught.value
    assert isinstance(error, matra.InputTooLargeError)
    assert error.kind == "input_too_large"
    assert error.details == {
        "what": "config_file",
        "limit": CONFIG_CAP,
        "actual": CONFIG_CAP + 1,
    }
    assert_valid("error", _as_data(error), "input_too_large")


def test_a_broken_contract_is_invalid_input_with_its_reason(tmp_path: Path) -> None:
    path = tmp_path / "config.toml"
    path.write_text('[summary]\nmethod = "lsa"\n')
    with pytest.raises(ValueError) as caught:
        matra.Engine.from_config(path)

    error = caught.value
    assert isinstance(error, matra.InvalidInputError)
    assert error.details["reason"] in str(error)
    assert "lsa" in str(error)
    assert_valid("error", _as_data(error), "invalid_input")


def test_a_named_config_file_that_is_missing_is_io_not_found(tmp_path: Path) -> None:
    """A file the caller names must exist, unlike the one the environment
    names, whose absence leaves the defaults standing."""
    missing = tmp_path / "absent.toml"
    with pytest.raises(FileNotFoundError) as caught:
        matra.Engine.from_config(missing)

    error = caught.value
    assert isinstance(error, matra.IoNotFoundError)
    assert error.details["path"] == str(missing)
    assert_valid("error", _as_data(error), "io")


def test_a_missing_model_is_model_not_found_with_its_path(tmp_path: Path) -> None:
    """An embedding directory without its artifacts: the first one looked
    for is named."""
    with pytest.raises(FileNotFoundError) as caught:
        matra.Model2Vec.from_dir(tmp_path)

    error = caught.value
    assert isinstance(error, matra.ModelNotFoundError)
    assert set(error.details) == {"path"}
    assert Path(error.details["path"]).parent == tmp_path
    assert_valid("error", _as_data(error), "model_not_found")


@pytest.mark.skipif(
    os.name != "posix" or os.geteuid() == 0, reason="needs a file this user may not read"
)
def test_an_unreadable_file_is_io_permission_denied(tmp_path: Path) -> None:
    """Defect 8 of EPR-0006's other half: ``Error::Io`` routes on the kind it
    wraps, so a refused read is a ``PermissionError``, and the path it
    concerns is a detail."""
    for name in ("model.safetensors", "tokenizer.json", "config.json"):
        artifact = tmp_path / name
        artifact.write_bytes(b"{}")
        artifact.chmod(0)
    try:
        with pytest.raises(PermissionError) as caught:
            matra.Model2Vec.from_dir(tmp_path)
    finally:
        for name in ("model.safetensors", "tokenizer.json", "config.json"):
            (tmp_path / name).chmod(0o600)

    error = caught.value
    assert isinstance(error, matra.IoPermissionError)
    assert error.kind == "io"
    assert error.details["io_kind"] == "permission_denied"
    assert Path(error.details["path"]).parent == tmp_path
    assert_valid("error", _as_data(error), "io")


def test_an_exception_survives_pickling(tmp_path: Path) -> None:
    """The classes name ``matra`` as their module and the package exports
    them, so an exception crosses a process boundary whole."""
    with pytest.raises(matra.InputTooLargeError) as caught:
        matra.Engine.from_config(_oversized_config(tmp_path))

    copy = pickle.loads(pickle.dumps(caught.value))
    assert type(copy) is matra.InputTooLargeError
    assert str(copy) == str(caught.value)
    assert copy.kind == caught.value.kind
    assert copy.details == caught.value.details


@pytest.mark.model
def test_a_missing_file_is_io_not_found_with_its_path(engine: Any, tmp_path: Path) -> None:
    """Regression (EPR-0006 milestone 4): a missing input's message named
    no path. It names it, and the path is a detail beside the I/O kind."""
    missing = tmp_path / "absent.md"

    with pytest.raises(FileNotFoundError) as caught:
        engine.analyze(file=missing)

    error = caught.value
    assert isinstance(error, matra.IoNotFoundError)
    assert error.kind == "io"
    assert error.details["io_kind"] == "not_found"
    assert error.details["path"] == str(missing)
    assert str(missing) in str(error)
    assert_valid("error", _as_data(error), "io")
