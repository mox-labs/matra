# Cluster sentences by meaning

Group the sentences of a document that say the same thing in different words, for example to audit LLM output for restatement, which lexical overlap cannot see. In Rust this needs the `model2vec` feature for the reference model; the Python package has it. [What the clustering threshold does](../explanation/semantic-clusters.md) explains what a cluster means, and [Semantic clusters](../reference/semantic-clusters.md) lists the value, the model and the bounds.

## What you get

A `SemanticClusters` value: the groups of sentences whose pairwise cosine similarity cleared the threshold you pass, each with the pairs that cleared it, plus the threshold and the identity of the model that scored them.

Start around 0.85 with the reference model on sentences, and calibrate on your own corpus: the threshold does not travel between models, domains or text lengths. [Comparing whole documents](#comparing-whole-documents) shows how to read the raw scores before you choose one.

## The model

Take the reference model, potion-base-8M. The first call downloads it (about 30 MB) and verifies it against a digest compiled into matra; every later call loads it from disk:

```rust,ignore
use matra::config::Config;
use matra::embed::model2vec::Model2Vec;

let model = Model2Vec::from_config(&Config::resolve()?)?;
```

```python
from matra import Model2Vec

model = Model2Vec.potion_base_8m()
```

It goes into a directory named for the configured embedding model (`models.embedding`, `potion-base-8M` as shipped) inside the configured model directory. To keep it somewhere else, name the directory: `Model2Vec::potion_base_8m(dir)` in Rust, `Model2Vec.potion_base_8m(dir)` in Python. The call refuses a directory that already holds another model's files and names it; remove them, or name another directory, or point `MATRA_MODEL_DIR` or `models.embedding` somewhere else.

To use files you already have, or a different model2vec artifact, put the three files in one directory and load it with `Model2Vec::from_dir(dir)`, which never reaches the network:

```console
$ mkdir -p ~/models/potion-base-8M && cd ~/models/potion-base-8M
$ for f in model.safetensors tokenizer.json config.json; do
    curl -sSfLO "https://huggingface.co/minishlab/potion-base-8M/resolve/main/$f"
  done
```

Nothing verifies files you load this way. To check that you have the reference model, compare `model.model_hash` with the digest in [the reference](../reference/semantic-clusters.md#the-reference-model).

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

With your own `Embedder` implementation in place of `Model2Vec`, `embed_and_cluster` needs no feature at all.

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

When you report the threshold, report the number you passed: `result["threshold"]` comes back as an `f32`, so `0.85` reads as `0.8500000238418579`.

## Comparing whole documents

`Matra.semantic_clusters` and `embed_and_cluster` both work over the sentences of one document. There is no cross-document primitive. Build one out of the two pieces above: embed each document as a single text, then cluster the resulting vectors.

One bound decides what the answer means. `Model2Vec` caps every text at 512 tokens, pre-truncating on bytes and then truncating the token ids, so "embed each document as a single text" embeds roughly the first 512 tokens of it and nothing after. Tokens are not words, and how many words 512 tokens buys depends on what the file holds. Say which basis a figure is on. Swept over the 25 pages of this book long enough to reach it, with the call below, which embeds the raw file text, the cap ran out between 141 and 377 words. Diagrams, code fences and tables pull the low end down; the recipe below reads files off disk, so the low end is the one that applies to it. Two long texts that agree for as few as their first 141 words can already embed to byte-identical vectors. That cuts both ways: documents sharing a boilerplate opening score as near-duplicates on the opening alone, and two real paraphrases that diverge inside their first few hundred words never get compared on the part that matters.

If the tail carries the content, split each document into chunks and compare the chunks, and size the chunks from a token count rather than a word count. A word count is not a safe proxy on raw markup, and the gap is not small: sliding a window across the same pages, a window of 54 words landed inside an inline SVG block and had already filled the cap. Any word figure has to be measured against your own files, because getting it wrong does not raise. It returns a score computed on less text than you handed it.

Pass a threshold of `-1.0` and every pair emits an edge, because a cosine is never below it. That turns the call into a way of reading the raw pairwise scores off `edges`, which is how you calibrate before choosing a real cutoff:

```python
from pathlib import Path

from matra import Model2Vec, semantic_clusters

model = Model2Vec.potion_base_8m()
docs = [
    Path("site/content/guides/cli.md"),
    Path("site/content/guides/rust.md"),
    Path("site/content/roadmap.md"),
]
vectors = model.embed([p.read_text() for p in docs])

pairs = semantic_clusters(vectors, -1.0, model.model_hash)
for cluster in pairs["clusters"]:
    for edge in cluster["edges"]:
        print(f"{edge['score']:.4f}  {docs[edge['a']].name} <-> {docs[edge['b']].name}")
```

Those three are pages of this book, run from a checkout of the repository, so the numbers below are yours to reproduce. Two of them cover the same ground for different surfaces, and the third is unrelated. It prints:

```text
0.8399  cli.md <-> rust.md
0.5857  cli.md <-> roadmap.md
0.6080  rust.md <-> roadmap.md
```

The separation is decisive, and the sentence-level starting point does not carry over: the same vectors at `0.85` produce zero clusters, so the near-duplicate pair would have been reported as unrelated. This is what "the threshold does not travel" costs when it is taken on faith. Calibrate on scores you have read.

Two things this route does not change. Attribution still travels: the vectors do not carry the model identity, you hand `model.model_hash` to `semantic_clusters` and it comes back on the result, so a cross-document score is as attributable as a sentence one. And a zero-magnitude vector, which is what an empty document embeds to, still gets no edge at any threshold.


On this route the 2,000-sentence cap counts documents, so one call compares at most 2,000 of them; [Bounds and failure](../reference/semantic-clusters.md#bounds-and-failure) has the cap and the errors.
