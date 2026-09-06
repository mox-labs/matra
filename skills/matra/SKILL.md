---
name: matra
description: Parses text into a typed structure tree, measures it, and ranks what is in it. Reach for it on questions of passive voice, density, repetition, keyphrases, summaries, or dependency arcs.
version: 0.2.0
---

# matra

## What matra is

A parser and a measuring tape. It turns text into a typed tree of sections, paragraphs, sentences and CoNLL-U tokens, computes numbers over that tree, and ranks the sentences and phrases inside it.

It reports structure and numbers, never a judgment. No output says whether writing is good, correct, original, or machine-generated.

One Rust library, a Python API on top of it, and one command line reachable from either. The binary and the wheel run the same program, so the flags, the output and the exit codes match.

## When to reach for it

Reach for matra when the user has prose in hand: documentation, a README, an essay, a specification, release notes, a draft, or text a model just produced.

Reach for it when the question is about passive voice, lexical density, readability, vocabulary variety, nominalization, repetition in wording, repetition in paraphrase, which sentences carry a document, or which phrases it keeps returning to.

Reach for it when something downstream needs sentences and dependency arcs as input: a rule that fires on modal verbs, a prompt that quotes only bare assertions, a check that counts hedges, a diff that compares two drafts at the sentence level.

Do not reach for it to score quality, detect authorship, or judge an argument. It measures surface patterns and stops there.

## Install and first run

The Python package ships the command, so `uvx matra --version` runs it with nothing installed and nothing configured. `cargo install matra --features cli` installs the Rust binary instead, and `uv add matra` puts the same command on a project.

```console
$ matra --version
```

The first command that needs a parse downloads the pinned English UDPipe model, about 16 MB, with no flag and no environment variable required. It is verified against a SHA-256 compiled into the library before it loads; a cached file that fails verification is replaced only once a verified download is in hand, so a failed fetch leaves the file that was there.

Files land in XDG locations: the config at `$XDG_CONFIG_HOME/matra/config.toml` (else `~/.config/matra/config.toml`), the data root at `$XDG_DATA_HOME/matra` (else `~/.local/share/matra`), and models in that root's `models` directory. `MATRA_CONFIG_FILE`, `MATRA_DATA_DIR` and `MATRA_MODEL_DIR` override each in turn, and `--model-dir` outranks all of them. An existing `~/.matra/models` from an older install still wins while the new location is absent.

## The commands

`analyze`, `summarize` and `keyphrases` each read one file, or `-` for stdin with `--stdin-filename` naming it. A directory is refused, because a command that reports on one document cannot report on a directory of them. Under `--json` each emits the same envelope, and so do `config show` and `--skill`; `completions` prints a script and ignores `--json`. Four keys at the top level: `format_version` (the integer `1` today), `command`, `input` (the path or the stdin name, and null for `--skill`, which reads none), and `result`. Pin a consumer to a `format_version` you tested against. See `json` for the field by field shape.

`analyze` parses the document and fills its metric slots. `result` is a `Document`: `sections`, each with `heading`, `level` and `paragraphs`, each paragraph with `text`, `in_blockquote`, `sentences` and three metric slots, plus the document-level `vocabulary_ttr`, `nominalization_ratio` and `passive_ratio`. Add `--sections` for a per-section table of counts.

<!-- needs: model -->

```console
$ matra analyze notes.md --json
```

`summarize` ranks the sentences and returns the top N, re-sorted into document order. `result` is a list of `ScoredSentence`: `text`, `score`, `position`. `-n` defaults to the configured `summarize.n`, `3` as shipped; `--method` takes `tfidf` (the default) or `textrank`.

<!-- needs: model -->

```console
$ matra summarize notes.md -n 3 --json
```

`keyphrases` ranks phrases and returns them highest score first. `result` is a list of `Keyphrase`: `phrase`, `score`. Phrases are lowercased lemmas, not the document's surface text. `-n` defaults to `keyphrases.n`, `10` as shipped; `--method` takes `rake` (the default) or `yake`.

<!-- needs: model -->

```console
$ matra keyphrases notes.md -n 10 --json
```

`config show` prints every resolved value with the rung it came from. Under `--json` each key carries its value, the rung, and what the rung pointed at, and `input` is the config file path. `config init` writes the shipped defaults there and refuses to overwrite without `--force`.

```console
$ matra config show --json
```

`completions` prints a shell completion script for `bash`, `zsh` or `fish`. It emits the script and no envelope, so `--json` has nothing to do there and is ignored.

```console
$ matra completions zsh
```

Global flags: `--json` (the envelope, on `analyze`, `summarize`, `keyphrases` and `config show`), `--model-dir DIR`, `--quiet` (suppresses the human table, leaves `--json` and the exit code alone), `--color auto|always|never` (honors `NO_COLOR`), and `--stdin-filename NAME`.

## Exit codes

| Code | Means |
|---|---|
| 0 | Succeeded, and where applicable something was found. A broken pipe is also 0 |
| 1 | Succeeded and found nothing: an empty summary, no keyphrases, a document with no sentences |
| 2 | Failed. The message goes to stderr prefixed `matra: ` |

Branch on the code, not on the text. Exit 1 is not an error, and treating it as one turns an empty document into a crash.

## Reading the numbers

A slot is null when the metric stage has not run, and null where the paragraph did not meet the metric's threshold. Null is not zero. Formulas and citations are in `metrics`.

| Field | Measures | Does not mean |
|---|---|---|
| `readability_grade` | Flesch-Kincaid grade, from word and syllable length, per paragraph over 10 words | Conceptual difficulty. Two paragraphs with the same shape score the same whatever they assert. Values are not comparable with another tool's |
| `lexical_density` | Content words over all words, per paragraph | That the content words are used well. It depends entirely on a 105-word stop list that is matra's own |
| `compression_ratio` | Brotli size over raw size, per paragraph over 50 words. Lower means more surface repetition | Redundancy of meaning. Precise technical prose that reuses terms compresses like filler |
| `vocabulary_ttr` | Distinct lemmas over total lemmas, per document | Richness. It falls as documents grow, so two documents of different lengths are not comparable on it |
| `nominalization_ratio` | Nouns ending in six suffixes over all words | A noun count. Plurals are missed, false positives are counted, and the denominator is every word, not every noun |
| `passive_ratio` | Sentences carrying a passive relation over all sentences | Bad writing. It counts sentences, not clauses, so three passives in one sentence count once |

## Limits and errors

Input is capped at 8 MiB, checked before decoding. Summarizers cap at 2,000 sentences, keyphrase extractors at 200,000 tokens, the config file at 64 KiB. Over a cap, the failure names which gate fired in a `what` label: `input`, `file_source`, `tfidf`, `textrank`, `rake`, `yake`, `semantic_clusters`, `embedding_download`, `config_file`.

Seven failure kinds exist, and `invalid_input` is the one that means the call site is wrong rather than the text: mismatched vector dimensions, a non-finite threshold, an unknown config key. Fix the call, not the document. Every kind, its Python exception, and what to do about it is in `errors`.

Blockquote paragraphs are never parsed and never measured. Markdown decomposition drops frontmatter, fenced code and table rows, and stops at a `## References` heading.

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
| `semantic` | Clusters, the threshold, model provisioning, and what `model_hash` is for |
| `python` | The Python API, the `Embedder` protocol, `analyze_path`, and exception mapping |
| `errors` | Every failure kind, its Python exception, and what to do about it |
