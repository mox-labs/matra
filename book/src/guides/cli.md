# Use the matra CLI

`matra` reads a file (or stdin) and writes to stdout. `matra --help` describes the tool in the line built into the command: "matra parses text into CoNLL-U structure and measures it. It reports what is there; interpretation is yours."

## Install

Nothing installed, nothing configured:

```bash
uvx matra analyze essay.md
```

The command ships two ways beyond that, and all three run the same program. The Python package's `matra` command is the Rust CLI reached through the extension module, not a second implementation, so the flags, the output, and the exit codes are the same whichever route you took.

```bash
# Rust binary, no Python involved
cargo install matra --features cli

# or through the Python package
uv add matra          # then: matra --help
```

Every route caches the model at the resolved model directory (about 16 MB) and downloads it the first time any command needs it. No flag and no environment variable is required to make that happen. If the cached file fails its SHA-256 check, matra downloads a replacement and only then removes it, so a fetch that fails leaves the file you had.

## Where matra keeps things

| | Path | Override |
|---|---|---|
| config file | `$XDG_CONFIG_HOME/matra/config.toml`, else `~/.config/matra/config.toml` | `MATRA_CONFIG_FILE` |
| data root | `$XDG_DATA_HOME/matra`, else `~/.local/share/matra` | `MATRA_DATA_DIR` |
| models | the data root's `models` directory | `MATRA_MODEL_DIR`, or `--model-dir` on the command line |

A pre-existing `~/.matra/models` from an older install is still used when the new location does not exist yet. matra never creates `~/.matra`; when a non-empty legacy cache is selected it is the resolved model directory, downloads and re-downloads included. Create the new location, or set `MATRA_MODEL_DIR`, to move off it.

Per key, `--model-dir` outranks every environment variable, which outranks the config file, which outranks the defaults compiled into the crate. The config file has no key for the model directory, so that value comes from the flag, the environment, or the defaults. `matra config show` prints which rung each value actually came from.

## Commands

`analyze`, `summarize`, and `keyphrases` each read one document. The path they take is a file or `-` for stdin; a directory is refused, because a command that reports on one document cannot report on a directory of them, and picking one file out of it would be an answer to a question nobody asked. Analyzing a directory is a library call today (`Engine::analyze` over `Ingest::path`).

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

## For an agent

`matra --skill` prints the agent skill: what matra is for, when to reach for it, every command with its JSON shape, how to read each number and what it does not mean, the limits, and the errors. It is the whole hand-off. If you are pointing an agent at matra, `uvx matra --skill` is the line to give it.

```bash
matra --skill                 # the skill itself
matra --skill -r              # the references, one per line: name, then summary
matra --skill -r json         # one reference
```

The text is compiled into the program with `include_str!` from `skills/matra/SKILL.md` and `skills/matra/references/*.md`, so what you read is what the installed version does, not a copy of the docs that may have moved on. The same files are what the repository ships as a plugin. Every command in them is executed against the command line by the test suite, so an incantation that no longer runs fails CI.

`--skill` outranks a subcommand: `matra analyze essay.md --skill` prints the skill and never looks at the file. `-r` without `--skill` is an error that says so, and an unknown reference name exits `2` naming the ones that exist. Everything else exits `0`. `--quiet` does not apply, as it does not to `completions`: the text is the command's output, not a rendering of a result.

Under `--json` the skill uses the same envelope every other command uses, with `input` null because the command reads no document:

```json
{
  "format_version": 1,
  "command": "skill",
  "input": null,
  "result": { "name": "SKILL", "body": "---\nname: matra\n..." }
}
```

`result.name` is `SKILL` for the top level and the reference's own name for `matra --skill -r <name> --json`. The list replaces `result` with one key:

```json
{
  "format_version": 1,
  "command": "skill",
  "input": null,
  "result": { "references": [{ "name": "json", "summary": "The JSON envelope every command emits, ..." }] }
}
```

The list is every file under `references/`, in file-name order, and each summary is read from that file's own frontmatter rather than from a second list that could drift from it.

The same files install as a plugin: `claude --plugin-dir <checkout>` over a clone of the repository picks up `skills/matra/`, which is where plugin discovery looks. And this documentation site serves [`llms.txt`](https://mox-labs.github.io/matra/llms.txt) at its root, the map of every page with its one-line summary, for an agent reading the human door instead.

## Reading from stdin

`-` in place of the path reads stdin. `--stdin-filename` gives that input a name, which is the label in the output and the envelope, and whose extension selects the decomposer. Without it the input is read as plain text and labelled `<stdin>`.

```bash
pandoc notes.docx -t markdown | matra analyze - --stdin-filename notes.md
cat essay.txt | matra keyphrases -
```

The 8 MiB cap applies to stdin as it does to a file, and it is enforced while reading rather than after, so an unbounded stream is refused instead of buffered.

## Color, quiet, and version

`--color` takes `auto` (the default), `always`, or `never`. Under `auto`, matra colors an interactive terminal and nothing else, and honors `NO_COLOR`: if that variable is present and not empty, color is off whatever its value. `--color always` and `--color never` are explicit requests and outrank `NO_COLOR` in both directions.

`--quiet` suppresses the human-readable output while leaving the exit code alone, so a script can branch on "found something" without collecting the table. It has no effect on `--json`. It also silences the one line the command prints to standard error before it downloads a model, which names the artifact, its size and where it is going. That line is the only output a first run produces during a wait that can reach half a minute, so silence it deliberately rather than by habit.

`--version` prints the version and then the features this build was compiled with:

```
matra 0.2.0
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

`command` is `analyze`, `summarize`, `keyphrases`, `config`, or `skill`. `input` is the path that was read, or the name `--stdin-filename` gave it, and it is null for `skill`, which reads no document. `result` is the serde form of the domain value the command produced: a `Document` for `analyze`, a list of `ScoredSentence` for `summarize`, a list of `Keyphrase` for `keyphrases`.

Stability: `format_version` increments on any change to the envelope or to a field's meaning; the `result` value is the serde form of the documented domain types. That is the same promise cargo makes for `cargo metadata --format-version`, and it is the whole promise. Pin your consumer to a `format_version` you have tested against.

## Exit codes

`0` on success when the command found something, `1` on success when it found nothing (an empty summary, no keyphrases, a file with zero sentences), and `2` when an error occurred. Model load failure, a missing or unreadable file, a directory where a file was expected, input over the size cap, a bad argument, and a parse failure all land on `2`, with the message on stderr prefixed `matra:`. A broken pipe, which is what you get piping into `head`, is treated as success and exits `0`.

```bash
matra keyphrases notes.txt > /dev/null
case $? in
  0) echo "phrases found" ;;
  1) echo "no phrases" ;;
  2) echo "failed" ;;
esac
```
