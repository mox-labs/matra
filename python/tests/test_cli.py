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

ROOT = Path(__file__).resolve().parents[2]
SPEC = ROOT / "spec" / "tests" / "cli" / "envelope.json"
SKILL = ROOT / "skills" / "matra" / "SKILL.md"


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


def test_the_skill_is_the_file_verbatim(capsys: pytest.CaptureFixture[str]) -> None:
    """The parity claim, on the text an agent is handed.

    `tests/cli.rs` asserts the same file through `cli::run`. Both
    launchers printing the file itself is what makes `uvx matra --skill`
    and the Rust binary interchangeable in a hand-off, and it is why the
    text is embedded rather than fetched.
    """
    assert cli_main(["--skill"]) == 0
    assert capsys.readouterr().out == SKILL.read_text()


def test_a_reference_is_the_file_verbatim(capsys: pytest.CaptureFixture[str]) -> None:
    reference = SKILL.parent / "references" / "json.md"
    assert cli_main(["--skill", "-r", "json"]) == 0
    assert capsys.readouterr().out == reference.read_text()


def test_an_unknown_reference_exits_two_and_names_the_known_ones(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert cli_main(["--skill", "-r", "jsn"]) == 2
    captured = capsys.readouterr()
    assert captured.out == ""
    for name in sorted(path.stem for path in (SKILL.parent / "references").glob("*.md")):
        assert name in captured.err


def test_a_missing_file_exits_two_on_stderr(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert cli_main(["analyze", str(tmp_path / "absent.txt")]) == 2
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "no such file" in captured.err


def test_a_directory_is_refused_by_name(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """The refusal reaches the launcher as an exit code, not an exception."""
    assert cli_main(["analyze", str(tmp_path)]) == 2
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "is a directory" in captured.err
    assert "pass a file" in captured.err


def test_a_non_utf8_argument_survives_the_crossing(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """An undecodable path is a path, not a crash.

    On unix a filename is bytes, and Python surfaces one it cannot decode
    with surrogate escapes. The launcher re-encodes with `os.fsencode`, so
    the command line sees the bytes the Rust binary would have seen. The
    file does not exist, so the run stops at the existence check with no
    model in sight, and what this asserts is that it got that far at all:
    an argument rejected at the boundary would raise instead.
    """
    path = str(tmp_path / "caf\udcff.txt")
    assert cli_main(["analyze", path]) == 2
    captured = capsys.readouterr()
    assert captured.out == ""
    assert "no such file" in captured.err


def test_an_argument_that_is_not_a_path_is_a_type_error() -> None:
    """A silently dropped argument would run a different command."""
    with pytest.raises(TypeError):
        cli_main(["analyze", 7])  # type: ignore[list-item]


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
