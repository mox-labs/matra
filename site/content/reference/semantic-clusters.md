# Semantic clusters

The value semantic clustering returns, the model it embeds with, and its bounds. [Cluster sentences by meaning](../guides/semantic-clusters.md) has the calls; [What the clustering threshold does](../explanation/semantic-clusters.md) explains what a cluster means.

## The value

`embed_and_cluster`, `extraction::semantic_clusters` and their Python forms return a `SemanticClusters`, a standalone value and never a field on `Document`:

```text
SemanticClusters
  model_hash   identity of the model whose vector space produced the scores
  threshold    the cutoff you supplied, narrowed to f32
  clusters     each: member sentence indices + the edges that cleared
```

| Type | Field | Holds |
|---|---|---|
| `SemanticClusters` | `model_hash` | The identity the embedder reports: for `Model2Vec`, the SHA-256 over its three artifacts |
| | `threshold` | The cosine cutoff the call was given, as an `f32` |
| | `clusters` | The connected components, ordered by smallest member index |
| `SemanticCluster` | `members` | Sentence indices in document order, sorted |
| | `edges` | Every pair in the cluster whose cosine cleared the threshold, sorted |
| `SemanticEdge` | `a`, `b` | The two sentence indices, `a < b` |
| | `score` | Their cosine similarity, as an `f32` |

A sentence with no pair above the threshold is in no cluster. Two members of one cluster need not share an edge: they can be joined through a chain of edges.

`threshold` is an `f32`, because that is the precision the whole similarity computation runs at. An `f64` you passed comes back as the nearest `f32`, so a Python caller who passed `0.85` reads `0.8500000238418579` out of the result, and the equality `result["threshold"] == 0.85` does not hold. Echo back the value you passed, or compare with a tolerance. Values that are exact in binary, `0.5` and `0.75` among them, round-trip unchanged, which is what makes the surprise intermittent.

## The model

The `Model2Vec` adapter, behind the `model2vec` feature, loads static embedding models in the model2vec artifact format: an embedding matrix (`model.safetensors`), a `tokenizer.json`, and a `config.json` in one directory. The clustering functions and the `Embedder` port are always compiled; with your own `Embedder` implementation, clustering needs no feature at all.

| Constructor | Rust | Python | Reaches the network |
|---|---|---|---|
| The configured directory | `Model2Vec::from_config(&cfg)` | `Model2Vec.potion_base_8m()` | when the pinned model is absent |
| A directory you name | `Model2Vec::potion_base_8m(dir)` | `Model2Vec.potion_base_8m(dir)` | when the pinned model is absent |
| Files you supply | `Model2Vec::from_dir(dir)` | `Model2Vec.from_dir(dir)` | never |

The configured directory is `models.embedding` (`potion-base-8M` as shipped) inside the configured model directory.

### The reference model

[potion-base-8M](https://huggingface.co/minishlab/potion-base-8M), about 30 MB, MIT per its model card. The SHA-256 over its three files, concatenated in the order above, is a constant compiled into the library:

```text
81c3592150873b1c5a8c4262850f795bff4fd568fbde80ac69889d087f16a0b4
```

It is the digest `spec/tests/semantic/reference-model.json` pins and the value `model_hash` reports once the model is loaded. For files loaded with `from_dir`, `model_hash` is identity rather than verification: it names the artifacts that produced a score, and says nothing about whether they are the ones you meant to fetch.

### Provisioning

- The three artifacts are verified as a set, in memory, before anything is parsed. A mismatch downloads once more without writing anything; a second mismatch raises with the directory as the call found it.
- Downloading happens only into a directory holding none of the three file names. If all three are there and the digest does not match, or only some of them are, the call raises and names the directory, having downloaded nothing and deleted nothing.
- Each artifact download is capped at 64 MiB.

### Inference

A static model is a lookup table: inference is a row gather, a mean, and a normalize, and the vectors are bit-identical on every platform and in every language binding. Every text is capped at 512 tokens, pre-truncated on bytes and then truncated on token ids, so a longer text is embedded from its opening only. An empty text embeds to the zero vector.

## Bounds and failure

| Condition | Rust | Python |
|---|---|---|
| More than 2,000 sentences, checked before the embedding pass runs | `InputTooLarge`, `what` is `"semantic_clusters"` | `ValueError` |
| Vectors that disagree on dimension, a non-finite value, a non-finite threshold | `InvalidInput` | `ValueError` |
| An embedder returning the wrong number of vectors | `InvalidInput` | `ValueError` |

The 2,000 cap counts whatever the vectors stand for, so clustering one vector per document caps the corpus at 2,000 documents. The contract violations mean the call site or the embedder is wrong, never the text. A zero-magnitude vector has no defined cosine with anything, so it gets no edge at any threshold and no cluster. [Errors](errors.md) has the full table, the provisioning failures included.
