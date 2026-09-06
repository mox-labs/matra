# Use the matra CLI

`matra` reads a file (or stdin) and writes to stdout. `matra --help` describes the tool in the line built into the command: "matra parses text into CoNLL-U structure and measures it. It reports what is there; interpretation is yours."

## Install

The command ships two ways, and both run the same program. The Python package's `matra` command is the Rust CLI reached through the extension module, not a second implementation, so the flags, the output, and the exit codes are the same whichever route you took.

```bash
# Rust binary, no Python involved
cargo install matra --features cli

# or through the Python package
uv add matra          # then: matra --help
uvx matra --help      # or without installing anything
```

Both cache the model at the resolved model directory (about 16 MB) and download it the first time any command needs it. If the cached file fails its SHA-256 check, matra removes it and downloads once more before giving up.

## Where matra keeps things

| | Path | Override |
|---|---|---|
| config file | `$XDG_CONFIG_HOME/matra/config.toml`, else `~/.config/matra/config.toml` | `MATRA_CONFIG_FILE` |
| data root | `$XDG_DATA_HOME/matra`, else `~/.local/share/matra` | `MATRA_DATA_DIR` |
| models | the data root's `models` directory | `MATRA_MODEL_DIR`, or `--model-dir` on the command line |

A pre-existing `~/.matra/models` from an older install is still read when the new location does not exist yet. matra never writes there.

`--model-dir` outranks every environment variable, which outranks the config file, which outranks the defaults compiled into the crate. `matra config show` prints which rung each value actually came from.

## Commands

### `matra analyze`

Analyzes a file and reports its metrics.

```bash
matra analyze essay.md
matra analyze essay.md --sections
matra analyze essay.md --json
```

`--sections` adds a per-section breakdown under the metric table: one row per section with its heading level, heading, and the paragraph, sentence, and word counts underneath it.

Every subcommand routes through the library pipeline, which rejects symlinks, rejects files over 8 MiB before reading them, and picks a decomposer from the extension.

On a markdown file, decomposition drops YAML frontmatter, fenced code blocks, and table rows beginning with `|`, and it stops entirely at a line reading `## References` or `*References*`. Content after that heading is not analyzed and does not appear in the output.

`--json` emits the full `Document` (`sections`, `vocabulary_ttr`, `nominalization_ratio`, `passive_ratio`) inside the envelope described below:

```bash
matra analyze essay.md --json | jq '.result.sections[0].paragraphs[0].readability_grade'
```

That query returns `null` more often than you might expect. Per-paragraph metrics have thresholds: `readability_grade` needs more than 10 words, `compression_ratio` needs more than 50, and blockquote paragraphs are skipped entirely. Filter for non-null before aggregating:

```bash
matra analyze essay.md --json \
  | jq '[.result.sections[].paragraphs[].readability_grade | select(. != null)] | add / length'
```

### `matra summarize`

Extracts the top-N sentences as an extractive summary.

```bash
matra summarize essay.md
matra summarize essay.md -n 5 --method textrank
```

| Option | Default | Values |
|---|---|---|
| `-n` | `summarize.n` from the config, `3` as shipped | any positive integer |
| `--method` | `summarize.algorithm` from the config, `tfidf` as shipped | `tfidf`, `textrank` |

TF-IDF scores each sentence by term frequency against the document. TextRank scores by a graph-coherence measure over sentence similarity. Reach for TextRank when sentences reference each other across the document and mutual reinforcement matters more than raw term weight. TF-IDF is the cheaper default and usually enough. Both reject input over 2000 sentences.

Both methods select their top-N sentences by score internally, then return that selection re-sorted into document order, not score order. The `score` field tells you how each sentence ranked; the list order tells you where each sentence sits in the source text.

```bash
matra summarize essay.md --json | jq '.result[0]'
# {"text": "...", "score": 0.312, "position": 4}
```

### `matra keyphrases`

Extracts ranked keyphrases.

```bash
matra keyphrases essay.md
matra keyphrases essay.md -n 20 --method yake
```

| Option | Default | Values |
|---|---|---|
| `-n` | `keyphrases.n` from the config, `10` as shipped | any positive integer |
| `--method` | `keyphrases.algorithm` from the config, `rake` as shipped | `rake`, `yake` |

RAKE splits on stop words and scores the remaining word runs by a co-occurrence degree-to-frequency ratio. It is rule-based and fast. YAKE scores individual terms by position, frequency, and how varied their surrounding context is, then builds one-word to three-word candidates from those scores. It tends to surface longer, more specific phrases and to notice terms that first appear later in a long document, at more computation than RAKE. Both reject input over 200000 tokens.

Unlike `summarize`, `keyphrases` output stays in score order, highest first. `Keyphrase` has no position field to sort back into, because a phrase is not tied to one location the way a sentence is.

Phrases are printed as lowercased lemmas, not as the surface text of the document. "Dependency Parses" appears as `dependency parse`. Phrases with equal scores can swap order between runs, and a tie at the `-n` boundary can change which phrase makes the list, so do not diff raw keyphrase output across runs without sorting it first.

```bash
matra keyphrases essay.md --json | jq -S '.result[0:3]'
```

### `matra config`

`matra config show` prints every resolved value with the rung it came from, one key per line, in the shape `cargo config get --show-origin` uses:

```bash
matra config show
```

```
data_dir = "/home/you/.local/share/matra" # default
model_dir = "/home/you/.local/share/matra/models" # default
models.udpipe = "english-ewt-ud-2.5-191206" # default
models.embedding = "potion-base-8M" # default
semantic.threshold = 0.85 # /home/you/.config/matra/config.toml
summarize.n = 3 # default
summarize.algorithm = "tfidf" # default
keyphrases.n = 10 # default
keyphrases.algorithm = "rake" # default
```

The origin is `default` for a value compiled into the crate, a path for one read from your config file, `environment variable ...` for one an environment variable set, and `command line` for one a flag set. With `--json`, each key carries its value, the rung, and what the rung pointed at.

`matra config init` writes the shipped defaults to the resolved config path and prints where it wrote them. It creates the parent directories, writes through a temporary file in the same directory so a reader never sees a half-written config, and refuses to overwrite an existing file unless you pass `--force`.

```bash
matra config init
matra config init --force
```

### `matra completions`

Prints a completion script for `bash`, `zsh`, or `fish`.

```bash
matra completions zsh > ~/.zfunc/_matra
matra completions bash > /etc/bash_completion.d/matra
```

## Reading from stdin

`-` in place of the path reads stdin. `--stdin-filename` gives that input a name, which is the label in the output and the envelope, and whose extension selects the decomposer. Without it the input is read as plain text and labelled `<stdin>`.

```bash
pandoc notes.docx -t markdown | matra analyze - --stdin-filename notes.md
cat essay.txt | matra keyphrases -
```

The 8 MiB cap applies to stdin as it does to a file, and it is enforced while reading rather than after, so an unbounded stream is refused instead of buffered.

## Color, quiet, and version

`--color` takes `auto` (the default), `always`, or `never`. Under `auto`, matra colors an interactive terminal and nothing else, and honors `NO_COLOR`: if that variable is present and not empty, color is off whatever its value. `--color always` and `--color never` are explicit requests and outrank `NO_COLOR` in both directions.

`--quiet` suppresses the human-readable output while leaving the exit code alone, so a script can branch on "found something" without collecting the table. It has no effect on `--json`.

`--version` prints the version and then the features this build was compiled with:

```
matra 0.1.0
features: udpipe model2vec python cli
```

## The JSON envelope

`--json` emits one object for every command:

```json
{
  "format_version": 1,
  "command": "analyze",
  "input": "essay.md",
  "result": { "sections": [], "vocabulary_ttr": null }
}
```

`command` is `analyze`, `summarize`, `keyphrases`, or `config`. `input` is the path that was read, or the name `--stdin-filename` gave it. `result` is the serde form of the domain value the command produced: a `Document` for `analyze`, a list of `ScoredSentence` for `summarize`, a list of `Keyphrase` for `keyphrases`.

Stability: `format_version` increments on any change to the envelope or to a field's meaning; the `result` value is the serde form of the documented domain types. That is the same promise cargo makes for `cargo metadata --format-version`, and it is the whole promise. Pin your consumer to a `format_version` you have tested against.

## Exit codes

`0` on success when the command found something, `1` on success when it found nothing (an empty summary, no keyphrases, a file with zero sentences), and `2` when an error occurred. Model load failure, a missing or unreadable file, input over the size cap, a bad argument, and a parse failure all land on `2`, with the message on stderr prefixed `matra:`. A broken pipe, which is what you get piping into `head`, is treated as success and exits `0`.

```bash
matra keyphrases notes.txt > /dev/null
case $? in
  0) echo "phrases found" ;;
  1) echo "no phrases" ;;
  2) echo "failed" ;;
esac
```
