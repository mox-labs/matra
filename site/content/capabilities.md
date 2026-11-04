# What matra gives you

Every value the pipeline returns, by tier. The tables say which language surfaces carry each value: fields cross to Python, methods do not.

## 1. Structure

Produced by `Engine::analyze`, as the document is read and parsed.

### `Token`

The ten CoNLL-U columns plus one derived flag. All cross to Python.

| Field | Holds |
|---|---|
| `id` | 1-based position in the sentence |
| `text` | the word as written |
| `lemma` | dictionary form (`approved` becomes `approve`) |
| `pos` | universal part of speech (`NOUN`, `VERB`, `ADJ`) |
| `xpos` | treebank-specific tag, finer grained than `pos` |
| `features` | morphology (tense, number, person), as a map from feature to value |
| `head` | `id` of the token this one depends on, `0` for the root |
| `dep` | the dependency relation to that head |
| `deps` | secondary dependencies, always `_` from the UDPipe adapter, whose binding does not surface them |
| `misc` | annotation |
| `is_punct` | punctuation flag |

`Token::feat(key)` reads one morphological feature. Rust only.

### `Sentence`

| Field | Holds |
|---|---|
| `text` | sentence text, rebuilt by the provider from its tokens |
| `tokens` | `Vec<Token>`, id-sorted |
| `word_count` | non-punctuation tokens |
| `tree_depth` | nesting depth, `None` on a malformed cycle |
| `passive` | whether the sentence carries `nsubj:pass`, `nsubjpass`, or `aux:pass` |
| `negations` | `Vec<Negation>`: cue id, cue lemma, head id |
| `modals` | `Vec<Modal>`: auxiliary id, lemma, head id |
| `clausal_complements` | `Vec<ClausalComplement>`: verb id and lemma, `ccomp` head id, subject when present |
| `root_adverbials` | `Vec<RootAdverbial>`: adverbial id and lemma |
| `hearst_pairs` | `Vec<HearstPair>`: pattern tag plus hypernym and hyponym `HearstSpan` values |
| `root_mood` | `Option<RootMood>`: the root clause's `Mood` feature and the root's id |

All eleven cross to Python. The methods below are Rust only.

| Method | Returns |
|---|---|
| `root_token()` | the token everything else hangs off |
| `head_of(id)` | the governor of a token |
| `children_of(id)` | its direct dependents |
| `subtree(id)` | the whole clause under it |
| `content_tokens()` | non-punctuation tokens |
| `clausal_complements_in(lexicon)` | the clausal complements whose verb lemma is in a list you supply |
| `root_adverbials_in(lexicon)` | the root adverbials whose lemma is in a list you supply |

### `Paragraph`, `Section`, `Document`

| Type | Fields (cross to Python) | Methods (Rust only) |
|---|---|---|
| `Paragraph` | `span`, `text`, `kind`, `sentences`, `metrics` | `word_count()`, `sentence_count()` |
| `Section` | `heading`, `level`, `paragraphs` | none |
| `Document` | `provenance`, `sections`, `metrics` | `paragraphs()`, `sentences()`, `tokens()` |

`provenance` records the matra version, the schema version (`"2.0"`), the parser with its model, digest and licence, the format, and the input's path, length and SHA-256.

## 2. Metrics

Produced by `Engine::analyze`, as its last step, into `ParagraphMetrics` on each paragraph and `DocumentMetrics` on the document. Every ratio is `Option<f64>`, and `None` means not computed, which is distinct from a computed zero. [Methodology](./reference/methodology.md) gives each formula and the exact condition for `None`.

| Field | On | Measures |
|---|---|---|
| `readability_grade` | `ParagraphMetrics` | Flesch-Kincaid grade level, from sentence and syllable length |
| `lexical_density` | `ParagraphMetrics` | content words as a share of all words |
| `compression_ratio` | `ParagraphMetrics` | brotli compressed size over raw size, a repetition proxy |
| `total_words`, `total_sentences`, `paragraph_count` | `DocumentMetrics` | the counts, as integers |
| `mean_sentence_length`, `sentence_length_std` | `DocumentMetrics` | words per sentence, and its sample standard deviation |
| `vocabulary_ttr` | `DocumentMetrics` | distinct lemmas over total words, and it falls as text grows, so documents of different lengths are not comparable on it |
| `nominalization_ratio` | `DocumentMetrics` | nouns ending in a nominalizing suffix (`-tion`, `-ment`, `-ness`, `-ity`, `-ence`, `-ance`) over all words, a surface proxy for nouns formed from verbs (`decide` becoming `decision`) |
| `passive_ratio` | `DocumentMetrics` | share of sentences carrying a passive construction |

Every measure is per document. Aggregating across documents is the caller's decision: the counts sum, and `vocabulary_ttr` does not.

## 3. Summarization

Free functions over a `Document`, needing no engine and no parser. Both return a `Summary`: the document's `provenance` with the `method` and `n` recorded, and `sentences`, each a `SummarySentence` (`index`, `span`, `text`, `score`), in document order, capped at 2,000 sentences.

| Function | Ranks by |
|---|---|
| `summarize(doc, n)`, `summarize_with(doc, n, SummaryMethod::TfIdf)` | mean TF-IDF of the sentence's lemmatized terms, each sentence its own document for IDF (`tfidf`, the default) |
| `summarize_with(doc, n, SummaryMethod::TextRank)` | PageRank over a similarity graph of shared content lemmas normalized by log length (`textrank`) |

Python spells them `matra.summarize(doc, n=3)` and `matra.summarize(doc, n=3, method="textrank")`, over the `Document` an engine returned.

TextRank's summary of The Federalist No. 10 at the default length, and every sentence's score it was chosen from:

<figure-textrank input="federalist-10" />

## 4. Keyphrases

Free functions over a `Document`. Both return a `Keyphrases`: the document's `provenance` with the `method` and `n` recorded, and `phrases`, each a `Keyphrase` (`phrase`, `score`, `occurrences`, the span of every place it was found), highest score first, capped at 200,000 tokens.

| Function | Ranks by |
|---|---|
| `keyphrases(doc, n)`, `keyphrases_with(doc, n, KeyphraseMethod::Rake)` | co-occurrence degree over frequency, on `NOUN`, `ADJ`, and `PROPN` runs between stop words (`rake`, the default) |
| `keyphrases_with(doc, n, KeyphraseMethod::Yake)` | per-term position, frequency, and context diversity, assembled into 1-word to 3-word candidates, score inverted so higher is more relevant (`yake`) |

Python spells them `matra.keyphrases(doc, n=10)` and `matra.keyphrases(doc, n=10, method="yake")`.

The two rank by different properties of the text, and on The Federalist No. 10 their top twelve lists share no phrase:

<figure-keyphrases input="federalist-10" />

## 5. Semantic clusters

Not a field on any type above. The port and `clusters` are always compiled; the `Model2Vec` adapter is behind the `model2vec` feature.

| Item | Is |
|---|---|
| `Embedder` | the port: `embed(&[&str])` returning one `Embedding` per text, plus `identity()` |
| `Model2Vec` | the adapter, reading `model.safetensors`, `tokenizer.json`, and `config.json` from a directory. `from_dir(dir)` loads what is there; `potion_base_8m(dir)`, with `dir` a models directory, and `from_config(cfg)` fetch the pinned reference model into its `potion-base-8M` subdirectory when it is absent |
| `clusters(doc, embedder, threshold)` | embeds a document's sentences with the embedder you pass, then clusters them; vectors you already hold arrive through an `Embedder` that returns them |
| `Clusters` | the document's `provenance` with the method (`cosine`), the `threshold` and the embedder's identity recorded, `clusters`, and `edges` |
| `Cluster`, `SimilarityEdge` | a cluster's `members` (sentence indices), and a pair `a`, `b` whose `score` cleared the threshold |

Capped at 2,000 sentences. Python spells them `matra.clusters(doc, embedder, threshold=0.85)` and `Model2Vec` (including `Model2Vec.potion_base_8m`); `matra.clusters` takes a `Model2Vec` or any object with `embed` and `identity`.

## The pipeline surface

| Value | Constructors | Carries |
|---|---|---|
| `Input` | `text(string, format)`, `bytes(data, format)`, `file(path)`, `blocks(blocks)` | one document: text, bytes, a file, or headings and paragraphs you supply |
| `Inputs` | `dir(path)` | the files in a directory, as an iterator of `Input` |
| `Engine` | `new()`, `from_config(cfg)`, `from_config_with_notice(cfg, notice)` | `analyze` one input, `analyze_all` of many |
| `Config` | `resolve()`, `from_file(path)`, `from_sources(...)`, `with_model_dir(dir)` | where models live and what the defaults are, plus the `ValueSource` each value came from |

`Engine::new()` is the no-setup constructor: locate the model directory from the environment, then build the standard pipeline, fetching the pinned English model if it is not on disk. It reads no configuration file; `Engine::from_config(&Config::resolve()?)` reads one.

`engine.analyze_all(inputs)` returns a lazy iterator of per-document results, one per input in order, and one bad file does not abort the stream. Python's `Engine.analyze_all(dir=path)` is the same lazy walk, yielding a `Document` or a `DocumentError` dict per file.

## Traceability

Every heading, paragraph, sentence and token carries a `span` into the analysed text, in UTF-8 bytes, and `skipped` lists what the reader saw and did not analyse (a table, a code block, front matter), each with its span. Every byte of a markdown or plain-text input that is not whitespace lies in exactly one heading, paragraph or skipped span, so nothing is dropped without a trace. The text fields are normalized: `Paragraph.text` is what the parser reads, a markdown paragraph's inline markup removed, and `Sentence.text` is rebuilt by the provider, the UDPipe adapter joining the token forms with a space unless a token carries `SpaceAfter=No`. The span, not the text, says where a value came from. Each paragraph is parsed on its own and its sentences are attached to it, so the chain from a token up through its sentence, paragraph, and section holds by construction rather than by text matching.

## Where to go next

[Domain model](./reference/domain-types.md) is the full type graph. [Methodology](./reference/methodology.md) is every formula. [Errors](./reference/errors.md) is every failure.
