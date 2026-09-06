"""The Python launcher against the shared CLI fixture.

`python/matra/cli.py` carries no behaviour: it hands `sys.argv[1:]` to
`_core.cli_main` and exits with the code. So the thing worth testing is
not the launcher's four lines, it is that the command reached through it
produces the same envelope the Rust runner asserts from the same file
(`spec/tests/cli/envelope.json`). Two launchers checked against one
contract, rather than against each other's output.

The envelope test requires the UDPipe model:

    uv run pytest python/tests/test_cli.py
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

cli_main = pytest.importorskip("matra._core", reason="matra wheel not built").cli_main

SPEC = Path(__file__).resolve().parents[2] / "spec" / "tests" / "cli" / "envelope.json"


def test_version_names_the_version_then_the_compiled_features(
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Launcher parity with no model in sight: same program, same text."""
    assert cli_main(["--version"]) == 0
    lines = capsys.readouterr().out.splitlines()
    assert lines[0].startswith("matra ")
    assert lines[1].startswith("features:")
    assert "udpipe" in lines[1]
    assert "python" in lines[1]


def test_a_missing_file_exits_two_on_stderr(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert cli_main(["analyze", str(tmp_path / "absent.txt")]) == 2
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "no such file" in captured.err


@pytest.mark.model
def test_envelope_matches_the_conformance_fixture(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    spec: dict[str, Any] = json.loads(SPEC.read_text())
    source = tmp_path / spec["filename"]
    source.write_text(spec["input"])

    subcommand, flag = spec["args"]
    assert cli_main([subcommand, str(source), flag]) == 0
    got = json.loads(capsys.readouterr().out)
    expect = spec["expect"]

    assert sorted(got) == expect["envelope_keys"]
    assert got["format_version"] == expect["format_version"]
    assert got["command"] == expect["command"]
    assert got["input"] == str(source)

    assert sorted(got["result"]) == expect["result_keys"]
    sections = got["result"]["sections"]
    assert len(sections) == expect["result_sections"]
    sentences = sum(
        len(paragraph["sentences"]) for section in sections for paragraph in section["paragraphs"]
    )
    assert sentences == expect["result_sentences"]
