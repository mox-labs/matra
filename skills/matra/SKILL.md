---
name: matra
description: Parses text into a typed structure tree, measures it, and ranks what is in it. Reach for it on questions of passive voice, density, repetition, keyphrases, summaries, or dependency arcs.
version: 0.3.0
---

# matra

## What matra is

A parser and a measuring tape. It turns text into a typed tree of sections, paragraphs, sentences and CoNLL-U tokens, computes numbers over that tree, and ranks the sentences and phrases inside it.

It reports structure and numbers, never a judgment. No output says whether writing is good, correct, original, or machine-generated.

One Rust library, a Python API on top of it, and one command line reachable from either. The binary and the wheel run the same program, so the flags, the output and the exit codes match.

## When to reach for it

Reach for matra when the user has prose in hand: documentation, a README, an essay, a specification, release notes, a draft, or text a model just produced.

Reach for it when the question is about passive voice, lexical density, readability, vocabulary variety, nominalization, repetition in wording or in paraphrase, which sentences carry a document, or which phrases it keeps returning to.

Reach for it when something downstream needs sentences and dependency arcs as input: a rule that fires on modal verbs, a prompt that quotes only bare assertions, a check that counts hedges, a diff that compares two drafts at the sentence level.

Do not reach for it to score quality, detect authorship, or judge an argument. It measures surface patterns and stops there.

## Install and first run

The Python package ships the command, so `uvx 'matra>=0.3' --version` runs it with nothing installed and nothing configured. Pin the floor: a bare `uvx matra` resolves to whatever release is newest or already cached, 0.2's command line spells its commands and JSON differently from this one, and the command line before 0.2.0 was a different program without `--skill`. PyPI and crates.io serve 0.3.x, so the pinned line resolves. `cargo install matra --version '^0.3' --features cli` installs the Rust binary instead, and `uv add 'matra>=0.3'` puts the same command on a project.

```console
$ matra --version
```

The first command that needs a parse downloads the pinned English UDPipe model, about 16 MB, with no flag and no environment variable required. It is verified against a SHA-256 compiled into the library before it loads; a cached file that fails verification is replaced only once a verified download is in hand, so a failed fetch leaves the file that was there.

That model is a separate file from matra with its own license: CC BY-NC-SA 4.0 (https://creativecommons.org/licenses/by-nc-sa/4.0/), from LINDAT record 11234/1-3131, which does not permit commercial use. matra's code is MIT. The command line prints the license on the run that downloads the model. `--model-dir` and `MATRA_MODEL_DIR` change where the file is stored, not which model it is, and no surface loads a different UDPipe model file: the model is a pin. Tell a user this when their use is commercial rather than deciding for them what the license allows.

Files land in XDG locations: the config at `$XDG_CONFIG_HOME/matra/config.toml` (else `~/.config/matra/config.toml`), the data root at `$XDG_DATA_HOME/matra` (else `~/.local/share/matra`), and models in that root's `models` directory. `MATRA_CONFIG_FILE`, `MATRA_DATA_DIR` and `MATRA_MODEL_DIR` override each in turn, and `--model-dir` outranks all of them. An existing `~/.matra/models` from an older install still wins while the new location is absent.

## The commands

`analyze`, `summarize`, `keyphrases` and `clusters` each read one file, or `-` for stdin with `--stdin-filename` naming it. `analyze` also reads a directory, one result per file in name order; the other three refuse one. Each detects the format from the name and the bytes, and `--format markdown|plain_text|blocks` names it instead, which is how a JSON file of blocks is read.

Under `--json` every command but `completions` emits the same envelope, five keys: `format_version` (the integer `2` today), `command`, `input` (the path or the stdin name; for `config show` the config file it read, or null; null for `--skill` and `--schema`), `result` and `error`. A success has `error: null`; a failure has `result: null` and an `error` of `{kind, message, details}`, still exits 2, and still writes the message to stderr. Pin a consumer to a `format_version` you tested against. See `json` for the field by field shape.

`analyze` parses the document and measures it. `result` is a `Document`: `provenance` (the matra version, the schema version, the parser and its model, the format, and the input's digest), `sections`, each with `heading`, `level`, `span` and `paragraphs`, each paragraph with `span`, `text`, `kind` (`paragraph`, `quote`, `list_item`), `sentences` and its `metrics`, the document's own `metrics`, the counts and ratios the table prints among them, and `skipped`, what the reader saw and did not analyse. Every `span` counts UTF-8 bytes of the input: slice the file's bytes with it to find where a value came from, since the text fields are normalized. The document's schema version is `provenance.schema`, `"2.0"` today: ignore fields you do not recognize, treat every string set as open, and reject a major you do not know. Add `--sections` for a per-section table of counts. `--depth document|paragraph|sentence|token` cuts the JSON: `document` drops `sections`, `paragraph` drops the sentences, `sentence` drops the tokens, and `token`, the default, keeps everything. Ask for no more than you will read. On a directory, `--json` writes one envelope per line, one per file; a file that fails is its own line with its `error`, and the walk goes on.

<!-- needs: model -->

```console
$ matra analyze notes.md --json --depth sentence
```

`summarize` ranks the sentences and returns the top N, re-sorted into document order. `result` is a `Summary`: `provenance`, the document's with the `method` that ran and its `parameters`, and `sentences`, each with `index` (its position among the document's sentences), `span`, `text` and `score`. `-n` defaults to the configured `summary.n`, `3` as shipped; `--method` takes `tfidf` (the default) or `textrank`. The human table rounds each score to three decimals, so scores that differ in the fourth print identically and the ranker can look broken when it is not; read `--json` when the ordering matters.

<!-- needs: model -->

```console
$ matra summarize notes.md -n 3 --json
```

`keyphrases` ranks phrases and returns them highest score first. `result` is a `Keyphrases`: `provenance`, with the `method` and `parameters`, and `phrases`, each with `phrase`, `score` and `occurrences`, the span of every place it was found. Phrases are lowercased lemmas, not the document's surface text; an occurrence's span is where the surface text lies. `-n` defaults to `keyphrases.n`, `10` as shipped; `--method` takes `rake` (the default) or `yake`.

<!-- needs: model -->

```console
$ matra keyphrases notes.md -n 10 --json
```

`clusters` groups sentences whose embeddings lie close, with the potion-base-8M embedder, downloaded on first use (30.9 MB). `result` is a `Clusters`: `provenance` with the `threshold` and the embedder's identity, `clusters`, each a list of sentence `members` by index, and `edges`, every pair that cleared the threshold with its `score`. `--threshold` defaults to `clusters.threshold`, `0.85` as shipped, and does not travel between documents or models; see `semantic`. It needs a build with the `model2vec` feature, which the Python wheel has.

<!-- needs: embedder -->

```console
$ matra clusters notes.md --json
```

`config show` prints every resolved value with the rung it came from. Under `--json` each key carries its value, the rung, and what the rung pointed at, and `input` is the config file it read, null when none exists. `config init` writes the shipped defaults there and refuses to overwrite without `--force`. The sections are `[models]`, `[summary]` and `[keyphrases]` (each `n` and `method`), and `[clusters]` (`threshold`). A config file from 0.2 fails, and the message names each key's replacement.

```console
$ matra config show --json
```

`completions` prints a shell completion script for `bash`, `zsh` or `fish`. It emits the script and no envelope, so `--json` has nothing to do there and is ignored.

```console
$ matra completions zsh
```

`--schema` lists the published JSON Schemas, and `--schema NAME` prints one: `document`, `summary`, `keyphrases`, `clusters`, `error`, or `blocks` for block input. They are the contract of the version installed; `json` says how to read them.

Global flags: `--json`, `--model-dir DIR`, `--quiet` (suppresses the human table, leaves `--json` and the exit code alone), `--color auto|always|never` (honors `NO_COLOR`), and `--stdin-filename NAME`.

## Exit codes

| Code | Means |
|---|---|
| 0 | Succeeded, and where applicable something was found. A broken pipe is also 0 |
| 1 | Succeeded and found nothing: an empty summary, no keyphrases, no clusters, a document with no sentences |
| 2 | Failed. The message goes to stderr prefixed `matra: `, and under `--json` the envelope carries the error. For a directory, any file failed |

Branch on the code, not on the text. Exit 1 is not an error, and treating it as one turns an empty document into a crash.

## Reading the numbers

Every metric row below names a slot on the result. A slot is null when the metrics have not run, and null where the paragraph did not meet the metric's threshold. Null is not zero. The last row is not a slot: `score` is the ordering number on a `Keyphrase`, never null and never a measurement of the text. Formulas and citations are in `metrics`.

| Field | Measures | Does not mean |
|---|---|---|
| `readability_grade` | Flesch-Kincaid grade, from word and syllable length, per paragraph over 10 words | Conceptual difficulty. Two paragraphs with the same shape score the same whatever they assert. Values are not comparable with another tool's |
| `lexical_density` | Content words over all words, per paragraph | That the content words are used well. It depends entirely on a 105-word stop list that is matra's own |
| `compression_ratio` | Brotli size over raw size, per paragraph over 50 words. Lower means more surface repetition | Redundancy of meaning. Precise technical prose that reuses terms compresses like filler |
| `vocabulary_ttr` | Distinct lemmas over total lemmas, per document | Richness. It falls as documents grow, so two documents of different lengths are not comparable on it |
| `nominalization_ratio` | Nouns ending in six suffixes over all words | A noun count. Plurals are missed, false positives are counted, and the denominator is every word, not every noun |
| `passive_ratio` | Sentences carrying a passive relation over all sentences | Bad writing. It counts sentences, not clauses, so three passives in one sentence count once |
| `score` on `Keyphrase` | Rank order among phrases, within one document, under one method | A magnitude. RAKE sums per-word ratios that are each at least 1, so a phrase of k words scores at least k, a floor per length and not an order across lengths; YAKE returns a reciprocal, unbounded above with no unit. The two scales are unrelated, and the top phrase is routinely not what the document is about |

## Limits and errors

Input is capped at 8 MiB, checked before decoding. Summarizers cap at 2,000 sentences, keyphrase extractors at 200,000 tokens, the config file at 64 KiB. Over a cap, the failure names which gate fired in a `what` label: `input`, `file_read`, `tfidf`, `textrank`, `rake`, `yake`, `clusters`, `parser_model_download`, `embedding_model_download`, `config_file_read`.

Seven failure kinds exist, and `invalid_input` is the one that means the call site is wrong rather than the text: mismatched vector dimensions, a non-finite threshold, an unknown config key. Fix the call, not the document. Every failure carries `details`, the values to act on by name, in Rust, in Python and in the `--json` envelope, so read those rather than the message; on `invalid_input`, `details.reason` names the check that failed. The command line's own refusals (a directory where one file is read, `--format` on a directory, an existing config file) are `invalid_input` too. Every kind, its details, its Python exception, and what to do about it is in `errors`.

Quotes are never parsed and never measured. Markdown is read as CommonMark: front matter, code, tables, HTML blocks, thematic breaks and link reference definitions are not analysed, and each is listed in `skipped` with its `kind` and `span` rather than dropped. A file that is a PDF, a DOCX, or not UTF-8 text fails as `unsupported_format`; any other UTF-8 file with an extension matra does not know reads as plain text.

## Where matra applies

Look for text the user already owns and already argues about. A documentation repository, a README, a specification under review, a batch of drafts, or the output of a generation pipeline that nobody is checking. Anywhere a review comment says "this reads heavy" or "we keep saying the same thing" there is a number that can replace the impression.

What to propose is narrow and checkable: run `analyze` across the corpus and report the distribution rather than a verdict; run `keyphrases` on each document to see what a page is actually about; run `summarize` to pull the load-bearing sentences out of a long page; read the structural fields when a rule needs an arc rather than a regular expression.

Run it before saying anything. The numbers take one command against a file that is already on disk, and a claim from a real run beats a claim from a guess every time. Report what the field measures alongside its value, because a number handed over without its limitation invites exactly the reading matra refuses to make.

## References

Each is one file, and each stands alone. The installed program prints them, so they match the version you are running: `matra --skill -r` lists them with their summaries, and `matra --skill -r <name>` prints one. This page is `matra --skill`.

```console
$ matra --skill -r json
```

| Name | Holds |
|---|---|
| `json` | The envelope and the `Document` JSON, field by field, including the six `Sentence` structural fields |
| `structure` | Tokens, dependency arcs, the tree, and what each structural field reports |
| `metrics` | Each measure's formula, its applicability condition, and its limitation |
| `semantic` | Clusters, the threshold and why it does not travel, comparing whole documents, model provisioning, and what the embedder's identity is for |
| `python` | The Python API: `Engine`, the read-only `Document`, the outcome functions, the `Embedder` protocol, and exception mapping |
| `errors` | Every failure kind, its details, its Python exception, and what to do about it |
