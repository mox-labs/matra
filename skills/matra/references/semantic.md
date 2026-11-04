---
name: semantic
summary: Similarity clusters over the sentences of one document, the threshold you must choose and why it does not travel, how to compare whole documents, and how the embedding model is provisioned.
---

# Semantic clusters

Everything else matra returns is deterministic structure, checkable against the source bytes. This is not. Clusters depend on a model's representation of meaning, they cannot be verified against the text, and the library treats that difference structurally: clusters arrive as a standalone value from a separate call, never as a field on a document or a sentence, and their provenance carries the identity of the model that produced them together with the threshold you chose.

`matra clusters FILE` runs it from the command line, with the reference embedder that `models.embedding` names (`potion-base-8M` as shipped, downloaded into the model directory on first use) and the threshold `--threshold` gives, or `clusters.threshold` from the config, `0.85` as shipped. `--json` writes the `Clusters` below in the envelope. It needs a build with the `model2vec` feature (below); without it the command refuses before it reads anything, naming the feature.

<!-- needs: embedder -->

```console
$ matra clusters notes.md --threshold 0.8 --json
```

The library and Python take the threshold as an argument and read no configuration. `Model2Vec.potion_base_8m()` with no argument provisions into the same directory the command line uses.

## What you get

Feed a document and an embedding model, get connected components of sentences whose pairwise cosine similarity cleared your threshold. The use it is built for is catching restatement: the same claim in different words, which lexical overlap cannot see.

```text
Clusters
  provenance   the document's, plus method "cosine" and parameters:
                 threshold   the cutoff you supplied
                 embedder    identity of the model whose vector space produced the scores
  clusters     each: member sentence indices
  edges        every pair that cleared the threshold, with its score
```

The threshold comes back exactly as you passed it. A score is computed in the vectors' own `f32` and widened, so it carries more digits than the model resolved: `0.92387664` reads `0.9238766431808472`. Compare scores at four decimal places, not by equality with a number you typed.

| Type | Fields |
|---|---|
| `Clusters` | `provenance` (with `method` and `parameters`), `clusters` (array), `edges` (array) |
| `Cluster` | `members` (sentence indices, ascending, at least two) |
| `SimilarityEdge` | `a`, `b` (indices, `a` < `b`), `score` (cosine similarity) |

Members are positions among the document's sentences, so a result re-anchors against the document it came from. Clusters are ordered by smallest member index and edges by `a`, then `b`, so the output is deterministic. Both ends of an edge are members of one cluster.

## Three things the shape means

**Co-membership is transitive, not pairwise.** Clusters are connected components. Sentence A and sentence C can share a cluster because both resemble B, without resembling each other. The above-threshold edges travel beside the clusters precisely so a consumer can see which pairs actually cleared the bar. A missing edge is no claim, not a low score. Never present co-membership as pairwise similarity.

**Singletons are excluded by construction.** A sentence with no above-threshold edge appears in no cluster, so "in no cluster" is a meaningful count rather than an artifact.

**The threshold is yours, and it does not travel.** The library knows no universal cutoff. Published values for paraphrase detection span 0.67 to 0.9 with no consensus, and the working value depends on the model, the domain, and text length. Raising it admits fewer edges: smaller clusters, more of them, more sentences in none. Lowering it merges components through chains. Start at 0.85 with the reference model **on sentences**, and calibrate on the corpus in front of you. That number is calibrated for sentences and is wrong at any other granularity: see "Comparing whole documents" below, where a near-duplicate pair of documents scores below 0.85 and that cutoff would have called them unrelated.

## The model and how it arrives

The adapter loads static embedding models in the model2vec artifact format: three files in one directory, `model.safetensors`, `tokenizer.json` and `config.json`. The reference model is potion-base-8M, about 30 MB, 256 dimensions. Its Hugging Face model card (minishlab/potion-base-8M) gives its license as MIT, separate from the parsing model's CC BY-NC-SA 4.0.

A static model is a lookup table, not a transformer: inference is a row gather, a mean, and a normalize. That costs roughly ten percent of a small transformer's benchmark quality and buys bit-identical vectors on every platform and in every language binding, which is what lets the conformance suite pin exact vectors rather than tolerances.

Provisioning rules, in the order they are checked:

1. **All three files present.** The SHA-256 over the three, concatenated in the order above, must equal the constant compiled into the library: `81c3592150873b1c5a8c4262850f795bff4fd568fbde80ac69889d087f16a0b4`. On a match it loads. On a mismatch it fails and names the directory, having downloaded nothing and deleted nothing.
2. **Some but not all three present.** It fails: a partial set carries no provenance. Nothing is downloaded over and nothing is removed.
3. **None of the three present.** The directory is this call's to fill. All three are fetched from URLs pinned in the source at an immutable revision, verified, and loaded. The set is verified in memory before anything is written, so a mismatch fetches once more and a second mismatch fails with the directory untouched.

The rule underneath all three: a provisioner never deletes what it did not write. Those three filenames belong to the artifact format rather than to this one model, so the directory may already hold a model of yours.

Verification happens before anything is parsed, and the bytes that were verified are the bytes that get parsed, with no second read of the disk in between. One artifact set can arrive this way and no other.

Bringing your own model loads what you supply and never reaches the network, whatever the directory holds. For a hand-placed model the hash is identity rather than proof: it tells you which artifacts produced a score, not that they are the ones you meant to fetch.

## `identity`

Whatever produced the vectors, its identity travels with every result derived from them, in `provenance.parameters.embedder`. That is the field that makes a score attributable: two runs with different identities are not comparable, and a cluster reported without it is a number with no provenance. When you write results down, write the hash down with them.

For a caller-supplied embedder, the identity is whatever that embedder's `identity()` returns. It is read once, when the object is handed over, so scores cannot be reattributed part way through a call. Two embedders that can disagree must not return the same string.

## Bounds and failures

Capped at 2,000 sentences, with the `what` label `clusters`. An artifact download is capped at 64 MiB with the label `embedding_download`, bounded at 300 seconds end to end and 30 seconds on the connection.

Contract violations are `invalid_input`, which means the call site is wrong and not the text: vectors disagreeing on dimension, a vector containing a non-finite value, a non-finite threshold, or an embedder returning the wrong number of vectors. An empty slice returns empty clusters rather than failing.

## Reaching it

From Rust, `matra::clusters(&document, &embedder, threshold)` embeds a document's sentences with the embedder you pass and clusters them; matra never builds one inside the call. It needs no feature flag with your own `Embedder` implementation, which is also how vectors you already hold get in: return them from `embed`. Only the shipped adapter sits behind the `model2vec` feature.

That feature is not in the default set and `cli` does not pull it in, so check before promising a user this works. A Rust caller needs `cargo add matra --features model2vec`, and a binary installed with `cargo install matra --features cli` reports `features: udpipe cli` with no embedding adapter compiled in, so its `matra clusters` refuses; `--features cli,model2vec` builds one that runs it. The published Python wheel is built with it, so from Python `Model2Vec` is always there. `--version` prints the feature list of the binary in front of you on its second line.

```console
$ matra --version
```

From Python, `matra.clusters(doc, embedder, threshold=0.85)` takes the `Document` an engine returned and a `Model2Vec` or any object with `embed` and `identity`, and returns the `Clusters` dict. Vectors you already hold get in through an object whose `embed` returns them. `Model2Vec.embed(texts)` returns raw vectors, one per text, in order, each of `model.dimensions` length. See `python` for the signatures.

## Comparing whole documents

**Every clustering call above is over the sentences of one document. There is no cross-document primitive, and matra has no per-document redundancy number at all.** Build the comparison yourself: embed each document as one text with `Model2Vec.embed`, then compute the cosine of each pair.

**The embedder caps every text at 512 tokens, so a whole-document vector is a vector of roughly the document's first 512 tokens.** Truncation happens twice, on bytes before tokenizing and on the token ids after, and nothing past the cap reaches the mean. Tokens are not words, and the ratio depends on what the file holds, so name the basis with any figure. Swept over the 25 pages of matra's own book long enough to reach it, by the recipe below, which embeds the raw file text, 512 tokens ran out between 141 and 378 words. Diagrams, code fences and tables pull the low end down, and a route that reads files off disk gets the low end. Two long texts agreeing for as few as their first 141 words can already embed to byte-identical vectors, so documents with a shared boilerplate opening report as near-duplicates on the opening alone, and paraphrases that diverge early are never compared on the rest. Say which part of the document a cross-document score covers. When the tail is the content, chunk, and size chunks from a token count: a word count is not a safe proxy on raw markup, where a 54-word window inside an inline SVG block already filled the cap.

Read every pairwise score before choosing a cutoff. That is what calibration looks like: read the scores first, choose the cutoff second.

```python
import math
from itertools import combinations
from pathlib import Path

from matra import Model2Vec

model = Model2Vec.potion_base_8m()
docs = [
    Path("site/content/guides/cli.md"),
    Path("site/content/guides/rust.md"),
    Path("site/content/roadmap.md"),
]
vectors = model.embed([p.read_text() for p in docs])


def cosine(a: list[float], b: list[float]) -> float:
    norm = math.hypot(*a) * math.hypot(*b)
    return sum(x * y for x, y in zip(a, b)) / norm if norm else 0.0


for a, b in combinations(range(len(docs)), 2):
    print(f"{cosine(vectors[a], vectors[b]):.4f}  {docs[a].name} <-> {docs[b].name}")
```

Three pages of matra's own book, run from a checkout: two covering the same ground for different surfaces, and one unrelated. Reproducible, unlike a corpus nobody else has.

```text
0.8181  cli.md <-> rust.md
0.5658  cli.md <-> roadmap.md
0.6138  rust.md <-> roadmap.md
```

The pair separates decisively, and the shipped 0.85 does not carry over: against `0.85` no pair clears, so an agent that took the starting value on faith would have reported the near-duplicates as unrelated. Document vectors are means over far more tokens than sentence vectors, so the whole scale shifts: unrelated documents sit well above zero, and near-duplicates need not reach the sentence band. Report the raw scores and the embedder's identity (`model.model_hash`), and say which granularity produced them.
