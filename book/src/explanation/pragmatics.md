# Pragmatics

## Choosing a summarizer

Both return the top N sentences in document order, as `ScoredSentence` values carrying the score and the original position.

**`tfidf_summarize`** scores each sentence by the mean TF-IDF of its lemmatized terms, treating every sentence as its own document for the IDF computation. It runs in linear time, and it favours sentences carrying vocabulary that is rare in the rest of the document.

**`textrank_summarize`** builds a similarity graph over the sentences (shared content lemmas divided by the log of the two lengths, so long sentences are not favoured) and runs iterative PageRank scoring over it. It favours sentences many others resemble. The dense similarity matrix is quadratic in sentence count, which is why the 2,000-sentence cap exists.

Rare-in-this-document against central-to-this-document is the whole difference. On a document with one repeated theme they tend to agree; where they diverge, TF-IDF has found the unusual sentence and TextRank has found the typical one.

## Choosing a keyphrase extractor

Both return `Keyphrase` values, highest score first.

**`rake_keyphrases`** splits at stop words and punctuation, keeps runs of `NOUN`, `ADJ`, and `PROPN` tokens as candidates, and scores each by the co-occurrence degree over frequency ratio. Multi-word phrases are what it is built to find, and it needs no corpus.

**`yake_keyphrases`** scores individual terms on position, frequency, and context diversity, then assembles 1-word to 3-word candidates. YAKE's own score is lower-is-better, and matra inverts it so higher is more relevant in the returned values.

Their caps differ from the summarizers': both are bounded on total tokens rather than sentence count, because a corpus of many one-token sentences costs them nothing.

## Reading a `Document`

| Question | Read |
|---|---|
| Is it passive-heavy? | `Document.passive_ratio`, the share of sentences carrying a passive construction |
| Is it dense? | `Paragraph.lexical_density` for the content-word share, alongside `readability_grade` |
| Is it repetitive in wording? | `Paragraph.compression_ratio`, where a lower value means the text compressed further and so repeats more on the surface |
| How varied is the vocabulary? | `Document.vocabulary_ttr`, but see the length caveat below |
| Is it repetitive in paraphrase? | none of the above: lexical measures cannot see restatement in different words. That is what `SemanticClusters` is for |
| How nominal is the style? | `Document.nominalization_ratio` |

Two cautions. `vocabulary_ttr` is a raw type-token ratio and falls as text grows, so two documents of different lengths are not comparable on it without normalizing first. And any of these slots can be `None`, which means the metric was not computed (too short, in a blockquote, or over a per-metric byte ceiling) and is distinct from a computed zero.

## The semantic threshold

`embed_and_cluster` takes the cosine threshold from you rather than choosing one. Published cutoffs for paraphrase detection span 0.67 to 0.9 with no consensus, and the working value depends on the model, the domain, and sentence length.

Raising the threshold admits fewer edges: clusters get smaller and more of them, and more sentences end up in none. Lowering it merges components through chains, because clusters are connected components rather than cliques. That transitivity is why the clearing edges travel inside each cluster: two sentences can share a cluster without a direct edge, and the edge list is where you check.

Start around 0.85 with the reference model and calibrate on your own corpus.

## Provisioning models

**UDPipe.** `Udpipe::english(dir)` downloads the pinned English model into `dir` on first use, verifies it against a pinned SHA-256, and re-downloads once on mismatch. Later calls with the same directory load the cached file. `Udpipe::from_path` skips the download entirely.

**model2vec.** Nothing is downloaded, ever. Place `model.safetensors`, `tokenizer.json`, and `config.json` in a directory and point `Model2Vec::from_dir` at it. The digest over those three files is the `model_hash`, and it appears on every `SemanticClusters` result. Check it after downloading: the hash is identity, so it tells you which artifacts produced a score, not that the artifacts are the ones you meant to fetch. The [semantic clusters guide](../guides/semantic-clusters.md) records the digest the conformance suite pins.

The pinned model is part of the contract in both cases. A different model produces a different parse or a different vector space, and the conformance fixtures will fail, correctly.

## Cost and limits

| You exceed | Cap | You get |
|---|---|---|
| input text through `annotate` | 8 MiB | `InputTooLarge` with `what` set to `"input"` |
| a file on disk | 8 MiB | `InputTooLarge` with `what` set to `"file_source"` |
| sentences into either summarizer | 2,000 | `InputTooLarge` with `what` set to `"tfidf"` or `"textrank"` |
| sentences into clustering | 2,000 | `InputTooLarge` with `what` set to `"semantic_clusters"`, checked before the embedding pass runs |
| tokens into either keyphrase extractor | 200,000 | `InputTooLarge` with `what` set to `"rake"` or `"yake"` |

In Python all five surface as `ValueError`. [Errors](../reference/errors.md) has the full routing table.

`InvalidInput` is the other one worth recognizing: it means the call site is wrong, not the text. Vectors that disagree on dimension, a non-finite threshold, an `Embedder` returning the wrong number of vectors.

The expensive thing is the model, not the call. Load an `Engine` once and reuse it.
