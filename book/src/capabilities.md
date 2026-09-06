# What matra gives you

Every value the pipeline returns, by tier. The tables say which language surfaces carry each value: fields cross to Python, methods do not.

## 1. Structure

Produced by `Engine::annotate`.

### `Token`

The ten CoNLL-U columns plus one derived flag. All cross to Python.

| Field | Holds |
|---|---|
| `id` | 1-based position in the sentence |
| `text` | the word as written |
| `lemma` | dictionary form (`approved` becomes `approve`) |
| `pos` | universal part of speech (`NOUN`, `VERB`, `ADJ`) |
| `xpos` | treebank-specific tag, finer grained than `pos` |
| `feats` | morphology (tense, number, person) |
| `head` | `id` of the token this one depends on, `0` for the root |
| `dep` | the dependency relation to that head |
| `deps` | secondary dependencies |
| `misc` | annotation |
| `is_punct` | punctuation flag |

`Token::feat(key)` reads one morphological feature. Rust only.

### `Sentence`

| Field | Holds |
|---|---|
| `text` | verbatim sentence text |
| `tokens` | `Vec<Token>`, id-sorted |
| `negations` | `Vec<Negation>`: cue id, cue lemma, head id |
| `modals` | `Vec<Modal>`: auxiliary id, lemma, head id |
| `bare_assertion` | `bool`: finite indicative root with no modal governing it |
| `reportings` | `Vec<Reporting>`: verb id and lemma, `ccomp` head id, subject when present |
| `root_adverbials` | `Vec<RootAdverbial>`: adverbial id and lemma |
| `hearst_pairs` | `Vec<HearstPair>`: pattern tag plus hypernym and hyponym `HearstSpan` values |

All eight cross to Python. The methods below are Rust only.

| Method | Returns |
|---|---|
| `root_token()` | the token everything else hangs off |
| `head_of(id)` | the governor of a token |
| `children_of(id)` | its direct dependents |
| `subtree(id)` | the whole clause under it |
| `tree_depth()` | nesting depth, `usize::MAX` on a malformed cycle |
| `is_passive()` | whether the sentence carries `nsubj:pass` or `aux:pass` |
| `content_tokens()` | non-punctuation tokens |
| `word_count()` | count of those |
| `reportings_in(lexicon)` | the reportings whose verb lemma is in a list you supply |
| `root_adverbials_in(lexicon)` | the root adverbials whose lemma is in a list you supply |

### `Paragraph`, `Section`, `Document`

| Type | Fields (cross to Python) | Methods (Rust only) |
|---|---|---|
| `Paragraph` | `text`, `in_blockquote`, `sentences`, `readability_grade`, `lexical_density`, `compression_ratio` | `word_count()`, `sentence_count()` |
| `Section` | `heading`, `level`, `paragraphs` | none |
| `Document` | `sections`, `vocabulary_ttr`, `nominalization_ratio`, `passive_ratio` | `paragraphs()`, `sentences()`, `tokens()`, `paragraph_count()`, `total_sentences()`, `total_words()`, `passive_ratio()`, `mean_sentence_length()`, `sentence_length_std()` |

## 2. Metrics

Produced by `Engine::compose`. Each is `Option<f64>`, and `None` means not computed, which is distinct from a computed zero. [Methodology](./reference/methodology.md) gives each formula and the exact condition for `None`.

| Field | On | Measures |
|---|---|---|
| `readability_grade` | `Paragraph` | Flesch-Kincaid grade level, from sentence and syllable length |
| `lexical_density` | `Paragraph` | content words as a share of all words |
| `compression_ratio` | `Paragraph` | brotli compressed size over raw size, a repetition proxy |
| `vocabulary_ttr` | `Document` | distinct words over total words, and it falls as text grows, so documents of different lengths are not comparable on it |
| `nominalization_ratio` | `Document` | share of nouns formed from verbs (`decide` becoming `decision`) |
| `passive_ratio` | `Document` | share of sentences carrying a passive construction |

`Corpus` adds three Rust-only aggregates across the documents of a directory: `total_words()`, `passive_ratio()`, `mean_readability()`.

## 3. Summarization

Free functions over a sentence slice. Both return `Vec<ScoredSentence>` (`text`, `score`, `position`) in document order, capped at 2,000 sentences.

| Function | Ranks by |
|---|---|
| `tfidf_summarize(sentences, n)` | mean TF-IDF of the sentence's lemmatized terms, each sentence its own document for IDF |
| `textrank_summarize(sentences, n)` | PageRank over a similarity graph of shared content lemmas normalized by log length |

Both are on the Python surface as `Matra.tfidf_summarize` and `Matra.textrank_summarize`, which parse the text and run the extractor in one call.

## 4. Keyphrases

Free functions over a sentence slice. Both return `Vec<Keyphrase>` (`phrase`, `score`), highest score first, capped at 200,000 tokens.

| Function | Ranks by |
|---|---|
| `rake_keyphrases(sentences, max)` | co-occurrence degree over frequency, on `NOUN`, `ADJ`, and `PROPN` runs between stop words |
| `yake_keyphrases(sentences, max)` | per-term position, frequency, and context diversity, assembled into 1-word to 3-word candidates, score inverted so higher is more relevant |

Both are on the Python surface as `Matra.rake_keyphrases` and `Matra.yake_keyphrases`.

## 5. Semantic clusters

Behind the `model2vec` feature. Not a field on any type above.

| Item | Is |
|---|---|
| `Embedder` | the port: `embed(&[&str])` returning one `Embedding` per text, plus `identity()` |
| `Model2Vec` | the adapter, loading `model.safetensors`, `tokenizer.json`, and `config.json` from a directory |
| `embed_and_cluster(doc, embedder, threshold)` | embeds a document's sentences, then clusters them |
| `extraction::semantic_clusters(embeddings, threshold, model_hash)` | clusters vectors you already hold |
| `SemanticClusters` | `model_hash`, `threshold`, and `clusters` |
| `SemanticCluster` | `members` (sentence indices) plus the `SemanticEdge` values that cleared the threshold |

Capped at 2,000 sentences. On the Python surface as `Matra.semantic_clusters`, `Model2Vec`, and the module-level `semantic_clusters`.

## The pipeline surface

| Value | Constructors | Carries |
|---|---|---|
| `Ingest` | `text(string, format)`, `path(file or directory)` | the source: a string is a stream of one, a directory a stream of many |
| `Engine` | `new(provider, decomposer table)` | `analyze` over a stream, `analyze_one`, or the stages `annotate` and `compose` |

`engine.analyze(ingest)` returns a lazy iterator of per-document results. Collecting it into `CorpusResult` partitions successes from failures, and one bad file does not abort the stream.

## Traceability

`Paragraph.text` and `Sentence.text` are verbatim slices of the input. Nothing is normalized, re-wrapped, or rewritten, and each paragraph is parsed on its own, so the chain from a token up through its sentence, paragraph, and section always holds, rather than depending on text matching.

## Where to go next

[Domain model](./reference/domain-types.md) is the full type graph. [Methodology](./reference/methodology.md) is every formula. [Errors](./reference/errors.md) is every failure.
