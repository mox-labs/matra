"""Launcher for the matra command line. The command itself is Rust."""

import sys

from matra._core import cli_main


def main() -> None:
    """Run the matra CLI and exit with the code it returns."""
    raise SystemExit(cli_main(sys.argv[1:]))
