# Installation

Install vaani and verify it works. By the end, you have a loaded model and a confirmed working pipeline.

---

## Requirements

**Rust:** 1.85 or later (MSRV). Check with `rustc --version`.

**Python:** 3.12 or later. Check with `python --version`.

---

## Install the Rust crate

Add vaani to your `Cargo.toml`:

```toml
[dependencies]
vaani = "0.0"
```

**Version pin note.** `"0.0"` pins you to the 0.0.x alpha line. The first release intended for downstream dependents is 0.1.0. If you need a reproducible build before 0.1.0 ships, pin the exact patch version (`"0.0.1"`). The alpha API is not considered stable across minor bumps.

---

## Install the Python package

```bash
pip install vaani
```

This installs the wheel built from the Rust core via PyO3 and maturin. No separate build step.

---

## Download the English model

vaani uses UDPipe for parsing. The English model is not bundled in the wheel; it downloads on first use. The call below downloads the model (~16 MB) into `~/.vaani/models` and caches it there.

```python
from pathlib import Path
from vaani import Vaani

v = Vaani.english(str(Path.home() / ".vaani" / "models"))
```

On subsequent calls with the same directory, the cached file is loaded directly. No network access after the first call.

**What happens during download.** The file is written to a temporary subdirectory, then atomically renamed into place. If the process is interrupted mid-download, the next call restarts cleanly. The downloaded bytes are SHA-256-verified against a pinned hash in the source before loading; a file that fails verification is re-downloaded once, then rejected if it still mismatches. The bytes that pass the hash check are the same bytes loaded into memory. There is no second disk read between verify and load, closing the window a swap attack lives in.

If you have a UDPipe model file already, use `Vaani.from_path("/path/to/model.udpipe")` instead.

---

## Verify the install

Run this snippet to confirm that the install, model download, and analysis pipeline all work:

```python
from pathlib import Path
from vaani import Vaani

v = Vaani.english(str(Path.home() / ".vaani" / "models"))

text = (
    "The committee approved the proposal without debate. "
    "Three amendments were submitted by the working group."
)
result = v.analyze(text)

print("sections:", len(result["sections"]))
print("vocabulary_ttr:", result["vocabulary_ttr"])
```

Expected output (values reflect the loaded model):

```
sections: 1
vocabulary_ttr: 0.8...
```

If `Vaani.english()` raises `RuntimeError`, the model download failed. Check your network connection and re-run. If it raises `FileNotFoundError`, the path you passed does not exist yet. `Vaani.english()` creates it; `Vaani.from_path()` does not.

---

## What you have

After these steps:

- `vaani` is installed as a Rust crate and/or as a Python package.
- The English UDPipe model is cached at `~/.vaani/models`.
- `v.analyze(text)` returns a `Document` dict you can traverse.

Next: [Quickstart](./quickstart.md) walks through your first structured parse, shows the full result shape, and runs summarization and keyphrase extraction on real text.
