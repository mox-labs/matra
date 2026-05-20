# CLI

Installing the Python wheel adds a `vaani` command. Auto-downloads the English UDPipe model into `~/.vaani/models/` on first use.

## `vaani analyze`

Run the full pipeline (parse + metrics) on a file and print a table.

```bash
vaani analyze essay.md

vaani: essay.md
┌──────────────────────┬─────────┐
│ Metric               │ Value   │
├──────────────────────┼─────────┤
│ Sentences            │ 312     │
│ Words                │ 5847    │
│                      │         │
│ Passive ratio        │ 8.3%    │
│ Mean sentence length │ 18.7    │
│ Vocabulary TTR       │ 0.42    │
│ Nominalization ratio │ 12.1%   │
└──────────────────────┴─────────┘
```

Options:

- `--json` (or `--json-output`): emit the serialized `Analysis` dict instead of the table.
- `-s` / `--sections`: show a per-section breakdown table after the document metrics.

## `vaani summarize`

Extract the top-N sentences as an extractive summary.

```bash
vaani summarize essay.md
vaani summarize essay.md -n 5
vaani summarize essay.md -n 3 --method textrank
vaani summarize essay.md --json
```

Options:

- `-n N`: number of sentences (default 3).
- `--method tfidf` / `--method textrank`: summarization algorithm (default `tfidf`).
- `--json`: emit JSON instead of formatted output.

## `vaani keyphrases`

Extract ranked keyphrases.

```bash
vaani keyphrases paper.md
vaani keyphrases paper.md -n 20 --method yake
vaani keyphrases paper.md --json
```

Options:

- `-n N`: maximum number of keyphrases (default 10).
- `--method rake` / `--method yake`: extraction algorithm (default `rake`).
- `--json`: emit JSON instead of formatted output.

## Working with directories

The CLI is single-file today. For corpus-level analysis, drop into Python:

```python
from pathlib import Path
from vaani import Vaani

v = Vaani.english(str(Path.home() / ".vaani" / "models"))

for md in Path("./docs").glob("**/*.md"):
    a = v.analyze_markdown(md.read_text())
    sents = sum(len(p["sentences"]) for s in a["sections"] for p in s["paragraphs"])
    print(f"{md}: {sents} sentences")
```

Or use the Rust `analyze_directory` API directly.

## Exit codes

- `0`: success.
- `1`: model load failed (caught at startup and reported to stderr).
- Non-zero on uncaught exceptions (per click's default behavior).
