# Cluster sentences by meaning

Group the sentences of a document that say the same thing in different words, for example to audit LLM output for restatement, which lexical overlap cannot see. In Rust this needs the `model2vec` feature for the reference model; the Python package has it. [What the clustering threshold does](../explanation/semantic-clusters.md) explains what a cluster means, and [Semantic clusters](../reference/semantic-clusters.md) lists the value, the model and the bounds.

## What you get

A `Clusters` value: the groups of sentences whose pairwise cosine similarity cleared the threshold you pass, the pairs that cleared it, and a provenance recording the threshold and the identity of the model that scored them.

Start around 0.85 with the reference model on sentences, and calibrate on your own corpus: the threshold does not travel between models, domains or text lengths. [Comparing whole documents](#comparing-whole-documents) shows how to read the raw scores before you choose one.

## The model

Take the reference model, potion-base-8M. The first call downloads it (30.9 MB) and verifies it against a digest compiled into matra; every later call loads it from disk:

```rust,ignore
use matra::config::Config;
use matra::embed::model2vec::Model2Vec;

let model = Model2Vec::from_config(&Config::resolve()?)?;
```

```python
from matra import Model2Vec

model = Model2Vec.potion_base_8m()
```

It goes into a directory named for the configured embedding model (`models.embedding`, `potion-base-8M` as shipped) inside the configured model directory. To keep it somewhere else, name a models directory: `Model2Vec::potion_base_8m(dir)` in Rust, `Model2Vec.potion_base_8m(dir)` in Python. `dir` is the directory that holds every model, the one `MATRA_MODEL_DIR` names, not the model's own: the three files land in `dir/potion-base-8M/`, beside the parsing model in `dir`, so passing the directory `matra config show` reports as `model_dir` reuses a model that is already there. The call refuses a directory that already holds another model's files and names it; remove them, or name another directory, or point `MATRA_MODEL_DIR` or `models.embedding` somewhere else.

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
use matra::{Engine, Input};

let cfg = Config::resolve()?;
let engine = Engine::from_config(&cfg)?;
let model = Model2Vec::from_config(&cfg)?;

let doc = engine.analyze(Input::text(text, matra::domain::Format::PlainText))?;
let clusters = matra::clusters(&doc, &model, 0.85)?;

for c in &clusters.clusters {
    println!("restated {} times: sentences {:?}", c.members.len(), c.members);
}
```

With your own `Embedder` implementation in place of `Model2Vec`, `clusters` needs no feature at all; an embedder that returns vectors you already hold is how they get in.

## Python

```python
import matra

model = matra.Model2Vec.potion_base_8m()
engine = matra.Engine()

doc = engine.analyze(text, format="markdown")   # or plain_text; a str alone reads as plain text
result = matra.clusters(doc, model, threshold=0.85)
for cluster in result["clusters"]:
    print("restated:", cluster["members"])
```

Name the format the text is in. Read as plain text, a markdown document's heading markers (`#`, `##`) become sentences of their own, and identical short sentences embed identically, so they come back as one cluster of "restatements". Read as markdown, a heading is a heading and never a sentence. `engine.analyze(file=path)` detects the format from the file.

Already hold embeddings? Pass an object whose `embed` returns them, one per sentence, and whose `identity` names the model that made them, as in Rust. And `model.embed(texts)` returns the raw vectors when you want to do something else with them.

The threshold comes back in `result["provenance"]["parameters"]["threshold"]` exactly as you passed it, beside the embedder's identity. An edge's score is computed in the vectors' `f32` and widened, so compare scores at a few decimal places rather than for equality.

## The command line

```bash
matra clusters notes.md --threshold 0.85 --json
```

`matra clusters` builds the reference embedder from the configuration, the model `models.embedding` names in the model directory, and takes its threshold from `--threshold`, else `clusters.threshold` in the config file, `0.85` as shipped. `--json` writes the same `Clusters` value inside the envelope; the table lists each cluster's sentence indices and text. It needs a build with `model2vec`, which the Python package has; a `cargo install --features cli` build refuses the command and names the feature. The [CLI guide](cli.md#matra-clusters) has the flags.

## Comparing whole documents

`clusters` works over the sentences of one document, in every language. There is no cross-document primitive. Build one out of what you have: embed each document as a single text with `model.embed`, then compare the vectors yourself.

One bound decides what the answer means. `Model2Vec` caps every text at 512 tokens, pre-truncating on bytes and then truncating the token ids, so "embed each document as a single text" embeds roughly the first 512 tokens of it and nothing after. Tokens are not words, and how many words 512 tokens buys depends on what the file holds. Say which basis a figure is on. Swept over the 26 pages of this book long enough to reach it, with the call below, which embeds the raw file text, the cap ran out between 141 and 378 words. Diagrams, code fences and tables pull the low end down; the recipe below reads files off disk, so the low end is the one that applies to it. Two long texts that agree for as few as their first 141 words can already embed to byte-identical vectors. That cuts both ways: documents sharing a boilerplate opening score as near-duplicates on the opening alone, and two real paraphrases that diverge inside their first few hundred words never get compared on the part that matters.

If the tail carries the content, split each document into chunks and compare the chunks, and size the chunks from a token count rather than a word count. A word count is not a safe proxy on raw markup, and the gap is not small: sliding a window across the same pages, a window of 54 words landed inside an inline SVG block and had already filled the cap. Any word figure has to be measured against your own files, because getting it wrong does not raise. It returns a score computed on less text than you handed it.

Read every pairwise score before you choose a cutoff. That is how you calibrate:

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

Those three are pages of this book, run from a checkout of the repository, so the numbers below are yours to reproduce. Two of them cover the same ground for different surfaces, and the third is unrelated. It prints:

```text
0.8272  cli.md <-> rust.md
0.5800  cli.md <-> roadmap.md
0.6114  rust.md <-> roadmap.md
```

The separation is decisive, and the sentence-level starting point does not carry over: against a cutoff of `0.85` no pair clears, so the near-duplicate pair would have been reported as unrelated. This is what "the threshold does not travel" costs when it is taken on faith. Calibrate on scores you have read.

Two things carry over from `clusters`, and on this route they are yours to keep. Attribution: the vectors do not carry the model identity, so record `model.model_hash` beside every score you keep, as `clusters` records it in `parameters["embedder"]["sha256"]`. And a zero-magnitude vector, which is what an empty document embeds to, makes no claim: the `cosine` above returns 0.0 for it rather than dividing by zero.

