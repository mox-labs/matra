# Semantic clusters

Everything else matra returns is deterministic structure, checkable against the source bytes. This page's output is not: semantic clusters depend on a model's representation of meaning, they cannot be verified against the text, and matra treats that difference structurally. Clusters arrive as a standalone `SemanticClusters` value from a separate call, never as a field on `Document`, and they carry the identity of the model that produced them plus the threshold you chose.

## What you get

Feed a document and an embedding model, get connected components of sentences whose pairwise cosine similarity cleared your threshold. The intended use is auditing LLM output for restatement: the model catches paraphrase (same claim, different words) that lexical overlap cannot see.

```text
SemanticClusters
  model_hash   identity of the model whose vector space produced the scores
  threshold    the cutoff you supplied, narrowed to f32
  clusters     each: member sentence indices + the edges that cleared
```

`threshold` is an `f32`, because that is the precision the whole similarity computation runs at. An `f64` you passed comes back as the nearest `f32`, so a Python caller who passed `0.85` reads `0.8500000238418579` out of the result, and the equality `result["threshold"] == 0.85` does not hold. Echo back the value you passed, or compare with a tolerance. Values that are exact in binary, `0.5` and `0.75` among them, round-trip unchanged, which is what makes the surprise intermittent.

Three things the shape means, stated once here and again in the type docs:

- **Co-membership is transitive, not pairwise.** Clusters are connected components, so sentence A and sentence C can share a cluster because both resemble B, without resembling each other. The edges travel in the result precisely so you can see which pairs actually cleared the bar. A missing edge is no claim, not a low score.
- **Singletons are always excluded.** A sentence with no above-threshold edge appears in no cluster, so "not in any cluster" is a meaningful count.
- **The threshold is yours, and it does not travel.** Published cutoffs for paraphrase detection span 0.67 to 0.9 with no consensus; the working value depends on the model, the domain, and the text length. Start around 0.85 with the reference model on sentences, and calibrate on your own corpus. A cutoff calibrated on sentences is not the cutoff for whole documents: a document vector is the mean over far more tokens, so unrelated documents sit well above zero and near-duplicates need not reach the sentence band. Read the raw scores at the granularity you are clustering before you pick a number.

## The model

The adapter loads static embedding models in the model2vec artifact format: an embedding matrix (`model.safetensors`), a `tokenizer.json`, and a `config.json` in one directory. The reference model is [potion-base-8M](https://huggingface.co/minishlab/potion-base-8M), about 30 MB, and you do not have to fetch it yourself:

```rust,ignore
use matra::config::Config;
use matra::embed::model2vec::Model2Vec;

let model = Model2Vec::from_config(&Config::resolve()?)?;
```

```python
from matra import Model2Vec

model = Model2Vec.potion_base_8m()
```

On the first call the three artifacts are downloaded into the configured model directory. On every later call they load from there. Name a directory instead of taking the configured one with `Model2Vec::potion_base_8m(dir)` or `Model2Vec.potion_base_8m(dir)`.

What arrives is one specific artifact set and nothing else. The SHA-256 over all three files, concatenated in the order above, must equal a constant compiled into the library: `81c3592150873b1c5a8c4262850f795bff4fd568fbde80ac69889d087f16a0b4`, the same digest `spec/tests/semantic/reference-model.json` pins and the same value `model_hash` reports once the model is loaded. Verification happens before anything is parsed. A mismatch over files the call downloaded removes them and downloads once more; a second mismatch removes them again and raises, so a partly-trusted model is never loaded and nothing is left behind for a later call to find.

Files it did not download are never files it removes. Those three names belong to the artifact format rather than to this one model, so the directory may already hold a model of yours. Downloading happens only into a directory holding none of the three. If all three are there and the digest does not match, or if only some of them are there, the call raises and names the directory, having downloaded nothing and deleted nothing: load your own files with `Model2Vec::from_dir`, remove them to provision the pinned model in their place, or point `MATRA_MODEL_DIR` or the configured embedding model name somewhere else.

Already have the files, or using a different model2vec artifact? `Model2Vec::from_dir(dir)` loads what is there and never reaches the network, whatever the directory holds:

```console
$ mkdir -p ~/models/potion-base-8M && cd ~/models/potion-base-8M
$ for f in model.safetensors tokenizer.json config.json; do
    curl -sSfLO "https://huggingface.co/minishlab/potion-base-8M/resolve/main/$f"
  done
```

For a hand-placed model the hash is identity rather than verification: it tells you which artifacts produced a score, not that they are the ones you meant to fetch. Compare it against the digest above if you care which you got.

A static model is a lookup table, not a transformer: inference is a row gather, a mean, and a normalize. That costs roughly ten percent of a small transformer's benchmark quality and buys bit-identical vectors on every platform and in every language binding, which is what lets the conformance suite pin exact vectors rather than tolerances.

## Rust

```rust,ignore
use matra::config::Config;
use matra::embed::model2vec::Model2Vec;
use matra::{embed_and_cluster, Engine};

let cfg = Config::resolve()?;
let engine = Engine::from_config(&cfg)?;
let model = Model2Vec::from_config(&cfg)?;

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
from matra import Matra, Model2Vec

model = Model2Vec.potion_base_8m()
v = Matra.english()

result = v.semantic_clusters(text, 0.85, model)
for cluster in result["clusters"]:
    print("restated:", cluster["members"])
```

Already hold embeddings? The module-level function clusters raw vectors: `semantic_clusters(vectors, 0.85, model.model_hash)`. And `model.embed(texts)` returns the raw vectors when you want to do something else with them.

## Comparing whole documents

`Matra.semantic_clusters` and `embed_and_cluster` both work over the sentences of one document. There is no cross-document primitive. Build one out of the two pieces above: embed each document as a single text, then cluster the resulting vectors.

One bound decides what the answer means. `Model2Vec` caps every text at 512 tokens, pre-truncating on bytes and then truncating the token ids, so "embed each document as a single text" embeds roughly the first 512 tokens of it and nothing after. Tokens are not words, and how many words 512 tokens buys depends on what the file holds. Say which basis a figure is on. Swept over the 27 markdown pages of this book with the call below, which embeds the raw file text, the cap ran out between 141 and 368 words. Swept over the same pages with the markup stripped first, so only prose reached the embedder, it ran out between 231 and 351. Diagrams, code fences and tables pull the low end down; the recipe below reads files off disk, so the low end is the one that applies to it. Two long texts that agree for as few as their first 141 words can already embed to byte-identical vectors. That cuts both ways: documents sharing a boilerplate opening score as near-duplicates on the opening alone, and two real paraphrases that diverge inside their first few hundred words never get compared on the part that matters.

If the tail carries the content, split each document into chunks and compare the chunks, and size the chunks from a token count rather than a word count. A word count is not a safe proxy on raw markup, and the gap is not small: sliding a window across the same pages, a window of 54 words landed inside an inline SVG block and had already filled the cap. Any word figure has to be measured against your own files, because getting it wrong does not raise. It returns a score computed on less text than you handed it.

Pass a threshold of `-1.0` and every pair emits an edge, because a cosine is never below it. That turns the call into a way of reading the raw pairwise scores off `edges`, which is how you calibrate before choosing a real cutoff:

```python
from pathlib import Path

from matra import Model2Vec, semantic_clusters

model = Model2Vec.potion_base_8m()
docs = [
    Path("book/src/guides/cli.md"),
    Path("book/src/guides/rust.md"),
    Path("book/src/roadmap.md"),
]
vectors = model.embed([p.read_text() for p in docs])

pairs = semantic_clusters(vectors, -1.0, model.model_hash)
for cluster in pairs["clusters"]:
    for edge in cluster["edges"]:
        print(f"{edge['score']:.4f}  {docs[edge['a']].name} <-> {docs[edge['b']].name}")
```

Those three are pages of this book, run from a checkout of the repository, so the numbers below are yours to reproduce. Two of them cover the same ground for different surfaces, and the third is unrelated. It prints:

```text
0.8334  cli.md <-> rust.md
0.5917  cli.md <-> roadmap.md
0.6035  rust.md <-> roadmap.md
```

The separation is decisive, and the sentence-level starting point does not carry over: the same vectors at `0.85` produce zero clusters, so the near-duplicate pair would have been reported as unrelated. This is what "the threshold does not travel" costs when it is taken on faith. Calibrate on scores you have read.

Two things this route does not change. Attribution still travels: the vectors do not carry the model identity, you hand `model.model_hash` to `semantic_clusters` and it comes back on the result, so a cross-document score is as attributable as a sentence one. And a zero-magnitude vector, which is what an empty document embeds to, still gets no edge at any threshold.

## Bounds and failure

The count cap is 2,000 sentences (the similarity matrix is quadratic), checked before the embedding pass runs, and it raises with the `"semantic_clusters"` gate label. It counts whatever the vectors stand for, so on the cross-document route above it caps the corpus at 2,000 documents. Contract violations (vectors disagreeing on dimension, non-finite values, a non-finite threshold) raise `InvalidInput` in Rust and `ValueError` in Python; they mean the call site is wrong, never the text. A zero-magnitude vector (an empty sentence embeds to zero) has no defined cosine with anything, so it gets no edges and no cluster: no claim rather than a fabricated score. [Errors](../reference/errors.md) has the full table.
