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

Both models work the same way, and the constructor you reach for is what decides whether anything is fetched.

**Downloading is pinned, or it does not happen.** `Udpipe::english(dir)` and `Model2Vec::potion_base_8m(dir)` fetch from URLs written in the source and load nothing whose digest does not equal a constant written in the source next to them. Verification happens before the bytes are parsed, and the bytes that were verified are the bytes that get parsed, with no second read of the disk in between. A mismatch removes the files and fetches once more; a second mismatch raises and removes them again, so a failed attempt leaves nothing behind for a later call to pick up. Exactly one artifact set can arrive this way, which is what makes "matra downloads a model" a bounded claim rather than an open one.

**The no-argument form.** `Engine::with_defaults()`, `Udpipe::from_config(cfg)`, and `Model2Vec::from_config(cfg)` resolve the directory through [`Config`](programming-model.md) and then do exactly the above. In Python, `Matra.english()` and `Model2Vec.potion_base_8m()` with no argument.

**Bringing your own.** `Udpipe::from_path(path)` and `Model2Vec::from_dir(dir)` load what you supply and never reach the network, whatever the directory holds. That is the path for a model this build does not pin: a different UDPipe language, a different model2vec artifact. Nothing verifies it, so the `model_hash` on a result is identity rather than proof: it tells you which artifacts produced a score, not that they are the ones you meant to fetch.

The pinned model is part of the contract in every case. A different model produces a different parse or a different vector space, and the conformance fixtures will fail, correctly. The [semantic clusters guide](../guides/semantic-clusters.md) records the embedding digest the conformance suite pins.

## Cost and limits

| You exceed | Cap | You get |
|---|---|---|
| input text through `annotate` | 8 MiB | `InputTooLarge` with `what` set to `"input"` |
| a file on disk | 8 MiB | `InputTooLarge` with `what` set to `"file_source"` |
| sentences into either summarizer | 2,000 | `InputTooLarge` with `what` set to `"tfidf"` or `"textrank"` |
| sentences into clustering | 2,000 | `InputTooLarge` with `what` set to `"semantic_clusters"`, checked before the embedding pass runs |
| tokens into either keyphrase extractor | 200,000 | `InputTooLarge` with `what` set to `"rake"` or `"yake"` |
| one embedding artifact being downloaded | 64 MiB | `InputTooLarge` with `what` set to `"embedding_download"`, the read stopping at the bound rather than continuing |

In Python every one of them surfaces as `ValueError`. [Errors](../reference/errors.md) has the full routing table.

`InvalidInput` is the other one worth recognizing: it means the call site is wrong, not the text. Vectors that disagree on dimension, a non-finite threshold, an `Embedder` returning the wrong number of vectors.

The expensive thing is the model, not the call. Load an `Engine` once and reuse it.
