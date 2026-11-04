---
name: json
summary: The JSON envelope the commands with JSON output emit, and the Document shape field by field.
---

# The JSON envelope and the Document shape

## The envelope

`--json` on `analyze`, `summarize`, `keyphrases` or `config show` emits exactly one object with four keys, and nothing else on stdout. `completions` is the one command it does not reach: that prints a shell script and ignores the flag.

```json
{
  "format_version": 2,
  "command": "analyze",
  "input": "notes.md",
  "result": {}
}
```

| Key | Type | Holds |
|---|---|---|
| `format_version` | integer | `2` today. It increments on any change to the envelope, and whenever the document schema's major version moves |
| `command` | string | `analyze`, `summarize`, `keyphrases`, `config`, or `skill` |
| `input` | string or null | The path that was read, or the name `--stdin-filename` gave stdin. For `config show` it is the config file path when that file exists, and null when it does not. Null for `skill`, which reads no document |
| `result` | varies | The serde form of the value the command produced |

`result` is a `Document` for `analyze`, a list of `ScoredSentence` for `summarize`, a list of `Keyphrase` for `keyphrases`, a map of resolved keys for `config show`, and for `skill` either `{"name", "body"}` or `{"references": [{"name", "summary"}]}`. Pin a consumer to a `format_version` it was tested against, the same promise cargo makes for `cargo metadata --format-version`.

## Versions, and the rules for reading

A `Document` carries its own schema version in `provenance.schema`, as `"major.minor"`: `"2.0"` today, and nowhere else. The JSON Schema for each major is published in the repository at `spec/schema/v2/document.schema.json`.

- A minor adds optional fields only. A major is anything else: a field removed, renamed or retyped, or a field whose meaning changes.
- Ignore fields you do not recognize; a newer minor adds them.
- Treat every string set you receive as open: `format`, `kind` (on paragraphs and on `skipped`), `pos`, `dep`, `mood`, `pattern` and the feature names in `features`. An unknown value is data, not an error.
- Reject a `provenance.schema` major you do not know.

`format_version` moves with the schema's major, so a consumer of the envelope sees the same break.

<!-- needs: model -->

```console
$ matra analyze draft.txt --json
```

## `Document`

Four keys: `{"provenance": {}, "sections": [], "metrics": {}, "skipped": []}`.

| Field | Type | Holds |
|---|---|---|
| `provenance` | object | Where the document came from and what produced it; see below |
| `sections` | array | The section tree, the only place paragraphs live |
| `metrics` | object | The document's aggregates |
| `skipped` | array | What the reader saw and did not analyse, in document order: `{"kind": string, "span": Span}`, `kind` one of `table`, `code`, `front_matter`, `html`, `thematic_break`, `link_definition`. Nothing is dropped without an entry here |

## `Span`

`{"start": integer, "end": integer}`: where a value lies in the input, in UTF-8 bytes, `start` inclusive and `end` exclusive. Headings, paragraphs, `skipped` entries, sentences and tokens carry one. Bytes, not characters: slice the file's bytes (`head -c`, or `bytes[start:end]` in Python) rather than a decoded string. The text fields are normalized, a sentence's rebuilt from its tokens and a markdown paragraph's with its markup removed, so the span, not the text, is where a value came from. Every byte of the input that is not whitespace lies in exactly one heading, paragraph or skipped span.

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
  "span": { "start": 0, "end": 127 },
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

## `ScoredSentence` and `Keyphrase`

`summarize` returns `{ "text": string, "score": float, "position": integer }` per sentence, in ascending `position`, which is document order and not score order.

`keyphrases` returns `{ "phrase": string, "score": float }`, highest score first. There is no position, because a phrase is not tied to one place the way a sentence is.

Phrases with equal scores can swap order between runs, and a tie at the `-n` boundary can change which phrase makes the list, so sort the array before diffing it across runs.

## Useful jq filters

Non-null readability across a document: `[.result.sections[].paragraphs[].metrics.readability_grade | select(. != null)] | add / length`.

Every indicative root clause with no modal on its root: `[.result.sections[].paragraphs[].sentences[] | select(.root_mood.mood == "Ind" and (.root_mood.root_id as $r | [.modals[].head_id] | index($r) | not)) | .text]`.

Every sentence that carries a modal: `[.result.sections[].paragraphs[].sentences[] | select(.modals | length > 0) | .text]`.

Every passive token relation: `[.result.sections[].paragraphs[].sentences[].tokens[] | select(.dep | test("pass"))]`.
