---
name: semantic
summary: Similarity clusters over the sentences of one document, the threshold you must choose and why it does not travel, how to compare whole documents, and how the embedding model is provisioned.
---

# Semantic clusters

Everything else matra returns is deterministic structure, checkable against the source bytes. This is not. Clusters depend on a model's representation of meaning, they cannot be verified against the text, and the library treats that difference structurally: clusters arrive as a standalone value from a separate call, never as a field on a document or a sentence, and they carry the identity of the model that produced them together with the threshold you chose.

There is no command line for this. It is a library and Python call. What the command line does expose is the configured defaults:

```console
$ matra config show
```

`models.embedding` names the model directory (`potion-base-8M` as shipped) and `semantic.threshold` carries the shipped starting point of `0.85`.

## What you get

Feed a document and an embedding model, get connected components of sentences whose pairwise cosine similarity cleared your threshold. The use it is built for is catching restatement: the same claim in different words, which lexical overlap cannot see.

```text
SemanticClusters
  model_hash   identity of the model whose vector space produced the scores
  threshold    the cutoff you supplied, narrowed to f32
  clusters     each: member sentence indices, plus the edges that cleared
```

`threshold` is an `f32`, the precision the similarity computation runs at, so a Python caller who passes `0.85` reads `0.8500000238418579` back, and the equality `result["threshold"] == 0.85` does not hold. Report the number the caller asked for, not the value the field hands back. Values exact in binary, such as `0.5` and `0.75`, round-trip unchanged, which is what makes the surprise intermittent.

| Type | Fields |
|---|---|
| `SemanticClusters` | `model_hash` (string), `threshold` (float), `clusters` (array) |
| `SemanticCluster` | `members` (sentence indices), `edges` |
| `SemanticEdge` | `a`, `b` (indices, `a` < `b`), `score` (cosine similarity) |

Members are positions in the sentence list you passed, so a result re-anchors against the document it came from. Clusters are ordered by smallest member index, and each cluster's members and edges are sorted, so the output is deterministic.

## Three things the shape means

**Co-membership is transitive, not pairwise.** Clusters are connected components. Sentence A and sentence C can share a cluster because both resemble B, without resembling each other. The above-threshold edges travel inside each cluster precisely so a consumer can see which pairs actually cleared the bar. A missing edge is no claim, not a low score. Never present co-membership as pairwise similarity.

**Singletons are excluded by construction.** A sentence with no above-threshold edge appears in no cluster, so "in no cluster" is a meaningful count rather than an artifact.

**The threshold is yours, and it does not travel.** The library knows no universal cutoff. Published values for paraphrase detection span 0.67 to 0.9 with no consensus, and the working value depends on the model, the domain, and text length. Raising it admits fewer edges: smaller clusters, more of them, more sentences in none. Lowering it merges components through chains. Start at 0.85 with the reference model **on sentences**, and calibrate on the corpus in front of you. That number is calibrated for sentences and is wrong at any other granularity: see "Comparing whole documents" below, where a near-duplicate pair of documents scores 0.74 and 0.85 would have called them unrelated.

## The model and how it arrives

The adapter loads static embedding models in the model2vec artifact format: three files in one directory, `model.safetensors`, `tokenizer.json` and `config.json`. The reference model is potion-base-8M, about 30 MB, 256 dimensions.

A static model is a lookup table, not a transformer: inference is a row gather, a mean, and a normalize. That costs roughly ten percent of a small transformer's benchmark quality and buys bit-identical vectors on every platform and in every language binding, which is what lets the conformance suite pin exact vectors rather than tolerances.

Provisioning rules, in the order they are checked:

1. **All three files present.** The SHA-256 over the three, concatenated in the order above, must equal the constant compiled into the library: `81c3592150873b1c5a8c4262850f795bff4fd568fbde80ac69889d087f16a0b4`. On a match it loads. On a mismatch it fails and names the directory, having downloaded nothing and deleted nothing.
2. **Some but not all three present.** It fails: a partial set carries no provenance. Nothing is downloaded over and nothing is removed.
3. **None of the three present.** The directory is this call's to fill. All three are fetched from URLs pinned in the source at an immutable revision, verified, and loaded. A mismatch over files this call downloaded removes them and fetches once more; a second mismatch removes them again and fails.

The rule underneath all three: a provisioner never deletes what it did not write. Those three filenames belong to the artifact format rather than to this one model, so the directory may already hold a model of yours.

Verification happens before anything is parsed, and the bytes that were verified are the bytes that get parsed, with no second read of the disk in between. One artifact set can arrive this way and no other.

Bringing your own model loads what you supply and never reaches the network, whatever the directory holds. For a hand-placed model the hash is identity rather than proof: it tells you which artifacts produced a score, not that they are the ones you meant to fetch.

## `model_hash`

Whatever produced the vectors, its identity travels with every result derived from them. That is the field that makes a score attributable: two runs with different `model_hash` values are not comparable, and a cluster reported without it is a number with no provenance. When you write results down, write the hash down with them.

For a caller-supplied embedder, the identity is whatever that embedder's `identity()` returns. It is read once, when the object is handed over, so scores cannot be reattributed part way through a call. Two embedders that can disagree must not return the same string.

## Bounds and failures

Capped at 2,000 sentences, with the `what` label `semantic_clusters`. An artifact download is capped at 64 MiB with the label `embedding_download`, bounded at 300 seconds end to end and 30 seconds on the connection.

Contract violations are `invalid_input`, which means the call site is wrong and not the text: vectors disagreeing on dimension, a vector containing a non-finite value, a non-finite threshold, or an embedder returning the wrong number of vectors. An empty slice returns empty clusters rather than failing.

## Reaching it

From Rust, `embed_and_cluster(document, embedder, threshold)` embeds a document's sentences and clusters them, and `extraction::semantic_clusters(embeddings, threshold, model_hash)` clusters vectors you already hold. The first needs no feature flag with your own `Embedder` implementation; only the shipped adapter sits behind the `model2vec` feature.

That feature is not in the default set and `cli` does not pull it in, so check before promising a user this works. A Rust caller needs `cargo add matra --features model2vec`, and a binary installed with `cargo install matra --features cli` reports `features: udpipe cli` with no embedding adapter compiled in. The published Python wheel is built with it, so from Python `Model2Vec` is always there. `--version` prints the feature list of the binary in front of you on its second line.

```console
$ matra --version
```

From Python, `Matra.semantic_clusters(text, threshold, model)` takes a `Model2Vec` or any object with `embed` and `identity`, and the module-level `semantic_clusters(embeddings, threshold, model_hash)` is the vectors-in twin. `Model2Vec.embed(texts)` is where the vectors for that twin come from: it returns one vector per text, in order, each of `model.dimensions` length. See `python` for the signatures.

## Comparing whole documents

**Every clustering call above is over the sentences of one document. There is no cross-document primitive, and matra has no per-document redundancy number at all.** Build the comparison out of the two pieces you already have: embed each document as one text with `Model2Vec.embed`, then cluster those vectors with the module-level `semantic_clusters`.

**The embedder caps every text at 512 tokens, so a whole-document vector is a vector of roughly the document's first 512 tokens.** Truncation happens twice, on bytes before tokenizing and on the token ids after, and nothing past the cap reaches the mean. Tokens are not words: over the prose pages of matra's own book, 512 tokens ran out between 220 and 350 words, so two long texts agreeing for as few as their first 220 words can already embed to byte-identical vectors. Documents with a shared boilerplate opening therefore report as near-duplicates on the opening alone, and paraphrases that diverge early are never compared on the rest. Say which part of the document a cross-document score covers, and chunk when the tail is the content, sizing chunks at 200 words or fewer rather than 512. The 2,000 cap also still applies, and on this route it counts documents rather than sentences.

Pass a threshold of `-1.0` and every pair emits an edge, because a cosine is never below it. That is the sanctioned way to read raw pairwise scores, and it is what calibration looks like: read the scores first, choose the cutoff second.

```python
from pathlib import Path

from matra import Model2Vec, semantic_clusters

model = Model2Vec.potion_base_8m()
docs = sorted(Path("corpus").glob("*.md"))
vectors = model.embed([p.read_text() for p in docs])

pairs = semantic_clusters(vectors, -1.0, model.model_hash)
for cluster in pairs["clusters"]:
    for edge in cluster["edges"]:
        print(f"{edge['score']:.4f}  {docs[edge['a']].name} <-> {docs[edge['b']].name}")
```

Three documents, two of them near-paraphrases and one unrelated:

```text
0.3563  index-design.md <-> oncall-rotation.md
0.3888  index-design.md <-> paging-policy.md
0.7426  oncall-rotation.md <-> paging-policy.md
```

The pair separates decisively, and the shipped 0.85 does not carry over: the same vectors at `0.85` yield zero clusters, so an agent that took the starting value on faith would have reported the near-duplicates as unrelated. Document vectors are means over far more tokens than sentence vectors, so the whole scale shifts: unrelated documents sit well above zero, and near-duplicates need not reach the sentence band. Report the raw edge scores and the `model_hash`, and say which granularity produced them.
