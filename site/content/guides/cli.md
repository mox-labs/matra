# Use the matra CLI

`matra` reads a file, a directory or stdin and writes to stdout. `matra --help` describes the tool in the line built into the command: "matra parses text into CoNLL-U structure and measures it. It reports what is there; interpretation is yours."

## Install

Nothing installed, nothing configured:

```bash
uvx 'matra>=0.3' analyze essay.md
```

The version floor is load-bearing. A bare `uvx matra` takes whatever release is newest, which is 0.3.x today and will not stay that way, and 0.1.0 and 0.2 are still on PyPI for anything pinned to them. 0.2's command line spells its JSON and its config file differently; [Migrate from 0.2 to 0.3](migrate-to-0-3.md#command-line) lists each change. 0.1.0's `analyze` is a separate Python implementation rather than this one missing a flag: `--json` prints a bare document with no envelope, the human table is a different renderer, and the model cache lands in `~/.matra/models` instead of the resolved model directory. It does not fail, it succeeds and gives you something else, so pin it.

With the floor in place the command ships two ways beyond that, and all three run the same program. The Python package's `matra` command is the Rust CLI reached through the extension module, not a second implementation, so the flags, the output, and the exit codes are the same whichever route you took. One thing does differ: which features were compiled in. `cargo install matra --version '^0.3' --features cli` reports `features: udpipe cli`, while the published wheel reports `features: udpipe model2vec python cli`, so the embedding adapter is present on the Python route and absent from a `--features cli` build, whose `matra clusters` refuses and names the feature. `cargo install matra --features cli,model2vec` builds one with it. `matra --version` prints the list of the binary in front of you on its second line.

```bash
# Rust binary, no Python involved
cargo install matra --version '^0.3' --features cli

# or through the Python package
uv add 'matra>=0.3'   # then: matra --help
```

Every route caches the model at the resolved model directory (about 16 MB) and downloads it the first time any command needs it. No flag and no environment variable is required to make that happen. If the cached file fails its SHA-256 check, matra never deletes it: a download that verifies replaces it through an atomic rename, and a download that fails leaves the file you had exactly as it was.

## Where matra keeps things

| What | Path | Override |
|---|---|---|
| config file | `$XDG_CONFIG_HOME/matra/config.toml`, else `~/.config/matra/config.toml` | `MATRA_CONFIG_FILE` |
| data root | `$XDG_DATA_HOME/matra`, else `~/.local/share/matra` | `MATRA_DATA_DIR` |
| models | the data root's `models` directory | `MATRA_MODEL_DIR`, or `--model-dir` on the command line |

A pre-existing `~/.matra/models` from an older install is still used when the new location does not exist yet. matra never creates `~/.matra`; when a non-empty legacy cache is selected it is the resolved model directory, downloads and re-downloads included. Create the new location, or set `MATRA_MODEL_DIR`, to move off it.

Per key, `--model-dir` outranks every environment variable, which outranks the config file, which outranks the defaults compiled into the crate. The config file has no key for the model directory, so that value comes from the flag, the environment, or the defaults. `matra config show` prints which rung each value actually came from.

## Commands

`analyze`, `summarize`, `keyphrases` and `clusters` each read a file, or `-` for stdin. `analyze` also reads a directory, one result per file. The other three report on one document, so they refuse a directory and name `analyze`, which reads one; picking one file out of it would answer a question nobody asked.

Every subcommand routes through the library pipeline, which rejects symlinks, rejects files over 8 MiB before reading them, and picks a reader from the file's bytes and extension. `--format markdown|plain_text|blocks` names the format instead, which is how a JSON file of blocks is read, since no extension implies one. A PDF or DOCX signature in the bytes still outranks it.

```bash
matra analyze notes.json --format blocks --json
```

### `matra analyze`

Analyzes a file and reports its metrics.

```bash
matra analyze essay.md
matra analyze essay.md --sections
matra analyze essay.md --json
```

`--sections` adds a per-section breakdown under the metric table: one row per section with its heading level, heading, and the paragraph, sentence, and word counts underneath it.

Given a directory, `analyze` reads every regular file in it, one level deep and in name order, and reports each one: a table per file, or under `--json` one envelope per line. A file that fails is reported where it happens, on stderr and under `--json` as its own line with its `error`, and the walk goes on to the next file. The exit code is `2` if any file failed. Symlinks and subdirectories are skipped, as the library's `Inputs::dir` skips them. `--format` is refused with a directory, because each of its files is read as its own name says.

```bash
matra analyze docs/ --json --depth document | jq -c '{input, words: .result.metrics.total_words}'
```

`--depth` decides how much of the parse `--json` writes. `document` writes the provenance, the metrics and `skipped`, and no `sections`; `paragraph` adds the sections and their paragraphs, with no sentences; `sentence` adds the sentences, with no tokens; and `token`, the default, writes everything. A level below the depth is absent as a key, never an empty array, because an empty `sentences` would say a paragraph has none, and what remains keeps the whole document's key order, so the two diff line for line. Tokens are most of a document's JSON, so a caller that reads only the measures pays for no more than it reads. `--depth` without `--json` is a usage error.

On a markdown file, front matter, code blocks, tables, HTML blocks, thematic breaks and link reference definitions are not analyzed. They are listed in the JSON's `skipped`, each with its `kind` and `span`, rather than dropped. `## References` is a heading like any other.

`--json` emits the full `Document` (`provenance`, `sections`, and `metrics`, which carries the counts and ratios the table prints) inside the envelope described below:

```bash
matra analyze essay.md --json | jq '.result.sections[0].paragraphs[0].metrics.readability_grade'
```

That query returns `null` more often than you might expect. Per-paragraph metrics have thresholds: `readability_grade` needs more than 10 words, `compression_ratio` needs more than 50, and quotes are skipped entirely. Filter for non-null before aggregating:

```bash
matra analyze essay.md --json \
  | jq '[.result.sections[].paragraphs[].metrics.readability_grade | select(. != null)] | add / length'
```

### `matra summarize`

Extracts the top-N sentences as an extractive summary.

```bash
matra summarize essay.md
matra summarize essay.md -n 5 --method textrank
```

| Option | Default | Values |
|---|---|---|
| `-n` | `summary.n` from the config, `3` as shipped | any positive integer |
| `--method` | `summary.method` from the config, `tfidf` as shipped | `tfidf`, `textrank` |

TF-IDF scores each sentence by term frequency against the document. TextRank scores by a graph-coherence measure over sentence similarity. TF-IDF is the default because it is cheap and needs no graph, not because it picks better sentences. It favours sentences dense in the document's rarer terms, and on narrative prose that can mean short sentences that make little sense on their own. Reach for TextRank when the summary should carry the document's main thread, when sentences restate and build on each other, or when the TF-IDF picks read as fragments. Read both on your own text before you rely on either. Both reject input over 2000 sentences.

Both methods select their top-N sentences by score internally, then return that selection re-sorted into document order, not score order. The `score` field tells you how each sentence ranked; the list order tells you where each sentence sits in the source text.

The human table prints each score at three decimals. Two sentences whose scores differ in the fourth therefore print the same number, and a summary of three sentences can arrive looking as though the ranker scored them identically when it did not. `--json` carries the full float, so reach for it whenever the ordering has to be defended rather than read.

```bash
matra summarize essay.md --json | jq '.result.sentences[0]'
# {"index": 4, "span": {"byte_start": 812, "byte_end": 951}, "text": "...", "score": 0.312}
matra summarize essay.md --json | jq '.result.provenance.method'
# "tfidf"
```

Under `--json` the `result` is a summary: the document's `provenance`, with the `method` that ran and its `parameters` (`{"n": 3}`), and the `sentences`, each with its `index` among the document's sentences and its `span` in the input.

### `matra keyphrases`

Extracts ranked keyphrases.

```bash
matra keyphrases essay.md
matra keyphrases essay.md -n 20 --method yake
```

| Option | Default | Values |
|---|---|---|
| `-n` | `keyphrases.n` from the config, `10` as shipped | any positive integer |
| `--method` | `keyphrases.method` from the config, `rake` as shipped | `rake`, `yake` |

RAKE splits on stop words and scores the remaining word runs by a co-occurrence degree-to-frequency ratio. It is rule-based and fast. YAKE scores individual terms by position, frequency, and how varied their surrounding context is, then builds one-word to three-word candidates from those scores. It tends to surface longer, more specific phrases and to notice terms that first appear later in a long document, at more computation than RAKE. Both reject input over 200000 tokens.

Unlike `summarize`, `keyphrases` output stays in score order, highest first. A phrase is not tied to one location the way a sentence is, so under `--json` each carries `occurrences`, the span of every place in the input it was found, beside the result's `provenance` with its `method` and `parameters`.

Phrases are printed as lowercased lemmas, not as the surface text of the document. "Dependency Parses" appears as `dependency parse`, and its occurrences point at the surface text. The lemma is the parser's, read in each sentence's context, and the parser does not always lemmatize one word the same way twice. "Reading room" can come out as `reading room` in one sentence and `read room` in another, two phrases for one surface form. Merge such pairs yourself if they matter. Phrases with equal scores come back in the same order on every run, the order they are first met, so a tie at the `-n` boundary is decided the same way each time.

#### Reading the scores

Neither number is a probability, a percentage, or comparable with the other. RAKE sums a per-word degree-over-frequency ratio across the words of the phrase, and that ratio is at least 1 for every word, so a phrase of k words scores at least k. That is a floor per length, not an order across lengths: on this book's methodology page the two-word `lexical density` scores `5.667` and the three-word `model file name` scores `5.200`. YAKE multiplies its words' term scores and returns the reciprocal, so it is unbounded above and carries no interpretable unit, and phrase length does not predict where a YAKE score lands. Use either score to order phrases within one document under one method. Do not read it as a magnitude, and do not compare a RAKE number with a YAKE number.

Expect the top phrase not to be the document's topic. On this book's [errors](../reference/errors.md) page, RAKE puts `error implement st ::error`, read off the code span `std::error::Error` and the words around it, first at `12.135` and scores the bare `error` at `1.468`, while YAKE's first is `move copy` at `2035.733`. What is low there is the score, not the position: `1.468` sits not far above `1.000`, the floor any single word scores, and 224 of the 571 phrases sit flat on that floor underneath it, which puts `error` around the middle of the ranking, 297th. Both methods are behaving as specified: they rank properties of word co-occurrence, not aboutness. [Methodology](../reference/methodology.md) carries both formulas and every departure from the published versions.

```bash
matra keyphrases essay.md --json | jq '.result.phrases[0:3]'
```

### `matra clusters`

Groups a document's sentences by the similarity of their embeddings.

```bash
matra clusters essay.md
matra clusters essay.md --threshold 0.8 --json
```

On prose that does not repeat itself, expect the first line to find nothing. 0.85 joins near-restatements, and an essay whose sentences each say something new has none, so the table is empty, stderr says so, and the exit code is `1`:

```text
matra: found no clusters in essay.md at --threshold 0.85; a lower threshold joins less similar sentences
```

That is a result, not a failure. `summarize` and `keyphrases` say the same when they find nothing, and under `--json` or `--quiet` none of the three writes the line.

| Option | Default | Values |
|---|---|---|
| `--threshold` | `clusters.threshold` from the config, `0.85` as shipped | a cosine similarity |

Each sentence is embedded with potion-base-8M, the embedding model `models.embedding` names, downloaded into the model directory on first use (30.9 MB), with the same two lines on stderr the parsing model's first download prints. Two sentences whose cosine similarity is at least the threshold are joined, and a cluster is a connected component of that graph, so co-membership is transitive and is not pairwise similarity. A sentence with no partner is in no cluster. The table prints each cluster with its members' sentence indices and text; `--json` writes the `Clusters` value, with `edges`, every pair that cleared the threshold and its score, and the embedder's identity in `provenance.parameters`. Finding no cluster is exit `1`.

The threshold does not travel between models, domains or granularities, and 0.85 is a starting point for sentences only. [Semantic clusters](semantic-clusters.md) says how to choose one.

The command needs a build with the `model2vec` feature, which the Python wheel has. A build without it refuses before it reads anything, with `invalid_input` naming the feature.

### `matra config`

`matra config show` prints every resolved value with the rung it came from, one key per line, in the shape `cargo config get --show-origin` uses:

```bash
matra config show
```

```
data_dir = "/home/you/.local/share/matra" # default
model_dir = "/home/you/.local/share/matra/models" # default
models.embedding = "potion-base-8M" # default
summary.n = 5 # /home/you/.config/matra/config.toml
summary.method = "tfidf" # default
keyphrases.n = 10 # default
keyphrases.method = "rake" # default
clusters.threshold = 0.85 # default
```

`models.embedding` is the name of the embedding model's directory under `model_dir`, which `matra clusters`, `Model2Vec::from_config` and Python's `Model2Vec.from_config()` provision into and load from. The file's sections are `[models]`, `[summary]` and `[keyphrases]`, each of the last two with `n` and `method`, and `[clusters]` with `threshold`. They configure the command line only: the library and Python take every count, method and threshold as an argument.

A config file from 0.2 does not load. 0.3.0 renamed `[summarize]` to `[summary]`, `algorithm` to `method`, and `semantic.threshold` to `clusters.threshold`, and dropped `models.udpipe`, since the parsing model is pinned in the source beside its digest. Every command that reads such a file fails with `invalid_input`, naming the file and each key's replacement, so the file can be fixed from the message. `matra config init --force` writes a fresh one.

The origin is `default` for a value compiled into the crate, a path for one read from your config file, `environment variable ...` for one an environment variable set, and `command line` for one a flag set. With `--json`, each key carries its value, the rung, and what the rung pointed at.

What that listing does not carry is the config file path. Every key whose origin is `default` names no file, so a run with no config file on disk prints no path at all, and "where is my config" is not a question the human table answers. Two commands do answer it, once there is a file. `matra config show --json` puts the config file's path in the envelope's `input` field when that file exists, and `null` when it does not: `input` names what the run read, and a run with no config file read none. `matra config init` prints the path it wrote to, which is also where `config show` looks.

```bash
matra config show --json | jq -r .input
```

`matra config init` writes the shipped defaults (`models.embedding`, and the summary, keyphrase and cluster keys) to the resolved config path and prints where it wrote them. It creates the parent directories, writes through a temporary file in the same directory so a reader never sees a half-written config, and refuses to overwrite an existing file unless you pass `--force`.

```bash
matra config init
matra config init --force
```

### `matra completions`

Prints a completion script for `bash`, `zsh`, or `fish`.

```bash
mkdir -p ~/.zfunc && matra completions zsh > ~/.zfunc/_matra   # ~/.zfunc must be on your fpath
matra completions bash > /etc/bash_completion.d/matra
```

## For an agent

`matra --skill` prints the agent skill: what matra is for, when to reach for it, every command with its JSON shape, how to read each number and what it does not mean, the limits, and the errors. It is the whole hand-off. If you are pointing an agent at matra and it has nothing installed, `uvx 'matra>=0.3' --skill` is the line to give it. Pin the version: `--skill` arrived in 0.2.0, and a bare `uvx matra` takes whatever release is newest, which before 0.2.0 was a different command line that does not have the flag.

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
  "format_version": 2,
  "command": "skill",
  "input": null,
  "result": { "name": "SKILL", "body": "---\nname: matra\n..." },
  "error": null
}
```

`result.name` is `SKILL` for the top level and the reference's own name for `matra --skill -r <name> --json`. The list replaces `result` with one key:

```json
{
  "format_version": 2,
  "command": "skill",
  "input": null,
  "result": { "references": [{ "name": "json", "summary": "The JSON envelope every command but completions emits, ..." }] },
  "error": null
}
```

The list is every file under `references/`, in file-name order, and each summary is read from that file's own frontmatter rather than from a second list that could drift from it.

The same files install as a plugin: `claude --plugin-dir <checkout>` over a clone of the repository picks up `skills/matra/`, which is where plugin discovery looks. And this documentation site serves [`llms.txt`](https://mox-labs.github.io/matra/llms.txt) at its root, the map of every page with its one-line summary, for an agent reading the human door instead.

## Reading from stdin

`-` in place of the path reads stdin, as bytes, and the format is detected the way it is for a file. `--stdin-filename` gives that input a name, which is the label in the output and the envelope, and whose extension stands in for a file's. `--format` names the format outright. With neither, the input is read as plain text and labelled `<stdin>`. Bytes that are not UTF-8 text fail as `unsupported_format`, as a file's do.

```bash
pandoc notes.docx -t markdown | matra analyze - --stdin-filename notes.md
pandoc notes.docx -t markdown | matra analyze - --format markdown
cat essay.txt | matra keyphrases -
```

The 8 MiB cap applies to stdin as it does to a file, and it is enforced while reading rather than after, so an unbounded stream is refused instead of buffered. Stdin is read before the model loads, so an oversized stream costs no download.

## Color, quiet, and version

`--color` takes `auto` (the default), `always`, or `never`. Under `auto`, matra colors an interactive terminal and nothing else, and honors `NO_COLOR`: if that variable is present and not empty, color is off whatever its value. `--color always` and `--color never` are explicit requests and outrank `NO_COLOR` in both directions.

`--quiet` suppresses the human-readable output while leaving the exit code alone, so a script can branch on "found something" without collecting the table. It has no effect on `--json`. It also silences the one line the command prints to standard error before it downloads a model, which names the artifact, its size and where it is going. That line is the only output a first run produces during a wait that can reach half a minute, so silence it deliberately rather than by habit.

`--version` prints the version and then the features this build was compiled with:

```
matra 0.3.0
features: udpipe model2vec python cli
```

## The JSON envelope

`--json` emits one object for every command but `completions`, success or failure:

```json
{
  "format_version": 2,
  "command": "analyze",
  "input": "essay.md",
  "result": { "provenance": {}, "sections": [], "metrics": {}, "skipped": [] },
  "error": null
}
```

`command` is `analyze`, `summarize`, `keyphrases`, `clusters`, `config`, `skill` or `schema`. `input` is the path that was read, or the name `--stdin-filename` gave it. For `config` it is the config file the run read, and null when there is none, and for `skill` and `schema`, which read no document, it is always null. `result` is the serde form of the domain value the command produced: a `Document` for `analyze`, a `Summary` for `summarize`, a `Keyphrases` for `keyphrases` and a `Clusters` for `clusters`, each of the last three carrying the document's provenance with the `method` that ran and its `parameters`. For `config` it is an object keyed by setting, each carrying `value`, `source` and `origin`, and for `skill` the shapes shown above.

All five keys are always present, and exactly one of `result` and `error` is null. A failure writes the envelope with `result` null and `error` holding the failure's `kind`, `message` and `details`, the same three Rust serializes and Python raises:

```json
{
  "format_version": 2,
  "command": "analyze",
  "input": "missing.md",
  "result": null,
  "error": {
    "kind": "io",
    "message": "io error: cannot read missing.md: No such file or directory (os error 2)",
    "details": { "io_kind": "not_found", "path": "missing.md", "cause": "cannot read missing.md: No such file or directory (os error 2)" }
  }
}
```

The message still goes to stderr and the exit code is still `2`, so a script that reads only the exit code is unchanged. Branch on `error.kind` and read values from `error.details`, never from the message. The command line's own refusals carry the kind whose remedy matches, which for every one of them is `invalid_input`, a call to change, with `error.details.reason` naming which refusal it is: a directory given to a command that reads one document, `--format` with a directory, `-r` without `--skill`, an unknown reference or schema name, `config init` over an existing file, and `clusters` in a build without an embedder. A usage error under `--json`, such as an unknown flag, is an envelope too, with `command` null when the arguments name none. [Errors](../reference/errors.md) lists every kind.

`analyze DIR --json` writes one envelope per line, one line per file, each compact rather than indented, so the output is a stream of JSON values that `jq` reads one at a time.

Stability: `format_version` increments on any change to the envelope, and whenever the document schema's major version moves; the `result` value is the serde form of the documented domain types. That is the same promise cargo makes for `cargo metadata --format-version`. Pin your consumer to a `format_version` you have tested against.

A `Document` also names its own schema version, in `result.provenance.schema` (`"2.0"`), and that is the only place it appears. A minor version adds optional fields only; anything else is a major. So ignore fields you do not recognize, treat every string set you receive as open (`format`, `pos`, `dep`, `mood`, `pattern`, the feature names), and reject a major you do not know.

The JSON Schemas for this major are compiled into the program. `matra --schema` lists them with their titles, and `matra --schema NAME` prints one: `document`, `summary`, `keyphrases`, `clusters` and `error` for the outputs, and `blocks` for block input. Like `--skill` it outranks a subcommand and reads no document, and under `--json` it is the envelope with `command` `schema`. The same files are in the repository under `spec/schema/v2/`.

```bash
matra --schema
matra --schema document > document.schema.json
```

## Exit codes

`0` on success when the command found something, `1` on success when it found nothing (an empty summary, no keyphrases, no clusters, a file with zero sentences), and `2` when an error occurred. Model load failure, a missing or unreadable file, a directory where one document is read, input over the size cap, a bad argument, and a parse failure all land on `2`, with the message on stderr prefixed `matra:` and, under `--json`, the envelope with its `error` on stdout. For a directory the code is `2` if any file failed, `0` if none failed and any had a sentence, and `1` otherwise. A broken pipe, which is what you get piping into `head`, is treated as success and exits `0`.

```bash
matra keyphrases notes.txt > /dev/null
case $? in
  0) echo "phrases found" ;;
  1) echo "no phrases" ;;
  2) echo "failed" ;;
esac
```
