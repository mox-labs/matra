# CLI guide

Three commands: `analyze`, `summarize`, `keyphrases`. Each reads a file and writes to stdout. The model downloads automatically on first use.

## Installation

```bash
pip install vaani
vaani --help
```

On first run, the English UDPipe model downloads to `~/.vaani/models/` (~16 MB). Subsequent calls load from cache.

## `vaani analyze`

Analyze a file. Prints document-level metrics.

```bash
vaani analyze essay.md
vaani analyze report.txt
```

Output:

```
           vaani: essay.md
┌──────────────────────┬────────┐
│ Metric               │ Value  │
├──────────────────────┼────────┤
│ Sentences            │ 42     │
│ Words                │ 631    │
│                      │        │
│ Passive ratio        │ 14.3%  │
│ Mean sentence length │ 15.0   │
│ Vocabulary TTR       │ 0.61   │
│ Nominalization ratio │  8.3%  │
└──────────────────────┴────────┘
```

**Options:**

`--sections` / `-s` adds a per-section breakdown (heading, paragraph count, sentence count, word count).

`--json` outputs the full `Document` as JSON. The JSON structure is the same dict shape documented in the Python guide.

```bash
vaani analyze essay.md --json | jq '.sections[0].paragraphs[0].readability_grade'
```

Format detection is by file extension: `.md` / `.markdown` uses the markdown decomposer; all other extensions are treated as plain text.

## `vaani summarize`

Extract the top-N sentences as an extractive summary.

```bash
vaani summarize essay.md
vaani summarize essay.md -n 5
vaani summarize essay.md --method textrank -n 3
```

**Options:**

| Option | Default | Values |
|---|---|---|
| `-n` | 3 | any positive integer |
| `--method` | `tfidf` | `tfidf`, `textrank` |
| `--json` | off | flag |

TF-IDF scores sentences by term frequency relative to the document. TextRank uses a graph-coherence score based on sentence similarity. Both return sentences ranked by score, not document order; the `position` field in JSON output preserves original position.

```bash
vaani summarize essay.md --json | jq '.[0]'
# {"text": "...", "score": 0.312, "position": 4}
```

## `vaani keyphrases`

Extract ranked keyphrases.

```bash
vaani keyphrases essay.md
vaani keyphrases essay.md -n 20 --method yake
```

**Options:**

| Option | Default | Values |
|---|---|---|
| `-n` | 10 | any positive integer |
| `--method` | `rake` | `rake`, `yake` |
| `--json` | off | flag |

RAKE is rule-based: it scores phrases by co-occurrence of content words and runs fast. YAKE is positional and statistical: it weights earlier appearances higher and tends to extract longer, more specific phrases. For most use cases RAKE is sufficient. Use YAKE when you need phrases that surface later in a long document or when RAKE returns too many single-word results.

```bash
vaani keyphrases essay.md --json | jq '.[0:3]'
# [{"phrase": "dependency parsing", "score": 4.0}, ...]
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Model load failed (download error, corrupt file) |
| 1 | File not found or not readable |
| 1 | Input exceeds the 8 MiB size cap |

All error messages go to stderr via the console; stdout carries only the result.
