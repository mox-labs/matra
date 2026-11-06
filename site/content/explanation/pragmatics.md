# Pragmatics

Which call to reach for, and how to read what it returns: the choices behind the summarizers, the keyphrase extractors, the document measures, the clustering threshold and the models, and why each answer is what it is. The caps each call enforces are listed under [`InputTooLarge`](../reference/errors.md#inputtoolarge); handing matra to an agent is [its own guide](../guides/agent.md).

## Choosing a summarizer

`summarize` returns the top N sentences in document order, as a `Summary` whose sentences carry the score and their index in the document, and whose provenance names the method. `summarize_with` chooses the method.

**TF-IDF** (`SummaryMethod::TfIdf`, the default) scores each sentence by the mean TF-IDF of its lemmatized terms, treating every sentence as its own document for the IDF computation. It runs in linear time, and it favours sentences carrying vocabulary that is rare in the rest of the document.

**TextRank** (`SummaryMethod::TextRank`) builds a similarity graph over the sentences (shared content lemmas divided by the log of the two lengths, so long sentences are not favoured) and runs iterative PageRank scoring over it. It favours sentences many others resemble. The dense similarity matrix is quadratic in sentence count, which is why the 2,000-sentence cap exists.

Rare-in-this-document against central-to-this-document is the whole difference. On a document with one repeated theme they tend to agree; where they diverge, TF-IDF has found the unusual sentence and TextRank has found the typical one.

## Choosing a keyphrase extractor

`keyphrases` returns `Keyphrase` values, highest score first, each with every place it occurs, and the provenance names the method. `keyphrases_with` chooses it.

**RAKE** (`KeyphraseMethod::Rake`, the default) splits at stop words and punctuation, keeps runs of `NOUN`, `ADJ`, and `PROPN` tokens as candidates, and scores each by the co-occurrence degree over frequency ratio. Multi-word phrases are what it is built to find, and it needs no corpus.

**YAKE** (`KeyphraseMethod::Yake`) scores individual terms on position, frequency, and context diversity, then assembles 1-word to 3-word candidates. YAKE's own score is lower-is-better, and matra inverts it so higher is more relevant in the returned values.

Their caps differ from the summarizers': both are bounded on total tokens rather than sentence count, because a corpus of many one-token sentences costs them nothing.

## Reading a `Document`

| Question | Read |
|---|---|
| Is it passive-heavy? | `DocumentMetrics.passive_ratio`, the share of sentences carrying a passive construction |
| Is it dense? | `ParagraphMetrics.lexical_density` for the content-word share, alongside `readability_grade` |
| Is it repetitive in wording? | `ParagraphMetrics.compression_ratio`, where a lower value means the text compressed further and so repeats more on the surface |
| How varied is the vocabulary? | `DocumentMetrics.vocabulary_ttr`, but see the length caveat below |
| Is it repetitive in paraphrase? | none of the above: lexical measures cannot see restatement in different words. That is what `Clusters` is for |
| How nominal is the style? | `DocumentMetrics.nominalization_ratio` |

Two cautions. `vocabulary_ttr` is a raw type-token ratio and falls as text grows, so two documents of different lengths are not comparable on it without normalizing first. And any of these slots can be `None`, which means the metric was not computed (too short, a quote, or over a per-metric byte ceiling) and is distinct from a computed zero.

## The semantic threshold

`clusters` takes the cosine threshold from you rather than choosing one, and records it in the result's provenance. Published cutoffs for paraphrase detection span 0.67 to 0.9 with no consensus, and the working value depends on the model, the domain, and sentence length.

Raising the threshold admits fewer edges: clusters get smaller and more of them, and more sentences end up in none. Lowering it merges components through chains, because clusters are connected components rather than cliques. That transitivity is why the clearing edges travel beside the clusters: two sentences can share a cluster without a direct edge, and the edge list is where you check.

Start around 0.85 with the reference model and calibrate on your own corpus.

## Provisioning models

Both models work the same way, and the constructor you reach for is what decides whether anything is fetched.

**Ask for nothing and you get the pinned model in the resolved place.** `Engine::new()`, `Engine::from_config(cfg)` and `Model2Vec::from_config(cfg)` resolve the directory through [`Config`](programming-model.md) and fetch the pinned model when it is absent. In Python that is `matra.Engine()` and `Model2Vec.potion_base_8m()` with no argument, which read the environment and no configuration file, and `Engine.from_config()` and `Model2Vec.from_config()`, which read one. This is the path to take unless you have a reason not to.

**Naming the directory changes where, not what.** `Engine::from_config(&cfg.with_model_dir(dir))`, Python's `matra.Engine()` under `MATRA_MODEL_DIR`, and `Model2Vec::potion_base_8m(dir)` fetch the same pinned artifacts into the models directory you name, each model in its own place there. Take them when the model belongs somewhere specific.

**Downloading is pinned, or it does not happen.** Either form fetches from URLs written in the source and loads nothing whose digest does not equal a constant written in the source next to them. Verification happens before the bytes are parsed, and the bytes that were verified are the bytes that get parsed, with no second read of the disk in between. A download that fails the digest is fetched once more into memory, and a second failed download raises; nothing that failed the digest is written at any point, so a failed attempt leaves nothing behind for a later call to pick up. Exactly one artifact set can arrive this way, which is what makes "matra downloads a model" a bounded claim rather than an open one.

**A provisioner never deletes what it did not write.** The embedding artifacts are named for their format (`model.safetensors`, `tokenizer.json`, `config.json`) rather than for one model, so a directory can already hold somebody else's. `Model2Vec::potion_base_8m(dir)` therefore downloads only into a directory holding none of the three. Finding all three with a digest that does not match, or finding only some of them, it raises and names the directory, having downloaded nothing and removed nothing. The UDPipe model file is named for its own release, so its provisioner has no such collision to guard against.

**Bringing your own.** `Model2Vec::from_dir(dir)` loads what you supply and never reaches the network, whatever the directory holds. That is the path for a model2vec artifact this build does not pin, from either language. The parsing model is a pin on every surface: no surface loads another UDPipe model file. Nothing verifies it, so the `model_hash` on a result is identity rather than proof: it tells you which artifacts produced a score, not that they are the ones you meant to fetch.

The pinned model is part of the contract in every case. A different model produces a different parse or a different vector space, and the conformance fixtures will fail, correctly. The [semantic clusters reference](../reference/semantic-clusters.md#the-reference-model) records the embedding digest the conformance suite pins.

The expensive thing is the model, not the call: [load an `Engine` once and reuse it](../architecture/design.md#the-model-is-the-expensive-thing).
