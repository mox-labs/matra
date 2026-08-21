# Use the matra CLI

Three subcommands, `analyze`, `summarize`, and `keyphrases`. Each reads one file and writes to stdout. `matra --help` describes the tool in the line built into the binary: "matra parses text into CoNLL-U structure and measures it. It reports what is there; interpretation is yours."

## Install

The `matra` command ships two ways, and they are not the same program underneath.

```bash
# Rust binary, no Python dependency
cargo install matra --features cli

# or the Python entry point
uv add matra
```

Both cache the model at `~/.matra/models/english-ewt-ud-2.5-191206.udpipe` (about 16 MB) and download it the first time any command needs it. If the cached file fails its SHA-256 check, both routes remove it and download once more before giving up.

## Which route you have

The two routes agree on the three subcommands, their algorithm choices, and the exact JSON payload. They differ everywhere else. Check this table before you script against either one.

| | Rust binary | Python entry point |
|---|---|---|
| `--model-dir DIR` | available, accepted anywhere on the line | not available, always `~/.matra/models` |
| `--json` position | anywhere, before or after the subcommand | after the subcommand only; `--json-output` is an accepted alias |
| `--sections` / `-s` on `analyze` | not available | available |
| markdown detection in `analyze` | `.md` and `.markdown` | `.md` only |
| `analyze` table rows | sentences, words, mean sentence length, sentence length standard deviation, passive ratio | sentences, words, passive ratio, mean sentence length, vocabulary TTR, nominalization ratio |
| 8 MiB text cap on `summarize` and `keyphrases` | enforced | not enforced; only the per-algorithm caps apply |
| exit codes | 0 found, 1 nothing found, 2 error | 0 on any completed command, 1 on model or library failure, 2 on a bad argument |
| `--json` payload | identical | identical |

If your workflow depends on a per-invocation model directory, on the `.markdown` extension, or on an exit code that distinguishes an empty result, use the Rust binary. If you want the per-section breakdown, use the Python entry point.

The Rust binary resolves its default model directory from `$HOME`. In an environment with no `$HOME`, it fails with a message telling you to pass `--model-dir`.

## `matra analyze`

Analyzes a file and reports its metrics.

```bash
matra analyze essay.md
matra analyze essay.md --json
```

The Rust binary routes every subcommand through the library pipeline, which rejects symlinks, rejects files over 8 MiB before reading them, and picks a decomposer from the extension.

On a markdown file, decomposition drops YAML frontmatter, fenced code blocks, and table rows beginning with `|`, and it stops entirely at a line reading `## References` or `*References*`. Content after that heading is not analyzed and does not appear in the output.

`--json` emits the full `Document` (`sections`, `vocabulary_ttr`, `nominalization_ratio`), the same shape documented in the Python guide:

```bash
matra analyze essay.md --json | jq '.sections[0].paragraphs[0].readability_grade'
```

That query returns `null` more often than you might expect. Per-paragraph metrics have thresholds: `readability_grade` needs more than 10 words, `compression_ratio` needs more than 50, and blockquote paragraphs are skipped entirely. Filter for non-null before aggregating:

```bash
matra analyze essay.md --json \
  | jq '[.sections[].paragraphs[].readability_grade | select(. != null)] | add / length'
```

## `matra summarize`

Extracts the top-N sentences as an extractive summary.

```bash
matra summarize essay.md
matra summarize essay.md -n 5 --method textrank
```

| Option | Default | Values |
|---|---|---|
| `-n` | `3` | any positive integer |
| `--method` | `tfidf` | `tfidf`, `textrank` |
| `--json` | off | flag |

TF-IDF scores each sentence by term frequency against the document. TextRank scores by a graph-coherence measure over sentence similarity. Reach for TextRank when sentences reference each other across the document and mutual reinforcement matters more than raw term weight. TF-IDF is the cheaper default and usually enough. Both reject input over 2000 sentences.

Both methods select their top-N sentences by score internally, then return that selection re-sorted into document order, not score order. The `score` field tells you how each sentence ranked; the list order tells you where each sentence sits in the source text.

```bash
matra summarize essay.md --json | jq '.[0]'
# {"text": "...", "score": 0.312, "position": 4}
```

## `matra keyphrases`

Extracts ranked keyphrases.

```bash
matra keyphrases essay.md
matra keyphrases essay.md -n 20 --method yake
```

| Option | Default | Values |
|---|---|---|
| `-n` | `10` | any positive integer |
| `--method` | `rake` | `rake`, `yake` |
| `--json` | off | flag |

RAKE splits on stop words and scores the remaining word runs by a co-occurrence degree-to-frequency ratio. It is rule-based and fast. YAKE scores individual terms by position, frequency, and how varied their surrounding context is, then builds one-word to three-word candidates from those scores. It tends to surface longer, more specific phrases and to notice terms that first appear later in a long document, at more computation than RAKE. Both reject input over 200000 tokens.

Unlike `summarize`, `keyphrases` output stays in score order, highest first. `Keyphrase` has no position field to sort back into, because a phrase is not tied to one location the way a sentence is.

Phrases are printed as lowercased lemmas, not as the surface text of the document. "Dependency Parses" appears as `dependency parse`. Phrases with equal scores can swap order between runs, and a tie at the `-n` boundary can change which phrase makes the list, so do not diff raw keyphrase output across runs without sorting it first.

```bash
matra keyphrases essay.md --json | jq -S '.[0:3]'
```

## What `summarize` and `keyphrases` feed the extractors

The two routes differ here. The Rust binary runs the file through the pipeline first, so on a `.md` file the extractors rank prose: frontmatter, fenced code, table rows, and blockquotes are stripped before anything is scored, and the symlink and file-size checks apply.

The Python entry point reads the file as a string and hands it to the extraction methods, which treat it as plain text. Paragraphs still split on blank lines and the 8 MiB cap still applies, but no markdown stripping happens: heading hashes, list markers, link brackets, and code fence contents go into the parse as prose and can appear inside a summarized sentence or a keyphrase. The symlink and file-size pre-read checks do not run on this route either, because the Python script does its own file read.

If you want markdown structure stripped before summarizing, use the Rust binary, or pre-process the file yourself.

## Exit codes

The Rust binary follows the convention documented in its source: `0` on success when the command found something, `1` on success when it found nothing (an empty summary, no keyphrases, a file with zero sentences), and `2` when an error occurred. Model load failure, a missing or unreadable file, input over the size cap, and a parse failure all land on `2`, with the message on stderr prefixed `matra:`. A broken pipe, which is what you get piping into `head`, is treated as success and exits `0`.

```bash
matra keyphrases notes.txt > /dev/null
case $? in
  0) echo "phrases found" ;;
  1) echo "no phrases" ;;
  2) echo "failed" ;;
esac
```

The Python entry point does not implement that three-way convention. A command that completes exits `0` even when it printed an empty result. A model that fails to load exits `1` after printing a red message. Any other library failure, such as input over an extraction cap, propagates as an uncaught Python exception, which exits `1` with a traceback rather than a formatted message. A missing file is caught by argument parsing before any work starts and exits `2` with a usage message.

Scripts that branch on "found nothing" need the Rust binary. Scripts that only need success or failure work on either route, as long as they treat any non-zero code as failure.
