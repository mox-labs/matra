"""What every Python lane shares: one engine per session.

The engine locates the UDPipe model the way every matra surface does,
from the environment (``MATRA_MODEL_DIR`` in CI), and downloads it on
first use. A run where the model cannot be had skips the tests that ask
for it rather than failing them; those tests carry the ``model`` marker.
"""

from __future__ import annotations

import pytest

matra = pytest.importorskip("matra", reason="matra wheel not built")


@pytest.fixture(scope="session")
def engine() -> matra.Engine:
    try:
        return matra.Engine()
    except Exception as exc:
        pytest.skip(f"UDPipe model unavailable: {exc}")
