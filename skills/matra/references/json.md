---
name: json
summary: The JSON envelope the commands with JSON output emit, and the Document shape field by field.
---

# The JSON envelope and the Document shape

## The envelope

`--json` on `analyze`, `summarize`, `keyphrases` or `config show` emits exactly one object with four keys, and nothing else on stdout. `completions` is the one command it does not reach: that prints a shell script and ignores the flag.

```json
{
  "format_version": 1,
  "command": "analyze",
  "input": "notes.md",
  "result": {}
}
```

| Key | Type | Holds |
|---|---|---|
| `format_version` | integer | `1` today. It increments on any change to the envelope or to the meaning of a field inside `result` |
| `command` | string | `analyze`, `summarize`, `keyphrases`, `config`, or `skill` |
| `input` | string or null | The path that was read, or the name `--stdin-filename` gave stdin. For `config show` it is the config file path. Null for `skill`, which reads no document |
| `result` | varies | The serde form of the value the command produced |

`result` is a `Document` for `analyze`, a list of `ScoredSentence` for `summarize`, a list of `Keyphrase` for `keyphrases`, a map of resolved keys for `config show`, and for `skill` either `{"name", "body"}` or `{"references": [{"name", "summary"}]}`. Pin a consumer to a `format_version` it was tested against; that is the whole stability promise, and it is the same one cargo makes for `cargo metadata --format-version`.

<!-- needs: model -->

```console
$ matra analyze draft.txt --json
```

## `Document`

```json
{
  "sections": [],
  "vocabulary_ttr": 0.76,
  "nominalization_ratio": 0.047,
  "passive_ratio": 0.75
}
```

| Field | Type | Holds |
|---|---|---|
| `sections` | array | The section tree, the only place paragraphs live |
| `vocabulary_ttr` | float or null | Distinct lemmas over total lemmas |
| `nominalization_ratio` | float or null | Suffix-matched nouns over total lemmas |
| `passive_ratio` | float or null | Sentences with a passive relation over all sentences |

The three floats are null until the metric stage runs, and null is not zero. Aggregate methods on the Rust type (`total_words`, `mean_sentence_length`, `sentence_length_std`) are Rust only and are absent from the JSON; compute them from `sections` if you need them.

## `Section`

| Field | Type | Holds |
|---|---|---|
| `heading` | string or null | The heading text. Null for a plain-text document and for markdown before its first heading |
| `level` | integer | `0` for plain text, `1` and up for markdown heading depth |
| `paragraphs` | array | Paragraphs in document order |

## `Paragraph`

| Field | Type | Holds |
|---|---|---|
| `text` | string | Verbatim paragraph text from the source |
| `in_blockquote` | bool | True when the paragraph sits in a blockquote. Those are never parsed and never measured, so `sentences` is empty and all three slots stay null |
| `sentences` | array | Sentences parsed from this paragraph alone |
| `readability_grade` | float or null | Null unless the paragraph has more than 10 non-punctuation tokens |
| `lexical_density` | float or null | Null when the paragraph has no words |
| `compression_ratio` | float or null | Null unless the paragraph has more than 50 non-punctuation tokens, and null above 262,144 bytes |

Null appears in these slots far more often than a caller expects. Filter for non-null before averaging.

## `Sentence`

Eight fields. `text` and `tokens` are the parse; the other six are structural detections computed once and carried in the JSON, so every language reads the same answer.

```json
{
  "text": "Reviewers should confirm that the rollback path still works, because the rollback path was never exercised on the new hardware.",
  "tokens": [],
  "negations": [{ "cue_id": 16, "cue_lemma": "never", "head_id": 17 }],
  "modals": [{ "aux_id": 2, "aux_lemma": "should", "head_id": 3 }],
  "bare_assertion": false,
  "reportings": [
    {
      "verb_id": 3,
      "verb_lemma": "confirm",
      "ccomp_id": 9,
      "subject_id": 1,
      "subject_lemma": "reviewer"
    }
  ],
  "root_adverbials": [],
  "hearst_pairs": []
}
```

| Field | Shape |
|---|---|
| `text` | String. Reconstructed from token surface forms with spacing from `SpaceAfter=No`, so it is not a byte slice of your input |
| `tokens` | Array of `Token`, sorted by `id` |
| `negations` | `cue_id`, `cue_lemma`, `head_id`. Cue lemma is one of `not`, `never`, `no`, `neither`, `nor` |
| `modals` | `aux_id`, `aux_lemma`, `head_id`. Lemma is one of ten: can, could, may, might, must, ought, shall, should, will, would |
| `bare_assertion` | Bool. True when the root clause is finite indicative and no modal governs it |
| `reportings` | `verb_id`, `verb_lemma`, `ccomp_id`, `subject_id`, `subject_lemma`. The last two are null when the parse has no subject for the verb in this sentence |
| `root_adverbials` | `adv_id`, `adv_lemma`. Every adverbial modifier attached to the root |
| `hearst_pairs` | `pattern`, `hypernym`, `hyponym`. Each span is `head_id`, `head_lemma`, `first_id`, `last_id` |

`pattern` is one of six strings: `such_as`, `such_np_as`, `including`, `especially`, `and_other`, `or_other`.

```json
{
  "pattern": "such_as",
  "hypernym": { "head_id": 1, "head_lemma": "animal", "first_id": 1, "last_id": 1 },
  "hyponym": { "head_id": 4, "head_lemma": "dog", "first_id": 4, "last_id": 4 }
}
```

Every id in these six fields is a token `id` inside the same sentence, so a detection can always be resolved back to the tokens it came from. `head_id` is `0` when the cue or the auxiliary is itself the root.

## `Token`

Ten CoNLL-U columns plus one derived flag, all present on every token.

| Field | Type | Column |
|---|---|---|
| `id` | integer | 1, the 1-based position in the sentence |
| `text` | string | 2, the surface form |
| `lemma` | string | 3, the dictionary form |
| `pos` | string | 4, the universal part of speech |
| `xpos` | string | 5, the treebank-specific tag |
| `feats` | string | 6, pipe-separated morphology, for example `Number=Sing` |
| `head` | integer | 7, the id this token depends on, `0` for the root |
| `dep` | string | 8, the relation to that head |
| `deps` | string | 9, always `_` with the shipped adapter |
| `misc` | string | 10, annotation such as `SpaceAfter=No` |
| `is_punct` | bool | Derived, not a CoNLL-U column |

## `ScoredSentence` and `Keyphrase`

`summarize` returns `{ "text": string, "score": float, "position": integer }` per sentence, in ascending `position`, which is document order and not score order.

`keyphrases` returns `{ "phrase": string, "score": float }`, highest score first. There is no position, because a phrase is not tied to one place the way a sentence is.

## Useful jq filters

Non-null readability across a document: `[.result.sections[].paragraphs[].readability_grade | select(. != null)] | add / length`.

Every sentence that carries a modal: `[.result.sections[].paragraphs[].sentences[] | select(.modals | length > 0) | .text]`.

Every passive token relation: `[.result.sections[].paragraphs[].sentences[].tokens[] | select(.dep | test("pass"))]`.
