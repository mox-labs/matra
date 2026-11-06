---
name: json
summary: The JSON envelope every command but completions emits, success or failure, and the Document shape field by field.
---

# The JSON envelope and the Document shape

## The envelope

`--json` writes the envelope on stdout and nothing else there: one object for one result, and one per line for `analyze DIR`, one line per file. Every command takes it except `completions`, which prints a shell script and ignores the flag.

```json
{ "format_version": 2, "command": "analyze", "input": "notes.md", "result": {}, "error": null }
```

| Key | Type | Holds |
|---|---|---|
| `format_version` | integer | `2` today. It increments on any change to the envelope, and whenever the document schema's major version moves |
| `command` | string or null | `analyze`, `summarize`, `keyphrases`, `clusters`, `config`, `skill` or `schema`. Null only for a usage failure whose arguments name no command |
| `input` | string or null | The path that was read, or the name `--stdin-filename` gave stdin. For `config show`, the config file it read, and null when none exists. Null for `skill` and `schema`, which read none |
| `result` | varies, or null | The serde form of the value the command produced. Null on a failure |
| `error` | object or null | Null on a success. On a failure `{kind, message, details}`, the form Rust serializes and Python raises; `errors` lists every kind |

All five keys are always present. `result` is a `Document` for `analyze`, a `Summary`, `Keyphrases` or `Clusters` for the outcomes, a map of resolved keys for `config show`, `{"name", "body"}` or `{"references": [...]}` for `skill`, and `{"name", "schema"}` or `{"schemas": [{"name", "title"}]}` for `schema`. A failure still exits 2 and writes `matra: <message>` to stderr. In a directory walk a file that fails is its own line with its `error`, the walk goes on, and the exit code is 2 if any file failed. Pin a consumer to a `format_version` it was tested against, the same promise cargo makes for `cargo metadata --format-version`.

<!-- needs: model -->

```console
$ matra analyze docs/ --json --depth document
```

`--depth` cuts what `analyze --json` writes: `document` drops `sections`, `paragraph` drops each paragraph's `sentences`, `sentence` drops each sentence's `tokens`, and `token`, the default, keeps everything. A key below the depth is absent, never an empty array.

## Versions, and the rules for reading

A `Document` carries its own schema version in `provenance.schema`, as `"major.minor"`: `"2.0"` today, and nowhere else. `format_version` moves with the schema's major, so a consumer of the envelope sees the same break. `matra --schema` lists the JSON Schemas of this major, compiled into the program, and `matra --schema document` prints one.

- A minor adds optional fields only. A major is anything else: a field removed, renamed or retyped, or a field whose meaning changes.
- Ignore fields you do not recognize; a newer minor adds them.
- Treat every string set you receive as open: `format`, `kind` (on paragraphs and on `skipped`), `pos`, `dep`, `mood`, `pattern` and the feature names in `features`. An unknown value is data, not an error.
- Reject a `provenance.schema` major you do not know.

```console
$ matra --schema document
```

## `Document`

Four keys: `{"provenance": {}, "sections": [], "metrics": {}, "skipped": []}`, with `sections` absent under `--depth document`.

| Field | Type | Holds |
|---|---|---|
| `provenance` | object | Where the document came from and what produced it; see below |
| `sections` | array | The section tree, the only place paragraphs live |
| `metrics` | object | The document's aggregates |
| `skipped` | array | What the reader saw and did not analyse, in document order: `{"kind": string, "span": Span}`, `kind` one of `table`, `code`, `front_matter`, `html`, `thematic_break`, `link_definition`. Nothing is dropped without an entry here |

## `Span`

`{"byte_start": integer, "byte_end": integer}`: where a value lies in the input, in UTF-8 bytes, `byte_start` inclusive and `byte_end` exclusive. Headings, paragraphs, `skipped` entries, sentences and tokens carry one. Bytes, not characters: slice the file's bytes (`head -c`, or `data[span["byte_start"]:span["byte_end"]]` over the encoded bytes in Python) rather than a decoded string. The text fields are normalized, a sentence's rebuilt from its tokens and a markdown paragraph's with its markup removed, so the span, not the text, is where a value came from. Every byte of the input that is not whitespace lies in exactly one heading, paragraph or skipped span.

`metrics` holds:

| Field | Type | Holds |
|---|---|---|
| `total_words` | integer | Non-punctuation tokens across every sentence |
| `total_sentences` | integer | Sentences across every paragraph |
| `paragraph_count` | integer | Paragraphs, quotes included |
| `mean_sentence_length` | float or null | Words per sentence. Null with no sentence |
| `sentence_length_std` | float or null | Sample standard deviation of words per sentence. Null with fewer than two sentences |
| `vocabulary_ttr` | float or null | Distinct lemmas over total lemmas. Falls as text grows, so documents of different lengths are not comparable on it |
| `nominalization_ratio` | float or null | Suffix-matched nouns over total lemmas |
| `passive_ratio` | float or null | Sentences with a passive relation over all sentences. Null with no sentence |

Null means the document did not meet the metric's condition; null is not zero.

## `provenance`

| Field | Type | Holds |
|---|---|---|
| `matra` | string | The matra version that produced the document |
| `schema` | string | The schema version, `"major.minor"`, `"2.0"` today |
| `parser` | object | `name`, `model`, `sha256`, `license`: `udpipe`, `english-ewt-ud-2.5-191206`, the model file's digest, `CC BY-NC-SA 4.0 (non-commercial)`. A model matra does not pin reports its digest and a null `model` and `license` |
| `format` | string | The format the input was read as: `markdown`, `plain_text` or `blocks` |
| `source` | object | `path` (null for stdin and in-memory text), and the `bytes` and `sha256` of the analysed text the spans point into: the input itself for markdown and plain text |

There is no timestamp: the same input through the same build gives the same document, byte for byte, which is what lets you diff two runs.

## `Section`

| Field | Type | Holds |
|---|---|---|
| `heading` | string or null | The heading text. Null for a plain-text document and for markdown before its first heading |
| `level` | integer | `0` for plain text, `1` and up for markdown heading depth |
| `span` | `Span` or null | The heading in the input, `#` marks included. Null with no heading |
| `paragraphs` | array | Paragraphs in document order |

## `Paragraph`

| Field | Type | Holds |
|---|---|---|
| `span` | `Span` | The paragraph in the input, trailing whitespace excluded; for markdown it covers the `>` and list markers on its lines |
| `text` | string | The text the parser read: for markdown the inline text with the markup removed (emphasis, links, code span backticks, `>` and list markers), soft line breaks as `\n`; for plain text a verbatim slice |
| `kind` | string | `paragraph`, `quote` or `list_item`. A quote, at any depth, is never parsed and never measured, so `sentences` is empty and every metric stays null |
| `sentences` | array | Sentences parsed from this paragraph alone |
| `metrics` | object | The paragraph's three measures, below |

`metrics` holds:

| Field | Type | Holds |
|---|---|---|
| `readability_grade` | float or null | Null unless the paragraph has more than 10 non-punctuation tokens |
| `lexical_density` | float or null | Null when the paragraph has no words |
| `compression_ratio` | float or null | Null unless the paragraph has more than 50 non-punctuation tokens, and null above 262,144 bytes |

Null appears in these slots far more often than a caller expects. Filter for non-null before averaging.

## `Sentence`

Twelve fields. `span`, `text` and `tokens` are the parse; the other nine are counts and structural detections computed once and carried in the JSON, so every language reads the same answer.

```json
{
  "span": { "byte_start": 0, "byte_end": 127 },
  "text": "Reviewers should confirm that the rollback path still works, because the rollback path was never exercised on the new hardware.",
  "tokens": [],
  "word_count": 20,
  "tree_depth": 3,
  "passive": true,
  "negations": [{ "cue_id": 16, "cue_lemma": "never", "head_id": 17 }],
  "modals": [{ "aux_id": 2, "aux_lemma": "should", "head_id": 3 }],
  "clausal_complements": [{ "verb_id": 3, "verb_lemma": "confirm", "ccomp_id": 9, "subject_id": 1, "subject_lemma": "reviewer" }],
  "root_adverbials": [],
  "hearst_pairs": [],
  "root_mood": null
}
```

| Field | Shape |
|---|---|
| `span` | `Span`. From the first token's start to the last token's end |
| `text` | String. Reconstructed from token surface forms with spacing from `SpaceAfter=No`, so it is not a byte slice of your input; `span` is |
| `tokens` | Array of `Token`, sorted by `id` |
| `word_count` | Integer. Non-punctuation tokens |
| `tree_depth` | Integer or null. The longest path from a token to the root, the root at depth 0. Null when the parse's head references loop, which a malformed parse can produce |
| `passive` | Bool. True when a token carries `nsubj:pass`, `nsubjpass` or `aux:pass` |
| `negations` | `cue_id`, `cue_lemma`, `head_id`. Cue lemma is one of `not`, `never`, `no`, `neither`, `nor` |
| `modals` | `aux_id`, `aux_lemma`, `head_id`. Lemma is one of ten: can, could, may, might, must, ought, shall, should, will, would |
| `clausal_complements` | `verb_id`, `verb_lemma`, `ccomp_id`, `subject_id`, `subject_lemma`. The last two are null when the parse has no subject for the verb in this sentence |
| `root_adverbials` | `adv_id`, `adv_lemma`. Every adverbial modifier attached to the root |
| `hearst_pairs` | `pattern`, `hypernym`, `hyponym`. Each span is `head_id`, `head_lemma`, `first_id`, `last_id` |
| `root_mood` | `root_id`, `mood`, or null. The `Mood` feature of the root clause, read off the root or its `cop`, `aux` or `aux:pass` child, with the root's token id. Null when neither carries one, as when a modal governs the root. `mood` is an open set: `Ind`, `Imp`, and whatever else the model assigns |

`pattern` is one of six strings: `such_as`, `such_np_as`, `including`, `especially`, `and_other`, `or_other`.

```json
{ "pattern": "such_as", "hypernym": { "head_id": 1, "head_lemma": "animal", "first_id": 1, "last_id": 1 }, "hyponym": { "head_id": 4, "head_lemma": "dog", "first_id": 4, "last_id": 4 } }
```

Every id in these detections is a token `id` inside the same sentence, so a detection can always be resolved back to the tokens it came from. `head_id` is `0` when the cue or the auxiliary is itself the root.

## `Token`

Ten CoNLL-U columns, the token's span and one derived flag, all present on every token.

| Field | Type | Column |
|---|---|---|
| `id` | integer | 1, the 1-based position in the sentence |
| `span` | `Span` | Not a CoNLL-U column: where the surface form lies in the input |
| `text` | string | 2, the surface form |
| `lemma` | string | 3, the dictionary form |
| `pos` | string | 4, the universal part of speech |
| `xpos` | string | 5, the treebank-specific tag |
| `features` | object | 6, morphology as a map from feature to value, for example `{"Number": "Plur"}`. A multi-valued feature keeps its value unsplit (`"Nom,Acc"`). Empty when the token has none |
| `head` | integer | 7, the id this token depends on, `0` for the root |
| `dep` | string | 8, the relation to that head |
| `deps` | string | 9, always `_` with the shipped adapter |
| `misc` | string | 10, annotation such as `SpaceAfter=No` |
| `is_punct` | bool | Derived, not a CoNLL-U column |

## `Summary` and `Keyphrases`

Each carries `provenance`: the document's, with two keys a document's lacks. `method` names what ranked the result (`tfidf`, `textrank`, `rake` or `yake`; treat the set as open) and `parameters` holds what it ran with, `{ "n": integer }`. Scores from different methods are on unrelated scales, so read `method` before comparing two results. `matra --schema summary` and `matra --schema keyphrases` print their schemas.

`summarize`'s `sentences` are `{ "index": integer, "span": { "byte_start", "byte_end" }, "text": string, "score": float }`, in ascending `index`, which is document order and not score order. `index` is the sentence's position among the document's sentences, and `span` is where it lies in the input.

`keyphrases`' `phrases` are `{ "phrase": string, "score": float, "occurrences": [{ "byte_start", "byte_end" }] }`, highest score first. Every occurrence is a span of the input, from the phrase's first word to its last, so a stop word the method passed over lies inside it.

Equal scores come back in the same order on every run, so a diff across runs of the same input and version is stable; a tie at the `-n` boundary still decides which phrase makes the list by that order, not by merit.

## Useful jq filters

Non-null readability across a document: `[.result.sections[].paragraphs[].metrics.readability_grade | select(. != null)] | add / length`.

Every indicative root clause with no modal on its root: `[.result.sections[].paragraphs[].sentences[] | select(.root_mood.mood == "Ind" and (.root_mood.root_id as $r | [.modals[].head_id] | index($r) | not)) | .text]`.

Every sentence that carries a modal: `[.result.sections[].paragraphs[].sentences[] | select(.modals | length > 0) | .text]`.

Every passive token relation: `[.result.sections[].paragraphs[].sentences[].tokens[] | select(.dep | test("pass"))]`.
