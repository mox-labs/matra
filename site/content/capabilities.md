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
| `Paragraph` | `text`, `in_blockquote`, `sentences`, `metrics` | `word_count()`, `sentence_count()` |
| `Section` | `heading`, `level`, `paragraphs` | none |
| `Document` | `provenance`, `sections`, `metrics` | `paragraphs()`, `sentences()`, `tokens()` |

`provenance` records the matra version, the schema version (`"2.0"`), the parser with its model, digest and licence, the format, and the input's path, length and SHA-256.

## 2. Metrics

Produced by `Engine::compose`, into `ParagraphMetrics` on each paragraph and `DocumentMetrics` on the document. Every ratio is `Option<f64>`, and `None` means not computed, which is distinct from a computed zero. [Methodology](./reference/methodology.md) gives each formula and the exact condition for `None`.

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

`Corpus` adds three Rust-only aggregates across the documents of a directory: `total_words()`, `passive_ratio()`, `mean_readability()`.

## 3. Summarization

Free functions over a sentence slice. Both return `Vec<ScoredSentence>` (`text`, `score`, `position`) in document order, capped at 2,000 sentences.

| Function | Ranks by |
|---|---|
| `tfidf_summarize(sentences, n)` | mean TF-IDF of the sentence's lemmatized terms, each sentence its own document for IDF |
| `textrank_summarize(sentences, n)` | PageRank over a similarity graph of shared content lemmas normalized by log length |

Both are on the Python surface as `Matra.tfidf_summarize` and `Matra.textrank_summarize`, which parse the text and run the extractor in one call.

TextRank's summary of The Federalist No. 10 at the default length, and every sentence's score it was chosen from:

<figure-textrank input="federalist-10" />

## 4. Keyphrases

Free functions over a sentence slice. Both return `Vec<Keyphrase>` (`phrase`, `score`), highest score first, capped at 200,000 tokens.

| Function | Ranks by |
|---|---|
| `rake_keyphrases(sentences, max)` | co-occurrence degree over frequency, on `NOUN`, `ADJ`, and `PROPN` runs between stop words |
| `yake_keyphrases(sentences, max)` | per-term position, frequency, and context diversity, assembled into 1-word to 3-word candidates, score inverted so higher is more relevant |

Both are on the Python surface as `Matra.rake_keyphrases` and `Matra.yake_keyphrases`.

The two rank by different properties of the text, and on The Federalist No. 10 their top twelve lists share no phrase:

<figure-keyphrases input="federalist-10" />

## 5. Semantic clusters

Not a field on any type above. The port and the clustering functions are always compiled; the `Model2Vec` adapter is behind the `model2vec` feature.

| Item | Is |
|---|---|
| `Embedder` | the port: `embed(&[&str])` returning one `Embedding` per text, plus `identity()` |
| `Model2Vec` | the adapter, reading `model.safetensors`, `tokenizer.json`, and `config.json` from a directory. `from_dir(dir)` loads what is there; `potion_base_8m(dir)` and `from_config(cfg)` fetch the pinned reference model when it is absent |
| `embed_and_cluster(doc, embedder, threshold)` | embeds a document's sentences, then clusters them |
| `extraction::semantic_clusters(embeddings, threshold, identity)` | clusters vectors you already hold |
| `SemanticClusters` | `identity`, `threshold`, and `clusters` |
| `SemanticCluster` | `members` (sentence indices) plus the `SemanticEdge` values that cleared the threshold |

Capped at 2,000 sentences. On the Python surface as `Matra.semantic_clusters`, `Model2Vec` (including `Model2Vec.potion_base_8m`), and the module-level `semantic_clusters`. `Matra.semantic_clusters` takes a `Model2Vec` or any object with `embed` and `identity`.

## The pipeline surface

| Value | Constructors | Carries |
|---|---|---|
| `Ingest` | `text(string, format)`, `path(file or directory)` | the source: a string is a stream of one, a directory a stream of many |
| `Engine` | `with_defaults()`, `from_config(cfg)`, `new(provider, decomposer table)` | `analyze` over a stream, `analyze_one`, or the stages `annotate` and `compose` |
| `Config` | `resolve()`, `from_sources(...)`, `with_model_dir(dir)` | where models live and what the defaults are, plus the `ValueSource` each value came from |

`Engine::with_defaults()` is the no-setup constructor: resolve a `Config`, then build the standard pipeline from it, fetching the pinned English model if it is not on disk. `Engine::new` is the explicit one, and it is what you reach for with your own provider or decomposer table.

`engine.analyze(ingest)` returns a lazy iterator of per-document results. Collecting it into `CorpusResult` partitions successes from failures, and one bad file does not abort the stream. On the Python surface the same walk is `Matra.analyze_path(path)`, which returns the items in order for the caller to partition.

## Traceability

`Paragraph.text` is the paragraph as the decomposer cut it from the input, trimmed at both ends, with a blockquote's `>` markers removed. `Sentence.text` is rebuilt by the provider: the UDPipe adapter joins the token forms with a space unless a token carries `SpaceAfter=No`, so its whitespace can differ from the source. Neither is matched back to the input to place it. Each paragraph is parsed on its own and its sentences are attached to it, so the chain from a token up through its sentence, paragraph, and section holds by construction rather than by text matching.

## Where to go next

[Domain model](./reference/domain-types.md) is the full type graph. [Methodology](./reference/methodology.md) is every formula. [Errors](./reference/errors.md) is every failure.
