# Semantic clusters

Everything else matra returns is deterministic structure, checkable against the source bytes. This page's output is not: semantic clusters come from a model's opinion about meaning, they cannot be verified against the text, and matra treats that difference structurally. Clusters arrive as a standalone `SemanticClusters` value from a separate call, never as a field on `Document`, and they carry the identity of the model that produced them plus the threshold you chose. That is the deal on this page; the rest is mechanics.

## What you get

Feed a document and an embedding model, get connected components of sentences whose pairwise cosine similarity cleared your threshold. The intended consumer pattern is auditing LLM output for restatement: the model catches paraphrase (same claim, different words) that lexical overlap cannot see.

```text
SemanticClusters
  model_hash   identity of the model whose geometry produced the scores
  threshold    the cutoff you supplied
  clusters     each: member sentence indices + the edges that cleared
```

Three things the shape means, stated once here and again in the type docs:

- **Co-membership is transitive, not pairwise.** Clusters are connected components, so sentence A and sentence C can share a cluster because both resemble B, without resembling each other. The edges travel in the result precisely so you can see which pairs actually cleared the bar. A missing edge is no claim, not a low score.
- **Singletons are excluded by construction.** A sentence with no above-threshold edge appears in no cluster, so "not in any cluster" is a meaningful count.
- **The threshold is yours.** Published cutoffs for paraphrase detection span 0.67 to 0.9 with no consensus; the working value depends on the model, the domain, and the text length. Start around 0.85 with the reference model and calibrate on your own corpus.

## The model

The adapter loads static embedding models in the model2vec artifact format: an embedding matrix (`model.safetensors`), a `tokenizer.json`, and a `config.json` in one directory. matra never downloads models; you supply the files. The reference model is [potion-base-8M](https://huggingface.co/minishlab/potion-base-8M) (about 30 MB):

```console
$ mkdir -p ~/.matra/models/potion-base-8M && cd ~/.matra/models/potion-base-8M
$ for f in model.safetensors tokenizer.json config.json; do
    curl -sSfLO "https://huggingface.co/minishlab/potion-base-8M/resolve/main/$f"
  done
```

A static model is a lookup table, not a transformer: inference is a row gather, a mean, and a normalize. That costs roughly ten percent of a small transformer's benchmark quality and buys bit-identical vectors on every platform and in every crust, which is what lets the conformance suite pin exact vectors rather than tolerances. The adapter hashes all three files on load, and that digest is the `model_hash` in every result.

## Rust

```rust,ignore
use matra::embed::model2vec::Model2Vec;
use matra::{embed_and_cluster, Engine, Ingest, standard_decomposers};
use matra::nlp::udpipe::Udpipe;

let nlp = Udpipe::english("/tmp/matra-models")?;
let engine = Engine::new(Box::new(nlp), standard_decomposers());
let model = Model2Vec::from_dir("~/.matra/models/potion-base-8M")?;

let raw = matra::domain::RawDocument::new(text, None, matra::domain::Format::PlainText);
let doc = engine.annotate(&raw)?;
let clusters = embed_and_cluster(&doc, &model, 0.85)?;

for c in &clusters.clusters {
    println!("restated {} times: sentences {:?}", c.members.len(), c.members);
}
```

`embed_and_cluster` is behind the `model2vec` feature only through its adapter; with your own `Embedder` implementation it needs no feature at all.

## Python

```python
from matra import Matra, Model2Vec, semantic_clusters
from pathlib import Path

model = Model2Vec.from_dir(str(Path.home() / ".matra" / "models" / "potion-base-8M"))
v = Matra.english(str(Path.home() / ".matra" / "models"))

result = v.semantic_clusters(text, 0.85, model)
for cluster in result["clusters"]:
    print("restated:", cluster["members"])
```

Already hold embeddings? The module-level function clusters raw vectors: `semantic_clusters(vectors, 0.85, model.model_hash)`. And `model.embed(texts)` returns the raw vectors when you want to do something else with them.

## Bounds and failure

The count cap is 2,000 sentences (the similarity matrix is quadratic), checked before the embedding pass runs, and it raises with the `"semantic_clusters"` gate label. Contract violations (vectors disagreeing on dimension, non-finite values, a non-finite threshold) raise `InvalidInput` in Rust and `ValueError` in Python; they mean the call site is wrong, never the text. A zero-magnitude vector (an empty sentence embeds to zero) has no defined cosine with anything, so it gets no edges and no cluster: no claim rather than a fabricated score. [Errors](../reference/errors.md) has the full table.
